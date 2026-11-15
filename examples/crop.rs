//! Cut a part of a picture out and make it bigger, so that it can be
//! looked at closely.
//!
//! `cargo run --release --example crop -- shot.png out.png x y w h [scale]`

// The picture is cut out by hand, so the casts are the ones the game
// does on purpose.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]

use std::io::BufReader;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 7 {
        eprintln!("crop <in> <out> <x> <y> <w> <h> [scale]");
        return;
    }
    let file = std::fs::File::open(&args[1]).expect("cannot open the picture");
    let reader = BufReader::new(file);
    let decoder = png::Decoder::new(reader);
    let mut reader = decoder.read_info().expect("cannot read the picture");
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("cannot read the frame");
    let width = info.width as usize;
    let height = info.height as usize;

    let x: usize = args[3].parse().unwrap_or(0);
    let y: usize = args[4].parse().unwrap_or(0);
    let w: usize = args[5].parse().unwrap_or(16);
    let h: usize = args[6].parse().unwrap_or(16);
    let scale: usize = args.get(7).and_then(|s| s.parse().ok()).unwrap_or(8);

    let out_w = w * scale;
    let out_h = h * scale;
    let mut out = vec![0u8; out_w * out_h * 4];
    for py in 0..out_h {
        for px in 0..out_w {
            let sx = (x + px / scale).min(width - 1);
            let sy = (y + py / scale).min(height - 1);
            let src = (sy * width + sx) * 4;
            let dst = (py * out_w + px) * 4;
            out[dst..dst + 4].copy_from_slice(&buf[src..src + 4]);
        }
    }

    let file = std::fs::File::create(&args[2]).expect("cannot create the picture");
    let writer = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, out_w as u32, out_h as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut png_writer = encoder.write_header().expect("cannot write the header");
    png_writer.write_image_data(&out).expect("cannot write the data");
    println!("wrote {} ({}x{})", args[2], out_w, out_h);
}
