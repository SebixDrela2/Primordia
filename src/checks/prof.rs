//! The profiler: what a frame costs.
//!
//! The world, the creatures and the renderer are measured apart, so
//! that the one that is too slow can be found.

use crate::creature::CreatureSystem;
use crate::render::{GpuRenderer, Place, Region};
use crate::world::World;
use macroquad::prelude::*;
use std::time::Instant;

#[must_use]
pub fn run(_args: &[String]) -> i32 {
    let seed = 4242u64;
    let mut w = World::new(1280, 720, seed);
    w.seed();
    let mut cs = CreatureSystem::new(seed);
    cs.populate(&w, 64);
    for _ in 0..900 {
        w.step();
        cs.step(&mut w);
    }
    let pop = cs.list.len();

    let n = 1200;
    let t = Instant::now();
    for _ in 0..n {
        w.step();
    }
    let world_ms = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
    let t = Instant::now();
    for _ in 0..n {
        cs.step(&mut w);
    }
    let creature_ms = t.elapsed().as_secs_f64() * 1000.0 / n as f64;

    // The renderer is measured on the region that is actually on the
    // screen, because that is the number that has to fit inside a
    // frame.
    let mut gpu = GpuRenderer::new();
    let region = Region {
        x: 400,
        y: 300,
        w: 480,
        h: 270,
    };
    let place = Place {
        x: 0.0,
        y: 0.0,
        w: 480.0,
        h: 270.0,
    };
    let frames = 400;
    let t = Instant::now();
    for i in 0..frames {
        gpu.render_region(
            &w,
            &cs,
            region,
            crate::render::Clock {
                time: i as f32 * 0.016,
                day_night: 0.3,
                season: 0,
                weather: crate::world::Weather::Clear,
            },
            place,
        );
    }
    let region_ms = t.elapsed().as_secs_f64() * 1000.0 / frames as f64;

    // And the whole world, which is what the picture of the world costs.
    let whole = Region {
        x: 0,
        y: 0,
        w: 1280,
        h: 720,
    };
    let whole_place = Place {
        x: 0.0,
        y: 0.0,
        w: screen_width(),
        h: screen_height(),
    };
    let t = Instant::now();
    for i in 0..40 {
        gpu.render_region(
            &w,
            &cs,
            whole,
            crate::render::Clock {
                time: i as f32 * 0.016,
                day_night: 0.3,
                season: 0,
                weather: crate::world::Weather::Clear,
            },
            whole_place,
        );
    }
    let world_render_ms = t.elapsed().as_secs_f64() * 1000.0 / 40_f64;

    println!(
        "pop {} | world {:.3} ms | creature {:.3} ms | render region {:.3} ms | render world {:.3} ms",
        pop, world_ms, creature_ms, region_ms, world_render_ms
    );
    let frame_ms = world_ms + creature_ms + region_ms;
    println!(
        "a frame at 60 fps has {:.2} ms: the simulation and the region take {:.3} ms of it",
        1000.0 / 60.0,
        frame_ms
    );
    0
}
