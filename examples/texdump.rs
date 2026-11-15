//! Write the atlas of the element tiles to a picture, so that it can be
//! looked at. The tiles are drawn six times as big, the way the game
//! draws them when the camera is close.

// The picture is put together by hand, so the casts are the ones the
// game does on purpose.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]

use primordia::render::textures;

fn main() {
    let bytes = textures::atlas();
    let scale = 6;
    let w = textures::ATLAS_W * scale;
    let h = textures::ATLAS_H * scale;
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let src = ((y / scale) * textures::ATLAS_W + (x / scale)) * 4;
            let dst = (y * w + x) * 4;
            // The shade is shown as it is, and the highlight is put on
            // top of it so that both can be seen.
            let shade = bytes[src] as f32 / 255.0;
            let hl = bytes[src + 1] as f32 / 255.0;
            let base = (shade * 1.3 + 0.45).min(1.0);
            let v = (base * 255.0).min(255.0);
            out[dst] = v as u8;
            out[dst + 1] = (v as f32 + hl * 120.0).min(255.0) as u8;
            out[dst + 2] = (v as f32 + hl * 120.0).min(255.0) as u8;
            out[dst + 3] = 255;
        }
    }
    let path = "atlas.png";
    let file = std::fs::File::create(path).expect("cannot create the picture");
    let writer = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, w as u32, h as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut png_writer = encoder.write_header().expect("cannot write the header");
    png_writer.write_image_data(&out).expect("cannot write the data");
    println!("wrote {path}");
}
