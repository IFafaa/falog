//! The assistant dock: conversations with an AI agent that manage tasks, typed or dictated.
//!
//! [`agent`] starts and talks to the agents (Claude Code by default), [`thread`] holds one
//! conversation, [`history`] saves them, [`voice`] records and transcribes speech and [`panel`]
//! draws the dock. [`Assistant`] owns the threads and the dictation state and ties them together.

pub mod agent;
pub mod agent_settings;
pub mod agents;
pub mod history;
pub mod panel;
pub mod slash;
pub mod thread;
pub mod voice;

use agent::registry::{self, Agent, AgentConfig, AgentId, Protocol};
use agent::{OptionKind, claude_code};
use agents::SavedAgents;
use eframe::egui;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use thread::Launcher;
pub use thread::{Item, Settings, Thread, ThreadId, ToolCall};
use voice::{Download, Recorder, Transcriber, VoiceLanguage};

/// How often the live preview is refreshed while dictating, and when the first one starts.
const PARTIAL_EVERY: Duration = Duration::from_millis(1000);
const FIRST_PARTIAL_AFTER: Duration = Duration::from_millis(1500);
/// Pending thread changes are written at most this often.
const SAVE_EVERY: Duration = Duration::from_secs(2);

/// User-facing assistant options (stored in the app preferences).
#[derive(Clone, Copy, Debug)]
pub struct AssistantOptions {
    pub language: VoiceLanguage,
    pub send_after_dictation: bool,
    /// The dock fills the window.
    pub zoomed: bool,
    /// Run speech recognition on the GPU (builds with the `gpu` feature).
    pub voice_gpu: bool,
}

#[derive(Debug, Default)]
pub enum VoiceState {
    #[default]
    Idle,
    Recording(Recorder),
    Transcribing,
    /// The user asked to dictate but the Whisper model is not downloaded yet.
    NeedsModel,
    Downloading(Download),
}

#[derive(Debug)]
pub struct Assistant {
    threads: Vec<Thread>,
    active: ThreadId,
    next_id: u64,
    /// The dock shows the thread list instead of the active thread.
    pub show_history: bool,
    pub voice: VoiceState,
    /// Latest partial transcript while dictating, shown greyed out in the composer.
    pub live_text: String,
    pub focus_composer: bool,
    /// The dictation was stopped with Enter: its transcript goes to the composer even when
    /// `send_after_dictation` is on, and the next Enter sends it.
    dictation_to_draft: bool,
    partial_pending: bool,
    last_partial: Instant,
    launcher: Launcher,
    /// Settings for new threads: the last choice made in any thread.
    defaults: Settings,
    history_path: PathBuf,
    saved_agents: SavedAgents,
    agents_path: PathBuf,
    agents_unsaved: bool,
    transcriber: Option<Transcriber>,
    unsaved: bool,
    last_save: Instant,
}

impl Assistant {
    /// Loads the saved threads and opens a fresh one.
    pub fn new(database: Option<&Path>) -> Self {
        let launcher = Launcher {
            database: database.map(Path::to_path_buf),
        };
        let history_path = launcher.workdir().join("threads.json");
        Self::with_history(launcher, history_path)
    }

    fn with_history(launcher: Launcher, history_path: PathBuf) -> Self {
        let threads: Vec<Thread> = history::load(&history_path)
            .into_iter()
            .map(Thread::from_record)
            .collect();
        let next_id = threads.iter().map(|t| t.id.0 + 1).max().unwrap_or(1);
        let agents_path = history_path.with_file_name("agents.json");
        let saved_agents = SavedAgents::load(&agents_path);
        let defaults = saved_agents.last.clone().unwrap_or_else(|| {
            threads
                .iter()
                .max_by_key(|t| t.updated_at)
                .map(|t| t.settings.clone())
                .unwrap_or_default()
        });
        let mut assistant = Self {
            threads,
            active: ThreadId(0),
            next_id,
            show_history: false,
            voice: VoiceState::Idle,
            live_text: String::new(),
            dictation_to_draft: false,
            focus_composer: false,
            partial_pending: false,
            last_partial: Instant::now(),
            launcher,
            defaults,
            history_path,
            saved_agents,
            agents_path,
            agents_unsaved: false,
            transcriber: None,
            unsaved: false,
            last_save: Instant::now(),
        };
        let mut threads = std::mem::take(&mut assistant.threads);
        for thread in &mut threads {
            assistant.prepare(thread);
        }
        assistant.threads = threads;
        assistant.start_new_thread();
        assistant
    }

    // ---- agents ----------------------------------------------------------------------------

    /// Every agent a thread can talk to: the presets, then the user's own.
    pub fn agents(&self) -> Vec<Agent> {
        registry::all(&self.saved_agents.custom)
    }

    pub fn custom_agents(&self) -> &[AgentConfig] {
        &self.saved_agents.custom
    }

    /// Replaces the custom agents (from Settings). Threads keep their agent id; a thread whose
    /// agent was removed says so when it next sends.
    pub fn set_custom_agents(&mut self, custom: Vec<AgentConfig>) {
        self.saved_agents.custom = custom;
        let mut threads = std::mem::take(&mut self.threads);
        for thread in threads.iter_mut().filter(|t| !t.busy) {
            self.prepare(thread);
        }
        self.threads = threads;
        self.agents_unsaved = true;
        self.save();
    }

    fn agent(&self, id: &AgentId) -> Option<Agent> {
        registry::find(&self.saved_agents.custom, id)
    }

    /// Fills in what the thread's agent offers, from what it reported last time.
    fn prepare(&self, thread: &mut Thread) {
        let agent = self.agent(&thread.settings.agent);
        if let Some(agent) = &agent {
            thread.agent_name = agent.name().to_owned();
        }
        let known = self.saved_agents.known.get(&thread.settings.agent);
        thread.commands = known.map(|k| k.commands.clone()).unwrap_or_default();
        thread.options = match agent.map(|a| a.protocol) {
            Some(Protocol::ClaudeCode) => claude_code::options(),
            _ => known.map(|k| k.options.clone()).unwrap_or_default(),
        };
    }

    /// Talks to another agent in the active thread. Only before the first message: the
    /// conversation lives in the agent, so switching mid-thread would lose it.
    pub fn set_agent(&mut self, id: AgentId) {
        if self.active().has_messages() || self.active().settings.agent == id {
            return;
        }
        let mut thread = Thread::new(self.active);
        thread.settings.agent = id;
        thread.draft = std::mem::take(&mut self.active_mut().draft);
        self.prepare(&mut thread);
        self.defaults = thread.settings.clone();
        *self.active_mut() = thread;
        self.remember_defaults();
    }

    fn remember_defaults(&mut self) {
        self.saved_agents.last = Some(self.defaults.clone());
        self.agents_unsaved = true;
    }

    /// Keeps what each agent reported (settings, commands) for threads that have not started it.
    fn learn(&mut self) {
        for thread in &self.threads {
            let known = self
                .saved_agents
                .known
                .entry(thread.settings.agent.clone())
                .or_default();
            if !thread.commands.is_empty() && known.commands != thread.commands {
                known.commands.clone_from(&thread.commands);
                self.agents_unsaved = true;
            }
            if !thread.options.is_empty() && known.options != thread.options {
                known.options.clone_from(&thread.options);
                self.agents_unsaved = true;
            }
        }
    }

    // ---- threads ---------------------------------------------------------------------------

    pub fn active(&self) -> &Thread {
        self.threads
            .iter()
            .find(|t| t.id == self.active)
            .unwrap_or_else(|| &self.threads[0])
    }

    pub fn active_mut(&mut self) -> &mut Thread {
        let index = self.threads.iter().position(|t| t.id == self.active).unwrap_or(0);
        &mut self.threads[index]
    }

    /// Threads with messages, most recently active first.
    pub fn recent_threads(&self) -> Vec<&Thread> {
        let mut threads: Vec<&Thread> = self.threads.iter().filter(|t| t.has_messages()).collect();
        threads.sort_by_key(|t| std::cmp::Reverse(t.updated_at));
        threads
    }

    /// Opens an empty thread, unless the current one is still empty.
    pub fn new_thread(&mut self) {
        self.show_history = false;
        self.focus_composer = true;
        if !self.active().has_messages() {
            return;
        }
        self.start_new_thread();
    }

    fn start_new_thread(&mut self) {
        let id = ThreadId(self.next_id);
        self.next_id += 1;
        let mut thread = Thread::new(id);
        thread.settings = self.defaults.clone();
        if self.agent(&thread.settings.agent).is_none() {
            thread.settings = Settings::default();
        }
        self.prepare(&mut thread);
        self.threads.push(thread);
        self.switch_to(id);
    }

    pub fn open_thread(&mut self, id: ThreadId) {
        if self.threads.iter().any(|t| t.id == id) {
            self.switch_to(id);
            self.show_history = false;
            self.focus_composer = true;
        }
    }

    /// Makes `id` active, forgets other empty threads and ends idle background sessions.
    fn switch_to(&mut self, id: ThreadId) {
        self.active = id;
        self.threads.retain(|t| t.id == id || t.has_messages() || t.busy);
        for thread in self.threads.iter_mut().filter(|t| t.id != id) {
            thread.release_session();
        }
    }

    pub fn delete_thread(&mut self, id: ThreadId) {
        self.threads.retain(|t| t.id != id);
        if self.active == id || self.threads.is_empty() {
            self.start_new_thread();
        }
        self.unsaved = true;
        self.save();
    }

    /// Writes what changed since the last save: the threads (with their drafts) and the agents.
    pub fn save(&mut self) {
        let drafts_changed = self.threads.iter().any(|t| t.draft != t.saved_draft);
        if self.unsaved || drafts_changed {
            let mut records: Vec<_> = self
                .threads
                .iter()
                .filter(|t| t.has_messages())
                .map(Thread::to_record)
                .collect();
            match history::save(&self.history_path, &mut records) {
                Ok(()) => {
                    self.unsaved = false;
                    for thread in &mut self.threads {
                        thread.saved_draft.clone_from(&thread.draft);
                    }
                }
                Err(err) => eprintln!("falog: could not save assistant threads: {err}"),
            }
        }
        if self.agents_unsaved {
            match self.saved_agents.save(&self.agents_path) {
                Ok(()) => self.agents_unsaved = false,
                Err(err) => eprintln!("falog: could not save assistant agents: {err}"),
            }
        }
        self.last_save = Instant::now();
    }

    // ---- conversation ----------------------------------------------------------------------

    pub fn is_recording(&self) -> bool {
        matches!(self.voice, VoiceState::Recording(_))
    }

    /// Whether the UI should keep animating (spinners, level meter).
    pub fn is_active(&self) -> bool {
        self.threads.iter().any(|t| t.busy)
            || !matches!(self.voice, VoiceState::Idle | VoiceState::NeedsModel)
    }

    pub fn send(&mut self, ctx: &egui::Context, text: String) {
        let launcher = self.launcher.clone();
        let agent = self.agent(&self.active().settings.agent);
        if self.active_mut().send(ctx, text, &launcher, agent.as_ref()) {
            self.unsaved = true;
        }
    }

    /// Interrupts the active thread's turn. The next message resumes the same conversation.
    pub fn stop(&mut self) {
        self.active_mut().stop();
        self.unsaved = true;
    }

    /// Picks the model or effort of the active thread; new threads start from it too.
    pub fn choose(&mut self, kind: OptionKind, value: String) {
        let thread = self.active_mut();
        thread.choose(kind, value);
        self.defaults = thread.settings.clone();
        self.unsaved = true;
        self.remember_defaults();
    }

    /// Processes pending events. Returns `true` when a tool finished, so tasks may have changed.
    pub fn poll(&mut self, ctx: &egui::Context, options: AssistantOptions) -> bool {
        let mut tasks_changed = false;
        for thread in &mut self.threads {
            let activity = thread.poll();
            tasks_changed |= activity.tasks_changed;
            self.unsaved |= activity.changed;
        }
        self.learn();

        self.schedule_partial(options);
        while let Some(done) = self.transcriber.as_ref().and_then(Transcriber::try_recv) {
            if done.is_final {
                self.finish_dictation(ctx, done.result, options);
            } else {
                self.partial_pending = false;
                let dictating = matches!(self.voice, VoiceState::Recording(_) | VoiceState::Transcribing);
                if let (true, Ok(text)) = (dictating, done.result) {
                    self.live_text = text;
                }
            }
        }

        if let VoiceState::Downloading(download) = &self.voice
            && let Some(result) = download.try_finish()
        {
            self.voice = VoiceState::Idle;
            let item = match result {
                Ok(()) => Item::Notice("Voice model ready. Press Ctrl+Space to talk.".into()),
                Err(message) => Item::Error(message),
            };
            self.active_mut().items.push(item);
        }

        if (self.unsaved || self.agents_unsaved) && self.last_save.elapsed() >= SAVE_EVERY {
            self.save();
        }
        tasks_changed
    }

    // ---- dictation -------------------------------------------------------------------------

    /// While recording, re-transcribes the clip so far for the live preview, one job at a time.
    fn schedule_partial(&mut self, options: AssistantOptions) {
        let (VoiceState::Recording(recorder), Some(transcriber)) = (&self.voice, &self.transcriber) else {
            return;
        };
        if self.partial_pending
            || recorder.elapsed() < FIRST_PARTIAL_AFTER
            || self.last_partial.elapsed() < PARTIAL_EVERY
        {
            return;
        }
        transcriber.submit(recorder.snapshot(), options.language, false);
        self.partial_pending = true;
        self.last_partial = Instant::now();
    }

    fn finish_dictation(
        &mut self,
        ctx: &egui::Context,
        result: Result<String, String>,
        options: AssistantOptions,
    ) {
        self.voice = VoiceState::Idle;
        self.live_text.clear();
        let to_draft = std::mem::take(&mut self.dictation_to_draft);
        match result {
            Ok(text) if text.trim().is_empty() => {
                self.active_mut()
                    .items
                    .push(Item::Notice("Didn't catch that.".into()));
            }
            Ok(text) if options.send_after_dictation && !to_draft => self.send(ctx, text),
            Ok(text) => {
                let draft = &mut self.active_mut().draft;
                *draft = append_dictation(draft, &text);
                self.focus_composer = true;
            }
            Err(message) => self.active_mut().items.push(Item::Error(message)),
        }
    }

    /// Stops recording and puts the transcript in the composer, without sending it.
    pub fn dictate_to_draft(&mut self, ctx: &egui::Context, options: AssistantOptions) {
        if matches!(self.voice, VoiceState::Recording(_)) {
            self.dictation_to_draft = true;
            self.toggle_dictation(ctx, options);
            // Too short to transcribe: nothing will arrive.
            self.dictation_to_draft = matches!(self.voice, VoiceState::Transcribing);
        }
    }

    /// Push-to-talk: starts recording, or stops and transcribes.
    pub fn toggle_dictation(&mut self, ctx: &egui::Context, options: AssistantOptions) {
        match std::mem::take(&mut self.voice) {
            VoiceState::Idle | VoiceState::NeedsModel if !voice::model_path().is_file() => {
                self.voice = VoiceState::NeedsModel;
            }
            VoiceState::Idle | VoiceState::NeedsModel => match Recorder::start(ctx.clone()) {
                Ok(recorder) => {
                    // Create the worker now so the model loads while the user is still speaking.
                    self.transcriber(ctx, options);
                    self.live_text.clear();
                    self.last_partial = Instant::now();
                    self.voice = VoiceState::Recording(recorder);
                }
                Err(message) => self.active_mut().items.push(Item::Error(message)),
            },
            VoiceState::Recording(recorder) => match recorder.finish() {
                Some(audio) => {
                    self.transcriber(ctx, options)
                        .submit(audio, options.language, true);
                    self.voice = VoiceState::Transcribing;
                }
                None => self.voice = VoiceState::Idle,
            },
            busy @ (VoiceState::Transcribing | VoiceState::Downloading(_)) => self.voice = busy,
        }
    }

    fn transcriber(&mut self, ctx: &egui::Context, options: AssistantOptions) -> &Transcriber {
        self.transcriber
            .get_or_insert_with(|| Transcriber::new(voice::model_path(), options.voice_gpu, ctx.clone()))
    }

    pub fn cancel_dictation(&mut self) {
        self.dictation_to_draft = false;
        if matches!(self.voice, VoiceState::Recording(_) | VoiceState::NeedsModel) {
            self.voice = VoiceState::Idle;
            self.live_text.clear();
        }
    }

    /// Drops the speech engine so the next dictation reloads it with new settings.
    pub fn reset_voice_engine(&mut self) {
        if !matches!(self.voice, VoiceState::Recording(_) | VoiceState::Transcribing) {
            self.transcriber = None;
        }
    }

    pub fn download_model(&mut self, ctx: &egui::Context) {
        self.voice = VoiceState::Downloading(Download::start(voice::model_path(), ctx.clone()));
    }
}

/// The draft with a transcript added after what was already typed.
fn append_dictation(draft: &str, text: &str) -> String {
    let (draft, text) = (draft.trim_end(), text.trim());
    if draft.is_empty() {
        text.to_owned()
    } else {
        format!("{draft} {text}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictation_adds_to_what_was_typed() {
        assert_eq!(append_dictation("", " remind me tomorrow "), "remind me tomorrow");
        assert_eq!(
            append_dictation("About #12: ", "it shipped"),
            "About #12: it shipped"
        );
    }

    fn assistant(name: &str) -> (Assistant, PathBuf) {
        let dir = std::env::temp_dir().join(format!("falog-assistant-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("threads.json");
        (
            Assistant::with_history(Launcher { database: None }, path.clone()),
            path,
        )
    }

    /// A message in the active thread, marked unsaved like a sent one.
    fn say(assistant: &mut Assistant, text: &str) {
        assistant.active_mut().items.push(Item::User(text.into()));
        assistant.unsaved = true;
    }

    #[test]
    fn saves_only_what_changed() {
        let (mut assistant, path) = assistant("unchanged");
        say(&mut assistant, "hello");
        assistant.save();
        assert!(path.is_file());

        std::fs::remove_file(&path).unwrap();
        assistant.save();
        assert!(!path.exists(), "nothing changed, nothing written");

        assistant.active_mut().draft = "half a thought".into();
        assistant.save();
        assert!(path.is_file(), "a new draft is saved");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn starts_on_an_empty_thread_and_reuses_it() {
        let (mut a, _) = assistant("empty");
        let first = a.active().id;
        a.new_thread();
        assert_eq!(a.active().id, first, "an empty thread is reused");
        say(&mut a, "hi");
        a.new_thread();
        assert_ne!(a.active().id, first);
        assert_eq!(a.recent_threads().len(), 1);
    }

    #[test]
    fn switching_forgets_empty_threads() {
        let (mut a, _) = assistant("switch");
        say(&mut a, "one");
        let one = a.active().id;
        a.new_thread();
        let empty = a.active().id;
        a.open_thread(one);
        assert_eq!(a.active().id, one);
        assert!(a.threads.iter().all(|t| t.id != empty));
    }

    #[test]
    fn threads_survive_a_restart() {
        let (mut a, path) = assistant("restart");
        say(&mut a, "remember me");
        let id = a.active().id;
        a.save();
        let restored = Assistant::with_history(Launcher { database: None }, path.clone());
        assert_eq!(restored.recent_threads().len(), 1);
        assert_eq!(restored.recent_threads()[0].id, id);
        assert_ne!(restored.active().id, id, "a fresh thread is opened on start");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn new_threads_use_the_last_agent_and_model() {
        let (mut a, path) = assistant("agents");
        assert_eq!(a.active().settings.agent, AgentId::default());
        a.set_agent(AgentId("gemini".into()));
        assert_eq!(a.active().agent_name, "Gemini CLI");
        assert!(a.active().options.is_empty(), "unknown until Gemini reports them");
        say(&mut a, "hi");
        a.set_agent(AgentId::default());
        assert_eq!(
            a.active().settings.agent.0,
            "gemini",
            "fixed once the thread has messages"
        );

        a.new_thread();
        assert_eq!(a.active().settings.agent.0, "gemini");
        a.set_agent(AgentId::default());
        a.choose(OptionKind::Model, "haiku".into());
        a.save();

        let restored = Assistant::with_history(Launcher { database: None }, path.clone());
        assert_eq!(restored.active().settings.agent, AgentId::default());
        assert_eq!(restored.active().settings.model.as_deref(), Some("haiku"));
        assert_eq!(restored.recent_threads()[0].agent_name, "Gemini CLI");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn remembers_what_agents_reported() {
        let (mut a, path) = assistant("known");
        a.set_agent(AgentId("gemini".into()));
        a.active_mut()
            .apply(agent::AgentEvent::Commands(vec![agent::SlashCommand {
                name: "compress".into(),
                description: String::new(),
                hint: String::new(),
            }]));
        a.learn();
        a.new_thread();
        say(&mut a, "x");
        a.new_thread();
        assert_eq!(a.active().commands.len(), 1, "known before Gemini starts again");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn deleting_the_active_thread_opens_a_new_one() {
        let (mut a, path) = assistant("delete");
        say(&mut a, "bye");
        let id = a.active().id;
        a.delete_thread(id);
        assert_ne!(a.active().id, id);
        assert!(a.recent_threads().is_empty());
        assert!(history::load(&path).is_empty());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
