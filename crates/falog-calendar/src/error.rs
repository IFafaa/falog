use std::path::PathBuf;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("add your Google OAuth client id and secret in Settings first")]
    NoClient,

    #[error("Google sign-in was cancelled")]
    Cancelled,

    #[error("Google sign-in timed out; try again")]
    TimedOut,

    #[error("Google sign-in failed: {0}")]
    Denied(String),

    /// The refresh token was revoked or expired: the account has to sign in again.
    #[error("{0} needs to sign in again")]
    Reauthorize(String),

    /// The account granted reading only: reconnecting it asks for the write permission.
    #[error(
        "{0} was connected for reading only; reconnect it in Settings › Calendar to let Falog create and change events"
    )]
    ReadOnly(String),

    #[error("Google answered {status}: {message}")]
    Api { status: u16, message: String },

    #[error("could not reach Google: {0}")]
    Network(String),

    #[error("unexpected answer from Google: {0}")]
    Parse(String),

    /// A calendar link that cannot be read. The message never contains the address: it is secret.
    #[error("{0}")]
    Link(String),

    #[error("could not use {path}: {source}")]
    File {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::Parse(err.to_string())
    }
}

impl From<ureq::Error> for Error {
    fn from(err: ureq::Error) -> Self {
        match err {
            ureq::Error::Status(status, response) => {
                let body = response.into_string().unwrap_or_default();
                Self::Api {
                    status,
                    message: google_message(&body).unwrap_or(body),
                }
            }
            ureq::Error::Transport(transport) => Self::Network(transport.to_string()),
        }
    }
}

/// The human part of a Google error body: `{"error": {"message": ...}}` for the APIs,
/// `{"error": "...", "error_description": ...}` for the token endpoint.
pub(crate) fn google_message(body: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    let error = json.get("error")?;
    let message = error
        .get("message")
        .or_else(|| json.get("error_description"))
        .or(Some(error))?;
    message.as_str().map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_google_error_shapes() {
        let api = r#"{"error": {"code": 403, "message": "Calendar API has not been used"}}"#;
        assert_eq!(google_message(api).unwrap(), "Calendar API has not been used");
        let token =
            r#"{"error": "invalid_grant", "error_description": "Token has been expired or revoked."}"#;
        assert_eq!(
            google_message(token).unwrap(),
            "Token has been expired or revoked."
        );
        let bare = r#"{"error": "invalid_client"}"#;
        assert_eq!(google_message(bare).unwrap(), "invalid_client");
        assert_eq!(google_message("<html>"), None);
    }
}
