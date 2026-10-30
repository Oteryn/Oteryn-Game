use super::*;

#[test]
fn quest_door_storage_uses_the_durable_track_and_absence_is_minus_one() {
    let mut copy = QuestStateCopy::default();
    assert!(!copy.meets_door_storage(4500));
    copy.tracks
        .insert("oteryn:quest-progress/storage/4500".into(), -1);
    assert!(!copy.meets_door_storage(4500));
    copy.tracks
        .insert("oteryn:quest-progress/storage/4500".into(), 0);
    assert!(copy.meets_door_storage(4500));
}

fn id(tag: u8) -> [u8; 16] {
    [tag, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
}

fn command(tag: u8, command_id: u64) -> CommandRef {
    CommandRef::new(
        GameSessionId::decode(&id(tag)).expect("session"),
        CommandId::new(command_id).expect("command"),
    )
}

fn request(cause: QuestCause, key: &str) -> QuestTransitionRequest {
    QuestTransitionRequest {
        transition_key: key.into(),
        cause,
    }
}

#[test]
fn the_quest_state_api_is_linked() {
    // Standalone durability suites path-load this module without quest cases.
    let _ = DurabilityRoot::commit_character_quest_transition;
    let _ = DurabilityRoot::reconcile_character_quest_transition;
    let _ = DurabilityRoot::read_character_quest_state;
    let _ = oteryn_gameplay_quests::quest::QuestStateCatalogue::track;
    let _ = admit_character_quest_state;
    let _ = request_pending_obligations;
    let _ = request_pending_quest_experience::<2>;
    let _ = QuestStateCopy::xp_obligations;
    let _ = |award: QuestTransitionAward| (award.transition, award.experience);
}

#[test]
fn the_binding_covers_the_request_only() {
    let character = CharacterId::from_bytes(id(1)).expect("character");
    let base = request(QuestCause::Command(command(2, 7)), "oteryn:t/1");
    let binding = request_binding(character, &base);
    assert_eq!(binding[0], 1);
    assert!(
        binding.len() <= oteryn_gameplay_quests::quest::QUESTSTATE0_RL_04,
        "RL-04"
    );
    assert_eq!(binding, request_binding(character, &base.clone()));
    let death = ExperienceRewardOccurrence::from_bytes(id(2)).expect("occurrence");
    for changed in [
        request(QuestCause::Use(command(2, 7)), "oteryn:t/1"),
        request(QuestCause::Step(command(2, 7)), "oteryn:t/1"),
        request(QuestCause::ClaimObligation(command(2, 7)), "oteryn:t/1"),
        request(QuestCause::CreatureDeath(death), "oteryn:t/1"),
        request(QuestCause::Command(command(2, 8)), "oteryn:t/1"),
        request(QuestCause::Command(command(3, 7)), "oteryn:t/1"),
        request(QuestCause::Command(command(2, 7)), "oteryn:t/2"),
    ] {
        assert_ne!(request_binding(character, &changed), binding, "{changed:?}");
    }
    let other = CharacterId::from_bytes(id(9)).expect("character");
    assert_ne!(request_binding(other, &base), binding);
}

#[test]
fn a_cause_round_trips_through_its_stored_shape() {
    let death = ExperienceRewardOccurrence::from_bytes(id(4)).expect("occurrence");
    for cause in [
        QuestCause::Command(command(2, 7)),
        QuestCause::Use(command(2, u64::MAX)),
        QuestCause::Step(command(2, u64::MAX)),
        QuestCause::ClaimObligation(command(3, 1)),
        QuestCause::CreatureDeath(death),
    ] {
        let (stored_id, ordinal) = cause.key();
        assert_eq!(
            QuestCause::stored(cause.kind(), stored_id, ordinal).expect("stored"),
            cause
        );
    }
    assert!(QuestCause::stored("command", id(2), 0).is_err());
    assert!(QuestCause::stored("creature_death", id(2), 1).is_err());
    assert!(QuestCause::stored("npc", id(2), 1).is_err());
}

#[test]
fn the_copy_applies_receipts_and_settles_obligations() {
    let claim = command(5, 1);
    let mut copy = QuestStateCopy {
        obligations: vec![QuestObligationRecord {
            claim,
            transition_key: "oteryn:t/1".into(),
            waiting_migration: false,
        }],
        ..QuestStateCopy::default()
    };
    assert_eq!(copy.pending_obligations().count(), 1);
    let committed = CommittedQuestTransition {
        character_id: CharacterId::from_bytes(id(1)).expect("character"),
        cause: QuestCause::ClaimObligation(claim),
        transition_key: "oteryn:t/1".into(),
        quest_key: "oteryn:quest/q".into(),
        original_character_revision: CharacterRevision::new(1).expect("revision"),
        committed_character_revision: CharacterRevision::new(2).expect("revision"),
        pinned_content_revision: "content-1".into(),
        definition_hash: [7; 32],
        completes: true,
        changes: vec![QuestTrackChange {
            track: "oteryn:quest-progress/q".into(),
            before: 0,
            after: 3,
        }],
        experience: None,
    };
    assert!(copy.apply(&committed), "within RL-08");
    assert_eq!(copy.tracks().get("oteryn:quest-progress/q"), Some(&3));
    assert!(copy.states()["oteryn:quest/q"].completed);
    assert!(copy.obligations().is_empty(), "the receipt consumed it");

    copy.obligations.push(QuestObligationRecord {
        claim,
        transition_key: "oteryn:t/1".into(),
        waiting_migration: false,
    });
    copy.settle_obligation(
        claim,
        &QuestTransitionOutcome::Refused(QuestRefusal::RevisionMismatch),
    );
    assert_eq!(copy.obligations().len(), 1);
    assert_eq!(copy.pending_obligations().count(), 0, "not retried");
    copy.settle_obligation(claim, &QuestTransitionOutcome::ObligationClosed);
    assert!(copy.obligations().is_empty());

    let mut full = QuestStateCopy::default();
    for index in 0..QUESTSTATE0_RL_08_TRACKS {
        full.tracks.insert(format!("oteryn:t/{index}"), 0);
    }
    assert!(!full.apply(&committed), "over RL-08");
}

#[test]
fn monk_copy_derives_count_once_and_reserves_projection_capacity_atomically() {
    use oteryn_gameplay_quests::quest::monk_shrines::{COUNT_TRACK, FLAG_PREFIX};
    let committed = CommittedQuestTransition {
        character_id: CharacterId::from_bytes(id(1)).expect("character"),
        cause: QuestCause::Use(command(5, 1)),
        transition_key: "oteryn:quest-callback/monk-guidance".into(),
        quest_key: "oteryn:quest.the_way_of_the_monk_quest".into(),
        original_character_revision: CharacterRevision::new(1).expect("revision"),
        committed_character_revision: CharacterRevision::new(2).expect("revision"),
        pinned_content_revision: "content-1".into(),
        definition_hash: [7; 32],
        completes: false,
        changes: vec![QuestTrackChange {
            track: format!("{FLAG_PREFIX}guidance"),
            before: 0,
            after: 1,
        }],
        experience: None,
    };
    let mut copy = QuestStateCopy::default();
    for index in 0..QUESTSTATE0_RL_08_TRACKS - 1 {
        copy.tracks.insert(format!("oteryn:t/{index}"), 0);
    }
    let before = copy.clone();
    assert!(!copy.apply(&committed));
    assert_eq!(
        copy, before,
        "a refused copy update cannot partially add a flag"
    );
    copy.tracks.remove("oteryn:t/0");
    assert!(copy.apply(&committed));
    assert_eq!(copy.tracks().len(), QUESTSTATE0_RL_08_TRACKS);
    assert_eq!(copy.tracks().get(COUNT_TRACK), Some(&1));
    assert!(copy.apply(&committed));
    assert_eq!(copy.tracks().get(COUNT_TRACK), Some(&1));
}
