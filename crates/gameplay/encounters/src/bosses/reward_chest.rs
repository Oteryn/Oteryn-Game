//! Personal reward-boss containers.
//!
//! A kill freezes the complete participant census before any reward is selected.  The frozen
//! record is deliberately value-owned: later combat events cannot add a recipient or alter the
//! contribution used by the loot selector.  Persistence adapters may store `PersonalRewardChest`
//! in either the depot reward inbox or the ordinary character inbox; collection remains an
//! all-or-nothing custody operation.

use oteryn_gameplay_encounter_rules::boss::RewardParticipation;

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
}

/// Immutable input captured under the encounter owner's death lock.
pub struct FrozenBossContribution {
    pub character: [u8; 16],
    pub contribution: RewardParticipation,
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
    let eligibility = oteryn_gameplay_encounter_rules::boss::reward_eligibility(
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
