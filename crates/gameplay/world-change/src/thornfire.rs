//! Shadowthorn credit follows the elf's authored spawn home across movement and Channels.

// Crystal 201189f8 data-global/world/world-monster.xml and the matching live spawn catalogue.
// The index is the persisted bit position. Preserve existing positions if the map changes.
const ELF_HOMES: [(&str, i32, i32, i16); 54] = [
    ("elf", 33073, 32169, 6),
    ("elf_scout", 33074, 32172, 6),
    ("elf_scout", 33077, 32168, 6),
    ("elf_scout", 33077, 32172, 6),
    ("elf_arcanist", 33043, 32196, 8),
    ("elf_scout", 33043, 32199, 8),
    ("elf_scout", 33046, 32196, 8),
    ("elf", 33047, 32194, 8),
    ("elf", 33047, 32202, 8),
    ("elf", 33051, 32160, 8),
    ("elf_scout", 33051, 32180, 8),
    ("elf", 33051, 32195, 8),
    ("elf_scout", 33053, 32198, 8),
    ("elf", 33053, 32202, 8),
    ("elf", 33054, 32157, 8),
    ("elf", 33054, 32194, 8),
    ("elf_scout", 33055, 32196, 8),
    ("elf", 33056, 32200, 8),
    ("elf_overseer", 33056, 32216, 8),
    ("elf_scout", 33057, 32181, 8),
    ("elf", 33058, 32209, 8),
    ("elf_scout", 33063, 32199, 8),
    ("elf_scout", 33070, 32193, 8),
    ("elf", 33074, 32144, 8),
    ("elf", 33074, 32155, 8),
    ("elf", 33077, 32140, 8),
    ("elf", 33077, 32145, 8),
    ("elf", 33077, 32183, 8),
    ("elf_scout", 33078, 32172, 8),
    ("elf", 33079, 32186, 8),
    ("elf", 33080, 32141, 8),
    ("elf_scout", 33080, 32177, 8),
    ("elf", 33081, 32144, 8),
    ("elf", 33081, 32152, 8),
    ("elf_overseer", 33086, 32199, 8),
    ("elf", 33100, 32148, 8),
    ("elf", 33101, 32139, 8),
    ("elf", 33103, 32152, 8),
    ("elf", 33110, 32146, 8),
    ("elf_overseer", 33122, 32152, 8),
    ("elf_scout", 33033, 32175, 9),
    ("elf_scout", 33034, 32173, 9),
    ("elf_arcanist", 33035, 32176, 9),
    ("elf_arcanist", 33036, 32172, 9),
    ("elf_scout", 33038, 32182, 9),
    ("elf_scout", 33040, 32173, 9),
    ("elf_arcanist", 33040, 32178, 9),
    ("elf_scout", 33041, 32182, 9),
    ("elf_arcanist", 33042, 32176, 9),
    ("elf_scout", 33043, 32173, 9),
    ("elf_arcanist", 33043, 32213, 9),
    ("elf_arcanist", 33046, 32210, 9),
    ("elf_arcanist", 33049, 32213, 9),
    ("elf_scout", 33106, 32146, 9),
];
pub const COMPLETE_PROGRESS: i64 = (1 << ELF_HOMES.len()) - 1;

pub fn elf_home_bit(creature: &str, x: i32, y: i32, floor: i16) -> Option<i64> {
    ELF_HOMES
        .iter()
        .position(|home| *home == (creature, x, y, floor))
        .map(|ordinal| 1 << ordinal)
}

/// The save consumes a complete set and carries an incomplete set into the next day.
pub const fn at_save(progress: i64) -> (i64, bool) {
    if progress == COMPLETE_PROGRESS {
        (0, true)
    } else {
        (progress, false)
    }
}

/// Crystal prepared_bucket.lua:70-83. Exact map-station use only.
pub const PREPARED_BUCKET: &str = "oteryn:item.tibia.i12819";
pub const BOG_WATER: &str = "oteryn:item.tibia.i12721";
pub const FILLED_BUCKET: &str = "oteryn:item.tibia.i12722";

pub fn refill_bucket(used: &str, station: &str) -> Option<&'static str> {
    (used == PREPARED_BUCKET && station == BOG_WATER).then_some(FILLED_BUCKET)
}

pub const PREPARED_BUCKET_ID: u16 = 12819;
pub const FIREFIGHTER_TOTAL: u32 = 500;

/// Crystal prepared_bucket.lua:9-23 plus the Shadowthorn fire region witnessed
/// in data-global/world/world.otbm. A portable field cannot supply a base key.
pub fn is_thornfire(placement: u64, appearance: u16) -> bool {
    let x = placement >> 32;
    let y = (placement >> 16) & 0xffff;
    let floor = (placement >> 8) & 0xff;
    matches!(
        appearance,
        232 | 5061..=5067 | 6288 | 6289 | 7803 | 19096 | 32021
    ) && match floor {
        6 => (33046..=33114).contains(&x) && (32135..=32209).contains(&y),
        7 => (33047..=33114).contains(&x) && (32137..=32204).contains(&y),
        8 => (33055..=33129).contains(&x) && (32149..=32219).contains(&y),
        _ => false,
    }
}

pub const WOLF: &str = "oteryn:creature.thornfire_wolf";

/// Crystal prepared_bucket.lua:24-29. The wiki gives a 0.1–1% range;
/// use one 0.5% draw for the World occurrence and choose one destination.
const WOLF_POSITIONS: [(i32, i32, i16); 4] = [
    (33091, 32149, 7),
    (33079, 32143, 7),
    (33104, 32160, 7),
    (33095, 32158, 7),
];

pub fn wolf_position(chance: u64, destination: u64) -> Option<(i32, i32, i16)> {
    ((u128::from(chance) * 1000) >> 64 < 5).then(|| WOLF_POSITIONS[(destination >> 62) as usize])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thornfire_wolf_uses_one_half_percent_draw_and_one_position() {
        let first_miss = u64::try_from((5_u128 << 64).div_ceil(1000)).ok();
        assert!(first_miss.is_some());
        if let Some(first_miss) = first_miss {
            assert_eq!(wolf_position(first_miss - 1, 0), Some(WOLF_POSITIONS[0]));
            assert_eq!(wolf_position(first_miss, 0), None);
        }
        for (index, &position) in WOLF_POSITIONS.iter().enumerate() {
            assert_eq!(wolf_position(0, (index as u64) << 62), Some(position));
        }
        assert_eq!(wolf_position(u64::MAX, u64::MAX), None);
    }

    #[test]
    fn thornfire_refill_requires_the_prepared_bucket_and_bog_water() {
        assert_eq!(
            refill_bucket(PREPARED_BUCKET, BOG_WATER),
            Some(FILLED_BUCKET)
        );
        assert_eq!(refill_bucket(FILLED_BUCKET, BOG_WATER), None);
        assert_eq!(refill_bucket(PREPARED_BUCKET, FILLED_BUCKET), None);
        assert_eq!(refill_bucket(BOG_WATER, PREPARED_BUCKET), None);
    }

    #[test]
    fn thornfire_requires_a_shadowthorn_fire_placement() {
        let fire = (33074_u64 << 32) | (32135 << 16) | (6 << 8) | 1;
        assert!(is_thornfire(fire, 5063));
        assert!(!is_thornfire(fire | (8 << 8), 5063));
        assert!(!is_thornfire(fire - (30 << 32), 5063));
        assert!(!is_thornfire(fire, 12721));
        assert_eq!(FIREFIGHTER_TOTAL, 500);
    }

    #[test]
    fn thornfire_requires_every_elf_home_and_all_three_overseers_at_save() {
        let mut progress = 0;
        for (ordinal, &(creature, x, y, floor)) in ELF_HOMES.iter().enumerate() {
            assert_eq!(elf_home_bit(creature, x, y, floor), Some(1 << ordinal));
            if creature != "elf_overseer" {
                progress |= 1 << ordinal;
            }
        }
        assert_eq!(elf_home_bit("elf", 33073, 32169, 7), None);
        assert_eq!(elf_home_bit("elf_scout", 33073, 32169, 6), None);
        let overseers: Vec<_> = ELF_HOMES
            .iter()
            .enumerate()
            .filter(|(_, (creature, ..))| *creature == "elf_overseer")
            .map(|(ordinal, _)| 1 << ordinal)
            .collect();
        assert_eq!(overseers.len(), 3);
        for bit in &overseers[..2] {
            progress |= bit;
            progress |= bit;
            assert_eq!(at_save(progress), (progress, false));
        }
        assert_eq!(at_save(progress | overseers[2]), (0, true));
        assert_eq!(at_save(0), (0, false));
    }
}
