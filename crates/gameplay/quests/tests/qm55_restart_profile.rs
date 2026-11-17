use oteryn_gameplay_quests::quest::{
    QuestComparison as C, QuestEffect, QuestEffectKind as E, QuestStateCatalogue, QuestTrack,
    QuestTransition, shared_world_progress,
};
const QUEST: &str = "oteryn:quest.qm55_profile";
const TRACK: &str = "oteryn:quest-progress/qm55/profile";
fn fixture() -> (Vec<QuestTrack>, Vec<QuestTransition>) {
    let effect = |from, after| QuestEffect {
        track: TRACK.into(),
        from,
        effect: E::Set(after),
    };
    let transition = |key: String, effects, completes| QuestTransition {
        key,
        quest: QUEST.into(),
        effects,
        completes,
        experience: None,
        world_change: Some("grimvale".into()),
    };
    (
        vec![QuestTrack {
            key: TRACK.into(),
            quest: QUEST.into(),
            initial: 0,
            min: 0,
            max: 2,
        }],
        vec![
            transition(
                shared_world_progress::save_event_restart_key(QUEST),
                vec![effect(C::Any, 0)],
                false,
            ),
            transition(
                "oteryn:quest-transition/qm55/profile-finish".into(),
                vec![effect(C::Between(0, 0), 1)],
                true,
            ),
        ],
    )
}
#[test]
fn qm55_restart_profile_accepts_explicit_pure_reset() -> Result<(), String> {
    let (tracks, transitions) = fixture();
    let catalogue = QuestStateCatalogue::new("content-1", tracks, transitions)
        .map_err(|error| format!("{error:?}"))?;
    assert!(catalogue.save_event_restart(QUEST).is_some());
    assert!(catalogue.save_event_restart("oteryn:quest.other").is_none());
    Ok(())
}
#[test]
fn qm55_restart_profile_rejects_rewards_and_inconsistent_scope() {
    for mutation in 0..6 {
        let (tracks, mut transitions) = fixture();
        match mutation {
            0 => transitions[0].experience = Some(1),
            1 => transitions[0].effects[0].effect = E::Set(1),
            2 => transitions[0].effects[0].from = C::Between(0, 0),
            3 => transitions[1].world_change = None,
            4 => {
                transitions[0].key =
                    format!("{}other", shared_world_progress::SAVE_EVENT_RESTART_PREFIX)
            }
            _ => transitions[0].completes = true,
        }
        assert!(
            QuestStateCatalogue::new("content-1", tracks, transitions).is_err(),
            "unsafe reset profile mutation {mutation}"
        );
    }
}
