//! ENERGY-RING-1: shield absorption after incoming mitigation, before HP damage.

/// Return (absorbed damage, mana spent). A spell shield takes precedence for this hit
/// at 1:1, limited by its capacity. Otherwise worn equipment shields at 2:1. Integer
/// division leaves an odd mana point available; a single mana cannot absorb damage.
/// This matches Canary 99902524 `Game::combatChangeHealth` and Global's 15.25 ring rule.
pub fn absorb_mana_shield(
    damage: u32,
    mana: u32,
    spell_capacity: Option<u32>,
    equipped: bool,
) -> (u32, u32) {
    let (capacity, ratio) = match spell_capacity.filter(|capacity| *capacity > 0) {
        Some(capacity) => (capacity, 1),
        None if equipped => (u32::MAX, 2),
        None => return (0, 0),
    };
    let absorbed = damage.min(capacity).min(mana / ratio);
    (absorbed, absorbed * ratio)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_ring_costs_two_mana_per_damage_including_odd_damage() {
        assert_eq!(absorb_mana_shield(5, 20, None, true), (5, 10));
        assert_eq!(absorb_mana_shield(0, 20, None, true), (0, 0));
        assert_eq!(
            absorb_mana_shield(u32::MAX, u32::MAX, None, true),
            (u32::MAX / 2, u32::MAX - 1)
        );
    }

    #[test]
    fn energy_ring_short_mana_absorbs_whole_damage_points_only() {
        assert_eq!(absorb_mana_shield(5, 7, None, true), (3, 6));
        assert_eq!(absorb_mana_shield(5, 1, None, true), (0, 0));
        assert_eq!(absorb_mana_shield(5, 0, None, true), (0, 0));
    }

    #[test]
    fn energy_ring_has_no_effect_when_not_worn() {
        assert_eq!(absorb_mana_shield(5, 20, None, false), (0, 0));
    }

    #[test]
    fn energy_ring_spell_precedence_keeps_one_to_one_and_no_same_hit_fallback() {
        assert_eq!(absorb_mana_shield(5, 20, Some(3), true), (3, 3));
        assert_eq!(absorb_mana_shield(5, 3, Some(10), true), (3, 3));
        assert_eq!(absorb_mana_shield(5, 20, Some(0), true), (5, 10));
        assert_eq!(absorb_mana_shield(5, 20, Some(10), false), (5, 5));
    }
}
