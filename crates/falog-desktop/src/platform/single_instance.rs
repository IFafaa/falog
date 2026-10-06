//! Keeps a single window: a second launch asks the running one to come to the front and exits.
//!
//! Uses a loopback TCP port as the lock, which works without platform-specific APIs and is
//! released automatically when the process dies.

use eframe::egui;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const PORT: u16 = 47_613;
const SHOW: &[u8] = b"show";

#[derive(Debug)]
pub enum Claim {
    /// This process owns the lock.
    Primary(TcpListener),
    /// Another instance is running and has been asked to show itself.
    AlreadyRunning,
    /// The port is taken by something else; run without single-instance behavior.
    Unavailable,
}

pub fn claim() -> Claim {
    match TcpListener::bind((Ipv4Addr::LOCALHOST, PORT)) {
        Ok(listener) => Claim::Primary(listener),
        Err(_) => match TcpStream::connect((Ipv4Addr::LOCALHOST, PORT)) {
            Ok(mut stream) => {
                if stream.write_all(SHOW).is_ok() {
                    Claim::AlreadyRunning
                } else {
                    Claim::Unavailable
                }
            }
            Err(_) => Claim::Unavailable,
        },
    }
}

/// Raises `flag` and wakes the UI whenever another launch asks to show the window.
pub fn listen(listener: TcpListener, ctx: egui::Context, flag: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let mut buf = [0u8; 8];
            if stream.read(&mut buf).is_ok_and(|n| &buf[..n] == SHOW) {
                flag.store(true, Ordering::Relaxed);
                ctx.request_repaint();
            }
        }
    });
}
