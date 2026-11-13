use super::*;
use oteryn_gameplay_quests::quest::{admission_binding, authored_bindings, world_callbacks};

#[test]
fn registered_native_surveys_keep_all_admission_guards() -> std::result::Result<(), String> {
    let state = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = state.catalogue();
    let callbacks = world_callbacks::load()?;
    let session = GameSessionId::decode(&[7, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 7])
        .map_err(|error| format!("{error:?}"))?;
    let command = CommandRef::new(
        session,
        CommandId::new(1).map_err(|error| format!("{error:?}"))?,
    );
    for key in [
        "oteryn:quest-callback/m01b/chakoya-iceberg-mini-world-change/s1/entry",
        "oteryn:quest-callback/m01b/chakoya-iceberg-mini-world-change/s3/entry",
        "oteryn:quest-callback/m01b/fire-from-the-earth-mini-world-change/s1/entry",
    ] {
        let callback = callbacks.iter().find(|c| c.key == key).ok_or(key)?;
        let transition = catalogue.transition(key).ok_or(key)?;
        assert!(matches!(
            authored_bindings::personal_event_profile(&transition.quest),
            Some(authored_bindings::PersonalEventProfile::Unavailable { .. })
        ));
        admission_binding::validate_resolved_world(catalogue, transition, true, &callbacks)?;
        assert!(
            admission_binding::validate_resolved_world(catalogue, transition, false, &callbacks)
                .is_err()
        );
        assert!(!unavailable_personal_event(
            transition,
            QuestCause::Step(command),
            true,
            Some(callback),
            false,
        ));
        for cause in [
            QuestCause::Command(command),
            QuestCause::Use(command),
            QuestCause::ClaimObligation(command),
        ] {
            assert!(unavailable_personal_event(
                transition,
                cause,
                true,
                Some(callback),
                false,
            ));
        }
        assert!(unavailable_personal_event(
            transition,
            QuestCause::Step(command),
            false,
            Some(callback),
            false,
        ));
        assert!(unavailable_personal_event(
            transition,
            QuestCause::Step(command),
            true,
            None,
            false,
        ));

        let mut another = transition.clone();
        another.key.push_str("-alias");
        assert!(unavailable_personal_event(
            &another,
            QuestCause::Step(command),
            true,
            Some(callback),
            false,
        ));
        let mut another = transition.clone();
        another.quest = "oteryn:quest.authored.bank_robbery_mini_world_change".into();
        assert!(matches!(
            authored_bindings::personal_event_profile(&another.quest),
            Some(authored_bindings::PersonalEventProfile::Unavailable { .. })
        ));
        assert!(unavailable_personal_event(
            &another,
            QuestCause::Step(command),
            true,
            Some(callback),
            false,
        ));

        let mut changed = callback.clone();
        changed
            .admission
            .as_mut()
            .ok_or("Premium admission")?
            .premium = false;
        assert!(unavailable_personal_event(
            transition,
            QuestCause::Step(command),
            true,
            Some(&changed),
            false,
        ));
        let mut changed = callback.clone();
        changed.world_change = if callback.world_change.is_some() {
            None
        } else {
            Some("fire_from_the_earth".into())
        };
        assert!(unavailable_personal_event(
            transition,
            QuestCause::Step(command),
            true,
            Some(&changed),
            false,
        ));
        let mut changed = callback.clone();
        changed
            .selector
            .native
            .as_mut()
            .ok_or("native anchor")?
            .position
            .x += 1;
        assert!(unavailable_personal_event(
            transition,
            QuestCause::Step(command),
            true,
            Some(&changed),
            false,
        ));
        let mut changed = transition.clone();
        changed.completes = true;
        assert!(unavailable_personal_event(
            &changed,
            QuestCause::Step(command),
            true,
            Some(callback),
            false,
        ));
    }
    Ok(())
}

#[test]
fn selected_personal_journals_require_the_exact_world_callback_and_cause()
-> std::result::Result<(), String> {
    let state = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|error| format!("{error:?}"))?;
    let callbacks = world_callbacks::load()?;
    let session = GameSessionId::decode(&[7, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 7])
        .map_err(|error| format!("{error:?}"))?;
    let command = CommandRef::new(
        session,
        CommandId::new(1).map_err(|error| format!("{error:?}"))?,
    );
    for key in [
        "oteryn:quest-callback/worldchg-m31/fire_from_the_earth/volcano",
        "oteryn:quest-callback/worldchg-m31/hive_outpost/outpost",
        "oteryn:quest-callback/worldchg-m31/hive_outpost/survey",
    ] {
        let callback = callbacks
            .iter()
            .find(|callback| callback.key == key)
            .ok_or(key)?;
        let transition = state.catalogue().transition(key).ok_or(key)?;
        let (cause, wrong_cause) = match callback.trigger {
            world_callbacks::Trigger::Use => (QuestCause::Use(command), QuestCause::Step(command)),
            world_callbacks::Trigger::StepIn => {
                (QuestCause::Step(command), QuestCause::Use(command))
            }
            _ => return Err("personal event World trigger".into()),
        };
        assert!(!unavailable_personal_event(
            transition,
            cause,
            true,
            Some(callback),
            false,
        ));
        for invalid in [
            wrong_cause,
            QuestCause::Command(command),
            QuestCause::ClaimObligation(command),
        ] {
            assert!(unavailable_personal_event(
                transition,
                invalid,
                true,
                Some(callback),
                false,
            ));
        }
        assert!(unavailable_personal_event(
            transition,
            cause,
            false,
            Some(callback),
            false
        ));
        assert!(unavailable_personal_event(
            transition, cause, true, None, false
        ));
        for mutation in 0..4 {
            let mut altered = callback.clone();
            match mutation {
                0 => {
                    altered
                        .selector
                        .position
                        .as_mut()
                        .ok_or("source position")?
                        .x += 1
                }
                1 => altered.storage.first_mut().ok_or("source state")?.set += 1,
                2 => altered.requires_world_admission = false,
                _ => altered.sources.clear(),
            }
            assert!(unavailable_personal_event(
                transition,
                cause,
                true,
                Some(&altered),
                false,
            ));
        }
        let mut altered = transition.clone();
        altered.completes = !altered.completes;
        assert!(unavailable_personal_event(
            &altered,
            cause,
            true,
            Some(callback),
            false
        ));
        altered = transition.clone();
        altered.key.push_str("-alias");
        assert!(unavailable_personal_event(
            &altered,
            cause,
            true,
            Some(callback),
            false
        ));
        for quest in [
            "oteryn:quest.authored.bank_robbery_mini_world_change",
            "oteryn:quest.authored.lumberjack_mini_world_change",
        ] {
            let mut unrelated = transition.clone();
            unrelated.quest = quest.into();
            let mut unrelated_callback = callback.clone();
            unrelated_callback.quest = quest.into();
            assert!(unavailable_personal_event(
                &unrelated,
                cause,
                true,
                Some(&unrelated_callback),
                false,
            ));
        }
    }
    Ok(())
}

#[test]
fn selected_personal_counters_still_require_validated_native_death()
-> std::result::Result<(), String> {
    let state = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|error| format!("{error:?}"))?;
    let bytes = [7, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 7];
    let command = CommandRef::new(
        GameSessionId::decode(&bytes).map_err(|error| format!("{error:?}"))?,
        CommandId::new(1).map_err(|error| format!("{error:?}"))?,
    );
    let cause = QuestCause::CreatureDeath(
        ExperienceRewardOccurrence::from_bytes(bytes).map_err(|error| format!("{error:?}"))?,
    );
    let enabled = ["fire_from_the_earth", "hive_outpost"]
        .into_iter()
        .map(|program| {
            (
                format!("oteryn:quest-progress/world-change-journal/{program}/enabled"),
                1,
            )
        })
        .collect();
    let mut active =
        oteryn_gameplay_quests::quest::native_kills::active_for(state.catalogue(), &enabled);
    // The catalogue also exposes authored routes eligible at their initial gate.
    // This test qualifies only the two selected personal journal owners.
    active.retain(|key| {
        oteryn_gameplay_quests::quest::native_kills::counter(key).is_some_and(|counter| {
            matches!(
                counter.quest,
                "oteryn:quest.authored.fire_from_the_earth_mini_world_change"
                    | "oteryn:quest.authored.hive_outpost_mini_world_change"
            )
        })
    });
    assert_eq!(
        active.len(),
        16,
        "all seven fiery and nine insect source counters"
    );
    assert_eq!(
        active
            .iter()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        [
            "oteryn:quest-transition/native-kills/m31/fire_from_the_earth/diabolic_imp",
            "oteryn:quest-transition/native-kills/m31/fire_from_the_earth/dragon",
            "oteryn:quest-transition/native-kills/m31/fire_from_the_earth/dragon_hatchling",
            "oteryn:quest-transition/native-kills/m31/fire_from_the_earth/dragon_lord",
            "oteryn:quest-transition/native-kills/m31/fire_from_the_earth/dragon_lord_hatchling",
            "oteryn:quest-transition/native-kills/m31/fire_from_the_earth/dragonling",
            "oteryn:quest-transition/native-kills/m31/fire_from_the_earth/hellfire_fighter",
            "oteryn:quest-transition/native-kills/m31/hive_outpost/crawler",
            "oteryn:quest-transition/native-kills/m31/hive_outpost/hive_overseer",
            "oteryn:quest-transition/native-kills/m31/hive_outpost/insectoid_worker",
            "oteryn:quest-transition/native-kills/m31/hive_outpost/kollos",
            "oteryn:quest-transition/native-kills/m31/hive_outpost/spidris",
            "oteryn:quest-transition/native-kills/m31/hive_outpost/spidris_elite",
            "oteryn:quest-transition/native-kills/m31/hive_outpost/spitter",
            "oteryn:quest-transition/native-kills/m31/hive_outpost/swarmer",
            "oteryn:quest-transition/native-kills/m31/hive_outpost/waspoid",
        ]
        .into_iter()
        .collect(),
        "exact selected personal source counter identities"
    );
    let mut seen = std::collections::BTreeSet::new();
    for key in active {
        let counter = oteryn_gameplay_quests::quest::native_kills::counter(&key).ok_or(&key)?;
        let transition = state.catalogue().transition(&key).ok_or(&key)?;
        seen.insert(counter.quest);
        assert!(!unavailable_personal_event(
            transition, cause, false, None, true
        ));
        assert!(unavailable_personal_event(
            transition, cause, false, None, false
        ));
        for invalid in [
            QuestCause::Use(command),
            QuestCause::Step(command),
            QuestCause::Command(command),
            QuestCause::ClaimObligation(command),
        ] {
            assert!(unavailable_personal_event(
                transition, invalid, false, None, true
            ));
        }
        let mut altered = transition.clone();
        altered.effects.first_mut().ok_or("counter gate")?.effect =
            oteryn_gameplay_quests::quest::QuestEffectKind::Add(1);
        assert!(unavailable_personal_event(
            &altered, cause, false, None, true
        ));
        altered = transition.clone();
        altered.key.push_str("-alias");
        assert!(unavailable_personal_event(
            &altered, cause, false, None, true
        ));
        altered = transition.clone();
        altered.quest = "oteryn:quest.authored.lumberjack_mini_world_change".into();
        assert!(unavailable_personal_event(
            &altered, cause, false, None, true
        ));
    }
    assert_eq!(
        seen.len(),
        2,
        "both canonical personal counter programs are exercised"
    );
    Ok(())
}
