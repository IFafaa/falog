//! Rasterizes the Falog app icon from its SVG masters.
//!
//! ```text
//! cargo run --release --manifest-path tools/icon/Cargo.toml              # regenerate every icon file
//! cargo run --release --manifest-path tools/icon/Cargo.toml -- sheet OUT.png A.svg B.svg ...
//! ```
//!
//! The first form rewrites `crates/falog-desktop/assets/icon` (PNGs, `.ico`, `.icns`) and
//! `docs/icon-preview.png`. The second renders any SVGs side by side at small sizes on dark and light
//! backgrounds, which is how the concepts in `docs/icon-concepts` were compared.

use resvg::tiny_skia::{Color, FilterQuality, Paint, Pixmap, PixmapPaint, Rect, Transform};
use resvg::usvg::{self, TreeParsing};
use std::path::{Path, PathBuf};
use std::{env, fs, process};

/// Sizes exported as `png/falog-N.png` (Linux hicolor theme, macOS bundle, the window icon).
const PNG_SIZES: [u32; 9] = [16, 24, 32, 48, 64, 128, 256, 512, 1024];
/// Sizes in `falog.ico`: what Explorer, the Start menu and the taskbar ask for at 100–200 % scaling.
const ICO_SIZES: [u32; 9] = [16, 20, 24, 32, 40, 48, 64, 128, 256];
/// Sizes with a pixel-hinted copy of the master, `falog-N.svg`, drawn on that size's pixel grid.
const HINTED: [u32; 2] = [16, 24];

/// Windows 11 taskbar backgrounds, dark and light.
const DARK: [u8; 3] = [0x1c, 0x1c, 0x1c];
const LIGHT: [u8; 3] = [0xee, 0xee, 0xee];

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        None => build(),
        Some("sheet") if args.len() >= 3 => sheet(Path::new(&args[1]), &args[2..]),
        _ => Err("usage: falog-icon [sheet OUT.png SVG...]".into()),
    };
    if let Err(err) = result {
        eprintln!("error: {err}");
        process::exit(1);
    }
}

type Result<T = ()> = std::result::Result<T, String>;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The master and its pixel-hinted copies.
struct Icon {
    master: resvg::Tree,
    hinted: Vec<(u32, resvg::Tree)>,
}

impl Icon {
    fn render(&self, size: u32) -> Pixmap {
        let hinted = self.hinted.iter().find(|(hinted, _)| *hinted == size);
        render(hinted.map_or(&self.master, |(_, tree)| tree), size)
    }
}

fn build() -> Result {
    let dir = root().join("crates/falog-desktop/assets/icon");
    let icon = Icon {
        master: load(&dir.join("falog.svg"))?,
        hinted: HINTED
            .iter()
            .map(|&size| Ok((size, load(&dir.join(format!("falog-{size}.svg")))?)))
            .collect::<Result<_>>()?,
    };

    fs::create_dir_all(dir.join("png")).map_err(|e| e.to_string())?;
    for size in PNG_SIZES {
        write(
            &dir.join(format!("png/falog-{size}.png")),
            &png(&icon.render(size))?,
        )?;
    }

    let ico_images: Vec<_> = ICO_SIZES.iter().map(|&size| icon.render(size)).collect();
    write(&dir.join("falog.ico"), &ico(&ico_images)?)?;

    let mac = load_str(&macos_svg(&read(&dir.join("falog.svg"))?)?)?;
    write(&dir.join("falog.icns"), &icns(&mac)?)?;

    let preview = stack(&[
        strip(
            DARK,
            &|size| icon.render(size),
            &[16, 24, 32, 48, 64, 128, 256],
            &[(16, 8), (24, 5), (32, 4)],
        ),
        strip(
            LIGHT,
            &|size| icon.render(size),
            &[16, 24, 32, 48, 64, 128, 256],
            &[(16, 8), (24, 5), (32, 4)],
        ),
    ]);
    write(&root().join("docs/icon-preview.png"), &png(&preview)?)?;
    println!("wrote crates/falog-desktop/assets/icon and docs/icon-preview.png");
    Ok(())
}

fn sheet(out: &Path, svgs: &[String]) -> Result {
    let mut strips = Vec::new();
    for svg in svgs {
        let tree = load(Path::new(svg))?;
        for bg in [DARK, LIGHT] {
            strips.push(strip(
                bg,
                &|size| render(&tree, size),
                &[16, 24, 32, 48, 64],
                &[(16, 6), (24, 4), (32, 3)],
            ));
        }
    }
    write(out, &png(&stack(&strips))?)
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn write(path: &Path, bytes: &[u8]) -> Result {
    fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn load(path: &Path) -> Result<resvg::Tree> {
    load_str(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))
}

fn load_str(svg: &str) -> Result<resvg::Tree> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|e| e.to_string())?;
    Ok(resvg::Tree::from_usvg(&tree))
}

fn render(tree: &resvg::Tree, size: u32) -> Pixmap {
    // Invariant: every size used here is a non-zero constant.
    let mut pixmap = Pixmap::new(size, size).expect("non-zero icon size");
    let scale = size as f32 / tree.size.width();
    tree.render(Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    pixmap
}

fn png(pixmap: &Pixmap) -> Result<Vec<u8>> {
    pixmap.encode_png().map_err(|e| e.to_string())
}

/// Wraps the master's content in Apple's icon grid: a 824 px tile centered on a 1024 px canvas with a
/// soft drop shadow, which is what macOS expects since Big Sur (it does not mask or pad icons itself).
fn macos_svg(master: &str) -> Result<String> {
    let start = master
        .find("<svg")
        .and_then(|i| master[i..].find('>').map(|j| i + j + 1));
    let end = master.rfind("</svg>");
    let (Some(start), Some(end)) = (start, end) else {
        return Err("falog.svg: no <svg> element".into());
    };
    // The master's tile spans 8..248 of a 256 viewBox.
    let scale = 824.0 / 240.0;
    let offset = 100.0 - 8.0 * scale;
    Ok(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
  <defs><filter id="mac-shadow" x="-10%" y="-10%" width="120%" height="125%">
    <feGaussianBlur in="SourceAlpha" stdDeviation="12"/>
    <feOffset dy="12" result="blur"/>
    <feComponentTransfer><feFuncA type="linear" slope="0.35"/></feComponentTransfer>
    <feMerge><feMergeNode/><feMergeNode in="SourceGraphic"/></feMerge>
  </filter></defs>
  <g filter="url(#mac-shadow)"><g transform="translate({offset} {offset}) scale({scale})">{}</g></g>
</svg>"#,
        &master[start..end]
    ))
}

/// A multi-size `.ico`: 32-bit BMP entries below 256 px (understood by every Windows tool), PNG at 256.
fn ico(images: &[Pixmap]) -> Result<Vec<u8>> {
    let entries = images
        .iter()
        .map(|image| {
            if image.width() >= 256 {
                png(image)
            } else {
                Ok(ico_bmp(image))
            }
        })
        .collect::<Result<Vec<_>>>()?;
    let mut out = Vec::new();
    out.extend(0u16.to_le_bytes()); // reserved
    out.extend(1u16.to_le_bytes()); // type: icon
    out.extend((images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for (image, data) in images.iter().zip(&entries) {
        let side = if image.width() >= 256 {
            0
        } else {
            image.width() as u8
        }; // 0 means 256
        out.extend([side, side, 0, 0]); // width, height, palette size, reserved
        out.extend(1u16.to_le_bytes()); // color planes
        out.extend(32u16.to_le_bytes()); // bits per pixel
        out.extend((data.len() as u32).to_le_bytes());
        out.extend((offset as u32).to_le_bytes());
        offset += data.len();
    }
    for data in &entries {
        out.extend(data);
    }
    Ok(out)
}

/// A BITMAPINFOHEADER, bottom-up BGRA rows and an all-zero AND mask (alpha does the masking).
fn ico_bmp(image: &Pixmap) -> Vec<u8> {
    let (width, height) = (image.width(), image.height());
    let mask_stride = width.div_ceil(32) * 4;
    let mut out = Vec::new();
    out.extend(40u32.to_le_bytes());
    out.extend(width.to_le_bytes());
    out.extend((height * 2).to_le_bytes()); // color rows + mask rows
    out.extend(1u16.to_le_bytes());
    out.extend(32u16.to_le_bytes());
    out.extend([0u8; 24]); // no compression, default sizes, resolution and palette
    for y in (0..height).rev() {
        for x in 0..width {
            // Invariant: x and y are inside the pixmap.
            let pixel = image.pixel(x, y).expect("pixel inside the image").demultiply();
            out.extend([pixel.blue(), pixel.green(), pixel.red(), pixel.alpha()]);
        }
    }
    out.resize(out.len() + (mask_stride * height) as usize, 0);
    out
}

/// A macOS `.icns` with PNG entries at every size Finder and the Dock use, 1x and 2x.
fn icns(tree: &resvg::Tree) -> Result<Vec<u8>> {
    const ENTRIES: [(&[u8; 4], u32); 10] = [
        (b"icp4", 16),
        (b"icp5", 32),
        (b"ic11", 32),
        (b"icp6", 64),
        (b"ic12", 64),
        (b"ic07", 128),
        (b"ic08", 256),
        (b"ic13", 256),
        (b"ic09", 512),
        (b"ic14", 512),
    ];
    let mut body = Vec::new();
    for (kind, size) in ENTRIES.iter().copied().chain([(b"ic10", 1024)]) {
        let data = png(&render(tree, size))?;
        body.extend(kind);
        body.extend((data.len() as u32 + 8).to_be_bytes());
        body.extend(data);
    }
    let mut out = b"icns".to_vec();
    out.extend((body.len() as u32 + 8).to_be_bytes());
    out.extend(body);
    Ok(out)
}

fn color([r, g, b]: [u8; 3]) -> Color {
    Color::from_rgba8(r, g, b, 255)
}

/// One row of renders on a background: each size at 1:1, then `(size, zoom)` pairs magnified with
/// nearest-neighbor sampling so single pixels can be judged.
fn strip(bg: [u8; 3], render: &dyn Fn(u32) -> Pixmap, sizes: &[u32], zooms: &[(u32, u32)]) -> Pixmap {
    const GAP: u32 = 32;
    let tallest = sizes
        .iter()
        .copied()
        .chain(zooms.iter().map(|(size, zoom)| size * zoom))
        .max()
        .unwrap_or(0);
    let width = GAP
        + sizes.iter().map(|size| size + GAP).sum::<u32>()
        + zooms.iter().map(|(size, zoom)| size * zoom + GAP).sum::<u32>();
    let height = tallest + 2 * GAP;
    // Invariant: the strip always has at least one gap in each direction.
    let mut out = Pixmap::new(width, height).expect("non-zero strip size");
    out.fill(color(bg));

    let mut x = GAP;
    let items = sizes.iter().map(|&size| (size, 1)).chain(zooms.iter().copied());
    for (size, zoom) in items {
        let side = size * zoom;
        let y = GAP + (tallest - side) / 2;
        let paint = PixmapPaint {
            quality: FilterQuality::Nearest,
            ..PixmapPaint::default()
        };
        let transform = Transform::from_row(zoom as f32, 0.0, 0.0, zoom as f32, x as f32, y as f32);
        out.draw_pixmap(0, 0, render(size).as_ref(), &paint, transform, None);
        x += side + GAP;
    }
    out
}

/// Strips one under the other, left-aligned, with the leftover space filled by each strip's color.
fn stack(strips: &[Pixmap]) -> Pixmap {
    let width = strips.iter().map(Pixmap::width).max().unwrap_or(1);
    let height = strips.iter().map(Pixmap::height).sum::<u32>().max(1);
    // Invariant: width and height are at least 1.
    let mut out = Pixmap::new(width, height).expect("non-zero sheet size");
    let mut y = 0;
    for strip in strips {
        // The top-left pixel is always background.
        let bg = strip.pixel(0, 0).map(|p| p.demultiply());
        if let (Some(bg), Some(rect)) = (
            bg,
            Rect::from_xywh(0.0, y as f32, width as f32, strip.height() as f32),
        ) {
            let mut paint = Paint::default();
            paint.set_color_rgba8(bg.red(), bg.green(), bg.blue(), 255);
            out.fill_rect(rect, &paint, Transform::identity(), None);
        }
        out.draw_pixmap(
            0,
            y as i32,
            strip.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
        y += strip.height();
    }
    out
}
