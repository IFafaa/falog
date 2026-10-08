#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Falog desktop app.

mod action;
mod app;
mod assistant;
mod calendar;
mod components;
mod fonts;
mod icons;
mod overlays;
mod platform;
mod prefs;
mod theme;
mod views;
mod workspace;

use app::FalogApp;
use eframe::egui;
use falog_core::Store;
use platform::single_instance::{self, Claim};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

fn main() -> eframe::Result {
    // A database picked with FALOG_DB (demo data, tests) gets its own window next to the real one.
    let separate = std::env::var_os(falog_core::store::DB_PATH_ENV).is_some_and(|path| !path.is_empty());
    let listener = if separate {
        None
    } else {
        match single_instance::claim() {
            Claim::AlreadyRunning => return Ok(()),
            Claim::Primary(listener) => Some(listener),
            Claim::Unavailable => None,
        }
    };

    let store = Store::open_default();
    let show_requested = Arc::new(AtomicBool::new(false));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Falog")
            .with_app_id("falog")
            .with_inner_size(app::WINDOW_SIZE)
            .with_min_inner_size(app::MIN_WINDOW_SIZE)
            .with_icon(icons::app_icon()),
        // Next to the database instead of eframe's default (AppData on Windows, which packaged apps
        // redirect); `Store::open_default` above moved an existing file here.
        persistence_path: Some(falog_core::paths::data_home().join(falog_core::paths::PREFERENCES_FILE)),
        ..Default::default()
    };

    eframe::run_native(
        "Falog",
        options,
        Box::new(move |cc| {
            if let Some(listener) = listener {
                single_instance::listen(listener, cc.egui_ctx.clone(), show_requested.clone());
            }
            Ok(Box::new(FalogApp::new(cc, store, show_requested)))
        }),
    )
}
