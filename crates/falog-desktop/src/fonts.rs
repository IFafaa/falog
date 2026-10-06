//! Bundled typefaces, the same ones Zed ships: IBM Plex Sans for UI and Lilex for monospace.
//! Both are licensed under the SIL Open Font License (see `assets/fonts`).

use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontId};

/// Font family name for IBM Plex Sans SemiBold.
pub const SEMIBOLD: &str = "plex-semibold";

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let mut add = |name: &str, bytes: &'static [u8]| {
        fonts
            .font_data
            .insert(name.to_owned(), FontData::from_static(bytes));
    };
    add("plex", include_bytes!("../assets/fonts/IBMPlexSans-Regular.ttf"));
    add(
        SEMIBOLD,
        include_bytes!("../assets/fonts/IBMPlexSans-SemiBold.ttf"),
    );
    add("lilex", include_bytes!("../assets/fonts/Lilex-Regular.ttf"));

    // Keep egui's defaults as fallbacks for glyphs these fonts lack.
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, "plex".to_owned());
    let fallbacks = proportional[1..].to_vec();
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "lilex".to_owned());
    fonts.families.insert(
        FontFamily::Name(SEMIBOLD.into()),
        [vec![SEMIBOLD.to_owned()], fallbacks].concat(),
    );

    ctx.set_fonts(fonts);
}
