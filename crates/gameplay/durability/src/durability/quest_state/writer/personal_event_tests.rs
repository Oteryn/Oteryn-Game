use super::*;
use oteryn_gameplay_quests::quest::{admission_binding, authored_bindings, world_callbacks};

#[test]
fn unavailable_missions_admit_only_the_three_registered_native_surveys()
-> std::result::Result<(), String> {
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
            ));
        }
        assert!(unavailable_personal_event(
            transition,
            QuestCause::Step(command),
            false,
            Some(callback),
        ));
        assert!(unavailable_personal_event(
            transition,
            QuestCause::Step(command),
            true,
            None,
        ));

        let mut another = transition.clone();
        another.key.push_str("-alias");
        assert!(unavailable_personal_event(
            &another,
            QuestCause::Step(command),
            true,
            Some(callback),
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
        ));
        let mut changed = transition.clone();
        changed.completes = true;
        assert!(unavailable_personal_event(
            &changed,
            QuestCause::Step(command),
            true,
            Some(callback),
        ));
    }
    Ok(())
}
