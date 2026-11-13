//! Crystal 201189f scripts/systems/hazard_primal.lua:25-80; config.lua.dist:255-259.
/// The source timer custom attribute uses integral wall-clock seconds; the
/// four-second addEvent itself is scheduled from the actual creation instant.
#[must_use]
pub fn source_timer_ms(now_us: u64) -> u64 {
    (now_us / 1_000_000) * 1_000
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PodStep {
    Ignore,
    Remove,
    DamageAndRemove { life_drain: u64 },
}

/// Only a Player consumes a Pod on step-in. The current Item owner separately
/// validates custody and atomically removes that exact physical Item.
#[must_use]
pub fn step_in(
    timer_ms: u64,
    now_ms: u64,
    maximum_health: u64,
    hazard: u16,
    is_player: bool,
) -> PodStep {
    if !is_player {
        return PodStep::Ignore;
    }
    let elapsed = now_ms.saturating_sub(timer_ms);
    if now_ms < timer_ms || !(2_000..4_000).contains(&elapsed) {
        return PodStep::Remove;
    }
    // Two ceil operations are observable: 101HP at hazard1 gives 7+500,
    // whereas one combined rounded percentage would yield 6+500.
    let base = (u128::from(maximum_health) * 5).div_ceil(100);
    let adjusted = (base * (100 + u128::from(hazard))).div_ceil(100) + 500;
    PodStep::DamageAndRemove {
        life_drain: u64::try_from(adjusted).unwrap_or(u64::MAX),
    }
}

pub use oteryn_gameplay_durability::durability::creature_source_items::primal_pod_step_item_indices as source_step_item_indices;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PodExpiry {
    pub item: u128,
    pub position: [i32; 3],
    pub force_spawn: bool,
}

/// Crystal resolves getItemById at callback position, not the originally minted
/// Item UUID. Calling this against current scoped custody preserves that rule.
#[must_use]
pub fn expire(
    position: [i32; 3],
    current_pod: impl FnOnce([i32; 3]) -> Option<u128>,
) -> Option<PodExpiry> {
    Some(PodExpiry {
        item: current_pod(position)?,
        position,
        force_spawn: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn primal_pod_source_damage_window_and_integer_ceilings() {
        assert_eq!(step_in(10_000, 11_999, 101, 1, true), PodStep::Remove);
        assert_eq!(
            step_in(10_000, 12_000, 101, 1, true),
            PodStep::DamageAndRemove { life_drain: 507 }
        );
        assert_eq!(
            step_in(10_000, 13_999, 1000, 12, true),
            PodStep::DamageAndRemove { life_drain: 556 }
        );
        assert_eq!(step_in(10_000, 14_000, 1000, 12, true), PodStep::Remove);
        assert_eq!(step_in(10_000, 12_000, 1000, 12, false), PodStep::Ignore);
    }
    #[test]
    fn primal_stacked_pods_preserve_donor_live_tile_index_removal_order() {
        // Crystal movement.cpp:385-398 iterates LIVE tile indices. Removing an
        // always-on-top Pod shifts its immediate neighbour below the next index
        // (tile.cpp:1297-1313,1536-1567). Source top order for39176 is3.
        let step_in_stack = |timers: &[u64], now, maximum, hazard, player| {
            source_step_item_indices(timers.len(), player)
                .into_iter()
                .map(|index| (index, step_in(timers[index], now, maximum, hazard, player)))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            step_in_stack(&[10_000, 11_000], 13_000, 101, 1, true),
            vec![(0, PodStep::DamageAndRemove { life_drain: 507 })]
        );
        assert_eq!(
            step_in_stack(&[10_000, 11_000, 12_000], 13_000, 101, 1, true),
            vec![
                (0, PodStep::DamageAndRemove { life_drain: 507 }),
                (2, PodStep::Remove)
            ]
        );
        assert_eq!(
            step_in_stack(&[10_000, 11_000], 13_000, 101, 1, false),
            vec![(0, PodStep::Ignore)]
        );
    }
    #[test]
    fn primal_pod_timer_uses_source_seconds_and_expiry_resolves_current_position_item() {
        assert_eq!(source_timer_ms(10_999_999), 10_000);
        let position = [33558, 32759, 15];
        assert_eq!(expire(position, |_| None), None);
        assert_eq!(
            expire(position, |_| Some(7)).map(|effect| (effect.item, effect.force_spawn)),
            Some((7, true))
        );
        // Two callbacks at one position each resolve the then-current Item;
        // removing the first Item does not permanently bind the second callback
        // to an obsolete Item UUID.
        assert_eq!(
            expire(position, |_| Some(9)).map(|effect| effect.item),
            Some(9)
        );
    }
}
