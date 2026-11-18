//! Source cooldown admission for the existing, independently qualified activity owner.
use crate::durability::activity_handoff::boss_room::BossRoomDefinition;
use oteryn_gameplay_encounters::bosses::endgame_entry::{Encounter, source_entry_policy};

use super::lore_keeper::LORE_KEEPER;
const LORE_KEEPER_COOLDOWN_MS: u64 = 14 * 24 * 60 * 60 * 1000;
const LORE_KEEPER_LIMIT_MS: u64 = 30 * 60 * 1000;

pub(super) fn matches_source_cooldown(definition: &BossRoomDefinition) -> bool {
    if definition.boss_creature_key == LORE_KEEPER {
        return matches_lore_keeper(&definition.policy)
            && definition
                .entry_positions
                .iter()
                .all(|cell| (cell.x, cell.y, cell.floor) == (31985, 32851, 14))
            && (
                definition.exit_position.x,
                definition.exit_position.y,
                definition.exit_position.floor,
            ) == (32035, 32859, 14);
    }
    matches(&definition.boss_creature_key, definition.policy.cooldown_ms)
}

pub(super) fn matches_source_initiator(
    definition: &BossRoomDefinition,
    position: crate::foundation::MovementLocalPosition,
) -> bool {
    definition.boss_creature_key != LORE_KEEPER
        || (position.x, position.y, position.floor) == (32019, 32844, 14)
}

/// Crystal 201189f, actions_the_last_lore_keeper.lua:31-82.
/// The shared boss-room planner applies the selected policy to every staged member.
fn matches_lore_keeper(
    policy: &oteryn_gameplay_encounter_rules::boss_room::BossRoomPolicy,
) -> bool {
    policy.minimum_players == 1
        && policy.maximum_players == 15
        && policy.minimum_level == 1
        && policy.cooldown_ms == LORE_KEEPER_COOLDOWN_MS
        && policy.time_limit_ms == LORE_KEEPER_LIMIT_MS
        && policy.lever_positions.len() == 15
        && policy
            .lever_positions
            .iter()
            .copied()
            .eq((32018..=32020).flat_map(|x| (32844..=32848).map(move |y| (x, y, 14))))
}

fn matches(key: &str, cooldown_ms: u64) -> bool {
    let Some(encounter) = Encounter::from_boss_key(key) else {
        return true;
    };
    source_entry_policy(encounter)
        .is_some_and(|policy| policy.retry_seconds.checked_mul(1000) == Some(cooldown_ms))
}

#[cfg(test)]
mod tests {
    use super::{LORE_KEEPER_COOLDOWN_MS, LORE_KEEPER_LIMIT_MS, matches, matches_lore_keeper};

    #[test]
    fn fknow_lore_keeper_requires_the_source_party_window() {
        use oteryn_gameplay_encounter_rules::boss_room::{
            AdmissionRefusal, BossRoomPolicy, ParticipantFact, plan_fresh_room, plan_timeout,
        };
        let mut policy = BossRoomPolicy {
            minimum_players: 1,
            maximum_players: 15,
            minimum_level: 1,
            cooldown_ms: LORE_KEEPER_COOLDOWN_MS,
            time_limit_ms: LORE_KEEPER_LIMIT_MS,
            lever_positions: (32018..=32020)
                .flat_map(|x| (32844..=32848).map(move |y| (x, y, 14)))
                .collect(),
        };
        assert!(matches_lore_keeper(&policy));
        let mut party = [
            ParticipantFact {
                character: [1; 16],
                position: (32019, 32844, 14),
                level: 8,
                cooldown_until_ms: 0,
            },
            ParticipantFact {
                character: [2; 16],
                position: (32020, 32845, 14),
                level: 8,
                cooldown_until_ms: 0,
            },
        ];
        let admitted = plan_fresh_room(&policy, &party, 1).expect("staged party");
        assert_eq!(admitted.cooldown_until_ms, 1 + LORE_KEEPER_COOLDOWN_MS);
        assert_eq!(admitted.deadline_ms, 1 + LORE_KEEPER_LIMIT_MS);
        assert_eq!(
            plan_timeout(&admitted.participants).kick_characters,
            vec![[1; 16], [2; 16]]
        );
        party[1].cooldown_until_ms = 2;
        assert_eq!(
            plan_fresh_room(&policy, &party, 1),
            Err(AdmissionRefusal::Cooldown)
        );
        policy.cooldown_ms -= 1;
        assert!(!matches_lore_keeper(&policy));
        policy.cooldown_ms += 1;
        policy.time_limit_ms += 1;
        assert!(!matches_lore_keeper(&policy));
        policy.time_limit_ms -= 1;
        assert!(policy.lever_positions.remove(&(32018, 32844, 14)));
        assert!(policy.lever_positions.insert((32018, 32843, 14)));
        assert!(!matches_lore_keeper(&policy));
    }

    #[test]
    fn m26_every_endgame_identity_retains_its_source_retry_policy() {
        for (boss, hours) in [
            ("the_primal_menace", 20),
            ("goshnar_s_spite", 20),
            ("goshnar_s_malice", 20),
            ("goshnar_s_greed", 20),
            ("goshnar_s_hatred", 20),
            ("goshnar_s_cruelty", 20),
            ("goshnar_s_megalomania_purple", 68),
            ("goshnar_s_megalomania_blue", 68),
            ("goshnar_s_megalomania_green", 68),
            ("ichgahal", 20),
            ("murcion", 20),
            ("vemiath", 20),
            ("chagorz", 20),
            ("bakragore", 68),
        ] {
            let key = format!("oteryn:creature.{boss}");
            let source_ms = hours * 3_600_000;
            assert!(matches(&key, source_ms), "{key}");
            for changed_ms in [0, source_ms - 1, source_ms + 1, u64::MAX] {
                assert!(!matches(&key, changed_ms), "{key}: {changed_ms}");
            }
        }
        assert!(!matches("oteryn:creature.bakragore", 20 * 3_600_000));
        assert!(matches("oteryn:creature.rat", 0));
    }
}
