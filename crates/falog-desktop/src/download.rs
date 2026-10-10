//! Downloads a file in the background and puts it in place only once it matches the SHA-256 (and the
//! size, when known) it was published with: the voice model and app updates are both run by Falog,
//! so a truncated or tampered file must never be used.

use crate::i18n::{tr, trf};
use eframe::egui;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

/// What the downloaded file must be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expected {
    /// Lowercase hex.
    pub sha256: String,
    /// Exact size in bytes, when it is known in advance.
    pub size: Option<u64>,
}

#[derive(Debug)]
pub struct Download {
    received: Arc<AtomicU64>,
    total: Arc<AtomicU64>,
    done: Receiver<Result<(), String>>,
}

impl Download {
    /// Starts downloading `url` to `destination` on a background thread.
    pub fn start(url: String, destination: PathBuf, expected: Expected, ctx: egui::Context) -> Self {
        let received = Arc::new(AtomicU64::new(0));
        let total = Arc::new(AtomicU64::new(0));
        let (tx, done) = mpsc::channel();
        let (r, t) = (Arc::clone(&received), Arc::clone(&total));
        thread::spawn(move || {
            let result = download(&url, &destination, &expected, &r, &t, &ctx)
                .map_err(|e| trf("Download failed: {err}", &[("err", &e)]));
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
    url: &str,
    destination: &Path,
    expected: &Expected,
    received: &AtomicU64,
    total: &AtomicU64,
    ctx: &egui::Context,
) -> io::Result<()> {
    if let Some(dir) = destination.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let response = ureq::get(url).call().map_err(io::Error::other)?;
    if let Some(length) = response.header("Content-Length").and_then(|l| l.parse().ok()) {
        total.store(length, Ordering::Relaxed);
    }
    let partial = destination.with_extension("part");
    let mut file = File::create(&partial)?;
    let mut last_repaint = Instant::now();
    let copied = copy_verified(response.into_reader(), &mut file, expected, |read| {
        received.fetch_add(read as u64, Ordering::Relaxed);
        if last_repaint.elapsed() > Duration::from_millis(100) {
            ctx.request_repaint();
            last_repaint = Instant::now();
        }
    });
    if let Err(err) = copied {
        drop(file);
        let _ = std::fs::remove_file(&partial);
        return Err(err);
    }
    file.sync_all()?;
    drop(file);
    std::fs::rename(&partial, destination)
}

/// Copies `reader` into `writer`, failing unless the bytes match `expected`. `progress` gets the size
/// of each chunk.
pub fn copy_verified(
    mut reader: impl Read,
    writer: &mut impl Write,
    expected: &Expected,
    mut progress: impl FnMut(usize),
) -> io::Result<()> {
    let damaged = || io::Error::other(tr("the file was damaged on the way; try again"));
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 16];
    let mut total = 0u64;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if expected.size.is_some_and(|size| total > size) {
            return Err(damaged());
        }
        hasher.update(&buffer[..read]);
        writer.write_all(&buffer[..read])?;
        progress(read);
    }
    let digest: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if expected.size.is_none_or(|size| total == size) && digest == expected.sha256 {
        Ok(())
    } else {
        Err(damaged())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SHA-256 of "abc".
    const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn copy(bytes: &[u8], size: Option<u64>) -> io::Result<Vec<u8>> {
        let mut out = Vec::new();
        let expected = Expected {
            sha256: ABC.into(),
            size,
        };
        copy_verified(bytes, &mut out, &expected, |_| {}).map(|()| out)
    }

    #[test]
    fn keeps_only_the_expected_file() {
        assert_eq!(copy(b"abc", Some(3)).unwrap(), b"abc");
        assert_eq!(copy(b"abc", None).unwrap(), b"abc");
        assert!(copy(b"abx", Some(3)).is_err(), "tampered");
        assert!(copy(b"abx", None).is_err(), "tampered, size unknown");
        assert!(copy(b"ab", Some(3)).is_err(), "truncated");
        assert!(copy(b"abcd", Some(3)).is_err(), "too long");
    }
}
