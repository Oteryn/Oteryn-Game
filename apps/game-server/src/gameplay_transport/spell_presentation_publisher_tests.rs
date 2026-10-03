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
    assert!(matches!(
        view(MovementLocalPosition {
            x: 0,
            y: 0,
            floor: -5
        }),
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
