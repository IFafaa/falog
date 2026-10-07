//! Google OAuth 2.0 for installed apps: loopback redirect plus PKCE.
//!
//! See <https://developers.google.com/identity/protocols/oauth2/native-app>. The app listens on a
//! random port of `127.0.0.1`, opens the consent page in the browser, and exchanges the code Google
//! redirects with for a refresh token.

use crate::config::Client;
use crate::error::google_message;
use crate::url::{encode, query_pairs};
use crate::{Error, Result};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";
pub const SCOPE: &str = "https://www.googleapis.com/auth/calendar.readonly";

/// How long to wait for the user to finish in the browser.
pub const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// A sign-in waiting for the browser to come back.
#[derive(Debug)]
pub struct Pending {
    listener: TcpListener,
    redirect_uri: String,
    verifier: String,
    state: String,
    url: String,
}

/// Starts listening and builds the consent URL to open in the browser.
pub fn begin(client: &Client) -> Result<Pending> {
    if !client.is_set() {
        return Err(Error::NoClient);
    }
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let redirect_uri = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
    let verifier = random_token(32)?;
    let state = random_token(16)?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let params = [
        ("client_id", client.id.trim()),
        ("redirect_uri", &redirect_uri),
        ("response_type", "code"),
        ("scope", SCOPE),
        ("code_challenge", &challenge),
        ("code_challenge_method", "S256"),
        ("state", &state),
        // Offline access plus the consent prompt make Google return a refresh token every time.
        ("access_type", "offline"),
        ("prompt", "consent select_account"),
    ];
    let query: Vec<String> = params
        .iter()
        .map(|(key, value)| format!("{key}={}", encode(value)))
        .collect();
    let url = format!("{AUTH_URL}?{}", query.join("&"));
    Ok(Pending {
        listener,
        redirect_uri,
        verifier,
        state,
        url,
    })
}

impl Pending {
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Waits for Google's redirect (until `cancel` is set or [`SIGN_IN_TIMEOUT`]) and exchanges the
    /// code for a refresh token.
    pub fn finish(self, client: &Client, cancel: &AtomicBool) -> Result<String> {
        self.listener.set_nonblocking(true)?;
        let deadline = Instant::now() + SIGN_IN_TIMEOUT;
        let code = loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            if Instant::now() > deadline {
                return Err(Error::TimedOut);
            }
            match self.listener.accept() {
                Ok((stream, _)) => {
                    if let Some(result) = self.answer(stream) {
                        break result?;
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(err) => return Err(err.into()),
            }
        };
        exchange(client, &code, &self.redirect_uri, &self.verifier)
    }

    /// Reads one browser request. `None` for requests that are not the redirect (the favicon).
    fn answer(&self, mut stream: TcpStream) -> Option<Result<String>> {
        stream.set_nonblocking(false).ok()?;
        stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
        let mut line = String::new();
        BufReader::new(&stream).read_line(&mut line).ok()?;
        let target = line.split_whitespace().nth(1)?;
        let (path, query) = target.split_once('?').unwrap_or((target, ""));
        if path != "/" {
            let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
            return None;
        }
        let result = read_redirect(query, &self.state);
        let message = match &result {
            Ok(_) => "Falog can now read your Google calendars. You can close this tab.",
            Err(_) => "Falog could not connect to Google Calendar. Go back to Falog for details.",
        };
        let page = format!(
            "<!doctype html><meta charset=utf-8><title>Falog</title>\
             <body style=\"font:15px system-ui;background:#282c33;color:#dce0e5;display:grid;\
             place-items:center;height:90vh\"><p>{message}</p></body>"
        );
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n{page}",
            page.len()
        );
        Some(result)
    }
}

/// The authorization code from the redirect's query, after checking `state`.
fn read_redirect(query: &str, state: &str) -> Result<String> {
    let pairs = query_pairs(query);
    let get = |key: &str| pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str());
    if let Some(error) = get("error") {
        return Err(Error::Denied(match error {
            "access_denied" => "access was not granted".to_owned(),
            other => other.to_owned(),
        }));
    }
    if get("state") != Some(state) {
        return Err(Error::Denied("the answer did not match this sign-in".into()));
    }
    get("code")
        .filter(|code| !code.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| Error::Denied("Google sent no authorization code".into()))
}

fn exchange(client: &Client, code: &str, redirect_uri: &str, verifier: &str) -> Result<String> {
    let json = token_request(&[
        ("client_id", client.id.trim()),
        ("client_secret", client.secret.trim()),
        ("code", code),
        ("code_verifier", verifier),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_uri),
    ])
    .map_err(|err| match err {
        Error::Api { message, .. } => Error::Denied(message),
        other => other,
    })?;
    json.get("refresh_token")
        .and_then(|token| token.as_str())
        .map(str::to_owned)
        .ok_or_else(|| Error::Denied("Google sent no refresh token".into()))
}

/// A short-lived access token.
#[derive(Clone, Debug)]
pub struct AccessToken {
    pub token: String,
    pub expires_at: Instant,
}

impl AccessToken {
    /// Still valid for at least another minute.
    pub fn is_fresh(&self) -> bool {
        Instant::now() + Duration::from_secs(60) < self.expires_at
    }
}

/// Trades a refresh token for an access token. A revoked or expired refresh token becomes
/// [`Error::Reauthorize`] for `account`.
pub fn refresh(client: &Client, refresh_token: &str, account: &str) -> Result<AccessToken> {
    let json = token_request(&[
        ("client_id", client.id.trim()),
        ("client_secret", client.secret.trim()),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ])
    .map_err(|err| match err {
        Error::Api {
            status: 400 | 401, ..
        } => Error::Reauthorize(account.to_owned()),
        other => other,
    })?;
    let token = json
        .get("access_token")
        .and_then(|t| t.as_str())
        .ok_or_else(|| Error::Parse("no access_token in the token answer".into()))?;
    let lifetime = json.get("expires_in").and_then(|e| e.as_u64()).unwrap_or(3600);
    Ok(AccessToken {
        token: token.to_owned(),
        expires_at: Instant::now() + Duration::from_secs(lifetime),
    })
}

/// Tells Google to forget the grant. Best effort: the token is deleted locally either way.
pub fn revoke(refresh_token: &str) {
    let _ = ureq::post(REVOKE_URL)
        .timeout(Duration::from_secs(10))
        .send_form(&[("token", refresh_token)]);
}

fn token_request(form: &[(&str, &str)]) -> Result<serde_json::Value> {
    let body = match ureq::post(TOKEN_URL)
        .timeout(Duration::from_secs(30))
        .send_form(form)
    {
        Ok(response) => response.into_string()?,
        Err(ureq::Error::Status(status, response)) => {
            let body = response.into_string().unwrap_or_default();
            return Err(Error::Api {
                status,
                message: google_message(&body).unwrap_or(body),
            });
        }
        Err(err) => return Err(err.into()),
    };
    Ok(serde_json::from_str(&body)?)
}

fn random_token(bytes: usize) -> Result<String> {
    let mut buffer = vec![0u8; bytes];
    getrandom::fill(&mut buffer).map_err(|err| Error::Io(std::io::Error::other(err.to_string())))?;
    Ok(URL_SAFE_NO_PAD.encode(buffer))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> Client {
        Client {
            id: "123.apps.googleusercontent.com".into(),
            secret: "shh".into(),
        }
    }

    #[test]
    fn needs_a_client() {
        assert!(matches!(begin(&Client::default()), Err(Error::NoClient)));
    }

    #[test]
    fn builds_a_pkce_consent_url() {
        let pending = begin(&client()).unwrap();
        let url = pending.url();
        assert!(url.starts_with(AUTH_URL));
        assert!(url.contains("client_id=123.apps.googleusercontent.com"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains(&format!("redirect_uri={}", encode(&pending.redirect_uri))));
        assert!(url.contains("access_type=offline"));
        assert_eq!(pending.verifier.len(), 43);
    }

    #[test]
    fn reads_the_redirect() {
        assert_eq!(
            read_redirect("state=s1&code=4%2Fabc&scope=x", "s1").unwrap(),
            "4/abc"
        );
        assert!(matches!(
            read_redirect("state=other&code=c", "s1"),
            Err(Error::Denied(_))
        ));
        assert!(matches!(
            read_redirect("error=access_denied&state=s1", "s1"),
            Err(Error::Denied(message)) if message == "access was not granted"
        ));
    }

    #[test]
    fn finishes_when_the_browser_comes_back() {
        let pending = begin(&client()).unwrap();
        let port = pending.redirect_uri.rsplit(':').next().unwrap().to_owned();
        let state = pending.state.clone();
        let browser = std::thread::spawn(move || {
            let mut favicon = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
            favicon.write_all(b"GET /favicon.ico HTTP/1.1\r\n\r\n").unwrap();
            let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
            write!(stream, "GET /?error=access_denied&state={state} HTTP/1.1\r\n\r\n").unwrap();
            let mut answer = String::new();
            std::io::Read::read_to_string(&mut stream, &mut answer).unwrap();
            answer
        });
        let result = pending.finish(&client(), &AtomicBool::new(false));
        assert!(matches!(result, Err(Error::Denied(_))));
        assert!(browser.join().unwrap().contains("could not connect"));
    }

    #[test]
    fn can_be_cancelled() {
        let pending = begin(&client()).unwrap();
        let result = pending.finish(&client(), &AtomicBool::new(true));
        assert!(matches!(result, Err(Error::Cancelled)));
    }
}
