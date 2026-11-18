//! Source cooldown admission for the existing, independently qualified activity owner.
use crate::durability::activity_handoff::boss_room::BossRoomDefinition;
use oteryn_gameplay_encounters::bosses::endgame_entry::{Encounter, source_entry_policy};

const LORE_KEEPER: &str = "oteryn:creature.the_last_lore_keeper";
const LORE_KEEPER_COOLDOWN_MS: u64 = 14 * 24 * 60 * 60 * 1000;
const LORE_KEEPER_LIMIT_MS: u64 = 30 * 60 * 1000;

pub(super) fn matches_source_cooldown(definition: &BossRoomDefinition) -> bool {
    if definition.boss_creature_key == LORE_KEEPER {
        return matches_lore_keeper(&definition.policy);
    }
    matches(&definition.boss_creature_key, definition.policy.cooldown_ms)
}

/// Crystal 201189f, actions_the_last_lore_keeper.lua:31-82.
/// The shared boss-room planner applies the selected policy to every staged member.
fn matches_lore_keeper(
    policy: &oteryn_gameplay_encounter_rules::boss_room::BossRoomPolicy,
) -> bool {
    policy.minimum_players == 1
        && policy.maximum_players == 15
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
        use oteryn_gameplay_encounter_rules::boss_room::BossRoomPolicy;
        let mut policy = BossRoomPolicy {
            minimum_players: 1,
            maximum_players: 15,
            minimum_level: 0,
            cooldown_ms: LORE_KEEPER_COOLDOWN_MS,
            time_limit_ms: LORE_KEEPER_LIMIT_MS,
            lever_positions: (32018..=32020)
                .flat_map(|x| (32844..=32848).map(move |y| (x, y, 14)))
                .collect(),
        };
        assert!(matches_lore_keeper(&policy));
        policy.cooldown_ms -= 1;
        assert!(!matches_lore_keeper(&policy));
        policy.cooldown_ms += 1;
        policy.time_limit_ms += 1;
        assert!(!matches_lore_keeper(&policy));
        policy.time_limit_ms -= 1;
        policy.lever_positions.swap(0, 1);
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
