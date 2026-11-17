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

#[cfg(test)]
mod tests {
    use super::*;

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
