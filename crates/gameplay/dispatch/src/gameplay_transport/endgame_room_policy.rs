//! Source cooldown admission for the existing, independently qualified activity owner.
use crate::durability::activity_handoff::boss_room::BossRoomDefinition;
use oteryn_gameplay_encounters::bosses::endgame_entry::{Encounter, source_entry_policy};

pub(super) fn matches_source_cooldown(definition: &BossRoomDefinition) -> bool {
    matches(&definition.boss_creature_key, definition.policy.cooldown_ms)
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
    use super::matches;

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
