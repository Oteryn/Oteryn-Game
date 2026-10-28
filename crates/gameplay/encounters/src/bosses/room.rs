//! Staged boss-room proposals, not admission, death or reward authority.
//! The committed activity handoff, InstanceRuntime and return/recovery owner are not installed.
//! A future owner must resolve current facts and persist the complete decision before mutation.
#![cfg_attr(not(test), allow(dead_code))]
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BossRoomPolicy {
    pub minimum_players: usize,
    pub maximum_players: usize,
    pub minimum_level: u32,
    pub cooldown_ms: u64,
    pub time_limit_ms: u64,
    pub lever_positions: BTreeSet<(i32, i32, i16)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParticipantFact {
    pub character: [u8; 16],
    pub position: (i32, i32, i16),
    pub level: u32,
    pub cooldown_until_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AdmissionRefusal {
    InvalidPolicy,
    Count,
    Duplicate,
    Position,
    Level,
    Cooldown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FreshRoomPlan {
    pub participants: Vec<[u8; 16]>,
    pub remove_existing_creatures: bool,
    pub spawn_boss: bool,
    pub cooldown_until_ms: u64,
    pub deadline_ms: u64,
}

pub(super) fn plan_fresh_room(
    policy: &BossRoomPolicy,
    participants: &[ParticipantFact],
    now_ms: u64,
) -> Result<FreshRoomPlan, AdmissionRefusal> {
    if policy.minimum_players == 0
        || policy.minimum_players > policy.maximum_players
        || policy.maximum_players > 15
        || policy.minimum_level == 0
        || policy.cooldown_ms == 0
        || policy.cooldown_ms > 2_592_000_000
        || policy.time_limit_ms == 0
        || policy.time_limit_ms > 86_400_000
        || now_ms > i64::MAX as u64
        || policy.lever_positions.len() < policy.maximum_players
    {
        return Err(AdmissionRefusal::InvalidPolicy);
    }
    if !(policy.minimum_players..=policy.maximum_players).contains(&participants.len()) {
        return Err(AdmissionRefusal::Count);
    }
    let mut characters = BTreeSet::new();
    let mut positions = BTreeSet::new();
    for participant in participants {
        if !characters.insert(participant.character) {
            return Err(AdmissionRefusal::Duplicate);
        }
        if !policy.lever_positions.contains(&participant.position)
            || !positions.insert(participant.position)
        {
            return Err(AdmissionRefusal::Position);
        }
        if participant.level < policy.minimum_level {
            return Err(AdmissionRefusal::Level);
        }
        if now_ms < participant.cooldown_until_ms {
            return Err(AdmissionRefusal::Cooldown);
        }
    }
    Ok(FreshRoomPlan {
        participants: characters.into_iter().collect(),
        remove_existing_creatures: true,
        spawn_boss: true,
        cooldown_until_ms: now_ms
            .checked_add(policy.cooldown_ms)
            .filter(|deadline| *deadline <= i64::MAX as u64)
            .ok_or(AdmissionRefusal::InvalidPolicy)?,
        deadline_ms: now_ms
            .checked_add(policy.time_limit_ms)
            .filter(|deadline| *deadline <= i64::MAX as u64)
            .ok_or(AdmissionRefusal::InvalidPolicy)?,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TerminalPlan {
    pub reward_characters: Vec<[u8; 16]>,
    pub kick_characters: Vec<[u8; 16]>,
}

pub(super) fn plan_defeat(admitted: &[[u8; 16]], qualified: &BTreeSet<[u8; 16]>) -> TerminalPlan {
    TerminalPlan {
        reward_characters: admitted
            .iter()
            .copied()
            .filter(|id| qualified.contains(id))
            .collect(),
        kick_characters: admitted.to_vec(),
    }
}

pub(super) fn plan_timeout(admitted: &[[u8; 16]]) -> TerminalPlan {
    TerminalPlan {
        reward_characters: Vec::new(),
        kick_characters: admitted.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fact(id: u8, x: i32) -> ParticipantFact {
        ParticipantFact {
            character: [id; 16],
            position: (x, 1, 7),
            level: 100,
            cooldown_until_ms: 0,
        }
    }
    fn policy() -> BossRoomPolicy {
        BossRoomPolicy {
            minimum_players: 2,
            maximum_players: 2,
            minimum_level: 50,
            cooldown_ms: 10_000,
            time_limit_ms: 60_000,
            lever_positions: [(1, 1, 7), (2, 1, 7)].into(),
        }
    }
    #[test]
    fn admission_resets_spawns_and_sets_group_cooldown() -> Result<(), AdmissionRefusal> {
        let plan = plan_fresh_room(&policy(), &[fact(2, 2), fact(1, 1)], 100)?;
        assert!(plan.remove_existing_creatures && plan.spawn_boss);
        assert_eq!(plan.participants, vec![[1; 16], [2; 16]]);
        assert_eq!((plan.cooldown_until_ms, plan.deadline_ms), (10_100, 60_100));
        Ok(())
    }
    #[test]
    fn one_cooldown_refuses_the_whole_group() {
        let mut blocked = fact(2, 2);
        blocked.cooldown_until_ms = 101;
        assert_eq!(
            plan_fresh_room(&policy(), &[fact(1, 1), blocked], 100),
            Err(AdmissionRefusal::Cooldown)
        );
    }
    #[test]
    fn admission_rejects_invalid_members_without_a_partial_plan() {
        for (participants, expected) in [
            (vec![fact(1, 1)], AdmissionRefusal::Count),
            (vec![fact(1, 1), fact(1, 2)], AdmissionRefusal::Duplicate),
            (vec![fact(1, 1), fact(2, 1)], AdmissionRefusal::Position),
            (vec![fact(1, 1), fact(2, 3)], AdmissionRefusal::Position),
            (
                vec![
                    fact(1, 1),
                    ParticipantFact {
                        level: 49,
                        ..fact(2, 2)
                    },
                ],
                AdmissionRefusal::Level,
            ),
        ] {
            assert_eq!(
                plan_fresh_room(&policy(), &participants, 100),
                Err(expected)
            );
        }
        let mut ready = fact(2, 2);
        ready.cooldown_until_ms = 100;
        assert!(plan_fresh_room(&policy(), &[fact(1, 1), ready], 100).is_ok());
    }

    #[test]
    fn admission_rejects_policy_and_persisted_deadline_overflow() {
        let base = policy();
        for invalid in [
            BossRoomPolicy {
                minimum_players: 0,
                ..base.clone()
            },
            BossRoomPolicy {
                minimum_players: 3,
                ..base.clone()
            },
            BossRoomPolicy {
                maximum_players: 16,
                ..base.clone()
            },
            BossRoomPolicy {
                minimum_level: 0,
                ..base.clone()
            },
            BossRoomPolicy {
                cooldown_ms: 0,
                ..base.clone()
            },
            BossRoomPolicy {
                cooldown_ms: 2_592_000_001,
                ..base.clone()
            },
            BossRoomPolicy {
                time_limit_ms: 0,
                ..base.clone()
            },
            BossRoomPolicy {
                time_limit_ms: 86_400_001,
                ..base.clone()
            },
            BossRoomPolicy {
                lever_positions: BTreeSet::new(),
                ..base
            },
        ] {
            assert_eq!(
                plan_fresh_room(&invalid, &[fact(1, 1), fact(2, 2)], 100),
                Err(AdmissionRefusal::InvalidPolicy),
            );
        }
        for now in [i64::MAX as u64, u64::MAX] {
            assert_eq!(
                plan_fresh_room(&policy(), &[fact(1, 1), fact(2, 2)], now),
                Err(AdmissionRefusal::InvalidPolicy),
            );
        }
    }

    #[test]
    fn defeat_rewards_only_qualified_and_timeout_rewards_nobody() {
        let admitted = [[1; 16], [2; 16]];
        let qualified = BTreeSet::from([[2; 16]]);
        assert_eq!(
            plan_defeat(&admitted, &qualified).reward_characters,
            vec![[2; 16]]
        );
        assert_eq!(
            plan_timeout(&admitted),
            TerminalPlan {
                reward_characters: vec![],
                kick_characters: admitted.to_vec()
            }
        );
    }
}
