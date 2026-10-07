//! The assistant dock: conversations with Claude that manage tasks, typed or dictated.
//!
//! [`claude`] drives Claude Code, [`thread`] holds one conversation, [`history`] saves them,
//! [`voice`] records and transcribes speech and [`panel`] draws the dock. [`Assistant`] owns the
//! threads and the dictation state and ties them together.

pub mod claude;
pub mod history;
pub mod panel;
pub mod thread;
pub mod voice;

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use thread::Launcher;
pub use thread::{Item, Thread, ThreadId, ToolCall};
use voice::{Download, Recorder, Transcriber, VoiceLanguage};

/// How often the live preview is refreshed while dictating, and when the first one starts.
const PARTIAL_EVERY: Duration = Duration::from_millis(1000);
const FIRST_PARTIAL_AFTER: Duration = Duration::from_millis(1500);
/// Pending thread changes are written at most this often.
const SAVE_EVERY: Duration = Duration::from_secs(2);

/// Which Claude model the assistant asks Claude Code for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssistantModel {
    /// Whatever Claude Code is configured to use.
    #[default]
    Default,
    Haiku,
    Sonnet,
    Opus,
}

impl AssistantModel {
    pub const ALL: [Self; 4] = [Self::Default, Self::Haiku, Self::Sonnet, Self::Opus];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Haiku => "Haiku",
            Self::Sonnet => "Sonnet",
            Self::Opus => "Opus",
        }
    }

    const fn alias(self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::Haiku => Some("haiku"),
            Self::Sonnet => Some("sonnet"),
            Self::Opus => Some("opus"),
        }
    }
}

/// User-facing assistant options (stored in the app preferences).
#[derive(Clone, Copy, Debug)]
pub struct AssistantOptions {
    pub model: AssistantModel,
    pub language: VoiceLanguage,
    pub send_after_dictation: bool,
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
    partial_pending: bool,
    last_partial: Instant,
    launcher: Launcher,
    history_path: PathBuf,
    transcriber: Option<Transcriber>,
    unsaved: bool,
    last_save: Instant,
}

impl Assistant {
    /// Loads the saved threads and opens a fresh one, as Zed does.
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
        let mut assistant = Self {
            threads,
            active: ThreadId(0),
            next_id,
            show_history: false,
            voice: VoiceState::Idle,
            live_text: String::new(),
            focus_composer: false,
            partial_pending: false,
            last_partial: Instant::now(),
            launcher,
            history_path,
            transcriber: None,
            unsaved: false,
            last_save: Instant::now(),
        };
        assistant.start_new_thread();
        assistant
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
        self.threads.push(Thread::new(id));
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

    pub fn save(&mut self) {
        let mut records: Vec<_> = self
            .threads
            .iter()
            .filter(|t| t.has_messages())
            .map(Thread::to_record)
            .collect();
        match history::save(&self.history_path, &mut records) {
            Ok(()) => self.unsaved = false,
            Err(err) => eprintln!("falog: could not save assistant threads: {err}"),
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

    pub fn send(&mut self, ctx: &egui::Context, text: String, options: AssistantOptions) {
        let launcher = self.launcher.clone();
        if self
            .active_mut()
            .send(ctx, text, &launcher, options.model.alias())
        {
            self.unsaved = true;
        }
    }

    /// Interrupts the active thread's turn. The next message resumes the same conversation.
    pub fn stop(&mut self) {
        self.active_mut().stop();
        self.unsaved = true;
    }

    /// Restarts Claude Code on the next message (e.g. after a model change), keeping the context.
    pub fn restart_session(&mut self) {
        for thread in &mut self.threads {
            thread.release_session();
        }
    }

    /// Processes pending events. Returns `true` when a tool finished, so tasks may have changed.
    pub fn poll(&mut self, ctx: &egui::Context, options: AssistantOptions) -> bool {
        let mut tasks_changed = false;
        for thread in &mut self.threads {
            let activity = thread.poll();
            tasks_changed |= activity.tasks_changed;
            self.unsaved |= activity.changed;
        }

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

        if self.unsaved && self.last_save.elapsed() >= SAVE_EVERY {
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
        match result {
            Ok(text) if text.trim().is_empty() => {
                self.active_mut()
                    .items
                    .push(Item::Notice("Didn't catch that.".into()));
            }
            Ok(text) if options.send_after_dictation => self.send(ctx, text, options),
            Ok(text) => {
                self.active_mut().draft = text;
                self.focus_composer = true;
            }
            Err(message) => self.active_mut().items.push(Item::Error(message)),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn assistant(name: &str) -> (Assistant, PathBuf) {
        let dir = std::env::temp_dir().join(format!("falog-assistant-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("threads.json");
        (
            Assistant::with_history(Launcher { database: None }, path.clone()),
            path,
        )
    }

    fn say(assistant: &mut Assistant, text: &str) {
        assistant.active_mut().items.push(Item::User(text.into()));
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
