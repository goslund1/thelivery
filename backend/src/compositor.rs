use image::{imageops, DynamicImage, Rgba, RgbaImage};
use serde::Deserialize;
use std::path::Path;

pub const CANVAS_W: u32 = 1200;
pub const CANVAS_H: u32 = 630;

static FONT_POSTCARD: &[u8] = include_bytes!("../fonts/BebasNeue-Regular.ttf");
static FONT_SIGNAL: &[u8] = include_bytes!("../fonts/Oswald-VF.ttf");

#[derive(Deserialize, Clone, Debug)]
pub struct OgTextBox {
    pub style: String,
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

    for tb in &config.text_boxes {
        if tb.content.trim().is_empty() || tb.w <= 0.0 || tb.h <= 0.0 {
            continue;
        }
        let x_px = (tb.x * CANVAS_W as f32).round() as i32;
        let y_px = (tb.y * CANVAS_H as f32).round() as i32;
        let w_px = ((tb.w * CANVAS_W as f32).round() as u32).max(1);
        let h_px = ((tb.h * CANVAS_H as f32).round() as u32).max(1);

        let font_data = font_for_style(&tb.style);
        let text_img = rasterize_text_box(font_data, &tb.content, w_px, h_px, &tb.style);
        blit_transformed(&mut canvas, &text_img, x_px, y_px, tb.rotate_deg, tb.shear_x);
    }

    let mut buf = Vec::new();
    DynamicImage::ImageRgba8(canvas).write_to(
        &mut std::io::Cursor::new(&mut buf),
        image::ImageFormat::Png,
    )?;
    Ok(buf)
}

fn font_for_style(style: &str) -> &'static [u8] {
    match style {
        "POSTCARD" => FONT_POSTCARD,
        _ => FONT_SIGNAL,
    }
}

fn text_alpha_for_style(style: &str) -> u8 {
    match style {
        "GHOST" => 170,
        _ => 255,
    }
}

/// Rasterize `text` into an RGBA image of exactly `w_px × h_px` pixels.
/// Text is scaled to fill the box both horizontally and vertically —
/// horizontal stretch is intentional (same as CSS scaleX to fill a box).
fn rasterize_text_box(font_data: &[u8], text: &str, w_px: u32, h_px: u32, style: &str) -> RgbaImage {
    let alpha = text_alpha_for_style(style);

    let font = match fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default()) {
        Ok(f) => f,
        Err(_) => return RgbaImage::new(w_px, h_px),
    };

    // Use h_px as the initial px size — glyphs fill the height by default.
    let px = h_px as f32;

    let (nat_w, ascent, descent) = measure_text(&font, text, px);
    if nat_w <= 0.0 || (ascent - descent) <= 0.0 {
        return RgbaImage::new(w_px, h_px);
    }

    let img_h = (ascent - descent).ceil() as u32;
    let img_w = nat_w.ceil() as u32;
    let baseline = ascent.ceil() as i32;

    let mut nat_img = RgbaImage::new(img_w.max(1), img_h.max(1));

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
                if coverage == 0 {
                    continue;
                }
                let px_x = glyph_left + gx as i32;
                let px_y = glyph_top + gy as i32;
                if px_x >= 0
                    && px_x < nat_img.width() as i32
                    && px_y >= 0
                    && px_y < nat_img.height() as i32
                {
                    let a = ((coverage as u32 * alpha as u32) / 255) as u8;
                    nat_img.put_pixel(px_x as u32, px_y as u32, Rgba([255, 255, 255, a]));
                }
            }
        }
        cursor_x += metrics.advance_width;
    }

    // Scale the natural buffer to exactly w_px × h_px.
    // The horizontal stretch fills the box width — same as CSS transform: scaleX().
    // Clamp alpha after resize: Lanczos3 sinc ringing can push alpha above the
    // source max at glyph edges, which would break GHOST-style opacity.
    let mut scaled = imageops::resize(&nat_img, w_px, h_px, imageops::FilterType::Lanczos3);
    for p in scaled.pixels_mut() {
        p[3] = p[3].min(alpha);
    }
    scaled
}

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
        if top > ascent {
            ascent = top;
        }
        if bot < descent {
            descent = bot;
        }
    }
    (width, ascent, descent)
}

/// Composite `src` (a w×h RGBA image) onto `canvas` with its top-left
/// at (`box_x`, `box_y`), applying rotation around the box center and shear.
///
/// Uses inverse mapping: for each output pixel, compute where it maps
/// back to in the source image and sample from there.
fn blit_transformed(
    canvas: &mut RgbaImage,
    src: &RgbaImage,
    box_x: i32,
    box_y: i32,
    rotate_deg: f32,
    shear_x: f32,
) {
    let sw = src.width() as f32;
    let sh = src.height() as f32;
    if sw == 0.0 || sh == 0.0 {
        return;
    }

    // Box center in canvas space
    let cx = box_x as f32 + sw / 2.0;
    let cy = box_y as f32 + sh / 2.0;

    // Inverse rotation angle
    let angle = -rotate_deg.to_radians();
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    // Bounding box for the scan — use diagonal so rotation can't escape it
    let half_diag = (sw * sw + sh * sh).sqrt() / 2.0 + 1.0;
    let min_x = ((cx - half_diag).floor() as i32).max(0);
    let max_x = ((cx + half_diag).ceil() as i32).min(canvas.width() as i32 - 1);
    let min_y = ((cy - half_diag).floor() as i32).max(0);
    let max_y = ((cy + half_diag).ceil() as i32).min(canvas.height() as i32 - 1);

    for py in min_y..=max_y {
        for px_out in min_x..=max_x {
            // Translate so box center is origin
            let dx = px_out as f32 - cx;
            let dy = py as f32 - cy;

            // Inverse rotation
            let rx = cos_a * dx - sin_a * dy;
            let ry = sin_a * dx + cos_a * dy;

            // Inverse shear (forward shear: x' = x + shear*y, so inverse: x = x' - shear*y)
            let ux = rx - shear_x * ry;
            let uy = ry;

            // Translate back to box-local coords (origin = top-left of box)
            let u = ux + sw / 2.0;
            let v = uy + sh / 2.0;

            if u < 0.0 || v < 0.0 || u >= sw || v >= sh {
                continue;
            }

            let src_pixel = src.get_pixel(u as u32, v as u32);
            if src_pixel[3] == 0 {
                continue;
            }

            let dst = canvas.get_pixel(px_out as u32, py as u32);
            canvas.put_pixel(px_out as u32, py as u32, alpha_over(*dst, *src_pixel));
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_rasterize_no_panic() {
        let img = rasterize_text_box(FONT_POSTCARD, "SMOKIN", 600, 113, "POSTCARD");
        assert_eq!(img.width(), 600);
        assert_eq!(img.height(), 113);
        // At least some pixels should be non-transparent
        let has_content = img.pixels().any(|p| p[3] > 0);
        assert!(has_content, "rasterized text should produce visible pixels");
    }

    #[test]
    fn test_rasterize_ghost_alpha() {
        let img = rasterize_text_box(FONT_SIGNAL, "TEST", 300, 80, "GHOST");
        let max_alpha = img.pixels().map(|p| p[3]).max().unwrap_or(0);
        assert!(max_alpha <= 170, "GHOST style alpha should be capped at 170");
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
