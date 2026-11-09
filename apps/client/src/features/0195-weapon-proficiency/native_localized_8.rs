use super::*;
pub(in crate::play) fn choose(p: &mut WeaponProficiencyPanel, field: usize, row: usize) -> bool {
    if p.pending || !p.selected {
        return false;
    }
    match field {
        0 => p.select_weapon(row),
        1 => p.choose_level(row),
        2 => {
            let valid = p
                .weapon_entry()
                .and_then(|entry| entry.levels.get(usize::from(p.level)))
                .is_some_and(|level| row < level.perks.len());
            if valid {
                p.perk = row as u8;
            }
            valid
        }
        3 if row < 2 => {
            p.slot = row as u8 + 1;
            p.shaping_choice = 0;
            true
        }
        5 => {
            let valid = p
                .shaping
                .as_ref()
                .and_then(|state| state.slots.get(usize::from(p.slot).wrapping_sub(1)))
                .and_then(Option::as_ref)
                .is_some_and(|slot| row < slot.offers.len());
            if valid {
                p.shaping_choice = row as u8;
            }
            valid
        }
        _ => false,
    }
}
