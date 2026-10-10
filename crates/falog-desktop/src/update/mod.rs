//! Updating Falog from inside the app: it looks for a newer release now and then, offers it in the
//! status bar, downloads it on request (checked against the release's SHA256SUMS) and, once ready,
//! installs it and starts the new version (.spec/releases.md, "Updating from the app").

pub mod feed;
mod install;

use crate::download::{Download, Expected};
use eframe::egui;
use feed::{Release, Version};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

/// The first automatic check waits until the window is up; later ones run this often.
const FIRST_CHECK_AFTER: Duration = Duration::from_secs(10);
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Default)]
pub enum State {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available(Release),
    /// Fetching the release's checksums, then its download.
    Preparing(Release),
    Downloading(Release, Download),
    /// Downloaded and verified: installing restarts Falog.
    Ready(Release, PathBuf),
    Failed(String),
}

enum Event {
    Checked(Result<Option<Release>, String>),
    Checksum(Release, Result<String, String>),
}

#[derive(Debug)]
pub struct Updater {
    pub state: State,
    /// False in builds that cannot replace themselves (run from a source checkout, demo windows).
    pub enabled: bool,
    started: Instant,
    last_check: Option<Instant>,
    sender: Sender<Event>,
    receiver: Receiver<Event>,
}

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("update::Event")
    }
}

impl Default for Updater {
    fn default() -> Self {
        let (sender, receiver) = channel();
        Self {
            state: State::Idle,
            enabled: install::can_update(),
            started: Instant::now(),
            last_check: None,
            sender,
            receiver,
        }
    }
}

impl Updater {
    /// Runs the automatic checks and collects background results; call it every frame.
    pub fn poll(&mut self, ctx: &egui::Context, auto_check: bool, preview: bool) {
        if self.enabled && auto_check && self.check_due() {
            self.check(ctx, preview);
        }
        while let Ok(event) = self.receiver.try_recv() {
            self.state = match event {
                Event::Checked(Ok(Some(release))) => State::Available(release),
                Event::Checked(Ok(None)) => State::UpToDate,
                Event::Checked(Err(message)) | Event::Checksum(_, Err(message)) => State::Failed(message),
                Event::Checksum(release, Ok(sha256)) => {
                    let name = feed::asset_name().unwrap_or_default();
                    let file = install::downloads_dir().join(name);
                    let download = Download::start(
                        feed::asset_url(&release.tag, name),
                        file,
                        Expected { sha256, size: None },
                        ctx.clone(),
                    );
                    State::Downloading(release, download)
                }
            };
        }
        if let State::Downloading(release, download) = &self.state
            && let Some(result) = download.try_finish()
        {
            self.state = match result {
                Ok(()) => {
                    let name = feed::asset_name().unwrap_or_default();
                    State::Ready(release.clone(), install::downloads_dir().join(name))
                }
                Err(message) => State::Failed(message),
            };
        }
    }

    fn check_due(&self) -> bool {
        let busy = matches!(
            self.state,
            State::Checking | State::Preparing(_) | State::Downloading(..) | State::Ready(..)
        );
        !busy
            && match self.last_check {
                None => self.started.elapsed() >= FIRST_CHECK_AFTER,
                Some(at) => at.elapsed() >= CHECK_EVERY,
            }
    }

    /// Looks for a release newer than this build, in the background.
    pub fn check(&mut self, ctx: &egui::Context, preview: bool) {
        if !self.enabled {
            return;
        }
        self.last_check = Some(Instant::now());
        self.state = State::Checking;
        let (sender, ctx) = (self.sender.clone(), ctx.clone());
        std::thread::spawn(move || {
            let result = fetch(&feed::feed_url())
                .map(|text| feed::newest(&feed::tags_in_feed(&text), &Version::current(), preview))
                .map_err(|err| format!("Could not check for updates: {err}"));
            let _ = sender.send(Event::Checked(result));
            ctx.request_repaint();
        });
    }

    /// Downloads the available release.
    pub fn download(&mut self, ctx: &egui::Context) {
        let State::Available(release) = &self.state else {
            return;
        };
        let release = release.clone();
        self.state = State::Preparing(release.clone());
        let (sender, ctx) = (self.sender.clone(), ctx.clone());
        std::thread::spawn(move || {
            let name = feed::asset_name().unwrap_or_default();
            let checksum = fetch(&feed::asset_url(&release.tag, "SHA256SUMS"))
                .map_err(|err| format!("Could not download the update: {err}"))
                .and_then(|sums| {
                    feed::checksum_for(&sums, name)
                        .ok_or_else(|| format!("The release has no checksum for {name}"))
                });
            let _ = sender.send(Event::Checksum(release, checksum));
            ctx.request_repaint();
        });
    }

    /// Installs the downloaded release. On success the caller quits Falog, and the new version starts.
    pub fn install(&mut self) -> Result<(), String> {
        let State::Ready(_, file) = &self.state else {
            return Err("No update is ready to install".into());
        };
        install::install(file).map_err(|err| {
            let message = format!("Could not install the update: {err}");
            self.state = State::Failed(message.clone());
            message
        })
    }
}

fn fetch(url: &str) -> Result<String, String> {
    ureq::get(url)
        .timeout(TIMEOUT)
        .call()
        .map_err(|err| err.to_string())?
        .into_string()
        .map_err(|err| err.to_string())
}
