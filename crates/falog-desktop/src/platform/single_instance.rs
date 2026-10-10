//! Keeps a single window: a second launch asks the running one to come to the front and exits.
//!
//! Uses a loopback TCP port as the lock, which works without platform-specific APIs and is
//! released automatically when the process dies. The port is derived from the user's data folder, so
//! two users of one machine each get their own, and the running instance answers with a greeting, so
//! another program on that port is never mistaken for Falog.

use eframe::egui;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const SHOW: &[u8] = b"show";
const GREETING: &[u8] = b"falog";
/// A connection that sends nothing must not hold up the listener (or a launch) for long.
const TIMEOUT: Duration = Duration::from_secs(2);

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
    let port = port_for(&falog_core::paths::data_home().to_string_lossy());
    match TcpListener::bind((Ipv4Addr::LOCALHOST, port)) {
        Ok(listener) => Claim::Primary(listener),
        Err(_) if ask_to_show(port) => Claim::AlreadyRunning,
        Err(_) => Claim::Unavailable,
    }
}

/// Asks the instance on `port` to show itself; `false` when nothing there answers like Falog.
fn ask_to_show(port: u16) -> bool {
    let Ok(mut stream) = TcpStream::connect((Ipv4Addr::LOCALHOST, port)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(TIMEOUT));
    let mut reply = [0u8; 8];
    stream.write_all(SHOW).is_ok() && stream.read(&mut reply).is_ok_and(|n| &reply[..n] == GREETING)
}

/// A port in 40000–47999 (below the range systems pick outgoing ports from) for this data folder.
fn port_for(data_home: &str) -> u16 {
    // FNV-1a: stable across builds, unlike the standard library's hasher.
    let hash = data_home.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    });
    40_000 + u16::try_from(hash % 8_000).unwrap_or_default()
}

/// Raises `flag` and wakes the UI whenever another launch asks to show the window.
pub fn listen(listener: TcpListener, ctx: egui::Context, flag: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let _ = stream.set_read_timeout(Some(TIMEOUT));
            let mut buf = [0u8; 8];
            if stream.read(&mut buf).is_ok_and(|n| &buf[..n] == SHOW) {
                let _ = stream.write_all(GREETING);
                flag.store(true, Ordering::Relaxed);
                ctx.request_repaint();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_data_folder_gets_its_own_port() {
        let ana = port_for("C:\\Users\\ana\\.falog");
        assert_eq!(ana, port_for("C:\\Users\\ana\\.falog"));
        assert_ne!(ana, port_for("C:\\Users\\bruno\\.falog"));
        assert!((40_000..48_000).contains(&ana));
    }

    #[test]
    fn a_program_that_is_not_falog_is_not_asked_to_show() {
        let other = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = other.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = other.accept().unwrap();
            let _ = stream.write_all(b"HTTP/1.1 400\r\n\r\n");
        });
        assert!(!ask_to_show(port));
        server.join().unwrap();

        let falog = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = falog.local_addr().unwrap().port();
        let flag = Arc::new(AtomicBool::new(false));
        listen(falog, egui::Context::default(), flag.clone());
        assert!(ask_to_show(port));
    }
}
