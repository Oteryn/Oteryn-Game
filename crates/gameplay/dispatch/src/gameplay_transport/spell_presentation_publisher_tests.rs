#![allow(clippy::unwrap_used)]
use super::super::spell_presentations::tests::{commit_fixture, runtime};
use super::*;
use crate::movement::interest::InterestEntity;
fn index(runtime: &ChannelRuntimeV1) -> InterestIndex {
    let mut index = InterestIndex::new();
    for (actor, position, _) in runtime.positioned_actor_census().unwrap() {
        index.upsert(InterestEntity {
            identity: actor.placement_identity(),
            position: view(position.position()).unwrap(),
            revision: actor.actor_local_generation(),
        });
    }
    index
}
fn commit(
    owner: &mut SpellPresentationOwner,
    runtime: &mut ChannelRuntimeV1,
    caster: ExactActorRef,
    session: GameSessionId,
    command: u64,
) {
    commit_fixture(owner, runtime, caster, session, command);
}
#[test]
fn actual_owner_cue_has_source_class_and_no_source_keys_in_encoded_bytes() {
    let (mut runtime, caster, other, session) = runtime();
    let mut owner = SpellPresentationOwner::new(&runtime);
    commit(&mut owner, &mut runtime, caster, session, 1);
    let turn = CandidatePresentationTurn::take(&mut owner, &runtime, 1).unwrap();
    let index = index(&runtime);
    let observer = owned_observer(&runtime, &index, caster, session, 1).unwrap();
    let mut publisher = CandidateSessionPublisher::default();
    assert!(
        publisher
            .publish_qualified(&observer, VisibilitySettings::REFERENCE, &turn)
            .unwrap()
    );
    assert!(
        !publisher
            .publish_qualified(&observer, VisibilitySettings::REFERENCE, &turn)
            .unwrap()
    );
    let delivered = publisher.pop_qualified(&observer).unwrap();
    assert_eq!((delivered.base_revision, delivered.revision), (0, 1));
    let wire = wire::decode_batch(&delivered.bytes).unwrap();
    assert_eq!(wire.events.len(), 1);
    assert!(matches!(
        wire.events[0].body,
        wire::Body::Effect {
            source: wire::Source::Own,
            ..
        }
    ));
    assert!(
        !delivered
            .bytes
            .windows(b"appearance:".len())
            .any(|bytes| bytes == b"appearance:")
    );
    let other_session = runtime
        .positioned_actor_census()
        .unwrap()
        .into_iter()
        .find(|(a, _, _)| *a == other)
        .unwrap()
        .2
        .unwrap();
    let other = owned_observer(&runtime, &index, other, other_session, 1).unwrap();
    let events = selected_events(&turn, &other, VisibilitySettings::REFERENCE).unwrap();
    assert!(matches!(
        events[0].body,
        wire::Body::Effect {
            source: wire::Source::Others,
            ..
        }
    ));
}
#[test]
fn current_index_positions_and_actor_generations_are_independently_checked() {
    let (mut runtime, caster, _, session) = runtime();
    let mut index = index(&runtime);
    let mut entry = *index.get(&caster.placement_identity()).unwrap();
    // VIS-2 revision is an opaque change stamp, never a substitute for slot generation.
    entry.revision = 987;
    index.upsert(entry);
    assert!(owned_observer(&runtime, &index, caster, session, 1).is_ok());
    entry.position = VisibilityPosition::new(99, 99, 0).unwrap();
    index.upsert(entry);
    assert!(matches!(
        owned_observer(&runtime, &index, caster, session, 1),
        Err(Error::InvalidIndex)
    ));
    entry.position = view(runtime.read_actor_position(caster).unwrap().position()).unwrap();
    index.upsert(entry);
    runtime.remove_test_actor(caster).unwrap();
    let reservation = runtime.reserve_fresh_session(session).unwrap();
    let replacement = runtime.commit_fresh_session(reservation).unwrap();
    runtime
        .initialize_first_entry_position(replacement)
        .unwrap();
    assert_eq!(replacement.actor_local_id(), caster.actor_local_id());
    assert_ne!(
        replacement.actor_local_generation(),
        caster.actor_local_generation()
    );
    assert!(matches!(
        owned_observer(&runtime, &index, caster, session, 1),
        Err(Error::Stale)
    ));
    assert!(matches!(
        owned_observer(&runtime, &index, replacement, session, 1),
        Err(Error::InvalidIndex)
    ));
    // VIS-1: a source world's native floor -5 is Tibia floor 5; -16 is outside every frame.
    assert!(
        view(MovementLocalPosition {
            x: 0,
            y: 0,
            floor: -5
        })
        .is_ok()
    );
    assert!(matches!(
        view(MovementLocalPosition {
            x: 0,
            y: 0,
            floor: -16
        }),
        Err(Error::UnknownFrame)
    ));
}

#[test]
fn native_floors_rank_like_their_tibia_floors() {
    let at = |x, y, floor| MovementLocalPosition { x, y, floor };
    // Native -7 sees native -6 one floor up: Tibia 7 sees 6, projected one tile south-east.
    assert_eq!(
        plane_distance(at(100, 100, -7), at(101, 101, -6)).unwrap(),
        plane_distance(at(100, 100, 7), at(101, 101, 6)).unwrap()
    );
    assert_eq!(
        plane_distance(at(100, 100, -7), at(101, 101, -6)).unwrap(),
        (1, 0)
    );
    assert!(matches!(
        plane_distance(at(0, 0, -7), at(0, 0, -16)),
        Err(Error::UnknownFrame)
    ));
}

#[test]
fn target_outside_view_and_stale_context_emit_nothing() {
    let (mut runtime, caster, _, session) = runtime();
    let mut owner = SpellPresentationOwner::new(&runtime);
    commit(&mut owner, &mut runtime, caster, session, 1);
    let mut turn = CandidatePresentationTurn::take(&mut owner, &runtime, 1).unwrap();
    let index = index(&runtime);
    let observer = owned_observer(&runtime, &index, caster, session, 1).unwrap();
    turn.events[0].event.position.x = 1000;
    assert!(
        selected_events(&turn, &observer, VisibilitySettings::REFERENCE)
            .unwrap()
            .is_empty()
    );
    turn.context.content = [9; 32];
    assert_eq!(
        selected_events(&turn, &observer, VisibilitySettings::REFERENCE),
        Err(Error::Stale)
    );
}
#[test]
fn slow_egress_drops_whole_batches_without_revision_and_old_connection_cannot_take() {
    let (mut runtime, caster, _, session) = runtime();
    let mut owner = SpellPresentationOwner::new(&runtime);
    let index = index(&runtime);
    let observer = owned_observer(&runtime, &index, caster, session, 1).unwrap();
    let mut publisher = CandidateSessionPublisher::from_retained_revision(41);
    for sync in 1..=3 {
        commit(&mut owner, &mut runtime, caster, session, sync);
        let turn = CandidatePresentationTurn::take(&mut owner, &runtime, sync).unwrap();
        assert_eq!(
            publisher
                .publish_qualified(&observer, VisibilitySettings::REFERENCE, &turn)
                .unwrap(),
            sync < 3
        );
    }
    assert_eq!(publisher.revision, 43);
    assert_eq!(publisher.pending.len(), 2);
    let newer = owned_observer(&runtime, &index, caster, session, 2).unwrap();
    assert!(publisher.pop_qualified(&newer).is_none());
    assert!(publisher.pending.is_empty());
}
#[test]
fn real_outbox_replay_emits_no_new_decision_or_sequence() {
    let (mut runtime, caster, _, session) = runtime();
    let mut owner = SpellPresentationOwner::new(&runtime);
    commit(&mut owner, &mut runtime, caster, session, 1);
    let first = CandidatePresentationTurn::take(&mut owner, &runtime, 1).unwrap();
    commit(&mut owner, &mut runtime, caster, session, 1);
    let second = CandidatePresentationTurn::take(&mut owner, &runtime, 2).unwrap();
    assert_eq!(first.events[0].decision_ordinal, 1);
    assert_eq!(first.events[0].emission_sequence, 1);
    assert!(second.events.is_empty());
}

#[test]
fn reaudit_ambient_sounds() {
    use oteryn_protocol_oteryn::presentation::PresentationEventKind;
    let (runtime, caster, other, session) = runtime();
    let mut publisher = SoundPublisher::default();
    let city = ambient_sounds::zone_key("oteryn:content.area.subregion.thais_city");
    assert!(ambient_sounds::zone_key("oteryn:content.area.region.thais").is_none());
    publisher.ambient(&runtime, caster, session, city, 600);
    let (revision, events) = publisher
        .observe(&runtime, caster, session, 0, true)
        .unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0].kind,
        PresentationEventKind::AmbientSound { cue_id: 25 }
    ));
    publisher.ambient(&runtime, caster, session, Some("thais-city"), 600);
    assert!(
        publisher
            .observe(&runtime, caster, session, revision, true)
            .unwrap()
            .1
            .is_empty()
    );
    publisher.ambient(&runtime, caster, session, Some("thais-city"), 1200);
    let (revision, events) = publisher
        .observe(&runtime, caster, session, revision, true)
        .unwrap();
    assert!(matches!(
        events[0].kind,
        PresentationEventKind::AmbientSound { cue_id: 28 }
    ));
    publisher.ambient(&runtime, caster, session, None, 1200);
    let (_, events) = publisher
        .observe(&runtime, caster, session, revision, true)
        .unwrap();
    assert!(matches!(
        events[0].kind,
        PresentationEventKind::AmbientSound { cue_id: 0 }
    ));
    let bytes = oteryn_protocol_oteryn::presentation::encode_presentation_batch(&events).unwrap();
    assert_eq!(
        oteryn_protocol_oteryn::presentation::decode_presentation_batch(&bytes).unwrap(),
        events
    );
    let other_session = runtime
        .positioned_actor_census()
        .unwrap()
        .into_iter()
        .find(|(a, _, _)| *a == other)
        .unwrap()
        .2
        .unwrap();
    assert!(
        publisher
            .observe(&runtime, other, other_session, 0, true)
            .unwrap()
            .1
            .is_empty()
    );
    assert!(
        publisher
            .observe(&runtime, caster, session, 0, false)
            .unwrap()
            .1
            .is_empty()
    );
}

#[test]
fn ambient_current_state_survives_effect_overflow_and_reconnect() {
    use oteryn_protocol_oteryn::presentation::{MAX_PRESENTATION_EVENTS, PresentationEventKind};
    let (runtime, actor, other, session) = runtime();
    let at = runtime.read_actor_position(actor).unwrap().position();
    let mut publisher = SoundPublisher::default();
    publisher.ambient(&runtime, actor, session, Some("thais-city"), 1049);
    for _ in 0..=MAX_PRESENTATION_EVENTS {
        publisher.record(&runtime, actor, at, 3, 0);
    }
    let (cursor, events) = publisher
        .observe(&runtime, actor, session, 0, true)
        .unwrap();
    assert_eq!(events.len(), MAX_PRESENTATION_EVENTS + 1);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event.kind, PresentationEventKind::Sound { .. }))
            .count(),
        MAX_PRESENTATION_EVENTS
    );
    assert!(matches!(
        events[0].kind,
        PresentationEventKind::AmbientSound { cue_id: 25 }
    ));
    publisher.ambient(&runtime, actor, session, Some("thais-city"), 1050);
    let (cursor, events) = publisher
        .observe(&runtime, actor, session, cursor, true)
        .unwrap();
    assert!(matches!(
        events[0].kind,
        PresentationEventKind::AmbientSound { cue_id: 28 }
    ));
    publisher.ambient(&runtime, actor, session, Some("carlin-city"), 1050);
    let (cursor, events) = publisher
        .observe(&runtime, actor, session, cursor, true)
        .unwrap();
    assert_eq!(
        events.len(),
        1,
        "a different zone emits even when it shares the cue"
    );
    publisher.ambient(&runtime, other, session, Some("thais-city"), 360);
    assert!(
        publisher
            .observe(&runtime, actor, session, cursor, true)
            .unwrap()
            .1
            .is_empty()
    );
    publisher.forget_ambient(session);
    publisher.ambient(&runtime, actor, session, Some("thais-city"), 360);
    let (_, events) = publisher
        .observe(&runtime, actor, session, cursor, true)
        .unwrap();
    assert!(matches!(
        events[0].kind,
        PresentationEventKind::AmbientSound { cue_id: 25 }
    ));
}

#[test]
fn production_sound_fans_out_hits_and_capability_gates() {
    use oteryn_protocol_oteryn::presentation::{PresentationEventKind, PresentationSource};
    let (runtime, caster, other, session) = runtime();
    let position = runtime.read_actor_position(caster).unwrap().position();
    let other_session = runtime
        .positioned_actor_census()
        .unwrap()
        .into_iter()
        .find(|(a, _, _)| *a == other)
        .unwrap()
        .2
        .unwrap();
    let mut publisher = SoundPublisher::default();
    publisher.record(&runtime, caster, position, 3, 0);
    let (revision, own) = publisher
        .observe(&runtime, caster, session, 0, true)
        .unwrap();
    assert_eq!(own.len(), 1);
    assert!(matches!(
        own[0].kind,
        PresentationEventKind::Sound {
            cue_id: 3,
            source: PresentationSource::Own,
            ..
        }
    ));
    let (_, others) = publisher
        .observe(&runtime, other, other_session, 0, true)
        .unwrap();
    assert!(matches!(
        others[0].kind,
        PresentationEventKind::Sound {
            source: PresentationSource::Others,
            ..
        }
    ));
    assert!(
        publisher
            .observe(&runtime, caster, session, 0, false)
            .unwrap()
            .1
            .is_empty()
    );
    assert!(
        publisher
            .observe(&runtime, caster, session, revision, true)
            .unwrap()
            .1
            .is_empty()
    );
}

#[test]
fn production_sound_emits_committed_spell_cast() {
    let (mut runtime, caster, _, session) = runtime();
    let mut owner = SpellPresentationOwner::new(&runtime);
    super::super::spell_presentations::tests::commit_sound_fixture(
        &mut owner,
        &mut runtime,
        caster,
        session,
    );
    let mut publisher = SoundPublisher::default();
    publisher.capture(&mut owner, &runtime).unwrap();
    let (_, events) = publisher
        .observe(&runtime, caster, session, 0, true)
        .unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0].kind,
        oteryn_protocol_oteryn::presentation::PresentationEventKind::Sound { cue_id: 1080, .. }
    ));
    assert!(
        publisher
            .observe(&runtime, caster, session, 0, false)
            .unwrap()
            .1
            .is_empty()
    );
}

#[test]
fn production_sound_normalizes_native_floors_for_the_allocated_wire() {
    let (runtime, caster, _, session) = runtime();
    let mut publisher = SoundPublisher::default();
    publisher.record(
        &runtime,
        caster,
        MovementLocalPosition {
            x: -7,
            y: -7,
            floor: -7,
        },
        1,
        0,
    );
    let (_, events) = publisher
        .observe(&runtime, caster, session, 0, true)
        .unwrap();
    assert_eq!(events.len(), 1);
    let bytes = oteryn_protocol_oteryn::presentation::encode_presentation_batch(&events).unwrap();
    assert_eq!(
        oteryn_protocol_oteryn::presentation::decode_presentation_batch(&bytes).unwrap(),
        events
    );
    assert!(matches!(
        events[0].kind,
        oteryn_protocol_oteryn::presentation::PresentationEventKind::Sound {
            position: ActorPosition { floor: 7, .. },
            ..
        }
    ));
}

#[test]
fn nx5_private_toy_effect_is_fenced_and_never_visible_to_other_observers() {
    let (runtime, actor, other, session) = runtime();
    let other_session = runtime
        .positioned_actor_census()
        .unwrap()
        .into_iter()
        .find(|(a, _, _)| *a == other)
        .unwrap()
        .2
        .unwrap();
    let position = runtime.read_actor_position(actor).unwrap().position();
    let mut publisher = SoundPublisher::default();
    publisher.record_item_effect(&runtime, actor, position, 1, true);
    assert!(
        publisher
            .observe(&runtime, other, other_session, 0, true)
            .unwrap()
            .1
            .is_empty()
    );
    assert!(
        publisher
            .observe(&runtime, actor, other_session, 0, true)
            .is_none()
    );
    let (revision, cues) = publisher
        .observe(&runtime, actor, session, 0, true)
        .unwrap();
    assert_eq!(cues.len(), 1);
    assert!(matches!(
        cues[0].kind,
        oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect {
            effect_id: 1,
            ..
        }
    ));
    assert!(
        publisher
            .observe(&runtime, actor, session, revision, true)
            .unwrap()
            .1
            .is_empty()
    );
    assert!(
        publisher
            .observe(&runtime, actor, session, 0, false)
            .unwrap()
            .1
            .is_empty()
    );
}

#[test]
fn w18_house_decoration_effects_follow_current_house_custody_and_hide_channel_cues() {
    use crate::foundation::{
        CharacterId, CharacterLease, LocalPosition, RuntimeScopeRefV1, ScopeOwnershipGeneration,
    };
    use oteryn_gameplay_entities::carrier::house_custody::HouseActorState;
    let (mut runtime, actor, other, source) = runtime();
    let position = runtime.read_actor_position(actor).unwrap().position();
    let other_session = runtime
        .positioned_actor_census()
        .unwrap()
        .into_iter()
        .find(|(present, _, _)| *present == other)
        .unwrap()
        .2
        .unwrap();
    let mut character_id = *source.as_bytes();
    character_id[5] = 6;
    character_id[15] = 6;
    let character = CharacterId::decode(&character_id).unwrap();
    runtime
        .bind_attacker_lease(actor, source, CharacterLease::new(character, 1).unwrap())
        .unwrap();
    let mut outbox = SpellPresentationOwner::new(&runtime);
    crate::gameplay_transport::spell_presentations::tests::commit_sound_fixture(
        &mut outbox,
        &mut runtime,
        actor,
        source,
    );
    let mut publisher = SoundPublisher::default();
    publisher.record_item_effect(
        &runtime,
        actor,
        runtime.read_actor_position(actor).unwrap().position(),
        1,
        false,
    );
    let mut id = *source.as_bytes();
    id[15] = 90;
    let session = GameSessionId::decode(&id).unwrap();
    let lease = CharacterLease::new(character, 2).unwrap();
    id[15] = 92;
    let scope = RuntimeScopeRefV1::instance(runtime.binding().world_id(), id).unwrap();
    let at = LocalPosition {
        x: position.x,
        y: position.y,
        floor: position.floor,
    };
    let actor = runtime
        .restore_transfer_actor(
            source,
            session,
            lease,
            at,
            Some(HouseActorState {
                house: "oteryn:house.presentation_fixture".into(),
                scope,
                generation: ScopeOwnershipGeneration::new(1).unwrap(),
                position: at,
                lease,
                origin_position: at,
            }),
        )
        .unwrap();
    // The real Channel outbox can drain after its caster has entered a House.
    // Its source scope is still Channel, even when both rooms use these coordinates.
    publisher.capture(&mut outbox, &runtime).unwrap();
    assert!(
        publisher
            .observe(&runtime, actor, session, 0, true)
            .unwrap()
            .1
            .is_empty(),
        "a House resident cannot receive the Channel journal"
    );
    let position = MovementLocalPosition {
        x: at.x,
        y: at.y,
        floor: at.floor,
    };
    publisher.record_house_item_effect(&runtime, actor, source, position, 175);
    assert!(
        publisher
            .observe(&runtime, actor, session, 0, true)
            .unwrap()
            .1
            .is_empty(),
        "a retired session cannot publish a House effect"
    );
    publisher.record_house_item_effect(&runtime, actor, session, position, 175);
    let (revision, effects) = publisher
        .observe(&runtime, actor, session, 0, true)
        .unwrap();
    assert_eq!(effects.len(), 1);
    assert!(matches!(
        effects[0].kind,
        oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect {
            effect_id: 175,
            ..
        }
    ));
    assert_eq!(
        publisher
            .observe(&runtime, other, other_session, 0, true)
            .unwrap()
            .1
            .len(),
        2,
        "the Channel observer receives the earlier and delayed Channel cues"
    );
    assert!(
        publisher
            .observe(&runtime, actor, source, 0, true)
            .is_none(),
        "a retired Channel session cannot observe House effects"
    );
    assert!(
        publisher
            .observe(&runtime, actor, session, revision, true)
            .unwrap()
            .1
            .is_empty()
    );
    let chat = crate::gameplay_transport::chat_intent::ChatRuntime::default();
    let now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1);
    chat.admit(session, "House Observer", now);
    chat.admit(other_session, "Channel Observer", now);
    let effects = oteryn_gameplay_items::novelty_items::NoveltyEffects {
        kind: oteryn_gameplay_items::novelty_items::NoveltyKind::HouseTransform,
        source_item_id: 2603,
        donor: "data/scripts/actions/objects/wall_mirror.lua:17-24".into(),
        ground: None,
        cues: Vec::new(),
        speech: Some("You look awesome!".into()),
        private_speech: false,
        require_head: false,
        require_player_target: false,
        dice_value: None,
        costume: None,
        damage: None,
        achievement: None,
        achievement_progress: Vec::new(),
        timed_output: None,
    };
    chat.publish_item_speech(&runtime, actor, session, &effects);
    assert!(
        matches!(
            chat.drain(session).as_slice(),
            [oteryn_protocol_oteryn::chat::ChatLine::Local {
                mode: oteryn_protocol_oteryn::chat::ChatSpeechMode::MonsterSay,
                ..
            }]
        ),
        "House mirror speech retains the donor's MonsterSay presentation"
    );
    assert!(
        chat.drain(other_session).is_empty(),
        "House speech cannot leak to a Channel session"
    );
    chat.publish_house_decoration_speech(&runtime, actor, source, "You look awesome!");
    assert!(
        chat.drain(session).is_empty(),
        "a retired session cannot publish House speech"
    );
    chat.publish_house_decoration_speech(&runtime, actor, session, "You look awesome!");
    assert_eq!(chat.drain(session).len(), 1);
    id[15] = 97;
    let foreign_session = GameSessionId::decode(&id).unwrap();
    id[15] = 98;
    let foreign_character = CharacterId::decode(&id).unwrap();
    runtime
        .bind_attacker_lease(
            other,
            other_session,
            CharacterLease::new(foreign_character, 1).unwrap(),
        )
        .unwrap();
    let foreign_lease = CharacterLease::new(foreign_character, 2).unwrap();
    id[15] = 99;
    let mut foreign_house = runtime.house_actor_state(actor, session).unwrap();
    foreign_house.house = "oteryn:house.other_presentation_fixture".into();
    foreign_house.scope = RuntimeScopeRefV1::instance(runtime.binding().world_id(), id).unwrap();
    foreign_house.lease = foreign_lease;
    let foreign = runtime
        .restore_transfer_actor(
            other_session,
            foreign_session,
            foreign_lease,
            at,
            Some(foreign_house),
        )
        .unwrap();
    assert!(
        publisher
            .observe(&runtime, foreign, foreign_session, 0, true)
            .unwrap()
            .1
            .is_empty(),
        "matching coordinates in another named House cannot reveal the original House journal"
    );
    chat.admit(foreign_session, "Other House Observer", now);
    chat.publish_house_decoration_speech(&runtime, actor, session, "You look awesome!");
    assert_eq!(chat.drain(session).len(), 1);
    assert!(
        chat.drain(foreign_session).is_empty(),
        "matching coordinates cannot leak House speech"
    );
}

#[test]
fn qm46_quest_speech_is_public_player_monster_say_and_session_fenced() {
    let (runtime, actor, other, session) = runtime();
    let other_session = runtime
        .positioned_actor_census()
        .unwrap()
        .into_iter()
        .find(|(a, _, _)| *a == other)
        .unwrap()
        .2
        .unwrap();
    let chat = crate::gameplay_transport::chat_intent::ChatRuntime::default();
    let now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1);
    chat.admit(session, "Quest Speaker", now);
    chat.admit(other_session, "Observer", now);
    chat.publish_quest_speech(&runtime, actor, other_session, "forged");
    assert!(chat.drain(session).is_empty());
    assert!(chat.drain(other_session).is_empty());
    chat.publish_quest_speech(
        &runtime,
        actor,
        session,
        "Due to being extra greasy, the leavers can now be moved.",
    );
    for listener in [session, other_session] {
        let lines = chat.drain(listener);
        assert_eq!(lines.len(), 1);
        assert!(matches!(
            &lines[0],
            oteryn_protocol_oteryn::chat::ChatLine::Local {
                mode: oteryn_protocol_oteryn::chat::ChatSpeechMode::MonsterSay,
                speaker, speaker_name, text, ..
            } if speaker.identity == actor.placement_identity()
                && speaker.generation.get() == actor.actor_local_generation()
                && speaker_name == "Quest Speaker"
                && text == "Due to being extra greasy, the leavers can now be moved."
        ));
    }
}
