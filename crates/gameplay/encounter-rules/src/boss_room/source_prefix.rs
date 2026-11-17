//! Crystal boss_lever.lua181-214 and lever.lua143-155: earlier qualifying
//! pads execute onUseExtra even when a later player's condition refuses entry.
use super::*;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePadPrefixPlan {
    pub eligible: usize,
    pub refusal: Option<AdmissionRefusal>,
}
pub fn plan_on_use_extra_prefix(
    policy: &BossRoomPolicy,
    participants_in_pad_order: &[ParticipantFact],
    now_ms: u64,
) -> SourcePadPrefixPlan {
    // Reuse the ordinary owner's structural policy/count/identity checks.
    // They describe a genuine pad census, not a later player's condition.
    let mut structural = participants_in_pad_order.to_vec();
    for participant in &mut structural {
        participant.level = policy.minimum_level;
        participant.cooldown_until_ms = 0;
    }
    if let Err(reason) = plan_fresh_room(policy, &structural, now_ms) {
        return SourcePadPrefixPlan {
            eligible: 0,
            refusal: Some(reason),
        };
    }
    for (eligible, participant) in participants_in_pad_order.iter().enumerate() {
        let refusal = if participant.level < policy.minimum_level {
            Some(AdmissionRefusal::Level)
        } else if now_ms < participant.cooldown_until_ms {
            Some(AdmissionRefusal::Cooldown)
        } else {
            None
        };
        if refusal.is_some() {
            return SourcePadPrefixPlan { eligible, refusal };
        }
    }
    SourcePadPrefixPlan {
        eligible: participants_in_pad_order.len(),
        refusal: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn policy(minimum_players: usize) -> BossRoomPolicy {
        BossRoomPolicy {
            minimum_players,
            maximum_players: 5,
            minimum_level: 250,
            cooldown_ms: 72_000_000,
            time_limit_ms: 1_200_000,
            lever_positions: (0..5).map(|i| (33773 + i, 31601, 14)).collect(),
        }
    }
    fn players() -> Vec<ParticipantFact> {
        (0_u8..3)
            .map(|i| ParticipantFact {
                character: [i + 1; 16],
                position: (33773 + i32::from(i), 31601, 14),
                level: 500,
                cooldown_until_ms: 0,
            })
            .collect()
    }
    #[test]
    fn srx3_source_pad_prefix_retains_earlier_extras_before_later_level_or_cooldown_failure() {
        let mut facts = players();
        assert_eq!(
            plan_on_use_extra_prefix(&policy(1), &facts, 1000),
            SourcePadPrefixPlan {
                eligible: 3,
                refusal: None
            }
        );
        facts[1].level = 249;
        assert_eq!(
            plan_on_use_extra_prefix(&policy(1), &facts, 1000),
            SourcePadPrefixPlan {
                eligible: 1,
                refusal: Some(AdmissionRefusal::Level)
            }
        );
        facts[1].level = 500;
        facts[1].cooldown_until_ms = 1001;
        assert_eq!(
            plan_on_use_extra_prefix(&policy(1), &facts, 1000),
            SourcePadPrefixPlan {
                eligible: 1,
                refusal: Some(AdmissionRefusal::Cooldown)
            }
        );
        assert_eq!(
            plan_on_use_extra_prefix(&policy(1), &facts, 1001),
            SourcePadPrefixPlan {
                eligible: 3,
                refusal: None
            }
        );
    }
    #[test]
    fn srx3_source_pad_prefix_checks_minimum_players_before_any_extra() {
        let facts = &players()[..1];
        assert_eq!(
            plan_on_use_extra_prefix(&policy(2), facts, 1000),
            SourcePadPrefixPlan {
                eligible: 0,
                refusal: Some(AdmissionRefusal::Count)
            }
        );
    }
    #[test]
    fn srx3_source_pad_prefix_later_level_failure_keeps_exact_earlier_extra() {
        let mut facts = players();
        facts[1].level = 249;
        assert_eq!(
            plan_on_use_extra_prefix(&policy(1), &facts, 1000),
            SourcePadPrefixPlan {
                eligible: 1,
                refusal: Some(AdmissionRefusal::Level)
            }
        );
    }
    #[test]
    fn srx3_source_pad_prefix_later_cooldown_failure_keeps_exact_earlier_extra() {
        let mut facts = players();
        facts[1].cooldown_until_ms = 1001;
        assert_eq!(
            plan_on_use_extra_prefix(&policy(1), &facts, 1000),
            SourcePadPrefixPlan {
                eligible: 1,
                refusal: Some(AdmissionRefusal::Cooldown)
            }
        );
    }
}
