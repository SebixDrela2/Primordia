//! The sprites of the creatures and of the effects.
//!
//! A creature used to be a flat shape: a triangle for a soldier, a
//! square for a priest, a dot for a worker. Now it is a little
//! picture - a body with fur or chitin on it, the gear of its role and
//! its eyes. The picture is in grey, so the colour of the species can
//! be put over it and every species keeps a look of its own.
//!
//! Every role has a few looks, and a creature takes the one that fits
//! its species, so a colony of one kind does not look like a photocopy
//! of a colony of another. The ZORGs have their own pictures: they are
//! hulking, chitinous and spiny, and they do not look like anything
//! that lives on the surface.
//!
//! The sprites are put in one atlas, and the effect pictures - the
//! flame of a fire, the spark of a shot, a bomb, a flake of snow - are
//! put in it as well.

use crate::creature::{Faction, Role};

use super::textures::{fbm, hash01};

/// How big one sprite is, in texels.
pub const SPRITE: usize = 32;
/// How many sprites are in a row of the atlas.
pub const COLS: usize = 10;
/// How many rows of sprites the atlas has.
pub const ROWS: usize = 6;
/// The size of the whole atlas, in texels.
pub const ATLAS_W: usize = COLS * SPRITE;
/// The size of the whole atlas, in texels.
pub const ATLAS_H: usize = ROWS * SPRITE;

/// How many looks a role has.
pub const VARIANTS: usize = 4;
/// The first sprite of a ZORG.
pub const ZORG_BASE: usize = 9 * VARIANTS;
/// The first sprite of an effect.
pub const EFFECT_BASE: usize = ZORG_BASE + VARIANTS;

/// The pictures of the effects.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// The flame of a fire, in four beats.
    Flame0,
    Flame1,
    Flame2,
    Flame3,
    /// The spark of a shot.
    Spark,
    /// A bomb, with its fuse lit.
    Bomb,
    /// A soft round glow.
    Glow,
    /// A flake of snow.
    Flake,
    /// A drop of rain.
    Rain,
    /// A mote of ash and of pollen.
    Ember,
}

/// Where the sprite with this number is in the atlas.
#[must_use]
pub fn tile_origin(index: usize) -> (f32, f32) {
    let col = index % COLS;
    let row = index / COLS;
    (
        col as f32 * SPRITE as f32,
        row as f32 * SPRITE as f32,
    )
}

/// The sprite of a creature of this role and this look.
#[must_use]
pub fn creature_tile(role: Role, variant: usize) -> usize {
    role as usize * VARIANTS + variant % VARIANTS
}

/// The sprite of a ZORG.
#[must_use]
pub fn zorg_tile(variant: usize) -> usize {
    ZORG_BASE + variant % VARIANTS
}

/// The sprite of an effect.
#[must_use]
pub fn effect_tile(effect: Effect) -> usize {
    EFFECT_BASE + effect as usize
}

/// The sprite of a creature: its own, or the ZORG's.
#[must_use]
pub fn tile_of(role: Role, faction: Faction, variant: usize) -> usize {
    if faction == Faction::Zorg {
        zorg_tile(variant)
    } else {
        creature_tile(role, variant)
    }
}

/// What a part of a sprite is made of. Every material has its own
/// grain, so a hide does not look like a piece of cloth.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mat {
    Fur,
    Chitin,
    Cloth,
    Metal,
    Stone,
    Smooth,
}

impl Mat {
    /// The grain of the material at this point of the sprite.
    fn grain(self, seed: u32, x: f32, y: f32) -> f32 {
        match self {
            Mat::Fur => fbm(seed, x * 0.30, y * 0.30, 8, 2),
            Mat::Chitin => {
                // Plates: bands that run across the body.
                let band = ((x * 0.55 + y * 0.22).sin() + 1.0) * 0.5;
                0.35 + band * 0.5 + fbm(seed, x * 0.2, y * 0.2, 6, 1) * 0.3
            }
            Mat::Cloth => {
                // A weave: one thread over the other.
                let warp = ((x * 0.8).sin() + 1.0) * 0.5;
                let weft = ((y * 0.8).sin() + 1.0) * 0.5;
                0.4 + (warp * 0.5 + weft * 0.5) * 0.6
            }
            Mat::Metal => {
                let streak = fbm(seed, x * 0.9, y * 0.15, 8, 1);
                0.45 + streak * 0.55
            }
            Mat::Stone => {
                let speck = hash01(seed, x as i32, y as i32);
                0.55 + speck * 0.45
            }
            Mat::Smooth => 0.5,
        }
    }
}

/// The shape of a part of a sprite.
#[derive(Clone, Copy)]
enum Shape {
    Disc {
        cx: f32,
        cy: f32,
        r: f32,
    },
    Ellipse {
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        rot: f32,
    },
    Tri {
        a: (f32, f32),
        b: (f32, f32),
        c: (f32, f32),
    },
    Quad {
        a: (f32, f32),
        b: (f32, f32),
        c: (f32, f32),
        d: (f32, f32),
    },
    Capsule {
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        r: f32,
    },
    Ring {
        cx: f32,
        cy: f32,
        r: f32,
        t: f32,
    },
    Radial {
        cx: f32,
        cy: f32,
        r: f32,
    },
}

/// Is a point in a triangle?
fn in_tri(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let sign = |p0: (f32, f32), p1: (f32, f32), p2: (f32, f32)| {
        (p0.0 - p1.0) * (p2.1 - p1.1) - (p2.0 - p1.0) * (p0.1 - p1.1)
    };
    let d1 = sign(p, a, b);
    let d2 = sign(p, b, c);
    let d3 = sign(p, c, a);
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}

/// How much of the shape is over this point: one inside it, half on
/// the edge, nothing outside it.
fn coverage(shape: Shape, px: f32, py: f32) -> f32 {
    match shape {
        Shape::Disc { cx, cy, r } => {
            let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
            (r + 0.5 - d).clamp(0.0, 1.0)
        }
        Shape::Ellipse { cx, cy, rx, ry, rot } => {
            let dx = px - cx;
            let dy = py - cy;
            let (sin, cos) = rot.sin_cos();
            let ex = dx * cos + dy * sin;
            let ey = -dx * sin + dy * cos;
            let d = (ex / rx).powi(2) + (ey / ry).powi(2);
            ((1.0 - d) * rx.min(ry)).clamp(0.0, 1.0)
        }
        Shape::Tri { a, b, c } => {
            if in_tri((px, py), a, b, c) {
                1.0
            } else {
                0.0
            }
        }
        Shape::Quad { a, b, c, d } => {
            if in_tri((px, py), a, b, c) || in_tri((px, py), a, c, d) {
                1.0
            } else {
                0.0
            }
        }
        Shape::Capsule { x0, y0, x1, y1, r } => {
            let dx = x1 - x0;
            let dy = y1 - y0;
            let len = dx * dx + dy * dy;
            let t = if len <= 0.0 {
                0.0
            } else {
                (((px - x0) * dx + (py - y0) * dy) / len).clamp(0.0, 1.0)
            };
            let nx = x0 + t * dx;
            let ny = y0 + t * dy;
            let d = ((px - nx).powi(2) + (py - ny).powi(2)).sqrt();
            (r + 0.5 - d).clamp(0.0, 1.0)
        }
        Shape::Ring { cx, cy, r, t } => {
            let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
            let e = (d - r).abs();
            (t * 0.5 + 0.5 - e).clamp(0.0, 1.0)
        }
        Shape::Radial { cx, cy, r } => {
            let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt() / r;
            (1.0 - d).clamp(0.0, 1.0).powi(2)
        }
    }
}

/// The middle of a shape: the light comes from that point.
fn center(shape: Shape) -> (f32, f32) {
    match shape {
        Shape::Disc { cx, cy, .. }
        | Shape::Ellipse { cx, cy, .. }
        | Shape::Radial { cx, cy, .. }
        | Shape::Ring { cx, cy, .. } => (cx, cy),
        Shape::Tri { a, b, c } => (
            (a.0 + b.0 + c.0) / 3.0,
            (a.1 + b.1 + c.1) / 3.0,
        ),
        Shape::Quad { a, b, c, d } => (
            (a.0 + b.0 + c.0 + d.0) / 4.0,
            (a.1 + b.1 + c.1 + d.1) / 4.0,
        ),
        Shape::Capsule { x0, y0, x1, y1, .. } => ((x0 + x1) * 0.5, (y0 + y1) * 0.5),
    }
}

/// One part of a sprite: its shape, how bright it is and what it is
/// made of.
#[derive(Clone, Copy)]
struct Layer {
    shape: Shape,
    base: f32,
    mat: Mat,
    seed: u32,
}

impl Layer {
    fn new(shape: Shape, base: f32, mat: Mat, seed: u32) -> Self {
        Layer {
            shape,
            base,
            mat,
            seed,
        }
    }
}

/// A round part.
fn disc(cx: f32, cy: f32, r: f32, base: f32, mat: Mat, seed: u32) -> Layer {
    Layer::new(Shape::Disc { cx, cy, r }, base, mat, seed)
}

/// A long round part.
fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32, base: f32, mat: Mat, seed: u32) -> Layer {
    Layer::new(
        Shape::Ellipse {
            cx,
            cy,
            rx,
            ry,
            rot: 0.0,
        },
        base,
        mat,
        seed,
    )
}

/// A part with three corners.
fn tri(
    a: (f32, f32),
    b: (f32, f32),
    c: (f32, f32),
    base: f32,
    mat: Mat,
    seed: u32,
) -> Layer {
    Layer::new(Shape::Tri { a, b, c }, base, mat, seed)
}

/// A part with four corners.
fn quad(
    a: (f32, f32),
    b: (f32, f32),
    c: (f32, f32),
    d: (f32, f32),
    base: f32,
    mat: Mat,
    seed: u32,
) -> Layer {
    Layer::new(Shape::Quad { a, b, c, d }, base, mat, seed)
}

/// A part that is a stick with round ends.
fn capsule(
    from: (f32, f32),
    to: (f32, f32),
    r: f32,
    base: f32,
    mat: Mat,
    seed: u32,
) -> Layer {
    Layer::new(
        Shape::Capsule {
            x0: from.0,
            y0: from.1,
            x1: to.0,
            y1: to.1,
            r,
        },
        base,
        mat,
        seed,
    )
}

/// A ring round something.
fn ring(cx: f32, cy: f32, r: f32, t: f32, base: f32, mat: Mat, seed: u32) -> Layer {
    Layer::new(Shape::Ring { cx, cy, r, t }, base, mat, seed)
}

/// How bright a point of a layer is: the grain of the material and the
/// light that falls on it.
fn layer_shade(layer: &Layer, x: f32, y: f32) -> f32 {
    let grain = layer.mat.grain(layer.seed, x, y);
    let (cx, cy) = center(layer.shape);
    let dx = x - cx;
    let dy = y - cy;
    let len = (dx * dx + dy * dy).sqrt().max(0.001);
    // The light comes from the top left of the picture.
    let light = (-dx - dy) / len * std::f32::consts::FRAC_1_SQRT_2;
    let shade = layer.base * (0.76 + 0.44 * grain) * (1.0 + 0.30 * light);
    shade.clamp(0.0, 1.55)
}

/// The four corners of a texel, where the picture is looked for.
const SUBS: [(f32, f32); 4] = [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)];

/// Draw the layers into a picture of one sprite.
fn render(layers: &[Layer]) -> Vec<u8> {
    let mut out = vec![0u8; SPRITE * SPRITE * 4];
    for py in 0..SPRITE {
        for px in 0..SPRITE {
            let mut cover = 0.0f32;
            let mut shade = 0.0f32;
            for (ox, oy) in SUBS {
                let x = px as f32 + ox;
                let y = py as f32 + oy;
                for layer in layers {
                    let c = coverage(layer.shape, x, y);
                    if c > 0.0 {
                        cover += c;
                        shade += layer_shade(layer, x, y) * c;
                        break;
                    }
                }
            }
            if cover <= 0.0 {
                continue;
            }
            let gray = ((shade / cover * 255.0).round() as i32).clamp(0, 255) as u8;
            let alpha = ((cover / 4.0).min(1.0) * 255.0).round() as u8;
            let at = (py * SPRITE + px) * 4;
            out[at] = gray;
            out[at + 1] = gray;
            out[at + 2] = gray;
            out[at + 3] = alpha;
        }
    }
    out
}

/// The body of a creature, with the gear of its role.
///
/// The picture is seen from above, and the creature faces the top of
/// its own sprite.
fn creature_layers(role: Role, variant: usize) -> Vec<Layer> {
    let seed = 0x5a + role as u32 * 31 + variant as u32 * 7;
    let scale = match variant {
        0 => 1.00,
        1 => 0.88,
        2 => 1.06,
        _ => 0.95,
    };
    let mut layers = role_layers(role, seed, scale);

    // The head, if the role did not put one on itself.
    if !matches!(role, Role::Queen | Role::Priest | Role::Shaman | Role::Soldier) {
        layers.push(disc(16.0, 9.2, 4.9 * scale, 1.00, Mat::Fur, seed + 7));
    }

    // The look of the creature: the spines of the third look, and the
    // pale belly of the fourth.
    if variant == 2 {
        layers.extend(spines(seed, 8.2 * scale));
    }
    if variant == 3 {
        layers.push(disc(
            16.0,
            19.0,
            8.2 * scale * 0.62,
            1.12,
            Mat::Fur,
            seed + 9,
        ));
    }

    layers
}

/// The parts that the role of the creature gives it.
fn role_layers(role: Role, seed: u32, scale: f32) -> Vec<Layer> {
    match role {
        Role::Worker => worker_layers(seed, scale),
        Role::Soldier => soldier_layers(seed, scale),
        Role::Queen => queen_layers(seed, scale),
        Role::Priest => priest_layers(seed, scale),
        Role::Shaman => shaman_layers(seed, scale),
        Role::Merchant => merchant_layers(seed, scale),
        Role::Builder => builder_layers(seed, scale),
        Role::Explorer => explorer_layers(seed, scale),
        Role::Envoy => envoy_layers(seed, scale),
    }
}

/// The spines round the back of the third look.
fn spines(seed: u32, body_r: f32) -> Vec<Layer> {
    let mut layers = Vec::new();
    for i in 0..7 {
        let a = i as f32 * std::f32::consts::TAU / 7.0;
        let cx = 16.0 + a.sin() * body_r;
        let cy = 17.5 + a.cos() * body_r;
        layers.push(tri(
            (cx + a.sin() * 3.0, cy + a.cos() * 3.0),
            (cx + a.cos() * 1.8, cy - a.sin() * 1.8),
            (cx - a.cos() * 1.8, cy + a.sin() * 1.8),
            1.18,
            Mat::Chitin,
            seed + 8,
        ));
    }
    layers
}

/// The worker: a round body and two arms.
fn worker_layers(seed: u32, scale: f32) -> Vec<Layer> {
    vec![
        disc(16.0, 17.5, 8.2 * scale, 0.92, Mat::Fur, seed),
        disc(16.0 - 7.4 * scale, 16.0, 2.7, 0.82, Mat::Fur, seed + 1),
        disc(16.0 + 7.4 * scale, 16.0, 2.7, 0.82, Mat::Fur, seed + 1),
    ]
}

/// The soldier: a body that comes to a point, a shield and a spear.
fn soldier_layers(seed: u32, scale: f32) -> Vec<Layer> {
    vec![
        tri((16.0, 4.0), (7.5, 27.0), (24.5, 27.0), 0.88, Mat::Fur, seed),
        disc(16.0, 19.0, 7.6 * scale, 0.92, Mat::Fur, seed + 1),
        ellipse(9.6, 16.0, 2.9, 5.4, 1.05, Mat::Metal, seed + 2),
        capsule((15.0, 26.0), (15.0, 6.0), 1.0, 0.72, Mat::Stone, seed + 3),
        tri((15.0, 1.0), (12.6, 7.0), (17.4, 7.0), 1.30, Mat::Smooth, seed + 4),
    ]
}

/// The queen: a great robe, a circlet and a crown.
fn queen_layers(seed: u32, scale: f32) -> Vec<Layer> {
    let mut layers = vec![
        disc(16.0, 17.5, 10.4 * scale, 0.96, Mat::Cloth, seed),
        ring(16.0, 9.2, 5.2 * scale, 1.5, 1.22, Mat::Metal, seed + 8),
    ];
    // The crown: five points round the front of the head.
    for i in 0..5 {
        let a = -2.5 + i as f32 * 0.62;
        let cx = 16.0 + a.sin() * 6.2;
        let cy = 9.2 - a.cos() * 6.2;
        layers.push(tri(
            (cx, cy - 3.6),
            (cx - 2.2, cy + 1.4),
            (cx + 2.2, cy + 1.4),
            1.32,
            Mat::Metal,
            seed + 5,
        ));
    }
    // The gem on the brow.
    layers.push(disc(16.0, 13.0, 2.3, 1.45, Mat::Smooth, seed + 6));
    layers
}

/// The priest: a robe, a hood and a staff with an orb on it.
fn priest_layers(seed: u32, scale: f32) -> Vec<Layer> {
    vec![
        tri((16.0, 4.0), (4.5, 28.5), (27.5, 28.5), 0.90, Mat::Cloth, seed),
        disc(16.0, 11.0, 5.6 * scale, 0.80, Mat::Cloth, seed + 1),
        disc(16.0, 11.6, 2.8, 0.42, Mat::Smooth, seed + 2),
        capsule((25.5, 29.0), (25.5, 5.0), 1.0, 0.70, Mat::Stone, seed + 3),
        disc(25.5, 4.0, 2.5, 1.40, Mat::Smooth, seed + 4),
    ]
}

/// The shaman: a mask, a staff and the bones on it.
fn shaman_layers(seed: u32, scale: f32) -> Vec<Layer> {
    let mut layers = vec![
        disc(16.0, 17.5, 7.6 * scale, 0.88, Mat::Fur, seed),
        disc(16.0, 9.2, 4.9 * scale, 1.02, Mat::Stone, seed + 1),
        disc(14.3, 9.0, 1.15, 1.45, Mat::Smooth, seed + 2),
        disc(17.7, 9.0, 1.15, 1.45, Mat::Smooth, seed + 2),
        capsule((7.0, 29.0), (7.0, 6.0), 1.0, 0.70, Mat::Stone, seed + 3),
    ];
    for i in 0..3 {
        layers.push(disc(
            6.0,
            12.0 + i as f32 * 5.0,
            1.5,
            1.28,
            Mat::Smooth,
            seed + 4,
        ));
    }
    layers
}

/// The merchant: a body and a heavy pack.
fn merchant_layers(seed: u32, scale: f32) -> Vec<Layer> {
    vec![
        disc(16.0, 17.5, 8.0 * scale, 0.90, Mat::Fur, seed),
        ellipse(16.0, 24.0, 6.2, 4.6, 0.70, Mat::Cloth, seed + 1),
        disc(22.6, 21.0, 1.5, 1.38, Mat::Metal, seed + 2),
    ]
}

/// The builder: a plank on the back and a hammer in the hand.
fn builder_layers(seed: u32, scale: f32) -> Vec<Layer> {
    vec![
        disc(16.0, 17.5, 8.4 * scale, 0.92, Mat::Fur, seed),
        quad(
            (9.0, 19.0),
            (23.0, 19.0),
            (23.0, 22.4),
            (9.0, 22.4),
            0.76,
            Mat::Stone,
            seed + 1,
        ),
        capsule((22.0, 22.0), (25.5, 13.0), 1.2, 0.74, Mat::Stone, seed + 2),
        quad(
            (22.6, 13.6),
            (28.4, 11.6),
            (29.4, 15.4),
            (23.6, 17.4),
            1.22,
            Mat::Metal,
            seed + 3,
        ),
    ]
}

/// The explorer: a slim body, a hat and a lantern.
fn explorer_layers(seed: u32, scale: f32) -> Vec<Layer> {
    vec![
        ellipse(16.0, 17.5, 6.4 * scale, 8.2 * scale, 0.90, Mat::Fur, seed),
        ellipse(16.0, 7.6, 6.6, 3.4, 0.74, Mat::Cloth, seed + 1),
        disc(16.0, 7.0, 3.4, 0.66, Mat::Cloth, seed + 2),
        disc(23.0, 20.0, 2.0, 1.42, Mat::Smooth, seed + 3),
    ]
}

/// The envoy: a body and a banner on a pole.
fn envoy_layers(seed: u32, scale: f32) -> Vec<Layer> {
    vec![
        disc(16.0, 17.5, 8.0 * scale, 0.90, Mat::Fur, seed),
        capsule((24.0, 27.0), (24.0, 2.0), 1.0, 0.72, Mat::Stone, seed + 1),
        quad(
            (24.0, 3.0),
            (30.4, 6.0),
            (30.4, 13.0),
            (24.0, 10.0),
            1.12,
            Mat::Cloth,
            seed + 2,
        ),
    ]
}

/// The picture of a ZORG: a hulking, spiny thing of chitin.
fn zorg_layers(variant: usize) -> Vec<Layer> {
    let seed = 0x9c + variant as u32 * 11;
    let mut layers = vec![disc(16.0, 18.0, 10.6, 0.80, Mat::Chitin, seed)];

    // The spines round the back.
    for i in 0..9 {
        let a = i as f32 * std::f32::consts::TAU / 9.0 + 0.2;
        let cx = 16.0 + a.sin() * 10.0;
        let cy = 18.0 + a.cos() * 10.0;
        layers.push(tri(
            (cx + a.sin() * 4.2, cy + a.cos() * 4.2),
            (cx + a.cos() * 2.4, cy - a.sin() * 2.4),
            (cx - a.cos() * 2.4, cy + a.sin() * 2.4),
            1.05,
            Mat::Chitin,
            seed + 1,
        ));
    }

    // The plates on the back.
    layers.push(ellipse(16.0, 19.0, 7.2, 5.6, 0.62, Mat::Chitin, seed + 2));
    for i in 0..3 {
        layers.push(ellipse(
            16.0,
            14.0 + i as f32 * 4.4,
            5.4 - i as f32,
            1.7,
            1.12,
            Mat::Chitin,
            seed + 3,
        ));
    }

    // The head, with the two eyes in it.
    layers.push(disc(16.0, 7.6, 5.0, 0.86, Mat::Chitin, seed + 4));
    layers.push(disc(13.9, 7.2, 1.3, 1.50, Mat::Smooth, seed + 5));
    layers.push(disc(18.1, 7.2, 1.3, 1.50, Mat::Smooth, seed + 5));

    // The mandibles.
    layers.push(capsule((13.0, 4.0), (11.0, 1.0), 0.9, 1.20, Mat::Chitin, seed + 6));
    layers.push(capsule((19.0, 4.0), (21.0, 1.0), 0.9, 1.20, Mat::Chitin, seed + 6));
    layers
}

/// The picture of an effect.
fn effect_layers(effect: Effect) -> Vec<Layer> {
    match effect {
        Effect::Flame0 | Effect::Flame1 | Effect::Flame2 | Effect::Flame3 => {
            flame_layers(effect as usize)
        }
        Effect::Spark => spark_layers(),
        Effect::Bomb => bomb_layers(),
        Effect::Glow => glow_layers(),
        Effect::Flake => flake_layers(),
        Effect::Rain => rain_layers(),
        Effect::Ember => ember_layers(),
    }
}

/// The flame of a fire, in four beats.
fn flame_layers(beat: usize) -> Vec<Layer> {
    let sway = (beat as f32 * std::f32::consts::FRAC_PI_2).sin();
    let height = 20.0 + sway * 3.0;
    let width = 6.2 + sway * 1.2;
    vec![
        tri(
            (16.0 + sway * 2.2, 28.0 - height),
            (16.0 - width, 28.0),
            (16.0 + width, 28.0),
            0.95,
            Mat::Smooth,
            0x21,
        ),
        disc(16.0, 25.0, 6.4, 1.18, Mat::Smooth, 0x22),
        disc(16.0, 26.5, 3.4, 1.48, Mat::Smooth, 0x23),
    ]
}

/// The spark of a shot.
fn spark_layers() -> Vec<Layer> {
    vec![
        Layer::new(
            Shape::Radial {
                cx: 16.0,
                cy: 16.0,
                r: 14.0,
            },
            1.05,
            Mat::Smooth,
            0x31,
        ),
        disc(16.0, 16.0, 3.6, 1.50, Mat::Smooth, 0x32),
    ]
}

/// A bomb, with its fuse lit.
fn bomb_layers() -> Vec<Layer> {
    vec![
        disc(16.0, 18.0, 9.0, 0.42, Mat::Stone, 0x41),
        Layer::new(
            Shape::Ellipse {
                cx: 13.0,
                cy: 15.0,
                rx: 3.4,
                ry: 2.4,
                rot: -0.5,
            },
            0.80,
            Mat::Smooth,
            0x42,
        ),
        capsule((16.0, 10.0), (19.0, 4.0), 0.9, 0.75, Mat::Stone, 0x43),
        Layer::new(
            Shape::Radial {
                cx: 19.5,
                cy: 3.5,
                r: 5.0,
            },
            1.45,
            Mat::Smooth,
            0x44,
        ),
    ]
}

/// A soft round glow.
fn glow_layers() -> Vec<Layer> {
    vec![Layer::new(
        Shape::Radial {
            cx: 16.0,
            cy: 16.0,
            r: 15.5,
        },
        1.10,
        Mat::Smooth,
        0x51,
    )]
}

/// A flake of snow.
fn flake_layers() -> Vec<Layer> {
    let mut layers = Vec::new();
    for i in 0..6 {
        let a = i as f32 * std::f32::consts::TAU / 6.0;
        layers.push(capsule(
            (16.0 - a.sin() * 10.0, 16.0 + a.cos() * 10.0),
            (16.0 + a.sin() * 10.0, 16.0 - a.cos() * 10.0),
            0.9,
            1.30,
            Mat::Smooth,
            0x61,
        ));
    }
    layers.push(disc(16.0, 16.0, 2.6, 1.45, Mat::Smooth, 0x62));
    layers
}

/// A drop of rain.
fn rain_layers() -> Vec<Layer> {
    vec![capsule(
        (16.0, 5.0),
        (16.0, 27.0),
        1.1,
        1.30,
        Mat::Smooth,
        0x71,
    )]
}

/// A mote of ash and of pollen.
fn ember_layers() -> Vec<Layer> {
    vec![
        Layer::new(
            Shape::Radial {
                cx: 16.0,
                cy: 16.0,
                r: 9.0,
            },
            1.20,
            Mat::Smooth,
            0x81,
        ),
        disc(16.0, 16.0, 2.2, 1.50, Mat::Smooth, 0x82),
    ]
}
/// The whole atlas: the picture of every creature and of every effect.
#[must_use]
pub fn atlas() -> &'static [u8] {
    use std::sync::OnceLock;
    static ATLAS: OnceLock<Vec<u8>> = OnceLock::new();
    ATLAS.get_or_init(build_atlas)
}

/// The bytes of the atlas, put together.
fn build_atlas() -> Vec<u8> {
    let mut bytes = vec![0u8; ATLAS_W * ATLAS_H * 4];
    for index in 0..COLS * ROWS {
        let picture = if index < ZORG_BASE {
            let r = index / VARIANTS;
            let v = index % VARIANTS;
            render(&creature_layers(role_from_index(r), v))
        } else if index < EFFECT_BASE {
            render(&zorg_layers(index - ZORG_BASE))
        } else {
            let effect = effect_from_index(index - EFFECT_BASE);
            render(&effect_layers(effect))
        };
        let (ox, oy) = tile_origin(index);
        let ox = ox as usize;
        let oy = oy as usize;
        for py in 0..SPRITE {
            for px in 0..SPRITE {
                let src = (py * SPRITE + px) * 4;
                let dst = ((oy + py) * ATLAS_W + ox + px) * 4;
                bytes[dst..dst + 4].copy_from_slice(&picture[src..src + 4]);
            }
        }
    }
    bytes
}

/// The role a sprite belongs to.
fn role_from_index(index: usize) -> Role {
    match index {
        0 => Role::Worker,
        1 => Role::Soldier,
        2 => Role::Queen,
        3 => Role::Priest,
        4 => Role::Merchant,
        5 => Role::Builder,
        6 => Role::Explorer,
        7 => Role::Shaman,
        _ => Role::Envoy,
    }
}

/// The effect a sprite belongs to.
fn effect_from_index(index: usize) -> Effect {
    match index {
        0 => Effect::Flame0,
        1 => Effect::Flame1,
        2 => Effect::Flame2,
        3 => Effect::Flame3,
        4 => Effect::Spark,
        5 => Effect::Bomb,
        6 => Effect::Glow,
        7 => Effect::Flake,
        8 => Effect::Rain,
        _ => Effect::Ember,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every sprite must have something in it.
    #[test]
    fn every_sprite_is_drawn() {
        let atlas = atlas();
        for index in 0..COLS * ROWS {
            let (ox, oy) = tile_origin(index);
            let ox = ox as usize;
            let oy = oy as usize;
            let mut painted = 0;
            for py in 0..SPRITE {
                for px in 0..SPRITE {
                    if atlas[((oy + py) * ATLAS_W + ox + px) * 4 + 3] > 0 {
                        painted += 1;
                    }
                }
            }
            assert!(painted > 40, "the sprite {index} is empty");
        }
    }

    /// The atlas must be the size the renderer expects.
    #[test]
    fn the_atlas_is_the_right_size() {
        assert_eq!(atlas().len(), ATLAS_W * ATLAS_H * 4);
    }
}