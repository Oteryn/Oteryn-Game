//! Preserve already published PC3 hashes without colliding with main's new effects.
use super::{QuestEffect, QuestEffectKind};

pub(super) fn applies(key: &str, effects: &[QuestEffect]) -> bool {
    key.starts_with("oteryn:quest-reward/questpc3/")
        || key.starts_with("oteryn:quest-transition/questpc3/")
        || key.starts_with("oteryn:quest-trade/questpc3/")
        || key == "oteryn:quest-transition/native-kills/beginning"
        || effects
            .iter()
            .any(|effect| matches!(effect.effect, QuestEffectKind::SetWhenAtLeast { .. }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pc3_trade_namespace_keeps_published_effect_tags() {
        assert!(applies(
            "oteryn:quest-trade/questpc3/beginning/carlos-food-sale",
            &[]
        ));
        assert!(!applies("oteryn:quest-trade/other/food-sale", &[]));
    }

    #[test]
    fn published_tags_survive_and_each_dialect_remains_injective() {
        let set = QuestEffectKind::SetWithBinding {
            value: 1,
            binding: [0; 32],
        };
        let add = QuestEffectKind::AddWithBinding {
            delta: 1,
            binding: [0; 32],
        };
        let conditional = QuestEffectKind::SetWhenAtLeast {
            other_effect: 1,
            minimum: 1,
            value: 1,
        };
        for (legacy, expected_set, expected_add, expected_conditional) in
            [(false, 7, 8, 11), (true, 8, 11, 7)]
        {
            let mut encoded = Vec::new();
            for (effect, tag) in [
                (set, expected_set),
                (add, expected_add),
                (conditional, expected_conditional),
            ] {
                let mut bytes = Vec::new();
                effect.encode(&mut bytes, legacy);
                assert_eq!(bytes[0], tag);
                assert!(
                    !encoded.contains(&bytes),
                    "different operations must not share a definition encoding"
                );
                encoded.push(bytes);
            }
        }
    }
}
