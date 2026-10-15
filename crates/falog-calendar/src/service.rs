//! What the desktop app and the MCP server share to talk to Google for a connected account:
//! access tokens kept in memory and refreshed when they expire, reading an account's calendars and
//! events, and creating, changing or deleting events.

use crate::config::{Account, Client};
use crate::google::{self, EventPatch, NewEvent};
use crate::model::{Calendar, Event};
use crate::oauth::{self, AccessToken};
use crate::{Error, Result};
use chrono::NaiveDate;
use std::collections::HashMap;
use std::sync::Mutex;

/// Access tokens by account email. They last about an hour; refresh tokens stay in the config.
#[derive(Debug, Default)]
pub struct Tokens(Mutex<HashMap<String, AccessToken>>);

impl Tokens {
    /// A valid access token for `account`, refreshing it when needed.
    pub fn access_token(&self, client: &Client, account: &Account) -> Result<String> {
        if let Some(token) = self
            .0
            .lock()
            .ok()
            .and_then(|tokens| tokens.get(&account.email).filter(|t| t.is_fresh()).cloned())
        {
            return Ok(token.token);
        }
        let fresh = oauth::refresh(client, &account.refresh_token, &account.email)?;
        let token = fresh.token.clone();
        self.insert(&account.email, fresh);
        Ok(token)
    }

    pub fn insert(&self, email: &str, token: AccessToken) {
        if let Ok(mut tokens) = self.0.lock() {
            tokens.insert(email.to_owned(), token);
        }
    }

    pub fn forget(&self, email: &str) {
        if let Ok(mut tokens) = self.0.lock() {
            tokens.remove(email);
        }
    }
}

/// The account's current calendar list (keeping the user's show/hide choices) and the events of its
/// visible calendars touching `from..to`.
pub fn fetch_account(
    client: &Client,
    account: &Account,
    tokens: &Tokens,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<(Vec<Calendar>, Vec<Event>)> {
    let token = tokens.access_token(client, account)?;
    let mut calendars = google::calendars(&token)?;
    for calendar in &mut calendars {
        if let Some(old) = account.calendars.iter().find(|c| c.id == calendar.id) {
            calendar.visible = old.visible;
        }
    }
    let mut events = Vec::new();
    for calendar in calendars.iter().filter(|c| c.visible) {
        events.extend(google::events(&token, &account.email, &calendar.id, from, to)?);
    }
    Ok((calendars, events))
}

/// Creates an event in one of the account's calendars.
pub fn create_event(
    client: &Client,
    account: &Account,
    tokens: &Tokens,
    calendar_id: &str,
    event: &NewEvent,
) -> Result<Event> {
    let token = writer(client, account, tokens)?;
    google::insert_event(&token, &account.email, calendar_id, event).map_err(|err| read_only(err, account))
}

pub fn update_event(
    client: &Client,
    account: &Account,
    tokens: &Tokens,
    calendar_id: &str,
    event_id: &str,
    patch: &EventPatch,
) -> Result<Event> {
    let token = writer(client, account, tokens)?;
    google::patch_event(&token, &account.email, calendar_id, event_id, patch)
        .map_err(|err| read_only(err, account))
}

pub fn delete_event(
    client: &Client,
    account: &Account,
    tokens: &Tokens,
    calendar_id: &str,
    event_id: &str,
) -> Result<()> {
    let token = writer(client, account, tokens)?;
    google::delete_event(&token, calendar_id, event_id).map_err(|err| read_only(err, account))
}

/// An access token for writing, or [`Error::ReadOnly`] for accounts connected with reading only.
fn writer(client: &Client, account: &Account, tokens: &Tokens) -> Result<String> {
    if !account.can_write {
        return Err(Error::ReadOnly(account.email.clone()));
    }
    tokens.access_token(client, account)
}

/// Google answers 403 "insufficient authentication scopes" when the grant lacks writing, for
/// instance after the user unticked it on the consent screen.
fn read_only(err: Error, account: &Account) -> Error {
    match err {
        Error::Api { status: 403, message } if message.to_lowercase().contains("insufficient") => {
            Error::ReadOnly(account.email.clone())
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(can_write: bool) -> Account {
        Account {
            email: "me@example.com".into(),
            refresh_token: "t".into(),
            calendars: Vec::new(),
            needs_sign_in: false,
            can_write,
            area: None,
        }
    }

    #[test]
    fn read_only_accounts_do_not_try_to_write() {
        let result = create_event(
            &Client::default(),
            &account(false),
            &Tokens::default(),
            "primary",
            &NewEvent::default(),
        );
        assert!(matches!(result, Err(Error::ReadOnly(email)) if email == "me@example.com"));
    }

    #[test]
    fn read_only_accounts_neither_change_nor_delete() {
        let tokens = Tokens::default();
        let patch = EventPatch::default();
        let (client, account) = (Client::default(), account(false));
        let changed = update_event(&client, &account, &tokens, "primary", "e1", &patch);
        assert!(matches!(changed, Err(Error::ReadOnly(_))));
        let deleted = delete_event(&client, &account, &tokens, "primary", "e1");
        assert!(matches!(deleted, Err(Error::ReadOnly(_))));
    }

    #[test]
    fn reuses_a_fresh_access_token_and_forgets_it() {
        let tokens = Tokens::default();
        let account = account(true);
        tokens.insert(
            &account.email,
            AccessToken {
                token: "cached".into(),
                expires_at: std::time::Instant::now() + std::time::Duration::from_secs(3600),
            },
        );
        // A fresh token is used as is: with no OAuth client, refreshing it would fail.
        let token = tokens.access_token(&Client::default(), &account);
        assert_eq!(token.unwrap(), "cached");
        tokens.forget(&account.email);
        assert!(tokens.0.lock().unwrap().is_empty());
    }

    #[test]
    fn only_a_forbidden_scope_error_means_read_only() {
        let api = |status: u16, message: &str| Error::Api {
            status,
            message: message.into(),
        };
        let shouting = api(403, "INSUFFICIENT PERMISSION");
        assert!(matches!(read_only(shouting, &account(true)), Error::ReadOnly(_)));
        let other_status = api(401, "insufficient authentication scopes");
        assert!(matches!(
            read_only(other_status, &account(true)),
            Error::Api { status: 401, .. }
        ));
        let network = Error::Network("timed out".into());
        assert!(matches!(read_only(network, &account(true)), Error::Network(_)));
    }

    #[test]
    fn names_a_missing_write_scope() {
        let scopes = Error::Api {
            status: 403,
            message: "Request had insufficient authentication scopes.".into(),
        };
        assert!(matches!(read_only(scopes, &account(true)), Error::ReadOnly(_)));
        let other = Error::Api {
            status: 403,
            message: "Calendar usage limits exceeded.".into(),
        };
        assert!(matches!(read_only(other, &account(true)), Error::Api { .. }));
    }
}
