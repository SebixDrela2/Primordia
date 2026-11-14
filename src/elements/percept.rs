use super::Elem;

#[derive(Clone, Copy, Default)]
pub struct Look {

    pub rgb: [f32; 3],

    pub bright: f32,

    pub warm: f32,

    pub soft: f32,

    pub wet: f32,

    pub sheen: f32,
}

impl Look {

    pub fn greenness(&self) -> f32 {
        self.rgb[1] - self.rgb[0]
    }

    pub fn colourfulness(&self) -> f32 {
        let r = self.rgb[0];
        let g = self.rgb[1];
        let b = self.rgb[2];
        let mx = r.max(g).max(b);
        let mn = r.min(g).min(b);
        if mx <= 0.0 {
            0.0
        } else {
            (mx - mn) / mx
        }
    }
}

pub fn look_table() -> [Look; Elem::TABLE_LEN] {
    let mut t = [Look::default(); Elem::TABLE_LEN];
    let mut i = 0;
    while i < Elem::TABLE_LEN {
        t[i] = look_of(Elem::from_u8(i as u8));
        i += 1;
    }
    t
}

fn look_of(e: Elem) -> Look {
    let c = e.color();
    let rgb = [
        c[0] as f32 / 255.0,
        c[1] as f32 / 255.0,
        c[2] as f32 / 255.0,
    ];

    let em = e.emission();
    let bright = em[0].max(em[1]).max(em[2]);
    Look {
        rgb,
        bright,
        warm: warmth_of(e),
        soft: softness_of(e),
        wet: wetness_of(e),
        sheen: sheen_of(e),
    }
}

fn warmth_of(e: Elem) -> f32 {
    match e {
        Elem::Fire | Elem::Lava | Elem::Magma | Elem::Plasma => 1.0,
        Elem::Ember => 0.7,
        Elem::Steam => 0.4,
        Elem::Water | Elem::Oil | Elem::Acid | Elem::Mercury | Elem::Blood => -0.15,
        Elem::Ice | Elem::Snow => -0.7,
        _ => 0.0,
    }
}

fn softness_of(e: Elem) -> f32 {
    match e {

        Elem::Plant
        | Elem::Spore
        | Elem::Fungus
        | Elem::Moss
        | Elem::Roots
        | Elem::Meat
        | Elem::Leather
        | Elem::Corpse
        | Elem::Egg
        | Elem::Food
        | Elem::Crop
        | Elem::Blood
        | Elem::Honey => 0.95,

        Elem::Sand
        | Elem::Ash
        | Elem::Dust
        | Elem::Salt
        | Elem::Sulfur
        | Elem::Gunpowder
        | Elem::Fertilizer
        | Elem::Mud
        | Elem::Snow
        | Elem::Rust => 0.7,

        Elem::Water
        | Elem::Oil
        | Elem::Acid
        | Elem::Mercury
        | Elem::Poison
        | Elem::Alcohol
        | Elem::Tar => 0.4,
        Elem::Wood => 0.55,
        Elem::Charcoal => 0.5,
        _ => 0.0,
    }
}

fn wetness_of(e: Elem) -> f32 {
    match e {
        Elem::Water => 1.0,
        Elem::Oil | Elem::Acid | Elem::Mercury | Elem::Poison | Elem::Alcohol => 0.85,
        Elem::Blood | Elem::Honey | Elem::Tar => 0.7,
        Elem::Snow => 0.6,
        Elem::Ice => 0.4,
        Elem::Mud => 0.5,
        Elem::Steam | Elem::Smoke => 0.3,
        _ => 0.0,
    }
}

fn sheen_of(e: Elem) -> f32 {
    match e {
        Elem::Metal
        | Elem::Copper
        | Elem::Tin
        | Elem::Bronze
        | Elem::Iron
        | Elem::Steel
        | Elem::Gold
        | Elem::Silver
        | Elem::Glass
        | Elem::Crystal
        | Elem::Gem
        | Elem::Diamond
        | Elem::Obsidian
        | Elem::Wire
        | Elem::Circuit => 0.9,
        Elem::Water | Elem::Mercury | Elem::Ice => 0.75,
        Elem::Oil => 0.6,
        _ => 0.1,
    }
}
