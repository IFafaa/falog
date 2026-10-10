//! Push-to-talk: microphone capture, local Whisper transcription and the model download.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use eframe::egui;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Whisper's expected input: 16 kHz mono f32.
pub const SAMPLE_RATE: u32 = 16_000;
/// Large v3 Turbo, 5-bit quantized: near large-v3 accuracy (Portuguese included) at CPU speed.
pub const MODEL_FILE: &str = "ggml-large-v3-turbo-q5_0.bin";
/// Pinned to a commit, so the file cannot change under [`MODEL_SHA256`].
pub const MODEL_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-large-v3-turbo-q5_0.bin";
/// The model is parsed by native code: a download that is not exactly this file is thrown away.
const MODEL_SHA256: &str = "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2";
const MODEL_BYTES: u64 = 574_041_195;
/// Rough size, shown before the download starts.
pub const MODEL_SIZE_MB: u32 = 574;
/// Frees the model's memory (~600 MB) after this long without dictation.
const UNLOAD_AFTER: Duration = Duration::from_secs(300);
/// Clips shorter than this are treated as accidental clicks.
const MIN_SPEECH: Duration = Duration::from_millis(400);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoiceLanguage {
    /// Detect the language on every clip. Accurate but slow: detection runs the full 30 s window.
    Auto,
    Portuguese,
    English,
}

/// The user's regional language when Whisper knows it well, English otherwise.
impl Default for VoiceLanguage {
    fn default() -> Self {
        if crate::platform::locale::is_portuguese() {
            Self::Portuguese
        } else {
            Self::English
        }
    }
}

impl VoiceLanguage {
    pub const ALL: [Self; 3] = [Self::Portuguese, Self::English, Self::Auto];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto (slower)",
            Self::Portuguese => "Português",
            Self::English => "English",
        }
    }

    const fn code(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Portuguese => "pt",
            Self::English => "en",
        }
    }
}

// ---- recording -------------------------------------------------------------------------------

/// An open microphone stream accumulating mono samples.
pub struct Recorder {
    _stream: cpal::Stream,
    samples: Arc<Mutex<Vec<f32>>>,
    level: Arc<AtomicU32>,
    sample_rate: u32,
    started: Instant,
}

impl fmt::Debug for Recorder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Recorder")
            .field("sample_rate", &self.sample_rate)
            .finish_non_exhaustive()
    }
}

impl Recorder {
    /// Starts capturing from the default input device. The UI repaints on its own while recording, so
    /// audio buffers (about a hundred a second) do not ask for frames.
    pub fn start() -> Result<Self, String> {
        let device = cpal::default_host()
            .default_input_device()
            .ok_or("No microphone was found.")?;
        let supported = device
            .default_input_config()
            .map_err(|e| format!("Microphone unavailable: {e}"))?;
        let sample_rate = supported.sample_rate();
        let channels = usize::from(supported.channels());
        let config = supported.config();

        let samples = Arc::new(Mutex::new(Vec::new()));
        let level = Arc::new(AtomicU32::new(0));
        let stream = match supported.sample_format() {
            SampleFormat::F32 => build::<f32>(&device, config, channels, &samples, &level),
            SampleFormat::I16 => build::<i16>(&device, config, channels, &samples, &level),
            SampleFormat::U16 => build::<u16>(&device, config, channels, &samples, &level),
            SampleFormat::I32 => build::<i32>(&device, config, channels, &samples, &level),
            other => return Err(format!("Unsupported microphone format {other:?}.")),
        }
        .map_err(|e| format!("Could not open the microphone: {e}"))?;
        stream
            .play()
            .map_err(|e| format!("Could not start the microphone: {e}"))?;

        Ok(Self {
            _stream: stream,
            samples,
            level,
            sample_rate,
            started: Instant::now(),
        })
    }

    /// Recent input loudness, 0.0–1.0.
    pub fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// The recording so far, without stopping (for live previews). Copying and resampling it is left
    /// to the transcriber's thread: a long clip is tens of megabytes.
    pub fn live(&self) -> Clip {
        Clip::Live {
            samples: Arc::clone(&self.samples),
            rate: self.sample_rate,
        }
    }

    /// Stops recording and returns 16 kHz audio, or `None` if the clip was too short.
    pub fn finish(self) -> Option<Vec<f32>> {
        if self.elapsed() < MIN_SPEECH {
            return None;
        }
        let samples = self.samples.lock().map(|s| s.clone()).unwrap_or_default();
        Some(resample(&samples, self.sample_rate, SAMPLE_RATE))
    }
}

fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    samples: &Arc<Mutex<Vec<f32>>>,
    level: &Arc<AtomicU32>,
) -> Result<cpal::Stream, cpal::Error>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let samples = Arc::clone(samples);
    let level = Arc::clone(level);
    device.build_input_stream::<T, _, _>(
        config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            let mono: Vec<f32> = data
                .chunks(channels.max(1))
                .map(|frame| frame.iter().map(|s| s.to_sample::<f32>()).sum::<f32>() / frame.len() as f32)
                .collect();
            let rms = (mono.iter().map(|s| s * s).sum::<f32>() / mono.len().max(1) as f32).sqrt();
            level.store((rms * 4.0).min(1.0).to_bits(), Ordering::Relaxed);
            if let Ok(mut buffer) = samples.lock() {
                buffer.extend_from_slice(&mono);
            }
        },
        |err| eprintln!("falog: microphone error: {err}"),
        None,
    )
}

/// Linear-interpolation resampler; plenty for speech going into Whisper.
pub fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = f64::from(from) / f64::from(to);
    let len = (input.len() as f64 / ratio).floor() as usize;
    (0..len)
        .map(|i| {
            let position = i as f64 * ratio;
            let index = position as usize;
            let next = input.get(index + 1).copied().unwrap_or(input[index]);
            let fraction = (position - index as f64) as f32;
            input[index] + (next - input[index]) * fraction
        })
        .collect()
}

// ---- transcription ---------------------------------------------------------------------------

/// Silence appended to every clip: without it, short clips with a small `audio_ctx` make Whisper
/// repeat itself ("Review the PR. Review the PR.").
const TRAILING_SILENCE: usize = SAMPLE_RATE as usize;

/// Audio to transcribe: ready at 16 kHz, or a recording still going on, read when the job runs.
#[derive(Debug)]
pub enum Clip {
    Ready(Vec<f32>),
    Live {
        samples: Arc<Mutex<Vec<f32>>>,
        rate: u32,
    },
}

impl Clip {
    /// The audio at 16 kHz.
    fn into_samples(self) -> Vec<f32> {
        match self {
            Self::Ready(samples) => samples,
            Self::Live { samples, rate } => {
                let recorded = samples.lock().map(|s| s.clone()).unwrap_or_default();
                resample(&recorded, rate, SAMPLE_RATE)
            }
        }
    }
}

struct Job {
    audio: Clip,
    language: VoiceLanguage,
    /// Partial jobs feed the live preview while recording; the final one is the transcript.
    is_final: bool,
}

/// The outcome of one transcription job.
#[derive(Debug)]
pub struct Transcription {
    pub is_final: bool,
    pub result: Result<String, String>,
}

/// Runs Whisper off the UI thread on two lanes sharing one loaded model: partial jobs (live
/// preview, stale ones dropped) and final jobs, which start right away instead of waiting for the
/// preview in progress.
#[derive(Debug)]
pub struct Transcriber {
    partial_jobs: Sender<Job>,
    final_jobs: Sender<Job>,
    results: Receiver<Transcription>,
}

impl Transcriber {
    /// `use_gpu` only has an effect in builds with the `gpu` feature.
    pub fn new(model: PathBuf, use_gpu: bool, ctx: egui::Context) -> Self {
        let cache = Arc::new(ModelCache::new(model, use_gpu));
        let final_running = Arc::new(AtomicBool::new(false));
        let (results_tx, results) = mpsc::channel();
        let cores = thread::available_parallelism().map_or(4, |n| n.get());
        let spawn_lane = |is_final: bool, threads: usize| {
            let (tx, rx) = mpsc::channel::<Job>();
            let lane = Lane {
                jobs: rx,
                cache: Arc::clone(&cache),
                results: results_tx.clone(),
                final_running: Arc::clone(&final_running),
                threads,
                is_final,
                ctx: ctx.clone(),
            };
            thread::spawn(move || lane.run());
            tx
        };
        let partial_jobs = spawn_lane(false, (cores / 2).clamp(1, 4));
        let final_jobs = spawn_lane(true, cores.min(8));
        Self {
            partial_jobs,
            final_jobs,
            results,
        }
    }

    pub fn submit(&self, audio: Clip, language: VoiceLanguage, is_final: bool) {
        let job = Job {
            audio,
            language,
            is_final,
        };
        let lane = if is_final {
            &self.final_jobs
        } else {
            &self.partial_jobs
        };
        let _ = lane.send(job);
    }

    pub fn try_recv(&self) -> Option<Transcription> {
        self.results.try_recv().ok()
    }
}

/// The loaded model, shared by both lanes and released after [`UNLOAD_AFTER`] without use.
struct ModelCache {
    path: PathBuf,
    use_gpu: bool,
    state: Mutex<(Option<engine::Model>, Instant)>,
}

impl ModelCache {
    fn new(path: PathBuf, use_gpu: bool) -> Self {
        Self {
            path,
            use_gpu,
            state: Mutex::new((None, Instant::now())),
        }
    }

    fn get(&self) -> Result<engine::Model, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Whisper model lock poisoned".to_string())?;
        if state.0.is_none() {
            state.0 = Some(engine::load(&self.path, self.use_gpu)?);
        }
        state.1 = Instant::now();
        state
            .0
            .clone()
            .ok_or_else(|| "Whisper model not loaded".to_string())
    }

    fn unload_if_idle(&self) {
        if let Ok(mut state) = self.state.lock()
            && state.1.elapsed() >= UNLOAD_AFTER
        {
            state.0 = None;
        }
    }
}

struct Lane {
    jobs: Receiver<Job>,
    cache: Arc<ModelCache>,
    results: Sender<Transcription>,
    /// Set while a final job runs, so no new preview competes with it for the CPU.
    final_running: Arc<AtomicBool>,
    threads: usize,
    is_final: bool,
    ctx: egui::Context,
}

impl Lane {
    fn run(self) {
        engine::init();
        loop {
            let job = match self.jobs.recv_timeout(UNLOAD_AFTER) {
                Ok(job) if self.is_final => job,
                Ok(job) => newest(job, self.jobs.try_iter()),
                Err(RecvTimeoutError::Timeout) => {
                    self.cache.unload_if_idle();
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            };
            if !self.is_final && self.final_running.load(Ordering::Relaxed) {
                continue;
            }
            if self.is_final {
                self.final_running.store(true, Ordering::Relaxed);
            }
            let mut audio = job.audio.into_samples();
            audio.resize(audio.len() + TRAILING_SILENCE, 0.0);
            let result = self
                .cache
                .get()
                .and_then(|model| engine::transcribe(&model, &audio, job.language.code(), self.threads));
            if self.is_final {
                self.final_running.store(false, Ordering::Relaxed);
            }
            if self
                .results
                .send(Transcription {
                    is_final: job.is_final,
                    result,
                })
                .is_err()
            {
                break;
            }
            self.ctx.request_repaint();
        }
    }
}

/// Picks the job worth running among those waiting: the newest one, except that a final job is
/// never dropped for a partial one. Stale previews are skipped so they never pile up.
fn newest(first: Job, pending: impl Iterator<Item = Job>) -> Job {
    pending.fold(first, |kept, next| {
        if kept.is_final && !next.is_final {
            kept
        } else {
            next
        }
    })
}

/// Encoder frames for a clip: 50 per second, 50% headroom, never above the 30 s window.
#[cfg_attr(not(feature = "whisper"), allow(dead_code))]
fn audio_context(samples: usize) -> i32 {
    let seconds = samples as f32 / SAMPLE_RATE as f32;
    ((seconds * 50.0 * 1.5) as i32 + 128).min(1500)
}

/// Whisper itself, behind the `whisper` feature so the app also builds without LLVM.
#[cfg(feature = "whisper")]
mod engine {
    use super::audio_context;
    use std::path::Path;
    use std::sync::Arc;
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    pub type Model = Arc<WhisperContext>;

    pub fn init() {
        whisper_rs::install_logging_hooks();
    }

    pub fn load(path: &Path, use_gpu: bool) -> Result<Model, String> {
        let mut parameters = WhisperContextParameters::default();
        parameters.use_gpu(use_gpu && cfg!(feature = "gpu"));
        WhisperContext::new_with_params(path, parameters)
            .map(Arc::new)
            .map_err(|e| format!("Could not load the Whisper model: {e}"))
    }

    pub fn transcribe(
        model: &Model,
        audio: &[f32],
        language: &str,
        threads: usize,
    ) -> Result<String, String> {
        let mut state = model
            .create_state()
            .map_err(|e| format!("Whisper failed to start: {e}"))?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(language));
        params.set_n_threads(threads as i32);
        // The encoder always works on a 30 s window (1500 frames). Shrinking it to the clip length
        // (plus headroom) is the biggest speedup for short dictations.
        params.set_audio_ctx(audio_context(audio.len()));
        // Dictations are short: one segment avoids repeated trailing phrases.
        params.set_single_segment(true);
        params.set_no_timestamps(true);
        params.set_suppress_blank(true);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);

        state
            .full(params, audio)
            .map_err(|e| format!("Transcription failed: {e}"))?;
        let text: Vec<String> = state
            .as_iter()
            .filter_map(|segment| segment.to_str_lossy().ok().map(|s| s.trim().to_owned()))
            .filter(|s| !s.is_empty())
            .collect();
        Ok(text.join(" "))
    }
}

#[cfg(not(feature = "whisper"))]
mod engine {
    use std::path::Path;

    /// Never constructed: [`load`] always fails without the `whisper` feature.
    #[derive(Clone)]
    pub struct Model;

    pub fn init() {}

    pub fn load(_: &Path, _: bool) -> Result<Model, String> {
        Err("This build of Falog has no speech recognition (compiled without the `whisper` feature).".into())
    }

    pub fn transcribe(_: &Model, _: &[f32], _: &str, _: usize) -> Result<String, String> {
        Err("This build of Falog has no speech recognition.".into())
    }
}

// ---- model download --------------------------------------------------------------------------

/// `models/<MODEL_FILE>` in the data folder ([`falog_core::paths::data_home`]), shared by every database.
pub fn model_path() -> PathBuf {
    falog_core::paths::data_home().join("models").join(MODEL_FILE)
}

#[derive(Debug)]
pub struct Download {
    received: Arc<AtomicU64>,
    total: Arc<AtomicU64>,
    done: Receiver<Result<(), String>>,
}

impl Download {
    pub fn start(destination: PathBuf, ctx: egui::Context) -> Self {
        let received = Arc::new(AtomicU64::new(0));
        let total = Arc::new(AtomicU64::new(0));
        let (tx, done) = mpsc::channel();
        let (r, t) = (Arc::clone(&received), Arc::clone(&total));
        thread::spawn(move || {
            let result = download(&destination, &r, &t, &ctx).map_err(|e| format!("Download failed: {e}"));
            let _ = tx.send(result);
            ctx.request_repaint();
        });
        Self {
            received,
            total,
            done,
        }
    }

    /// 0.0–1.0, or `None` until the size is known.
    pub fn progress(&self) -> Option<f32> {
        let total = self.total.load(Ordering::Relaxed);
        (total > 0).then(|| self.received.load(Ordering::Relaxed) as f32 / total as f32)
    }

    pub fn received_mb(&self) -> u64 {
        self.received.load(Ordering::Relaxed) / 1_000_000
    }

    pub fn try_finish(&self) -> Option<Result<(), String>> {
        self.done.try_recv().ok()
    }
}

fn download(
    destination: &Path,
    received: &AtomicU64,
    total: &AtomicU64,
    ctx: &egui::Context,
) -> io::Result<()> {
    if let Some(dir) = destination.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let response = ureq::get(MODEL_URL).call().map_err(io::Error::other)?;
    if let Some(length) = response.header("Content-Length").and_then(|l| l.parse().ok()) {
        total.store(length, Ordering::Relaxed);
    }
    let partial = destination.with_extension("part");
    let mut file = File::create(&partial)?;
    let mut last_repaint = Instant::now();
    let copied = copy_verified(
        response.into_reader(),
        &mut file,
        MODEL_BYTES,
        MODEL_SHA256,
        |read| {
            received.fetch_add(read as u64, Ordering::Relaxed);
            if last_repaint.elapsed() > Duration::from_millis(100) {
                ctx.request_repaint();
                last_repaint = Instant::now();
            }
        },
    );
    if let Err(err) = copied {
        drop(file);
        let _ = std::fs::remove_file(&partial);
        return Err(err);
    }
    file.sync_all()?;
    drop(file);
    std::fs::rename(&partial, destination)
}

/// Copies `reader` into `writer`, failing unless it is exactly `size` bytes with SHA-256 `sha256`.
/// `progress` gets the size of each chunk.
fn copy_verified(
    mut reader: impl Read,
    writer: &mut impl Write,
    size: u64,
    sha256: &str,
    mut progress: impl FnMut(usize),
) -> io::Result<()> {
    let corrupted = || io::Error::other("The voice model download was damaged; try again");
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 16];
    let mut total = 0u64;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if total > size {
            return Err(corrupted());
        }
        hasher.update(&buffer[..read]);
        writer.write_all(&buffer[..read])?;
        progress(read);
    }
    let digest: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if total == size && digest == sha256 {
        Ok(())
    } else {
        Err(corrupted())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_the_expected_model() {
        // SHA-256 of "abc".
        let sha = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let copy = |bytes: &[u8], size| {
            let mut out = Vec::new();
            copy_verified(bytes, &mut out, size, sha, |_| {}).map(|()| out)
        };
        assert_eq!(copy(b"abc", 3).unwrap(), b"abc");
        assert!(copy(b"abd", 3).is_err(), "tampered");
        assert!(copy(b"ab", 3).is_err(), "truncated");
        assert!(copy(b"abcd", 3).is_err(), "too long");
    }

    #[test]
    fn resampling_keeps_duration() {
        let one_second_48k = vec![0.5; 48_000];
        let out = resample(&one_second_48k, 48_000, SAMPLE_RATE);
        assert_eq!(out.len(), 16_000);
        assert!(out.iter().all(|s| (s - 0.5).abs() < 1e-6));
    }

    #[test]
    fn resampling_interpolates() {
        let out = resample(&[0.0, 1.0, 0.0, 1.0], 2, 1);
        assert_eq!(out, vec![0.0, 0.0]);
        let up = resample(&[0.0, 1.0], 1, 2);
        assert_eq!(up, vec![0.0, 0.5, 1.0, 1.0]);
    }

    #[test]
    fn audio_context_follows_clip_length() {
        assert_eq!(audio_context(5 * SAMPLE_RATE as usize), 503);
        assert_eq!(audio_context(60 * SAMPLE_RATE as usize), 1500);
    }

    fn job(id: f32, is_final: bool) -> Job {
        Job {
            audio: Clip::Ready(vec![id]),
            language: VoiceLanguage::English,
            is_final,
        }
    }

    #[test]
    fn keeps_only_the_newest_pending_job() {
        let picked = newest(job(1.0, false), [job(2.0, false), job(3.0, false)].into_iter());
        assert_eq!(picked.audio.into_samples(), vec![3.0]);
    }

    #[test]
    fn never_drops_a_final_job_for_a_partial_one() {
        let picked = newest(job(1.0, false), [job(2.0, true), job(3.0, false)].into_iter());
        assert!(picked.is_final);
        assert_eq!(picked.audio.into_samples(), vec![2.0]);
    }

    #[test]
    fn same_rate_is_identity() {
        assert_eq!(resample(&[0.1, 0.2], 16_000, 16_000), vec![0.1, 0.2]);
    }
}
