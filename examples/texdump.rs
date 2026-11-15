//! Write the pictures of the elements and of the creatures to files, so
//! that they can be looked at.
//!
//! The atlas of the elements is drawn six times as big, the way the
//! game draws it when the camera is close. The atlas of the creatures
//! is drawn four times as big.

// The picture is put together by hand, so the casts are the ones the
// game does on purpose.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::unnecessary_cast
)]

use primordia::render::{sprites, textures};

fn write_png(bytes: &[u8], w: usize, h: usize, path: &str) {
    let file = std::fs::File::create(path).expect("cannot create the picture");
    let writer = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, w as u32, h as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut png_writer = encoder.write_header().expect("cannot write the header");
    png_writer.write_image_data(bytes).expect("cannot write the data");
    println!("wrote {path}");
}

fn main() {
    // The atlas of the elements: the shade is shown as it is, and the
    // highlight is put on top of it so that both can be seen.
    let bytes = textures::atlas();
    let mut lo = 255u8;
    let mut hi = 0u8;
    let mut sum = 0u64;
    for i in (0..bytes.len()).step_by(4) {
        lo = lo.min(bytes[i]);
        hi = hi.max(bytes[i]);
        sum += bytes[i] as u64;
    }
    let count = (bytes.len() / 4) as u64;
    println!(
        "the atlas: the shade runs from {lo} to {hi}, the mean is {}",
        sum / count
    );
    let scale = 6;
    let bytes = textures::atlas();
    let w = textures::ATLAS_W * scale;
    let h = textures::ATLAS_H * scale;
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let src = ((y / scale) * textures::ATLAS_W + (x / scale)) * 4;
            let dst = (y * w + x) * 4;
            let shade = bytes[src] as f32 / 255.0;
            let hl = bytes[src + 1] as f32 / 255.0;
            // The shade runs from 0.45 to 1.75: show that whole range.
            let base = ((shade * 1.3 + 0.45) - 0.45) / 1.3;
            let v = (base * 255.0).min(255.0);
            out[dst] = v as u8;
            out[dst + 1] = (v as f32 + hl * 120.0).min(255.0) as u8;
            out[dst + 2] = (v as f32 + hl * 120.0).min(255.0) as u8;
            out[dst + 3] = 255;
        }
    }
    write_png(&out, w, h, "atlas.png");

    // The atlas of the creatures: in colour, the way the game draws
    // them.
    let scale = 4;
    let bytes = sprites::atlas();
    let w = sprites::ATLAS_W * scale;
    let h = sprites::ATLAS_H * scale;
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let src = ((y / scale) * sprites::ATLAS_W + (x / scale)) * 4;
            let dst = (y * w + x) * 4;
            let g = bytes[src] as f32 / 255.0;
            // A red body, so that the picture can be seen.
            out[dst] = (g * 235.0).min(255.0) as u8;
            out[dst + 1] = (g * 120.0).min(255.0) as u8;
            out[dst + 2] = (g * 90.0).min(255.0) as u8;
            out[dst + 3] = bytes[src + 3];
        }
    }
    write_png(&out, w, h, "sprites.png");
}
