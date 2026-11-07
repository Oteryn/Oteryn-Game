//! Personal reward-boss containers.
//!
//! A kill freezes the complete participant census before any reward is selected.  The frozen
//! record is deliberately value-owned: later combat events cannot add a recipient or alter the
//! contribution used by the loot selector.  Persistence adapters may store `PersonalRewardChest`
//! in either the depot reward inbox or the ordinary character inbox; collection remains an
//! all-or-nothing custody operation.

use super::RewardParticipation;

/// Global's personal boss rewards remain claimable for exactly seven days from their mint.
pub const REWARD_EXPIRY_MS: u64 = 7 * 24 * 60 * 60 * 1_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersonalRewardChest<I> {
    pub character: [u8; 16],
    pub boss_key: String,
    pub death: [u8; 16],
    pub contribution: RewardParticipation,
    pub expires_at_unix_ms: u64,
    items: Vec<I>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollectAllRefusal {
    Expired,
    InventoryFull,
}

impl<I> PersonalRewardChest<I> {
    /// Rehydrates one retained personal group. Its expiry is derived from the mint, never
    /// supplied by the client. The durable owner proves ownership before calling this.
    pub fn reopen(
        boss_key: String,
        death: [u8; 16],
        minted_at_unix_ms: u64,
        participant: FrozenBossContribution,
        items: Vec<I>,
    ) -> Option<Self> {
        Some(Self {
            character: participant.character,
            boss_key,
            death,
            contribution: participant.contribution,
            expires_at_unix_ms: reward_expiry(minted_at_unix_ms)?,
            items,
        })
    }

    pub fn items(&self) -> &[I] {
        &self.items
    }

    /// Moves every item or none. Capacity is checked before the chest is mutated, so a refused
    /// command cannot strand a prefix in the inventory.
    pub fn collect_all(
        &mut self,
        now_unix_ms: u64,
        inventory: &mut Vec<I>,
        inventory_capacity: usize,
    ) -> Result<(), CollectAllRefusal> {
        if now_unix_ms >= self.expires_at_unix_ms {
            self.items.clear();
            return Err(CollectAllRefusal::Expired);
        }
        if inventory.len().saturating_add(self.items.len()) > inventory_capacity {
            return Err(CollectAllRefusal::InventoryFull);
        }
        inventory.append(&mut self.items);
        Ok(())
    }

    /// The expiry worker removes the contents exactly at the frozen deadline.
    pub fn expire(&mut self, now_unix_ms: u64) -> bool {
        if now_unix_ms < self.expires_at_unix_ms {
            return false;
        }
        self.items.clear();
        true
    }

    /// Crystal `game.cpp:3502-3542`: a collection may deliver a prefix. The custody owner
    /// supplies the already weight-qualified slot budget; each selected prefix still passes
    /// through the atomic `collect_all` operation. Zero `max_items` means no count limit.
    pub fn collect_prefix(
        &mut self,
        now_unix_ms: u64,
        inventory: &mut Vec<I>,
        inventory_capacity: usize,
        max_items: usize,
    ) -> Result<usize, CollectAllRefusal> {
        if self.expire(now_unix_ms) {
            return Err(CollectAllRefusal::Expired);
        }
        if self.items.is_empty() {
            return Ok(0);
        }
        let available = inventory_capacity.saturating_sub(inventory.len());
        let requested = if max_items == 0 {
            self.items.len()
        } else {
            max_items
        };
        let count = self.items.len().min(available).min(requested);
        if count == 0 {
            return Err(CollectAllRefusal::InventoryFull);
        }
        let remainder = self.items.split_off(count);
        self.collect_all(now_unix_ms, inventory, inventory_capacity)?;
        self.items = remainder;
        Ok(count)
    }
}

/// Immutable input captured under the encounter owner's death lock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrozenBossContribution {
    pub character: [u8; 16],
    pub contribution: RewardParticipation,
}

/// Crystal `data/scripts/systems/reward_chest.lua:28-83`. Totals start at 0.1; damage,
/// incoming damage and healing of others have equal shares. Nonzero participants determine
/// the expected share. Oteryn's explicitly retained anti-leech eligibility gate is separate.
pub fn contribution_loot_factors(participants: &[FrozenBossContribution]) -> Vec<f64> {
    let scores = contribution_scores(participants);
    let count = scores.iter().filter(|score| **score > 0.0).count();
    if count == 0 {
        return vec![0.0; participants.len()];
    }
    let count = count as f64;
    let base = count.powf(-1.0 / 3.0);
    scores
        .into_iter()
        .map(|score| {
            if score == 0.0 {
                0.0
            } else {
                base * (1.0 + base).powf(score * count)
            }
        })
        .collect()
}

/// Unique boss loot goes to the highest score (Crystal `functions/monster.lua:215-220`).
/// Lua does not define equal-score order; Character identity gives ties a stable replay order.
pub fn top_contributor(participants: &[FrozenBossContribution]) -> Option<[u8; 16]> {
    participants
        .iter()
        .zip(contribution_scores(participants))
        .filter(|(_, score)| *score > 0.0)
        .max_by(|(left, ls), (right, rs)| {
            ls.total_cmp(rs)
                .then_with(|| right.character.cmp(&left.character))
        })
        .map(|(participant, _)| participant.character)
}

fn contribution_scores(participants: &[FrozenBossContribution]) -> Vec<f64> {
    let totals = participants
        .iter()
        .fold([0.1; 3], |mut total, participant| {
            total[0] += participant.contribution.damage_dealt as f64;
            total[1] += participant.contribution.damage_received as f64;
            total[2] += participant.contribution.healing_others as f64;
            total
        });
    participants
        .iter()
        .map(|participant| {
            (participant.contribution.damage_dealt as f64 / totals[0]
                + participant.contribution.damage_received as f64 / totals[1]
                + participant.contribution.healing_others as f64 / totals[2])
                / 3.0
        })
        .collect()
}

/// Builds one private container per eligible contributor. `rewards` is invoked only after the
/// census has been copied, and receives the frozen damage/healing/tanking tuple.
pub fn freeze_personal_rewards<I>(
    boss_key: &str,
    death: [u8; 16],
    expires_at_unix_ms: u64,
    participants: &[FrozenBossContribution],
    mut rewards: impl FnMut([u8; 16], RewardParticipation) -> Vec<I>,
) -> Vec<PersonalRewardChest<I>> {
    let eligibility = super::reward_eligibility(
        &participants
            .iter()
            .map(|participant| participant.contribution)
            .collect::<Vec<_>>(),
    );
    let frozen: Vec<_> = participants
        .iter()
        .zip(eligibility)
        .filter(|(_, eligible)| *eligible)
        .map(|(participant, _)| (participant.character, participant.contribution))
        .collect();
    frozen
        .into_iter()
        .map(|(character, contribution)| PersonalRewardChest {
            character,
            boss_key: boss_key.to_owned(),
            death,
            contribution,
            expires_at_unix_ms,
            items: rewards(character, contribution),
        })
        .collect()
}

/// Computes the fixed Global deadline without allowing caller-selected retention.
pub fn reward_expiry(minted_at_unix_ms: u64) -> Option<u64> {
    minted_at_unix_ms.checked_add(REWARD_EXPIRY_MS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contribution(damage: u64, tanking: u64, healing: u64) -> RewardParticipation {
        RewardParticipation {
            damage_dealt: damage,
            damage_received: tanking,
            healing_others: healing,
        }
    }

    #[test]
    fn donor_score_keeps_all_three_dimensions_and_zero_participants_safe() {
        let participants = [
            FrozenBossContribution {
                character: [1; 16],
                contribution: contribution(100, 0, 0),
            },
            FrozenBossContribution {
                character: [2; 16],
                contribution: contribution(0, 100, 100),
            },
            FrozenBossContribution {
                character: [3; 16],
                contribution: contribution(0, 0, 0),
            },
        ];
        let factors = contribution_loot_factors(&participants);
        assert!(factors[1] > factors[0]);
        assert_eq!(factors[2], 0.0);
        assert_eq!(top_contributor(&participants), Some([2; 16]));
        assert_eq!(contribution_loot_factors(&participants[2..]), vec![0.0]);
        assert_eq!(top_contributor(&participants[2..]), None);
        assert!(contribution_loot_factors(&[]).is_empty());
    }

    #[test]
    fn donor_factor_uses_point_one_totals_and_stable_equal_score_order() {
        let solo = [FrozenBossContribution {
            character: [2; 16],
            contribution: contribution(1, 0, 0),
        }];
        let expected = 2.0_f64.powf((1.0 / 1.1) / 3.0);
        assert!((contribution_loot_factors(&solo)[0] - expected).abs() < 1e-12);
        let tie = [
            solo[0],
            FrozenBossContribution {
                character: [1; 16],
                ..solo[0]
            },
        ];
        assert_eq!(top_contributor(&tie), Some([1; 16]));
    }

    #[test]
    fn collecting_a_prefix_leaves_the_remainder_for_a_later_command() -> Result<(), &'static str> {
        let mut chest = PersonalRewardChest::reopen(
            "oteryn:boss.test".into(),
            [8; 16],
            1,
            FrozenBossContribution {
                character: [1; 16],
                contribution: contribution(1, 0, 0),
            },
            vec![1, 2, 3],
        )
        .ok_or("bounded timestamp")?;
        let mut inventory = vec![];
        assert_eq!(chest.collect_prefix(2, &mut inventory, 3, 1), Ok(1));
        assert_eq!(inventory, vec![1]);
        assert_eq!(chest.items(), &[2, 3]);
        assert_eq!(chest.collect_prefix(2, &mut inventory, 2, 0), Ok(1));
        assert_eq!(inventory, vec![1, 2]);
        assert_eq!(chest.items(), &[3]);
        assert_eq!(
            chest.collect_prefix(2, &mut inventory, 2, 0),
            Err(CollectAllRefusal::InventoryFull)
        );
        Ok(())
    }

    #[test]
    fn two_contributors_receive_private_frozen_rewards_and_non_contributor_receives_none() {
        let participants = [
            FrozenBossContribution {
                character: [1; 16],
                contribution: contribution(40, 10, 0),
            },
            FrozenBossContribution {
                character: [2; 16],
                contribution: contribution(1, 0, 30),
            },
            FrozenBossContribution {
                character: [3; 16],
                contribution: contribution(0, 99, 99),
            },
        ];
        let chests = freeze_personal_rewards(
            "oteryn:boss.fixture",
            [9; 16],
            500,
            &participants,
            |character, frozen| vec![(character[0], frozen.damage_dealt, frozen.healing_others)],
        );
        assert_eq!(chests.len(), 2);
        assert_eq!(chests[0].items(), &[(1, 40, 0)]);
        assert_eq!(chests[1].items(), &[(2, 1, 30)]);
        assert!(chests.iter().all(|chest| chest.character != [3; 16]));
    }

    #[test]
    fn expiry_removes_items_and_collect_all_is_atomic_when_inventory_is_full() {
        let mut chest = freeze_personal_rewards(
            "oteryn:boss.fixture",
            [9; 16],
            500,
            &[FrozenBossContribution {
                character: [1; 16],
                contribution: contribution(1, 2, 3),
            }],
            |_, _| vec![10, 11],
        )
        .remove(0);
        let mut inventory = vec![1];
        assert_eq!(
            chest.collect_all(499, &mut inventory, 2),
            Err(CollectAllRefusal::InventoryFull)
        );
        assert_eq!(inventory, vec![1]);
        assert_eq!(chest.items(), &[10, 11]);
        assert!(chest.expire(500));
        assert!(chest.items().is_empty());
    }

    #[test]
    fn collect_all_moves_every_item_when_capacity_is_available() {
        let mut chest = freeze_personal_rewards(
            "oteryn:boss.fixture",
            [9; 16],
            500,
            &[FrozenBossContribution {
                character: [1; 16],
                contribution: contribution(1, 2, 3),
            }],
            |_, _| vec![10, 11],
        )
        .remove(0);
        let mut inventory = vec![1];
        assert_eq!(chest.collect_all(499, &mut inventory, 3), Ok(()));
        assert_eq!(inventory, vec![1, 10, 11]);
        assert!(chest.items().is_empty());
    }

    #[test]
    fn expiry_is_exactly_seven_days_and_overflow_refuses() {
        assert_eq!(reward_expiry(9), Some(9 + REWARD_EXPIRY_MS));
        assert_eq!(reward_expiry(u64::MAX), None);
    }

    #[test]
    fn contribution_threshold_excludes_a_one_hit_leech_before_drawing() {
        let participants = [
            FrozenBossContribution {
                character: [1; 16],
                contribution: contribution(1_000_000, 500_000, 0),
            },
            FrozenBossContribution {
                character: [2; 16],
                contribution: contribution(1, 0, 0),
            },
        ];
        let chests = freeze_personal_rewards(
            "oteryn:boss.fixture",
            [9; 16],
            REWARD_EXPIRY_MS,
            &participants,
            |character, _| vec![character[0]],
        );
        assert_eq!(chests.len(), 1);
        assert_eq!(chests[0].character, [1; 16]);
    }
}
