use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, ImageOutputFormat, imageops::FilterType};
use rayon::prelude::*;
use std::io::Cursor;

// =====================================================================
// 1) Hitung posisi watermark
// =====================================================================
// Menghitung koordinat (x,y) untuk menempatkan watermark di pojok kanan bawah
// dengan jarak 'margin' dari tepi foto
pub fn compute_position(
    base_width: u32,
    base_height: u32,
    wm_width: u32,
    wm_height: u32,
    margin: u32,
) -> (u32, u32) {
    // Hitung posisi X dan Y menggunakan saturating_sub (pengurangan aman tanpa overflow)
    // Rumus: posisi = ukuran_foto - ukuran_watermark - margin
    let x = base_width.saturating_sub(wm_width + margin);
    let y = base_height.saturating_sub(wm_height + margin);
    (x, y)
}

// =====================================================================
// 2) Watermark Overlay (alpha blending manual)
// =====================================================================
// Menggabungkan watermark ke foto dengan efek transparansi (alpha blending)
pub fn apply_watermark(
    base: &DynamicImage,
    watermark: &DynamicImage,
    opacity: f32,
    margin: u32,
) -> DynamicImage {
    // Ambil dimensi foto dan watermark, lalu hitung posisi penempatan
    let (bw, bh) = base.dimensions();
    let (ww, wh) = watermark.dimensions();
    let (x, y) = compute_position(bw, bh, ww, wh, margin);

    // Buat image baru dengan memproses setiap pixel
    // Pixel di area watermark: blend dengan alpha, pixel lainnya: copy asli
    let output = ImageBuffer::from_fn(bw, bh, |i, j| {
        // Cek apakah pixel ini dalam area watermark
        if i >= x && i < x + ww && j >= y && j < y + wh {
            // Ambil pixel watermark dan foto, lalu blend dengan formula alpha
            let wp = watermark.get_pixel(i - x, j - y);
            let bp = base.get_pixel(i, j);
            
            // Hitung transparansi dan blend warna: hasil = foto*(1-alpha) + watermark*alpha
            let alpha = (wp.0[3] as f32 * opacity) / 255.0;
            
            Rgba([
                (bp.0[0] as f32 * (1.0 - alpha) + wp.0[0] as f32 * alpha) as u8,
                (bp.0[1] as f32 * (1.0 - alpha) + wp.0[1] as f32 * alpha) as u8,
                (bp.0[2] as f32 * (1.0 - alpha) + wp.0[2] as f32 * alpha) as u8,
                255,
            ])
        } else {
            base.get_pixel(i, j)
        }
    });

    // Convert ke DynamicImage untuk kompatibilitas dengan library
    DynamicImage::ImageRgba8(output)
}

// =====================================================================
// 3) Parallel processing untuk banyak foto
// =====================================================================
// Memproses banyak foto secara paralel menggunakan Rayon (multithreading otomatis)
pub fn process_photos_parallel(
    photos: Vec<(String, Vec<u8>)>,
    wm_img: DynamicImage,
    opacity: f32,
) -> Vec<(String, Vec<u8>)> 
{
    photos
        // into_par_iter: bagi pekerjaan ke banyak thread secara otomatis
        .into_par_iter()
        .map(|(filename, bytes)| {
            // Decode foto dari bytes, resize watermark agar proporsional (1/4 ukuran foto)
            let img = image::load_from_memory(&bytes).unwrap();
            let (w, h) = img.dimensions();
            
            let wm_small = wm_img.resize(w / 4, h / 4, FilterType::Lanczos3);
            
            // Tempelkan watermark ke foto dengan margin 20px
            let result_img = apply_watermark(&img, &wm_small, opacity, 20);

            // Encode hasil ke PNG format dan return sebagai bytes
            // mut: Buffer perlu diisi bertahap saat encoding gambar ke format PNG (header -> data -> footer)
            let mut out_bytes = Vec::new();
            result_img
                .write_to(&mut Cursor::new(&mut out_bytes), ImageOutputFormat::Png)
                .unwrap();

            (filename, out_bytes)
        })
        // collect: tunggu semua thread selesai dan kumpulkan hasilnya
        .collect()
}
