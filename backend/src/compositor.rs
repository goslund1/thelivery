use image::{imageops, DynamicImage, Rgba, RgbaImage};
use serde::Deserialize;
use std::path::Path;

pub const CANVAS_W: u32 = 1200;
pub const CANVAS_H: u32 = 630;

static FONT_BEBAS_NEUE:            &[u8] = include_bytes!("../fonts/BebasNeue-Regular.ttf");
static FONT_OSWALD:                &[u8] = include_bytes!("../fonts/Oswald-VF.ttf");
static FONT_CINZEL:                &[u8] = include_bytes!("../fonts/Cinzel-Bold.ttf");
static FONT_BLACK_OPS_ONE:         &[u8] = include_bytes!("../fonts/BlackOpsOne-Regular.ttf");
static FONT_ANTON:                 &[u8] = include_bytes!("../fonts/Anton-Regular.ttf");
static FONT_RACING_SANS_ONE:       &[u8] = include_bytes!("../fonts/RacingSansOne-Regular.ttf");
static FONT_ORBITRON:              &[u8] = include_bytes!("../fonts/Orbitron-Bold.ttf");
static FONT_GRADUATE:              &[u8] = include_bytes!("../fonts/Graduate-Regular.ttf");
static FONT_RUSSO_ONE:             &[u8] = include_bytes!("../fonts/RussoOne-Regular.ttf");
static FONT_BARLOW_CONDENSED:      &[u8] = include_bytes!("../fonts/BarlowCondensed-ExtraBold.ttf");
static FONT_AUDIOWIDE:             &[u8] = include_bytes!("../fonts/Audiowide-Regular.ttf");
static FONT_BIG_SHOULDERS_DISPLAY: &[u8] = include_bytes!("../fonts/BigShouldersDisplay-ExtraBold.ttf");

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TextStyle {
    Postcard,
    Signal,
    Ghost,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FontChoice {
    BebasNeue,
    Oswald,
    Cinzel,
    BlackOpsOne,
    Anton,
    RacingSansOne,
    Orbitron,
    Graduate,
    RussoOne,
    BarlowCondensed,
    Audiowide,
    BigShouldersDisplay,
}

#[derive(Deserialize, Clone, Debug)]
pub struct OgTextBox {
    pub style: TextStyle,
    #[serde(default)]
    pub font: Option<FontChoice>,
    pub content: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    #[serde(default)]
    pub rotate_deg: f32,
    #[serde(default)]
    pub shear_x: f32,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OgConfig {
    pub photo_id: i64,
    #[serde(default)]
    pub logo_visible: bool,
    #[serde(default)]
    pub text_boxes: Vec<OgTextBox>,
}

pub fn compose(photo_path: &Path, config: &OgConfig) -> anyhow::Result<Vec<u8>> {
    let photo = image::open(photo_path)?;
    let mut canvas = photo
        .resize_to_fill(CANVAS_W, CANVAS_H, imageops::FilterType::Lanczos3)
        .to_rgba8();

    // Bottom scrim: gradient if any visible box overlaps the lower half of the frame.
    let needs_scrim = config.text_boxes.iter().any(|tb| {
        !tb.content.trim().is_empty() && tb.w > 0.0 && tb.h > 0.0 && tb.y + tb.h > 0.45
    });
    if needs_scrim {
        apply_bottom_scrim(&mut canvas);
    }

    for tb in &config.text_boxes {
        if tb.content.trim().is_empty() || tb.w <= 0.0 || tb.h <= 0.0 {
            continue;
        }
        let x_px = (tb.x * CANVAS_W as f32).round() as i32;
        let y_px = (tb.y * CANVAS_H as f32).round() as i32;
        // Enforce a minimum rendered size so sub-pixel boxes don't produce garbage.
        let w_px = ((tb.w * CANVAS_W as f32).round() as u32).max(10);
        let h_px = ((tb.h * CANVAS_H as f32).round() as u32).max(8);

        // SIGNAL: dark backdrop plate (chyron / broadcast lower-third look).
        if tb.style == TextStyle::Signal {
            blit_backdrop(&mut canvas, x_px, y_px, w_px as i32, h_px as i32, tb.rotate_deg, tb.shear_x);
        }

        let font_data = font_for_box(&tb.style, &tb.font);
        let text_img = rasterize_text_box(font_data, &tb.content, w_px, h_px, &tb.style);

        // Drop shadow on POSTCARD and SIGNAL — GHOST stays intentionally delicate.
        if tb.style != TextStyle::Ghost {
            blit_transformed_tinted(
                &mut canvas, &text_img,
                x_px + 2, y_px + 3,
                tb.rotate_deg, tb.shear_x,
                Rgba([0, 0, 0, 150]),
            );
        }

        blit_transformed(&mut canvas, &text_img, x_px, y_px, tb.rotate_deg, tb.shear_x);
    }

    if config.logo_visible {
        render_logo(&mut canvas);
    }

    let mut buf = Vec::new();
    DynamicImage::ImageRgba8(canvas).write_to(
        &mut std::io::Cursor::new(&mut buf),
        image::ImageFormat::Png,
    )?;
    Ok(buf)
}

// ── Scrim & backdrop ──────────────────────────────────────────────────────────

/// Quadratic-ease dark gradient from y=45% → y=100%. Max alpha ≈ 63% at the
/// very bottom, enough to lift white text off any background.
fn apply_bottom_scrim(canvas: &mut RgbaImage) {
    let h = canvas.height() as f32;
    let w = canvas.width();
    let scrim_start = 0.45f32;

    for y in 0..canvas.height() {
        let yf = y as f32 / h;
        if yf < scrim_start {
            continue;
        }
        let t = (yf - scrim_start) / (1.0 - scrim_start); // 0→1 across scrim zone
        let scrim_alpha = (t * t * 160.0).round() as u8;  // quadratic ease, max 160
        for x in 0..w {
            let bg = *canvas.get_pixel(x, y);
            canvas.put_pixel(x, y, alpha_over(bg, Rgba([0, 0, 0, scrim_alpha])));
        }
    }
}

/// Dark semi-transparent plate behind a SIGNAL text box (same rotation/shear as text).
fn blit_backdrop(
    canvas: &mut RgbaImage,
    box_x: i32,
    box_y: i32,
    box_w: i32,
    box_h: i32,
    rotate_deg: f32,
    shear_x: f32,
) {
    let pad = 10i32;
    let pw = (box_w + pad * 2) as u32;
    let ph = (box_h + pad * 2) as u32;
    let mut plate = RgbaImage::new(pw.max(1), ph.max(1));
    for p in plate.pixels_mut() {
        *p = Rgba([0, 0, 0, 190]);
    }
    blit_transformed(canvas, &plate, box_x - pad, box_y - pad, rotate_deg, shear_x);
}

// ── Logo mark ─────────────────────────────────────────────────────────────────

/// Renders "THE LIVERY" in Bebas Neue at small size, bottom-right corner.
/// Placeholder until a real logo asset is embedded.
fn render_logo(canvas: &mut RgbaImage) {
    let logo = rasterize_text_natural(FONT_BEBAS_NEUE, "THE LIVERY", 22.0, 180);
    let margin = 24i32;
    let x = CANVAS_W as i32 - logo.width() as i32 - margin;
    let y = CANVAS_H as i32 - logo.height() as i32 - margin;
    // Shadow first, white text on top.
    blit_transformed_tinted(canvas, &logo, x + 1, y + 2, 0.0, 0.0, Rgba([0, 0, 0, 120]));
    blit_transformed(canvas, &logo, x, y, 0.0, 0.0);
}

// ── Font / style helpers ─────────────────────────────────────────────────────

fn font_for_box(style: &TextStyle, font: &Option<FontChoice>) -> &'static [u8] {
    if let Some(f) = font {
        return match f {
            FontChoice::BebasNeue         => FONT_BEBAS_NEUE,
            FontChoice::Oswald            => FONT_OSWALD,
            FontChoice::Cinzel            => FONT_CINZEL,
            FontChoice::BlackOpsOne       => FONT_BLACK_OPS_ONE,
            FontChoice::Anton             => FONT_ANTON,
            FontChoice::RacingSansOne     => FONT_RACING_SANS_ONE,
            FontChoice::Orbitron          => FONT_ORBITRON,
            FontChoice::Graduate          => FONT_GRADUATE,
            FontChoice::RussoOne          => FONT_RUSSO_ONE,
            FontChoice::BarlowCondensed   => FONT_BARLOW_CONDENSED,
            FontChoice::Audiowide         => FONT_AUDIOWIDE,
            FontChoice::BigShouldersDisplay => FONT_BIG_SHOULDERS_DISPLAY,
        };
    }
    // Style defaults
    match style {
        TextStyle::Postcard => FONT_BEBAS_NEUE,
        _ => FONT_OSWALD,
    }
}

fn text_alpha_for_style(style: &TextStyle) -> u8 {
    match style {
        TextStyle::Ghost => 170,
        _ => 255,
    }
}

// ── Rasterizers ──────────────────────────────────────────────────────────────

/// Inner glyph renderer. Returns the text at natural (unstretched) size, or
/// `None` if the text produces no measurable glyphs.
fn render_glyphs(font: &fontdue::Font, text: &str, px: f32, alpha: u8) -> Option<RgbaImage> {
    let (nat_w, ascent, descent) = measure_text(font, text, px);
    if nat_w <= 0.0 || (ascent - descent) <= 0.0 {
        return None;
    }
    let img_w = nat_w.ceil() as u32;
    let img_h = (ascent - descent).ceil() as u32;
    let baseline = ascent.ceil() as i32;
    let mut img = RgbaImage::new(img_w.max(1), img_h.max(1));
    let mut cursor_x = 0.0f32;
    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, px);
        if metrics.width == 0 || metrics.height == 0 {
            cursor_x += metrics.advance_width;
            continue;
        }
        let glyph_left = (cursor_x as i32) + metrics.xmin;
        let glyph_top = baseline - (metrics.ymin + metrics.height as i32);
        for gy in 0..metrics.height {
            for gx in 0..metrics.width {
                let coverage = bitmap[gy * metrics.width + gx];
                if coverage == 0 { continue; }
                let px_x = glyph_left + gx as i32;
                let px_y = glyph_top + gy as i32;
                if px_x >= 0 && px_x < img_w as i32 && px_y >= 0 && px_y < img_h as i32 {
                    let a = ((coverage as u32 * alpha as u32) / 255) as u8;
                    img.put_pixel(px_x as u32, px_y as u32, Rgba([255, 255, 255, a]));
                }
            }
        }
        cursor_x += metrics.advance_width;
    }
    Some(img)
}

/// Rasterize `text` into an RGBA image of exactly `w_px × h_px` pixels.
/// Text is scaled to fill the box both horizontally and vertically.
fn rasterize_text_box(font_data: &[u8], text: &str, w_px: u32, h_px: u32, style: &TextStyle) -> RgbaImage {
    let alpha = text_alpha_for_style(style);
    let font = match fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default()) {
        Ok(f) => f,
        Err(_) => return RgbaImage::new(w_px, h_px),
    };
    let nat_img = match render_glyphs(&font, text, h_px as f32, alpha) {
        Some(img) => img,
        None => return RgbaImage::new(w_px, h_px),
    };
    // Scale the natural buffer to exactly w_px × h_px.
    // Clamp alpha after resize: Lanczos3 sinc ringing can push alpha above the
    // source max at glyph edges, which would break GHOST-style opacity.
    let mut scaled = imageops::resize(&nat_img, w_px, h_px, imageops::FilterType::Lanczos3);
    for p in scaled.pixels_mut() {
        p[3] = p[3].min(alpha);
    }
    scaled
}

/// Rasterize `text` at natural (unstretched) width and the given `px` size.
/// Used for the logo mark and any other fixed-size glyphs.
fn rasterize_text_natural(font_data: &[u8], text: &str, px: f32, alpha: u8) -> RgbaImage {
    let font = match fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default()) {
        Ok(f) => f,
        Err(_) => return RgbaImage::new(1, 1),
    };
    render_glyphs(&font, text, px, alpha).unwrap_or_else(|| RgbaImage::new(1, 1))
}

// ── Text measurement ──────────────────────────────────────────────────────────

/// Returns (total_advance_width, ascent_above_baseline, descent_below_baseline)
/// where descent is negative (glyphs that dip below the baseline).
fn measure_text(font: &fontdue::Font, text: &str, px: f32) -> (f32, f32, f32) {
    let mut width = 0.0f32;
    let mut ascent = 0.0f32;
    let mut descent = 0.0f32;
    for ch in text.chars() {
        let m = font.metrics(ch, px);
        width += m.advance_width;
        let top = (m.ymin + m.height as i32) as f32;
        let bot = m.ymin as f32;
        if top > ascent { ascent = top; }
        if bot < descent { descent = bot; }
    }
    (width, ascent, descent)
}

// ── Blitters ─────────────────────────────────────────────────────────────────

/// Shared inverse-mapping pixel loop. For each output pixel that maps back into
/// `src`, calls `pixel_fn(dst, src_pixel)` and writes the result to canvas.
fn blit_with_transform(
    canvas: &mut RgbaImage,
    src: &RgbaImage,
    box_x: i32,
    box_y: i32,
    rotate_deg: f32,
    shear_x: f32,
    pixel_fn: impl Fn(Rgba<u8>, Rgba<u8>) -> Rgba<u8>,
) {
    let sw = src.width() as f32;
    let sh = src.height() as f32;
    if sw == 0.0 || sh == 0.0 { return; }

    let cx = box_x as f32 + sw / 2.0;
    let cy = box_y as f32 + sh / 2.0;
    let angle = -rotate_deg.to_radians();
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    let half_diag = (sw * sw + sh * sh).sqrt() / 2.0 + 1.0;
    let min_x = ((cx - half_diag).floor() as i32).max(0);
    let max_x = ((cx + half_diag).ceil()  as i32).min(canvas.width()  as i32 - 1);
    let min_y = ((cy - half_diag).floor() as i32).max(0);
    let max_y = ((cy + half_diag).ceil()  as i32).min(canvas.height() as i32 - 1);

    for py in min_y..=max_y {
        for px_out in min_x..=max_x {
            let dx = px_out as f32 - cx;
            let dy = py     as f32 - cy;
            let rx = cos_a * dx - sin_a * dy;
            let ry = sin_a * dx + cos_a * dy;
            // Inverse shear: x = x' - shear*y
            let u = rx - shear_x * ry + sw / 2.0;
            let v = ry + sh / 2.0;
            if u < 0.0 || v < 0.0 || u >= sw || v >= sh { continue; }
            let src_pixel = *src.get_pixel(u as u32, v as u32);
            if src_pixel[3] == 0 { continue; }
            let dst = *canvas.get_pixel(px_out as u32, py as u32);
            canvas.put_pixel(px_out as u32, py as u32, pixel_fn(dst, src_pixel));
        }
    }
}

/// Composite `src` onto `canvas` at (`box_x`, `box_y`) with rotation + shear.
fn blit_transformed(
    canvas: &mut RgbaImage,
    src: &RgbaImage,
    box_x: i32,
    box_y: i32,
    rotate_deg: f32,
    shear_x: f32,
) {
    blit_with_transform(canvas, src, box_x, box_y, rotate_deg, shear_x, alpha_over);
}

/// Same as `blit_transformed` but replaces each source pixel's colour with `tint`,
/// preserving the source alpha shape. Used for drop shadows and colour-cast effects.
fn blit_transformed_tinted(
    canvas: &mut RgbaImage,
    src: &RgbaImage,
    box_x: i32,
    box_y: i32,
    rotate_deg: f32,
    shear_x: f32,
    tint: Rgba<u8>,
) {
    blit_with_transform(canvas, src, box_x, box_y, rotate_deg, shear_x, |dst, src_px| {
        // Scale tint alpha by source alpha so glyph edges stay smooth.
        let a = ((src_px[3] as u32 * tint[3] as u32) / 255) as u8;
        alpha_over(dst, Rgba([tint[0], tint[1], tint[2], a]))
    });
}

// ── Porter-Duff ───────────────────────────────────────────────────────────────

fn alpha_over(dst: Rgba<u8>, src: Rgba<u8>) -> Rgba<u8> {
    let sa = src[3] as f32 / 255.0;
    let da = dst[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a < 1e-6 {
        return Rgba([0, 0, 0, 0]);
    }
    let blend = |s: u8, d: u8| -> u8 {
        ((s as f32 * sa + d as f32 * da * (1.0 - sa)) / out_a).round() as u8
    };
    Rgba([
        blend(src[0], dst[0]),
        blend(src[1], dst[1]),
        blend(src[2], dst[2]),
        (out_a * 255.0).round() as u8,
    ])
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_rasterize_no_panic() {
        let img = rasterize_text_box(FONT_BEBAS_NEUE, "SMOKIN", 600, 113, &TextStyle::Postcard);
        assert_eq!(img.width(), 600);
        assert_eq!(img.height(), 113);
        let has_content = img.pixels().any(|p| p[3] > 0);
        assert!(has_content, "rasterized text should produce visible pixels");
    }

    #[test]
    fn test_rasterize_ghost_alpha() {
        let img = rasterize_text_box(FONT_OSWALD, "TEST", 300, 80, &TextStyle::Ghost);
        let max_alpha = img.pixels().map(|p| p[3]).max().unwrap_or(0);
        assert!(max_alpha <= 170, "GHOST style alpha should be capped at 170, got {max_alpha}");
    }

    #[test]
    fn test_rasterize_natural_no_panic() {
        let img = rasterize_text_natural(FONT_BEBAS_NEUE, "THE LIVERY", 22.0, 180);
        assert!(img.width() > 0 && img.height() > 0);
        let has_content = img.pixels().any(|p| p[3] > 0);
        assert!(has_content, "logo text should produce visible pixels");
    }

    #[test]
    fn test_scrim_darkens_bottom() {
        let mut canvas = RgbaImage::from_pixel(CANVAS_W, CANVAS_H, Rgba([200, 200, 200, 255]));
        apply_bottom_scrim(&mut canvas);
        // Top row should be unmodified (above scrim_start = 45%)
        assert_eq!(canvas.get_pixel(600, 0)[0], 200);
        // Bottom row should be darker
        let bottom = canvas.get_pixel(600, CANVAS_H - 1)[0];
        assert!(bottom < 200, "bottom should be darker after scrim, got {bottom}");
    }

    #[test]
    fn test_compose_missing_photo_errors() {
        let config = OgConfig {
            photo_id: 0,
            logo_visible: false,
            text_boxes: vec![],
        };
        let result = compose(Path::new("/nonexistent/photo.jpg"), &config);
        assert!(result.is_err());
    }
}
