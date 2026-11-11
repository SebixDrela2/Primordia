use crate::creature::CreatureSystem;
use crate::elements::Elem;
use crate::world::World;

pub fn render_world(world: &World, _creatures: &CreatureSystem, buf: &mut [u8]) {
    let w = world.w;
    let h = world.h;

    for i in 0..w * h {
        let e = world.kind[i];
        let o = i * 4;

        if e == Elem::Empty as u8 {
            // Deep background with a faint vertical gradient
            let y = (i / w) as f32 / h as f32;
            let shade = 8.0 + y * 10.0;
            buf[o] = (shade * 0.6) as u8;
            buf[o + 1] = shade as u8;
            buf[o + 2] = (shade + 8.0) as u8;
            buf[o + 3] = 255;
            continue;
        }

        let c = world.col[i];
        let mut r = c[0] as i32;
        let mut g = c[1] as i32;
        let mut b = c[2] as i32;

        let elem = Elem::from_u8(e);
        let heat = world.heat[i] as i32;
        if matches!(
            elem,
            Elem::Fire | Elem::Lava | Elem::Plasma | Elem::Energy
        ) {
            let boost = 40 + heat / 3;
            r = (r + boost).min(255);
            g = (g + boost * 2 / 3).min(255);
            b = (b + boost / 3).min(255);
        }

        buf[o] = r as u8;
        buf[o + 1] = g as u8;
        buf[o + 2] = b as u8;
        buf[o + 3] = 255;
    }

    // Creatures drawn on top of the terrain
    // (names are now drawn in main.rs with macroquad's font)
}