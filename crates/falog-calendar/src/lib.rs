//! Read-only Google Calendar access for Falog.
//!
//! Signing in follows Google's flow for installed apps: the browser opens the consent page, Google
//! redirects to a one-shot listener on `127.0.0.1` ([`oauth`]), and the refresh token is kept in
//! [`config`]. [`google`] lists calendars and events over the REST API, and [`layout`] holds the
//! pure rules the week view uses to place events. Nothing here knows about the UI.

pub mod config;
mod error;
pub mod google;
pub mod layout;
pub mod model;
pub mod oauth;
mod url;

pub use config::{Account, CalendarConfig, Client, EventCache, Files};
pub use error::{Error, Result};
pub use model::{Calendar, Event, EventTime};
