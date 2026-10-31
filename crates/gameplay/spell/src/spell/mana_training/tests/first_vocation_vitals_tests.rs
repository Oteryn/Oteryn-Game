use super::*;
use crate::spell::Vocation;
use crate::spell::cast::{CharacterCastFacts, PlayerSpellState};
use oteryn_simulation_determinism::SemanticTimeMicros;

#[test]
fn first_vocation_vitals_refill_pools_preserve_conditions_cooldowns_and_replay() {
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
        assert_eq!(
            (next.health, next.mana, next.soul),
            (max_health, max_mana, 100)
        );
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
        assert!(
            dead.after_first_vocation(
                &before,
                &after,
                facts,
                &formula,
                SemanticTimeMicros::from_micros(1)
            )
            .is_none()
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

#[test]
fn first_vocation_vitals_refill_at_levels_8_9_20_50_for_two_choices() {
    let formula = formula();
    let before = DurableBuildState::default();
    for (level, old_health, old_mana, knight, druid) in [
        (8, 185, 90, (185, 90), (185, 90)),
        (9, 190, 95, (200, 95), (190, 120)),
        (20, 245, 150, (365, 150), (245, 450)),
        (50, 395, 300, (815, 300), (395, 1350)),
    ] {
        let mut state = PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::None,
                level,
                magic_level: 0,
                max_health: old_health,
                max_mana: old_mana,
                max_soul: 100,
            },
            0,
            0,
        )
        .unwrap();
        state.load_owned_build(&before).unwrap();
        state.restore_saved_vitals(1, 0);
        state.stance_key = Some("protector".into());
        for (key, vocation, (max_health, max_mana)) in [
            ("knight", Vocation::Knight, knight),
            ("druid", Vocation::Druid, druid),
        ] {
            let after =
                crate::durability::character_build::convert_vocation(&formula, &before, key)
                    .unwrap();
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
            assert_eq!(
                (next.vitals().health, next.vitals().mana),
                (max_health, max_mana)
            );
            assert_eq!(next.stance_key, state.stance_key);
            let mut spent = next.clone();
            spent.restore_saved_vitals(1, 0);
            assert_eq!(
                spent.after_first_vocation(
                    &before,
                    &after,
                    facts,
                    &formula,
                    SemanticTimeMicros::from_micros(1)
                ),
                Some(spent.clone())
            );
        }
    }
}

#[test]
fn s16c_vocation_trial_refills_only_above_level_8_and_refuses_level_20_or_same_vocation() {
    let formula = formula();
    let none = DurableBuildState::default();
    let knight =
        crate::durability::character_build::convert_vocation(&formula, &none, "knight").unwrap();
    let sorcerer =
        crate::durability::character_build::convert_vocation(&formula, &knight, "sorcerer")
            .unwrap();
    for (level, refill) in [(1, false), (8, false), (9, true), (19, true), (20, false)] {
        let mut state = PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::Knight,
                level,
                magic_level: u32::from(knight.magic().0),
                max_health: 300,
                max_mana: 100,
                max_soul: 100,
            },
            0,
            0,
        )
        .unwrap();
        state.load_owned_build(&knight).unwrap();
        state.enable_owned_training(&knight, &formula).unwrap();
        state.restore_saved_vitals(250, 30);
        let facts = CharacterCastFacts {
            vocation: Vocation::Sorcerer,
            magic_level: u32::from(sorcerer.magic().0),
            max_health: 200,
            max_mana: 400,
            ..state.facts
        };
        let now = SemanticTimeMicros::from_micros(0);
        assert_eq!(
            state.after_vocation_trial(&knight, &knight, state.facts, &formula, now),
            None,
            "same vocation at level {level}"
        );
        let next = state.after_vocation_trial(&knight, &sorcerer, facts, &formula, now);
        if level >= 20 {
            assert_eq!(next, None);
            continue;
        }
        let next = next.unwrap();
        assert_eq!(next.facts.vocation, Vocation::Sorcerer);
        assert_eq!(next.training.as_ref().unwrap().durable(), &sorcerer);
        assert_eq!(
            (next.health, next.mana),
            if refill { (200, 400) } else { (200, 30) },
            "level {level}"
        );
        // Stepping back on the knight tile converts again from the installed sorcerer.
        let back =
            crate::durability::character_build::convert_vocation(&formula, &sorcerer, "knight")
                .unwrap();
        let knight_facts = CharacterCastFacts {
            vocation: Vocation::Knight,
            magic_level: u32::from(back.magic().0),
            max_health: 300,
            max_mana: 100,
            ..next.facts
        };
        let back_state = next
            .after_vocation_trial(&sorcerer, &back, knight_facts, &formula, now)
            .unwrap();
        assert_eq!(back_state.facts.vocation, Vocation::Knight);
    }
}
