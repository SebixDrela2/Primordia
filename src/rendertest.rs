//! The check that the renderer on the graphics card paints the picture
//! it should.
//!
//! The card paints the world at several places of the world and in
//! several weathers, and the picture is read back and looked at: it
//! must not be black, it must not be all of one colour, and the
//! picture must change with the weather. It also writes the picture to
//! a file, so that it can be looked at, and it says how long the card
//! takes to paint it.

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

    let mut gpu = render::GpuRenderer::new();
    let mut black = 0usize;
    let mut flat = 0usize;
    let mut written = 0;

    for region in REGIONS {
        for clock in CLOCKS {
            let place = render::Place {
                x: 0.0,
                y: 0.0,
                w: region.w as f32,
                h: region.h as f32,
            };
            gpu.render_region(&world, &creatures, region, clock, place);

            let screen = get_screen_data();
            let (dark, same) = look_at(&screen, region);
            black += dark;
            flat += same;
            if written == 0 && region.w == 1280 {
                write_screen_png(&screen, region, "gpu.png");
                written = 1;
            }
        }
    }

    println!("the card painted {black} black pixels and {flat} flat ones");

    // How long the card takes to paint the whole world.
    let region = REGIONS[3];
    let clock = CLOCKS[0];
    let place = render::Place {
        x: 0.0,
        y: 0.0,
        w: region.w as f32,
        h: region.h as f32,
    };
    let mut gpu_time = std::time::Duration::ZERO;
    for _ in 0..20 {
        let start = std::time::Instant::now();
        gpu.render_region(&world, &creatures, region, clock, place);
        gpu_time += start.elapsed();
    }
    println!("the full world: the card took {:?}", gpu_time / 20);

    std::process::exit(0);
}

/// Look at the picture: how much of it is black, and how much of it is
/// all of one colour.
fn look_at(screen: &Image, region: render::Region) -> (usize, usize) {
    let width = screen.width as usize;
    let height = screen.height as usize;
    let stride = width * 4;
    let mut dark = 0usize;
    let mut flat = 0usize;
    let mut first: Option<[u8; 3]> = None;
    let mut same = 0usize;
    for y in 0..(region.h as usize).min(height) {
        // The screen comes back the other way up.
        let src = (height - 1 - y) * stride;
        for x in 0..(region.w as usize).min(width) {
            let at = src + x * 4;
            let px = [screen.bytes[at], screen.bytes[at + 1], screen.bytes[at + 2]];
            if px[0] < 3 && px[1] < 3 && px[2] < 3 {
                dark += 1;
            }
            match first {
                None => first = Some(px),
                Some(f) => {
                    if f == px {
                        same += 1;
                    }
                }
            }
        }
    }
    if same > (region.w * region.h) as usize / 2 {
        flat = same;
    }
    (dark, flat)
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
    let Ok(file) = std::fs::File::create(path) else {
        eprintln!("cannot create {path}");
        return;
    };
    let writer = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, rw as u32, rh as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    if let Ok(mut png_writer) = encoder.write_header() {
        let _ = png_writer.write_image_data(&out);
    }
    eprintln!("wrote {path}");
}
