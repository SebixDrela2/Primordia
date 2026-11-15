//! A texture of its own for every element.
//!
//! Every element gets a small tile: what that thing looks like close
//! up. The tiles are put side by side in one atlas, and the renderer
//! reads the tile of the element that is in the cell it is painting.
//!
//! The tile holds two numbers in every texel: how much darker or
//! brighter the cell is (the shade) and how much light glints off it
//! (the highlight). The shade is stored as a byte between the two ends
//! of the range, and the highlight as a byte between nothing and full.
//!
//! The same tile is drawn in every cell of the element, but every cell
//! takes a different bite out of the tile, so a field of stone is not a
//! wall of one colour but a wall of many.

use crate::elements::Elem;

use super::hash2;

/// How big one tile is, in texels.
pub const TILE: usize = 32;
/// How many tiles are in a row of the atlas.
pub const COLS: usize = 12;
/// How many rows of tiles the atlas has.
pub const ROWS: usize = 8;
/// The size of the whole atlas, in texels.
pub const ATLAS_W: usize = COLS * TILE;
/// The size of the whole atlas, in texels.
pub const ATLAS_H: usize = ROWS * TILE;

/// How many cells of the world one tile covers. The picture of an
/// element runs through the whole field and does not begin again in
/// every cell, so that a wall of stone does not look like a wall of
/// squares.
pub const TILE_CELLS: f32 = 3.0;

/// The darkest a tile may make a cell.
pub const SHADE_LO: f32 = 0.45;
/// How much brighter a tile may make a cell, over the darkest.
pub const SHADE_SPAN: f32 = 1.3;
/// How bright the glint of a tile may be.
pub const HIGHLIGHT: f32 = 40.0;

/// The kind of picture a tile holds.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pattern {
    /// Stone and the things that are cut from it.
    Rock,
    /// Sand, snow, powder and the fine stuff.
    Grit,
    /// Metal, brushed and pitted.
    Metal,
    /// Anything that pours.
    Liquid,
    /// Anything that floats.
    Gas,
    /// Leaves, moss and roots.
    Leaf,
    /// Wood and the things that are made of it.
    Wood,
    /// Crystal, glass and ice.
    Crystal,
    /// Flesh, bone and the soft things.
    Flesh,
    /// The things that are built.
    Built,
    /// Fire and the light of it.
    Fire,
    /// Nothing to see.
    Plain,
}

/// The hash of a point of the lattice, the same one the noise uses.
pub(crate) fn hash01(seed: u32, x: i32, y: i32) -> f32 {
    let mut h = seed
        ^ (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (y as u32).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h >> 8) as f32 / 16_777_216.0
}

/// The noise between the points of the lattice. It repeats every
/// `period` points, so a tile has no seam in it.
pub(crate) fn noise(seed: u32, x: f32, y: f32, period: i32) -> f32 {
    let x0 = x.floor();
    let y0 = y.floor();
    let fx = x - x0;
    let fy = y - y0;
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);
    let xi = x0 as i32;
    let yi = y0 as i32;
    let wrap = |i: i32| ((i % period) + period) % period;
    let (xa, xb) = (wrap(xi), wrap(xi + 1));
    let (ya, yb) = (wrap(yi), wrap(yi + 1));
    let c00 = hash01(seed, xa, ya);
    let c10 = hash01(seed, xb, ya);
    let c01 = hash01(seed, xa, yb);
    let c11 = hash01(seed, xb, yb);
    (c00 * (1.0 - ux) + c10 * ux) * (1.0 - uy) + (c01 * (1.0 - ux) + c11 * ux) * uy
}

/// The noise of several octaves, each one finer than the last.
pub(crate) fn fbm(seed: u32, x: f32, y: f32, period: i32, octaves: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 1.0;
    let mut norm = 0.0;
    let mut p = period;
    let mut sx = x;
    let mut sy = y;
    for o in 0..octaves {
        sum += amp * noise(seed.wrapping_add(o), sx, sy, p);
        norm += amp;
        amp *= 0.5;
        sx *= 2.0;
        sy *= 2.0;
        p *= 2;
    }
    sum / norm
}

/// A speck of grit: a single point of the lattice, taken hard. The
/// lattice wraps, so that the tile has no seam in it.
fn speck(seed: u32, u: f32, v: f32, scale: f32) -> f32 {
    let wrap = |i: i32, n: i32| ((i % n) + n) % n;
    hash01(
        seed,
        wrap((u * scale) as i32, scale as i32),
        wrap((v * scale) as i32, scale as i32),
    )
}

/// The tile of an element: what kind of picture it holds.
fn pattern_of(elem: Elem) -> (Pattern, u32) {
    let pattern = match elem {
        Elem::Empty | Elem::Void => Pattern::Plain,

        Elem::Stone
        | Elem::Granite
        | Elem::Obsidian
        | Elem::Bone => Pattern::Rock,

        Elem::Brick
        | Elem::Concrete
        | Elem::Wall
        | Elem::Tower
        | Elem::Bridge
        | Elem::Road
        | Elem::Temple
        | Elem::Totem
        | Elem::Beacon => Pattern::Built,

        Elem::Sand
        | Elem::Salt
        | Elem::Ash
        | Elem::Dust
        | Elem::Gunpowder
        | Elem::Sulfur
        | Elem::Fertilizer
        | Elem::Mud
        | Elem::Snow
        | Elem::Crop
        | Elem::Food
        | Elem::Charcoal => Pattern::Grit,

        Elem::Metal
        | Elem::Copper
        | Elem::Tin
        | Elem::Bronze
        | Elem::Iron
        | Elem::Steel
        | Elem::Gold
        | Elem::Silver
        | Elem::Rust
        | Elem::Anvil
        | Elem::Machine
        | Elem::Tool
        | Elem::Wire
        | Elem::Circuit => Pattern::Metal,

        Elem::Water
        | Elem::Oil
        | Elem::Acid
        | Elem::Mercury
        | Elem::Blood
        | Elem::Poison
        | Elem::Alcohol
        | Elem::Honey
        | Elem::Tar
        | Elem::Lava
        | Elem::Magma => Pattern::Liquid,

        Elem::Steam
        | Elem::Smoke
        | Elem::Gas
        | Elem::Chlorine
        | Elem::Methane
        | Elem::Oxygen
        | Elem::Plasma
        | Elem::Energy
        | Elem::Mana
        | Elem::Soul
        | Elem::Portal
        | Elem::Rune
        | Elem::Ward => Pattern::Gas,

        Elem::Plant
        | Elem::Moss
        | Elem::Roots
        | Elem::Spore
        | Elem::Fungus
        | Elem::Farm => Pattern::Leaf,

        Elem::Wood
        | Elem::Door
        | Elem::Chest
        | Elem::Hearth
        | Elem::Furnace
        | Elem::Forge
        | Elem::Market
        | Elem::Workshop
        | Elem::Granary
        | Elem::Leather => Pattern::Wood,

        Elem::Crystal
        | Elem::Gem
        | Elem::Diamond
        | Elem::Glass
        | Elem::Ice => Pattern::Crystal,

        Elem::Meat | Elem::Corpse | Elem::Egg | Elem::Virus | Elem::Bacteria => Pattern::Flesh,

        Elem::Fire | Elem::Ember => Pattern::Fire,
    };
    (pattern, elem as u32)
}

/// The shade and the highlight of one point of a tile.
fn tile_value(pattern: Pattern, seed: u32, u: f32, v: f32) -> (f32, f32) {
    match pattern {
        Pattern::Plain => (1.0, 0.0),

        Pattern::Rock => {
            let broad = fbm(seed, u * 2.0, v * 2.0, 2, 3);
            let fine = noise(seed ^ 0x07, u * 8.0, v * 8.0, 8);
            let band = ((v * 2.0 + broad * 0.7).sin() + 1.0) * 0.5;
            let mut shade = 0.90 + (broad - 0.5) * 0.80 + (fine - 0.5) * 0.45 + (band - 0.5) * 0.30;
            let grit = speck(seed ^ 0x51, u, v, 8.0);
            if grit > 0.90 {
                shade += 0.40 * (grit - 0.90) / 0.10;
            } else if grit < 0.10 {
                shade -= 0.40 * (0.10 - grit) / 0.10;
            }
            (shade, 0.0)
        }

        Pattern::Grit => {
            let fine = noise(seed ^ 0x03, u * 8.0, v * 8.0, 8);
            let grit = speck(seed ^ 0x09, u, v, 8.0);
            let shade = 0.95 + (fine - 0.5) * 0.40 + (grit - 0.5) * 0.55;
            (shade, 0.0)
        }

        Pattern::Metal => {
            let streak = noise(seed ^ 0x05, u * 8.0, v * 2.0, 8);
            let broad = fbm(seed ^ 0x06, u * 2.0, v * 2.0, 2, 2);
            let shade = 0.82 + (streak - 0.5) * 0.85 + (broad - 0.5) * 0.35;
            let highlight = smoothstep(0.74, 0.96, streak) * 0.35;
            (shade, highlight)
        }

        Pattern::Liquid => {
            let n = fbm(seed ^ 0x0b, u * 3.0, v * 3.0, 3, 2);
            // The wave runs right across the tile, so it has to end
            // where it begins.
            let wave = ((u * 2.0 + v) * std::f32::consts::TAU + n * 1.6).sin();
            let shade = 0.95 + wave * 0.10 + (n - 0.5) * 0.16;
            let highlight = smoothstep(0.88, 1.0, wave) * 0.28;
            (shade, highlight)
        }

        Pattern::Gas => {
            let n = fbm(seed ^ 0x0d, u * 2.0, v * 2.0, 2, 3);
            let shade = 0.74 + n * 0.52;
            let highlight = smoothstep(0.66, 0.96, n) * 0.45;
            (shade, highlight)
        }

        Pattern::Leaf => {
            let n = fbm(seed ^ 0x11, u * 4.0, v * 4.0, 4, 2);
            let vein = ((u * 3.0).sin() + 1.0) * 0.5;
            let shade = 0.88 + (n - 0.5) * 0.32 + (vein - 0.5) * 0.20;
            let highlight = smoothstep(0.88, 1.0, vein) * 0.18;
            (shade, highlight)
        }

        Pattern::Wood => {
            let n = fbm(seed ^ 0x13, u * 2.0, v * 6.0, 6, 2);
            let rings = ((v * 4.0 + n * 0.9).sin() + 1.0) * 0.5;
            let shade = 0.86 + (rings - 0.5) * 0.32 + (n - 0.5) * 0.18;
            (shade, 0.0)
        }

        Pattern::Crystal => {
            // Four points, and the cell of the tile belongs to the
            // nearest of them. The facets are the cells. The distance
            // is taken around the tile as well, so that the facets run
            // over the edge without a break.
            let points = [(0.22f32, 0.30f32), (0.72, 0.24), (0.34, 0.74), (0.76, 0.68)];
            let mut best = 1.0f32;
            for (px, py) in points {
                for wrap_x in [-1.0f32, 0.0, 1.0] {
                    for wrap_y in [-1.0f32, 0.0, 1.0] {
                        let dx = u - (px + wrap_x);
                        let dy = v - (py + wrap_y);
                        let d = (dx * dx + dy * dy).sqrt();
                        if d < best {
                            best = d;
                        }
                    }
                }
            }
            let shade = 0.86 + (0.34 - best).clamp(0.0, 0.34);
            let highlight = smoothstep(0.20, 0.02, best) * 0.55;
            (shade, highlight)
        }

        Pattern::Flesh => {
            let broad = fbm(seed ^ 0x17, u * 3.0, v * 3.0, 3, 3);
            let vein = noise(seed ^ 0x18, u * 8.0, v * 8.0, 8);
            let shade = 0.90 + (broad - 0.5) * 0.42 + (vein - 0.5) * 0.20;
            (shade, 0.0)
        }

        Pattern::Built => {
            let rows = 4.0;
            let row = (v * rows).floor() as i32;
            let shift = if row % 2 == 0 { 0.0 } else { 0.5 };
            let bx = (u * 2.0 + shift).fract();
            let by = (v * rows).fract();
            let mortar = bx < 0.10 || by < 0.16;
            let n = noise(seed ^ 0x1d, u * 8.0, v * 8.0, 8);
            let mut shade = 0.93 + (n - 0.5) * 0.20;
            if mortar {
                shade -= 0.32;
            }
            (shade, 0.0)
        }

        Pattern::Fire => {
            let n = fbm(seed ^ 0x1f, u * 3.0, v * 3.0, 3, 3);
            let shade = 0.58 + n * 0.86;
            let highlight = n * n * n * 0.85;
            (shade, highlight)
        }
    }
}

/// The step of the curve, the same one the renderer uses.
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The whole atlas: the tile of every element, side by side.
pub fn atlas() -> &'static [u8] {
    use std::sync::OnceLock;
    static ATLAS: OnceLock<Vec<u8>> = OnceLock::new();
    ATLAS.get_or_init(build_atlas)
}

/// The bytes of the atlas, put together.
fn build_atlas() -> Vec<u8> {
    let mut bytes = vec![0u8; ATLAS_W * ATLAS_H * 4];
    for kind in 0..COLS * ROWS {
        let elem = Elem::from_u8(kind as u8);
        let (pattern, seed) = pattern_of(elem);
        let col = kind % COLS;
        let row = kind / COLS;
        for ly in 0..TILE {
            for lx in 0..TILE {
                let u = (lx as f32 + 0.5) / TILE as f32;
                let v = (ly as f32 + 0.5) / TILE as f32;
                let (shade, highlight) = tile_value(pattern, seed, u, v);
                let at = ((row * TILE + ly) * ATLAS_W + col * TILE + lx) * 4;
                bytes[at] = encode_shade(shade);
                bytes[at + 1] = (highlight.clamp(0.0, 1.0) * 255.0).round() as u8;
                bytes[at + 2] = 0;
                bytes[at + 3] = 255;
            }
        }
    }
    bytes
}

/// The shade of a tile, as it is stored in the atlas.
fn encode_shade(shade: f32) -> u8 {
    let t = ((shade - SHADE_LO) / SHADE_SPAN).clamp(0.0, 1.0);
    (t * 255.0).round() as u8
}

/// The shade a texel of the atlas stands for.
#[must_use]
pub fn decode_shade(byte: u8) -> f32 {
    SHADE_LO + (byte as f32 / 255.0) * SHADE_SPAN
}

/// The highlight a texel of the atlas stands for.
#[must_use]
pub fn decode_highlight(byte: u8) -> f32 {
    byte as f32 / 255.0
}

/// The bite of the tile that the cell at this place takes.
///
/// Every cell takes a different bite, so the cells of one element do
/// not all look the same.
#[must_use]
pub fn bite(x: i32, y: i32) -> (u32, u32) {
    let h = hash2(x, y);
    let mask = (TILE - 1) as u32;
    (h & mask, (h >> 4) & mask)
}

/// The shade and the highlight of the cell at this place.
///
/// The processor paints one pixel to a cell, so it takes the middle of
/// the bite. The card takes the middle of the bite as well when the
/// cell is only one pixel big, which keeps the two pictures the same.
#[must_use]
pub fn sample(kind: u8, x: i32, y: i32) -> (f32, f32) {
    if kind as usize >= COLS * ROWS {
        return (1.0, 0.0);
    }
    let (ox, oy) = bite(x, y);
    let lx = (TILE / 2 + ox as usize) % TILE;
    let ly = (TILE / 2 + oy as usize) % TILE;
    let col = kind as usize % COLS;
    let row = kind as usize / COLS;
    let at = ((row * TILE + ly) * ATLAS_W + col * TILE + lx) * 4;
    let atlas = atlas();
    (
        decode_shade(atlas[at]),
        decode_highlight(atlas[at + 1]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The atlas and the tile of every element must agree.
    #[test]
    fn every_element_has_a_tile() {
        for kind in 0..=Elem::MAX {
            let (shade, _) = sample(kind, 3, 5);
            assert!(
                (SHADE_LO..=SHADE_LO + SHADE_SPAN).contains(&shade),
                "the tile of {kind} is out of range: {shade}"
            );
        }
    }

    /// The atlas must be the size the renderer expects.
    #[test]
    fn the_atlas_is_the_right_size() {
        let atlas = atlas();
        assert_eq!(atlas.len(), ATLAS_W * ATLAS_H * 4);
    }
}
