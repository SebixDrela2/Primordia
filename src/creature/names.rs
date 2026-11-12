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

pub(crate) fn species_team_color(rng: &mut Rng, species: u32) -> [u8; 3] {
    let mut seed = species.wrapping_mul(0x9E3779B9);
    seed ^= seed >> 13;
    seed ^= seed << 17;
    seed ^= seed >> 5;
    let r = ((seed >> 16) & 0xFF) as u8;
    let g = ((seed >> 8) & 0xFF) as u8;
    let b = (seed & 0xFF) as u8;
    [r.max(80), g.max(80), b.max(80)]
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

