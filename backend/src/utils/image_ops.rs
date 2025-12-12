use image::{
    DynamicImage, GenericImageView, ImageBuffer, Rgba,
    ImageOutputFormat, imageops::FilterType,
};
use rayon::prelude::*;
use std::io::Cursor;

// =====================================================================
// 1) Hitung posisi watermark (PURE FUNCTION)
// =====================================================================
pub fn compute_position(
    base_width: u32,
    base_height: u32,
    wm_width: u32,
    wm_height: u32,
    margin: u32,
) -> (u32, u32) {
    let x = base_width.saturating_sub(wm_width + margin);
    let y = base_height.saturating_sub(wm_height + margin);
    (x, y)
}

// =====================================================================
// 2) Hitung warna pixel hasil blending (PURE FUNCTION)
// =====================================================================
// Bertugas:
// - Mengecek apakah pixel (i, j) berada di area watermark
// - Jika iya → lakukan alpha blending
// - Jika tidak → kembalikan pixel asli
pub fn blend_pixel(
    base: &DynamicImage,
    watermark: &DynamicImage,
    i: u32,
    j: u32,
    pos_x: u32,
    pos_y: u32,
    opacity: f32,
) -> Rgba<u8> {

    let base_px = base.get_pixel(i, j);
    let (ww, wh) = watermark.dimensions();

    let is_inside_watermark =
        i >= pos_x && i < pos_x + ww &&
        j >= pos_y && j < pos_y + wh;

    if !is_inside_watermark {
        return base_px;
    }

    let wm_px = watermark.get_pixel(i - pos_x, j - pos_y);
    let alpha = (wm_px.0[3] as f32 * opacity) / 255.0;

    Rgba([
        (base_px.0[0] as f32 * (1.0 - alpha) + wm_px.0[0] as f32 * alpha) as u8,
        (base_px.0[1] as f32 * (1.0 - alpha) + wm_px.0[1] as f32 * alpha) as u8,
        (base_px.0[2] as f32 * (1.0 - alpha) + wm_px.0[2] as f32 * alpha) as u8,
        255,
    ])
}

// =====================================================================
// 3) Apply watermark (FULL IMMUTABLE, PURE)
// =====================================================================
pub fn apply_watermark(
    base: &DynamicImage,
    watermark: &DynamicImage,
    opacity: f32,
    margin: u32,
) -> DynamicImage {

    let (bw, bh) = base.dimensions();
    let (ww, wh) = watermark.dimensions();
    let (x, y) = compute_position(bw, bh, ww, wh, margin);

    let output = ImageBuffer::from_fn(bw, bh, |i, j| {
        blend_pixel(base, watermark, i, j, x, y, opacity)
    });

    DynamicImage::ImageRgba8(output)
}

// =====================================================================
// 4) Parallel processing (Rayon – MULTI THREAD CPU)
// =====================================================================
pub fn process_photos_parallel(
    photos: Vec<(String, Vec<u8>)>,
    wm_img: DynamicImage,
    opacity: f32,
) -> Vec<(String, Vec<u8>)> {

    photos
        .into_par_iter() // MULTI THREAD (Rayon)
        .map(|(filename, bytes)| {
            let img = image::load_from_memory(&bytes).unwrap();
            let (w, h) = img.dimensions();

            let wm_small = wm_img.resize(w / 4, h / 4, FilterType::Lanczos3);
            let result = apply_watermark(&img, &wm_small, opacity, 20);

            let out_bytes = {
                let mut buffer = Vec::new();
                result
                    .write_to(&mut Cursor::new(&mut buffer), ImageOutputFormat::Png)
                    .unwrap();
                buffer
            };

            (filename, out_bytes)
        })
        .collect()
}
