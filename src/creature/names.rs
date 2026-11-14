use super::*;

pub(crate) fn generate_name(rng: &mut Rng, role: Role, species: u32) -> [u8; 10] {
    const PREFIXES: &[&str] = &[
        "A", "Ba", "Ca", "Da", "Ea", "Fa", "Ga", "Ha", "Ia", "Ja",
        "Ka", "La", "Ma", "Na", "Oa", "Pa", "Qa", "Ra", "Sa", "Ta",
        "Ua", "Va", "Wa", "Xa", "Ya", "Za", "Kra", "Dra", "Gra", "Tra",
    ];
    const MIDDLES: &[&str] = &[
        "r", "t", "v", "d", "m", "k", "z", "x", "q", "v",
        "m", "d", "g", "th", "v", "z", "p", "n", "l", "v",
        "g", "dr", "k", "v", "z", "my", "ty", "k", "v", "z",
    ];
    const SUFFIXES: &[&str] = &[
        "", "s", "r", "x", "x", "x", "n", "n", "n", "n",
        "k", "k", "k", "k", "k", "r", "r", "r", "l", "l",
    ];
    
    let prefix = PREFIXES[rng.range(PREFIXES.len() as u32) as usize];
    let middle = MIDDLES[rng.range(MIDDLES.len() as u32) as usize];
    let suffix = SUFFIXES[rng.range(SUFFIXES.len() as u32) as usize];
    
    let role_suffix = match role {
        Role::Queen => "a",
        Role::Soldier => "x",
        Role::Worker => "",
        Role::Priest => "o",
        Role::Merchant => "i",
        Role::Builder => "u",
        Role::Explorer => "e",
        Role::Shaman => "y",
        Role::Envoy => "n",
    };
    
    let species_mod = (species * 7 + 11) as usize % MIDDLES.len();
    let middle2 = MIDDLES[species_mod];
    
    let full_name = format!("{}{}{}{}{}", prefix, middle, middle2, suffix, role_suffix);
    let mut name_bytes = [0u8; 10];
    let bytes = full_name.as_bytes();
    let len = bytes.len().min(9);
    name_bytes[..len].copy_from_slice(&bytes[..len]);
    name_bytes
}

/// The colours of the player species, in order.
///
/// The first colony of a run is the red one, the second is the orange one,
/// the third the amber one, and so on around the wheel. They are fixed
/// colours rather than random ones, so the map can be read at a glance:
/// the green colony is *the* green colony from one run to the next.
///
/// Purple is not in the list. Purple belongs to the ZORGs, so a ZORG is
/// never mistaken for a player colony.
pub const FOUNDER_COLORS: [[u8; 3]; 16] = [
    [222, 72, 62],    // red
    [236, 138, 44],   // orange
    [234, 196, 62],   // amber
    [186, 206, 68],   // olive
    [106, 200, 84],   // green
    [58, 194, 142],   // spring green
    [56, 194, 188],   // teal
    [72, 188, 224],   // cyan
    [72, 150, 224],   // sky
    [78, 116, 214],   // blue
    [104, 122, 196],  // steel
    [224, 92, 128],   // rose
    [236, 122, 166],  // pink
    [168, 118, 72],   // brown
    [214, 190, 140],  // sand
    [140, 150, 158],  // slate
];

/// The colour of a species.
///
/// The colour belongs to the species and never changes: the lineage that
/// was the colour of an amber on the first day is still that amber when
/// it has become an empire. The ZORGs are always purple.
pub(crate) fn species_team_color(_rng: &mut Rng, species: u32) -> [u8; 3] {
    if species == ZORG_SPECIES {
        return ZORG_COLOR;
    }
    // The colour is the species' place in the list. A run with more
    // species than colours starts again at the top of the list, and the
    // ones that come round a second time are made a little darker so
    // that they are still told apart.
    let n = FOUNDER_COLORS.len();
    let idx = (species.saturating_sub(1) as usize) % n;
    let round = (species.saturating_sub(1) as usize) / n;
    let base = FOUNDER_COLORS[idx];
    // The shade shifts a little with every turn of the wheel.
    let k = 1.0 - 0.11 * (round % 4) as f32;
    [
        (base[0] as f32 * k).clamp(0.0, 255.0) as u8,
        (base[1] as f32 * k).clamp(0.0, 255.0) as u8,
        (base[2] as f32 * k).clamp(0.0, 255.0) as u8,
    ]
}

/// The colour of a species, without needing a random number.
///
/// The colour of a species never changes, so this is the same every time.
pub(crate) fn species_color(species: u32) -> [u8; 3] {
    let mut rng = Rng::new(species.wrapping_mul(0x9E3779B9) as u64 | 1);
    species_team_color(&mut rng, species)
}

/// Turn a hue, a saturation and a value into a colour.
fn hue_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let c = v * s;
    let hp = (h / 60.0).fract();
    let x = c * (1.0 - ((hp * 2.0 - 1.0).abs()));
    let m = v - c;
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [
        ((r + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((g + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((b + m) * 255.0).clamp(0.0, 255.0) as u8,
    ]
}

/// The name of a species.
///
/// The name is generated from the species id, so it never changes: the
/// lineage that was called the Karthani on the first day is still called
/// the Karthani when it has split and started a dynasty of its own.
pub fn species_name_for(species: u32) -> String {
    const ONSET: &[&str] = &[
        "K", "V", "Th", "Gr", "Ul", "Br", "Sk", "Z", "Dr", "Kh", "Mor", "Ash", "Tor", "Nal",
        "Ves", "Cor", "Zan", "Rhu", "Bel", "Dra",
    ];
    const NUCLEUS: &[&str] = &[
        "ar", "or", "ith", "un", "el", "an", "ur", "os", "ir", "ak", "esh", "yn", "ag", "il",
        "ond", "esh", "ai", "ul", "om", "esh",
    ];
    const CODA: &[&str] = &[
        "ni", "ra", "the", "ndar", "kesh", "mir", "loth", "var", "sh", "nak", "dai", "mor",
        "rin", "gal", "thas", "vor",
    ];
    let mut rng = Rng::new(
        species.wrapping_mul(0x9E37_79B9) as u64 ^ 0x51ED_C0DE_u64,
    );
    let mut name = String::new();
    name.push_str(ONSET[rng.range(ONSET.len() as u32) as usize]);
    name.push_str(NUCLEUS[rng.range(NUCLEUS.len() as u32) as usize]);
    // Most species names have a third syllable; some have none, and a few
    // have a fourth.
    let roll = rng.range(10);
    if roll < 8 {
        name.push_str(CODA[rng.range(CODA.len() as u32) as usize]);
    }
    if roll == 9 {
        name.push_str(CODA[rng.range(CODA.len() as u32) as usize]);
        name.push_str(CODA[rng.range(CODA.len() as u32) as usize]);
    }
    name
}

