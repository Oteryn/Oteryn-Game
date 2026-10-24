use super::*;
use crate::spell::Vocation;
use crate::spell::cast::{CharacterCastFacts, PlayerSpellState};
use oteryn_simulation_determinism::SemanticTimeMicros;

#[test]
fn first_vocation_vitals_preserve_pools_conditions_cooldowns_and_replay() {
    let formula = formula();
    let before = DurableBuildState::default();
    let mut state = PlayerSpellState::new(
        CharacterCastFacts {
            vocation: Vocation::None,
            level: 20,
            magic_level: 0,
            max_health: 245,
            max_mana: 150,
            max_soul: 100,
        },
        0,
        0,
    )
    .unwrap();
    assert_eq!(state.saved_vitals(), Some((245, 150)));
    state.load_owned_build(&before).unwrap();
    state.enable_owned_training(&before, &formula).unwrap();
    state.restore_saved_vitals(70, 30);
    state = state.after_food(20, 0).unwrap();
    state
        .cooldowns
        .spells
        .insert("spell:test".into(), SemanticTimeMicros::from_micros(50));
    state.regen_health_ms = 2000;
    state.regen_mana_ms = 1000;
    for (key, vocation, max_health, max_mana) in [
        ("druid", Vocation::Druid, 245, 450),
        ("sorcerer", Vocation::Sorcerer, 245, 450),
        ("knight", Vocation::Knight, 365, 150),
        ("paladin", Vocation::Paladin, 305, 270),
        ("monk", Vocation::Monk, 305, 210),
    ] {
        let after =
            crate::durability::character_build::convert_vocation(&formula, &before, key).unwrap();
        let facts = CharacterCastFacts {
            vocation,
            max_health,
            max_mana,
            ..state.facts
        };
        let next = state
            .after_first_vocation(
                &before,
                &after,
                facts,
                &formula,
                SemanticTimeMicros::from_micros(0),
            )
            .unwrap();
        assert_eq!((next.health, next.mana, next.soul), (70, 30, 100));
        assert_eq!(next.conditions, state.conditions);
        assert_eq!(next.cooldowns, state.cooldowns);
        assert_eq!((next.regen_health_ms, next.regen_mana_ms), (2000, 1000));
        assert_eq!(next.revision(), state.revision() + 1);
        assert_eq!(next.training.as_ref().unwrap().durable(), &after);
        assert_eq!(next.monk.is_some(), vocation == Vocation::Monk);
        assert_eq!(
            next.after_first_vocation(
                &before,
                &after,
                facts,
                &formula,
                SemanticTimeMicros::from_micros(1)
            ),
            Some(next.clone())
        );
        let mut dirty = state.clone();
        dirty
            .training
            .as_mut()
            .unwrap()
            .record_skill_tries(SkillFamily::Sword, 1, 1)
            .unwrap();
        let converted_dirty = dirty
            .after_first_vocation(
                &before,
                &after,
                facts,
                &formula,
                SemanticTimeMicros::from_micros(1),
            )
            .unwrap();
        assert_eq!(
            converted_dirty.training.as_ref().unwrap().pending_skills(),
            dirty.training.as_ref().unwrap().pending_skills()
        );
        let mut dead = state.clone();
        dead.health = 0;
        assert_eq!(
            dead.after_first_vocation(
                &before,
                &after,
                facts,
                &formula,
                SemanticTimeMicros::from_micros(1)
            )
            .unwrap()
            .health,
            0
        );
        let mut replay_with_training = next.clone();
        replay_with_training
            .training
            .as_mut()
            .unwrap()
            .record_skill_tries(SkillFamily::Sword, 1, 1)
            .unwrap();
        assert_eq!(
            replay_with_training.after_first_vocation(
                &before,
                &after,
                facts,
                &formula,
                SemanticTimeMicros::from_micros(2)
            ),
            Some(replay_with_training)
        );
    }
}
