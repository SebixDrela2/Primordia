//! Earthquakes.
//!
//! The ground is not still. Every so often the world shivers: dust falls from
//! the ceiling, weak rock collapses and the caves of the world change shape.
//! A colony that dug into a hillside has to live with that.

use super::*;

impl World {
    /// Shake the ground around a point.
    ///
    /// Dust falls from the roof of every cave in the area and any rock that
    /// has nothing under it comes down. The caves are therefore never quite
    /// the same twice.
    pub(crate) fn tremor(&mut self, cx: i32, cy: i32, radius: i32) {
        let w = self.w as i32;
        let h = self.h as i32;
        let r2 = radius * radius;

        for x in (cx - radius).max(1)..(cx + radius).min(w - 1) {
            let dx = x - cx;
            for y in (cy - radius).max(1)..(cy + radius).min(h - 4) {
                let dy = y - cy;
                if dx * dx + dy * dy > r2 {
                    continue;
                }
                let e = self.get(x, y);
                // Dust falls out of the roof of a cave.
                if e == Elem::Empty && self.rng.chance(0.06) {
                    let above = self.get(x, y - 3);
                    if above == Elem::Stone || above == Elem::Sand {
                        self.set_xy(x, y, Elem::Dust);
                    }
                }
                // Rock with nothing under it comes down.
                if (e == Elem::Stone || e == Elem::Sand) && self.rng.chance(0.04) {
                    let mut empty_below = true;
                    for k in 1..=4 {
                        let below = self.get(x, y + k);
                        if below != Elem::Empty && below != Elem::Dust && below != Elem::Water {
                            empty_below = false;
                            break;
                        }
                    }
                    if empty_below {
                        self.set_xy(x, y, Elem::Sand);
                    }
                }
            }
        }

        // The ground itself is shaken: loose dust on the surface lifts and
        // scatters.
        let surf = self.surface_y(cx);
        for dx in -radius..=radius {
            let x = cx + dx;
            if x <= 0 || x >= w - 1 {
                continue;
            }
            if self.rng.chance(0.25) {
                let y = self.surface_y(x) - 1;
                if y > 0 && self.get(x, y) == Elem::Empty {
                    self.set_xy(x, y, Elem::Dust);
                }
            }
        }
        let _ = surf;
    }
}
