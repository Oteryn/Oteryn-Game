//! Exaltation Forge creature arithmetic.
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeCreature {
    Influenced(u8),
    Fiendish,
}

pub const FIENDISH_RESPAWN_SECONDS: u16 = 270;
pub const FIENDISH_SIMULTANEOUS_MAX: u8 = 3;
pub const INFLUENCED_SIMULTANEOUS_MAX: u16 = 300;
pub const SHARED_XP_RANGE_XY: u16 = 30;
pub const FIENDISH_RESPAWN_MICROS: u64 = FIENDISH_RESPAWN_SECONDS as u64 * 1_000_000;

/// Stable facts owned by the spawn lane. `eligible` is content admission's decision (bosses,
/// summons and harmless creatures are excluded there); this module deliberately does not infer
/// eligibility from a creature name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgeSpawnCandidate {
    pub spawn_identity: u64,
    pub eligible: bool,
    pub alive: bool,
}

/// A bounded, deterministic world-tick plan. The production owner applies these identities to
/// its live spawn records; keeping the plan value-only makes retrying a tick side-effect free.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiendishTickPlan {
    pub selected: Vec<u64>,
    pub next_tick_micros: u64,
}

/// Select enough eligible live spawns to restore the pinned three-fiend population. Existing
/// fiends are retained first and candidates are then ordered by a deterministic per-tick draw, so
/// replicas produce the same plan without random or wall-clock authority.
#[must_use]
pub fn plan_fiendish_tick(
    now_micros: u64,
    world_seed: u64,
    existing: &[u64],
    candidates: &[ForgeSpawnCandidate],
) -> FiendishTickPlan {
    let mut selected: Vec<u64> = existing
        .iter()
        .copied()
        .filter(|id| {
            candidates
                .iter()
                .any(|c| c.spawn_identity == *id && c.eligible && c.alive)
        })
        .collect();
    selected.sort_unstable();
    selected.dedup();
    selected.truncate(usize::from(FIENDISH_SIMULTANEOUS_MAX));

    let mut eligible: Vec<u64> = candidates
        .iter()
        .filter(|c| c.eligible && c.alive && !selected.contains(&c.spawn_identity))
        .map(|c| c.spawn_identity)
        .collect();
    eligible.sort_unstable_by_key(|id| {
        let draw: [u8; 32] = Sha256::new()
            .chain_update(b"FORGECREATURE/fiend-selection/v1")
            .chain_update(world_seed.to_be_bytes())
            .chain_update(now_micros.to_be_bytes())
            .chain_update(id.to_be_bytes())
            .finalize()
            .into();
        (draw, *id)
    });
    let missing = usize::from(FIENDISH_SIMULTANEOUS_MAX).saturating_sub(selected.len());
    selected.extend(eligible.into_iter().take(missing));
    FiendishTickPlan {
        selected,
        next_tick_micros: now_micros.saturating_add(FIENDISH_RESPAWN_MICROS),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgeRecipientFacts {
    pub premium: bool,
    pub logout_blocked: bool,
    pub delta_x: u16,
    pub delta_y: u16,
    pub floor_delta: u8,
}

/// Global dust recipient gate. Premium is deliberately only a dust gate;
/// corpse slivers are ordinary loot and never pass through this function.
#[must_use]
pub const fn eligible_dust_recipient(facts: ForgeRecipientFacts) -> bool {
    facts.premium
        && facts.logout_blocked
        && facts.delta_x <= SHARED_XP_RANGE_XY
        && facts.delta_y <= SHARED_XP_RANGE_XY
        && facts.floor_delta <= 1
}

/// Pinned fiendish sliver quantity from a deterministic roll.
#[must_use]
pub const fn slivers(roll: u8) -> u16 {
    3 + (roll % 5) as u16
}

impl ForgeCreature {
    pub fn new_influenced(stacks: u8) -> Option<Self> {
        (1..=5)
            .contains(&stacks)
            .then_some(Self::Influenced(stacks))
    }

    #[must_use]
    pub const fn reward_stacks(self) -> u8 {
        match self {
            Self::Influenced(stacks) => stacks,
            Self::Fiendish => 15,
        }
    }

    /// Global health scaling: base 35%, plus 15% for every stack.
    #[must_use]
    pub const fn health_percent(self) -> u16 {
        135 + self.reward_stacks() as u16 * 15
    }

    /// Exact pinned OTS arithmetic. Both operands of `(stack + 10) / 10` are integers there,
    /// so influenced stacks retain the base award and the fiendish stack doubles it.
    #[must_use]
    pub const fn experience_multiplier(self) -> u8 {
        (self.reward_stacks() + 10) / 10
    }

    #[must_use]
    pub fn experience(self, base: i64) -> Option<i64> {
        base.checked_mul(self.experience_multiplier() as i64)
    }

    /// One through three dust per reward stack; randomness belongs to the simulation owner.
    #[must_use]
    pub const fn dust(self, roll: u8) -> u32 {
        let stacks = self.reward_stacks() as u32;
        stacks + roll as u32 % (2 * stacks + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::{ForgeCreature, ForgeRecipientFacts};

    #[test]
    fn global_stack_health_and_dust_bounds() {
        assert!(ForgeCreature::new_influenced(0).is_none());
        assert!(ForgeCreature::new_influenced(6).is_none());
        for stacks in 1..=5 {
            if let Some(creature) = ForgeCreature::new_influenced(stacks) {
                assert_eq!(creature.health_percent(), 135 + u16::from(stacks) * 15);
                assert_eq!(creature.dust(0), u32::from(stacks));
                assert_eq!(creature.dust(2 * stacks), u32::from(stacks) * 3);
                for roll in 0..=u8::MAX {
                    assert!(
                        (u32::from(stacks)..=u32::from(stacks) * 3).contains(&creature.dust(roll))
                    );
                }
            } else {
                assert!(ForgeCreature::new_influenced(stacks).is_some());
            }
        }
        assert_eq!(ForgeCreature::Fiendish.health_percent(), 360);
        assert_eq!(ForgeCreature::Influenced(1).experience(100), Some(100));
        assert_eq!(ForgeCreature::Influenced(5).experience(100), Some(100));
        assert_eq!(ForgeCreature::Fiendish.experience(5), Some(10));
        assert_eq!(ForgeCreature::Fiendish.dust(0), 15);
        assert_eq!(ForgeCreature::Fiendish.dust(30), 45);
    }

    #[test]
    fn recipient_requires_every_global_gate() {
        let eligible = ForgeRecipientFacts {
            premium: true,
            logout_blocked: true,
            delta_x: 30,
            delta_y: 30,
            floor_delta: 1,
        };
        assert!(super::eligible_dust_recipient(eligible));
        assert!(!super::eligible_dust_recipient(ForgeRecipientFacts {
            premium: false,
            ..eligible
        }));
        assert!(!super::eligible_dust_recipient(ForgeRecipientFacts {
            logout_blocked: false,
            ..eligible
        }));
        assert!(!super::eligible_dust_recipient(ForgeRecipientFacts {
            delta_x: 31,
            ..eligible
        }));
        assert!(!super::eligible_dust_recipient(ForgeRecipientFacts {
            delta_y: 31,
            ..eligible
        }));
        assert!(!super::eligible_dust_recipient(ForgeRecipientFacts {
            floor_delta: 2,
            ..eligible
        }));
    }

    #[test]
    fn fiendish_slivers_stay_in_the_pinned_range() {
        assert_eq!(super::slivers(0), 3);
        assert_eq!(super::slivers(4), 7);
        assert_eq!(super::slivers(u8::MAX), 3);
    }

    #[test]
    fn tick_retains_live_fiends_and_refills_to_three() {
        use super::{FIENDISH_RESPAWN_MICROS, ForgeSpawnCandidate};
        let candidates: Vec<_> = (1..=7)
            .map(|spawn_identity| ForgeSpawnCandidate {
                spawn_identity,
                eligible: spawn_identity != 6,
                alive: spawn_identity != 7,
            })
            .collect();
        let plan = super::plan_fiendish_tick(9, 3, &[2, 7, 99], &candidates);
        assert_eq!(plan.selected, vec![2, 4, 1]);
        assert_eq!(
            plan,
            super::plan_fiendish_tick(9, 3, &[2, 7, 99], &candidates)
        );
        assert_eq!(
            super::plan_fiendish_tick(10, 3, &[2, 7, 99], &candidates).selected,
            vec![2, 5, 4]
        );
        assert_eq!(plan.next_tick_micros, 9 + FIENDISH_RESPAWN_MICROS);
    }

    #[test]
    fn tick_never_exceeds_available_or_global_cap() {
        use super::ForgeSpawnCandidate;
        let candidates = [ForgeSpawnCandidate {
            spawn_identity: 8,
            eligible: true,
            alive: true,
        }];
        assert_eq!(
            super::plan_fiendish_tick(u64::MAX, 0, &[], &candidates).selected,
            [8]
        );
        assert_eq!(
            super::plan_fiendish_tick(u64::MAX, 0, &[], &candidates).next_tick_micros,
            u64::MAX
        );
    }
}
