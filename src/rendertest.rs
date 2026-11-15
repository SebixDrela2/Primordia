//! The check that the renderer on the graphics card paints the very
//! same picture as the renderer on the processor.
//!
//! Both renderers are given the same world and the same clock, one
//! paints into a buffer and the other paints onto the screen, and the
//! two are then compared pixel by pixel. The clock is fixed, so the
//! pictures are the pictures and not two moments of the same picture.

use macroquad::prelude::*;
use primordia::creature::CreatureSystem;
use primordia::render;
use primordia::world::{World, Weather};
use primordia::{H, W};

/// The windows of the world that are looked at.
const REGIONS: [render::Region; 4] = [
    render::Region {
        x: 0,
        y: 0,
        w: 480,
        h: 270,
    },
    render::Region {
        x: 400,
        y: 300,
        w: 480,
        h: 270,
    },
    render::Region {
        x: 800,
        y: 450,
        w: 480,
        h: 270,
    },
    render::Region {
        x: 0,
        y: 0,
        w: 1280,
        h: 720,
    },
];

/// The moments of the day and the kinds of weather that are looked at.
const CLOCKS: [render::Clock; 6] = [
    render::Clock {
        time: 12.0,
        day_night: 0.25,
        season: 0,
        weather: Weather::Clear,
    },
    render::Clock {
        time: 30.5,
        day_night: 0.75,
        season: 1,
        weather: Weather::Rain,
    },
    render::Clock {
        time: 3.25,
        day_night: 0.05,
        season: 2,
        weather: Weather::Fog,
    },
    render::Clock {
        time: 44.0,
        day_night: 0.5,
        season: 3,
        weather: Weather::Snow,
    },
    render::Clock {
        time: 7.5,
        day_night: 0.9,
        season: 1,
        weather: Weather::Storm,
    },
    render::Clock {
        time: 19.0,
        day_night: 0.15,
        season: 0,
        weather: Weather::Heatwave,
    },
];

/// Run the check. It ends the process when it is done.
pub fn run(args: &[String]) {
    let seed = args
        .first()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(4242);

    let mut world = World::new(W, H, seed);
    world.seed();
    let mut creatures = CreatureSystem::new(seed);
    creatures.populate(&world, 64);
    for _ in 0..900 {
        world.step();
        creatures.step(&mut world);
    }

    let mut cpu = render::CpuRenderer::new();
    let mut gpu = render::GpuRenderer::new();

    let mut total_wrong = 0usize;
    let mut total_pixels = 0usize;
    let mut worst = 0i32;
    let mut texture_wrong = 0usize;

    for region in REGIONS {
        let mut buffer = vec![0u8; (region.w * region.h * 4) as usize];
        for clock in CLOCKS {
            cpu.render_region(&world, &creatures, &mut buffer, region, clock);

            // First the picture of the processor, put on the screen as
            // a texture, the way the game puts it there. This shows
            // whether the screen is read back as it was drawn. It goes
            // below the picture of the card, so that it cannot hide it.
            let image = Image {
                bytes: buffer.clone(),
                width: region.w as u16,
                height: region.h as u16,
            };
            let texture = Texture2D::from_image(&image);
            texture.set_filter(FilterMode::Nearest);
            draw_texture_ex(
                &texture,
                0.0,
                region.h as f32,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(region.w as f32, region.h as f32)),
                    ..Default::default()
                },
            );
            let screen = get_screen_data();
            let (wrong, _, _) = compare(&screen, &buffer, region, region.h);
            texture_wrong += wrong;

            // Then the picture of the card, painted straight onto the
            // screen, above the picture of the processor.
            let place = render::Place {
                x: 0.0,
                y: 0.0,
                w: region.w as f32,
                h: region.h as f32,
            };
            gpu.render_region(&world, &creatures, region, clock, place);

            let screen = get_screen_data();
            let (wrong, pixels, delta) = compare(&screen, &buffer, region, 0);
            if wrong > 0 {
                write_png(&buffer, region.w as usize, region.h as usize, "cpu.png");
                write_screen_png(&screen, region, "gpu.png");
                report(&world, &buffer, &screen, region, clock, wrong, pixels, delta);
            }
            total_wrong += wrong;
            total_pixels += pixels;
            worst = worst.max(delta);
        }
    }

    println!("the picture of the processor, read back, had {texture_wrong} wrong channels");
    println!(
        "the card painted {total_wrong} of {total_pixels} pixels differently, by up to {worst}"
    );

    // How long the two ways of painting take.
    time_both(&world, &creatures, &mut cpu, &mut gpu);

    if total_wrong == 0 {
        println!("the two renderers paint the same picture");
    }
    std::process::exit(0);
}

/// How long the two ways of painting the whole world take.
fn time_both(
    world: &World,
    creatures: &CreatureSystem,
    cpu: &mut render::CpuRenderer,
    gpu: &mut render::GpuRenderer,
) {
    let region = REGIONS[3];
    let mut buffer = vec![0u8; (region.w * region.h * 4) as usize];
    let clock = CLOCKS[0];
    let place = render::Place {
        x: 0.0,
        y: 0.0,
        w: region.w as f32,
        h: region.h as f32,
    };
    let mut cpu_time = std::time::Duration::ZERO;
    let mut gpu_time = std::time::Duration::ZERO;
    for _ in 0..20 {
        let start = std::time::Instant::now();
        cpu.render_region(world, creatures, &mut buffer, region, clock);
        cpu_time += start.elapsed();
        let start = std::time::Instant::now();
        gpu.render_region(world, creatures, region, clock, place);
        gpu_time += start.elapsed();
    }
    println!(
        "the full world: the processor took {:?}, the card took {:?}",
        cpu_time / 20,
        gpu_time / 20
    );
}

/// Say which cells came out different, so that the picture of the card
/// and the picture of the processor can be lined up.
#[allow(clippy::too_many_arguments)]
fn report(
    world: &World,
    buffer: &[u8],
    screen: &Image,
    region: render::Region,
    clock: render::Clock,
    wrong: usize,
    pixels: usize,
    delta: i32,
) {
    let sw = screen.width as usize;
    let sh = screen.height as usize;
    let at = |x: usize, y: usize| -> [u8; 4] {
        let row = (sh - 1 - y) * sw * 4 + x * 4;
        [
            screen.bytes[row],
            screen.bytes[row + 1],
            screen.bytes[row + 2],
            screen.bytes[row + 3],
        ]
    };
    let table = primordia::render::fx::fx_table();
    let mut shown = 0;
    for y in 0..region.h as usize {
        for x in 0..region.w as usize {
            let cpu = &buffer[(y * region.w as usize + x) * 4..][..4];
            let gpu = at(x, y);
            if cpu != gpu && shown < 4 {
                let wx = region.x as usize + x;
                let wy = region.y as usize + y;
                let kind = world.kind[wy * W + wx];
                let depth = (wy as f32 - world.ground[wx] as f32).max(0.0);
                println!(
                    "  ({x},{y}) kind {kind} fx {:?} depth {depth:.3} ground {} col {:?} cpu {cpu:?} gpu {gpu:?}",
                    table[kind as usize].kind as u8,
                    world.ground[wx],
                    world.col[wy * W + wx],
                );
                shown += 1;
            }
        }
    }
    println!(
        "region {}x{} at {},{} time {:.1}: {wrong} of {pixels} pixels differ, by up to {delta}",
        region.w, region.h, region.x, region.y, clock.time
    );
}

/// Write a picture to a file, so that it can be looked at.
///
/// The bytes are given the way up, so they are written as they are.
fn write_png(bytes: &[u8], w: usize, h: usize, path: &str) {
    let file = match std::fs::File::create(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("cannot create {path}: {e}");
            return;
        }
    };
    let writer = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, w as u32, h as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    if let Ok(mut png_writer) = encoder.write_header() {
        let _ = png_writer.write_image_data(bytes);
    }
    eprintln!("wrote {path}");
}

/// Write the picture of the card to a file. The screen comes back the
/// other way up and it is the whole screen, so the part that holds the
/// picture is cut out of it and turned the right way up.
fn write_screen_png(screen: &Image, region: render::Region, path: &str) {
    let sw = screen.width as usize;
    let sh = screen.height as usize;
    let rw = region.w as usize;
    let rh = region.h as usize;
    let stride = sw * 4;
    let mut out = vec![0u8; rw * rh * 4];
    for y in 0..rh.min(sh) {
        let src = (sh - 1 - y) * stride;
        out[y * rw * 4..(y + 1) * rw * 4].copy_from_slice(&screen.bytes[src..src + rw * 4]);
    }
    write_png(&out, rw, rh, path);
}

/// Compare a part of the picture on the screen with the picture in the
/// buffer. The part starts at the given row of the screen.
fn compare(
    screen: &Image,
    buffer: &[u8],
    region: render::Region,
    top: i32,
) -> (usize, usize, i32) {
    let width = screen.width as usize;
    let height = screen.height as usize;
    let stride = width * 4;
    let mut wrong = 0usize;
    let mut pixels = 0usize;
    let mut worst = 0i32;

    for y in 0..region.h as usize {
        let screen_y = top as usize + y;
        if screen_y >= height {
            break;
        }
        // The screen comes back the other way up.
        let src = (height - 1 - screen_y) * stride;
        let dst = y * region.w as usize * 4;
        for x in 0..region.w as usize {
            if x >= width {
                break;
            }
            pixels += 1;
            for c in 0..4 {
                let a = screen.bytes[src + x * 4 + c] as i32;
                let b = buffer[dst + x * 4 + c] as i32;
                if a != b {
                    wrong += 1;
                    worst = worst.max((a - b).abs());
                }
            }
        }
    }
    (wrong, pixels, worst)
}
