use super::*;
use crate::foundation::{
    ChannelContentPin, ChannelId, CharacterId, CommandId, CommandRef, GameSessionId, NodeId,
    WorldId,
};
use crate::spell::combat_batch::{OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding};

fn id(tag: u8) -> [u8; 16] {
    [1, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
}
pub(crate) fn runtime() -> (
    ChannelRuntimeV1,
    ExactActorRef,
    ExactActorRef,
    GameSessionId,
) {
    let world = WorldId::decode(&id(1)).unwrap();
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&id(2)).unwrap(),
        NodeId::decode(&id(3)).unwrap(),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        ChannelContentPin::test(world),
    )
    .unwrap();
    let session = GameSessionId::decode(&id(4)).unwrap();
    let reserved = runtime.reserve_fresh_session(session).unwrap();
    let caster = runtime.commit_fresh_session(reserved).unwrap();
    runtime.initialize_first_entry_position(caster).unwrap();
    let other_session = GameSessionId::decode(&id(5)).unwrap();
    let reserved = runtime.reserve_fresh_session(other_session).unwrap();
    let other = runtime.commit_fresh_session(reserved).unwrap();
    runtime.initialize_first_entry_position(other).unwrap();
    runtime
        .admit_pinned_test_creature(MovementLocalPosition {
            x: 10,
            y: 0,
            floor: 0,
        })
        .unwrap();
    runtime
        .admit_pinned_test_creature(MovementLocalPosition {
            x: 11,
            y: 0,
            floor: 0,
        })
        .unwrap();
    (runtime, caster, other, session)
}
/// Queue tests deliberately start after source cue qualification. Their receipts
/// come from the real physical batch owner, never a forged history constructor.
pub(crate) fn prepared(
    owner: &mut SpellPresentationOwner,
    runtime: &ChannelRuntimeV1,
    caster: ExactActorRef,
    session: GameSessionId,
    sequence: u64,
    count: usize,
) -> PreparedPresentation {
    owner.reserve_before_draw(runtime, count).unwrap();
    let target_x = if session.as_bytes() == &id(4) { 10 } else { 11 };
    let target = runtime
        .positioned_actor_census()
        .unwrap()
        .into_iter()
        .find(|(_, p, s)| s.is_none() && p.position().x == target_x)
        .unwrap()
        .0;
    let effects = vec![OwnerCombatEffect {
        target,
        sub_ordinal: 0,
        change: OwnerCombatChange::Damage {
            target_atom: runtime.creature_spell_target_atom(target).unwrap(),
            magnitude: 1,
        },
    }];
    let batch = OwnerCombatBatch {
        caster,
        attacker: CharacterId::decode(&id(6)).unwrap(),
        current_lease_generation: 1,
        command: CommandRef::new(session, CommandId::new(sequence).unwrap()),
        occurrence: SpellOccurrenceBinding {
            id: "queue-owned-cast".into(),
            revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"].map(str::to_owned),
        },
        binding: sequence.to_be_bytes().to_vec(),
        anchor: None,
        now_ms: 100,
        effects,
        deferred: None,
    };
    let event = Presentation {
        source_binding: "appearance:effect/magic_green".into(),
        cue: Cue::Effect(18),
        actor: Some(caster),
        position: runtime.read_actor_position(caster).unwrap().position(),
    };
    let cause = std::sync::Arc::new(PresentationCause::Cast {
        caster,
        origin: runtime.read_actor_position(caster).unwrap().position(),
        occurrence: batch.occurrence.clone(),
    });
    PreparedPresentation {
        content: owner.content,
        scope: owner.scope,
        generation: owner.generation,
        batch,
        cause,
        events: vec![event; count],
        tiles: vec![],
        source_origin: runtime.read_actor_position(caster).unwrap().position(),
    }
}
#[test]
fn unrelated_publication_preserves_pending_original_hold_and_replay_emits_nothing() {
    let (mut runtime, caster, other, session) = runtime();
    let mut owner = SpellPresentationOwner::new(&runtime);
    let original = prepared(&mut owner, &runtime, caster, session, 1, 2);
    owner
        .hold_prepared_before_sql(&runtime, &original, &original.batch)
        .unwrap();
    let other_session = runtime
        .positioned_actor_census()
        .unwrap()
        .into_iter()
        .find(|(actor, _, _)| *actor == other)
        .unwrap()
        .2
        .unwrap();
    let unrelated = prepared(&mut owner, &runtime, other, other_session, 1, 3);
    owner
        .hold_prepared_before_sql(&runtime, &unrelated, &unrelated.batch)
        .unwrap();
    let staged = runtime.stage_spell_batch(&unrelated.batch).unwrap();
    let receipt = runtime.commit_spell_batch(staged).unwrap();
    owner.install_preflighted(unrelated, &receipt);
    assert_eq!(owner.take_for_current_publisher(&runtime).unwrap().len(), 3);
    owner
        .validate_prepared(&runtime, &original, &original.batch)
        .unwrap();
    let batch = original.batch.clone();
    let staged = runtime.stage_spell_batch(&batch).unwrap();
    let receipt = runtime.commit_spell_batch(staged).unwrap();
    owner.install_preflighted(original, &receipt);
    assert_eq!(owner.take_for_current_publisher(&runtime).unwrap().len(), 2);
    let replay = prepared(&mut owner, &runtime, caster, session, 1, 2);
    owner
        .hold_prepared_before_sql(&runtime, &replay, &replay.batch)
        .unwrap();
    let staged = runtime.stage_spell_batch(&batch).unwrap();
    let receipt = runtime.commit_spell_batch(staged).unwrap();
    assert!(!receipt.applied);
    owner.install_preflighted(replay, &receipt);
    assert!(
        owner
            .take_for_current_publisher(&runtime)
            .unwrap()
            .is_empty()
    );
    assert!(owner.held.is_empty());
}
#[test]
fn unknown_cast_holds_entire_queue_budget_until_known_noncommit() {
    let (runtime, caster, other, session) = runtime();
    let mut owner = SpellPresentationOwner::new(&runtime);
    let original = prepared(&mut owner, &runtime, caster, session, 1, 256);
    owner
        .hold_prepared_before_sql(&runtime, &original, &original.batch)
        .unwrap();
    owner
        .hold_prepared_before_sql(&runtime, &original, &original.batch)
        .unwrap();
    assert_eq!(owner.held.len(), 1);
    let other_session = runtime
        .positioned_actor_census()
        .unwrap()
        .into_iter()
        .find(|(actor, _, _)| *actor == other)
        .unwrap()
        .2
        .unwrap();
    let unrelated = prepared(&mut owner, &runtime, other, other_session, 1, 256);
    owner
        .hold_prepared_before_sql(&runtime, &unrelated, &unrelated.batch)
        .unwrap();
    assert_eq!(owner.reserve_before_draw(&runtime, 1), Err(Error::Capacity));
    let mut changed = PreparedPresentation {
        content: original.content,
        scope: original.scope,
        generation: original.generation,
        batch: original.batch.clone(),
        cause: original.cause.clone(),
        events: original.events.clone(),
        tiles: vec![],
        source_origin: original.source_origin,
    };
    changed.batch.binding.push(1);
    assert_eq!(
        owner.hold_prepared_before_sql(&runtime, &changed, &changed.batch),
        Err(Error::InvalidBatch)
    );
    owner.release_definitely_uncommitted(&original);
    assert!(owner.reserve_before_draw(&runtime, 256).is_ok());
    owner
        .validate_prepared(&runtime, &unrelated, &unrelated.batch)
        .unwrap();
}

/// Test-only producer after source-cue qualification, using the actual physical owner receipt.
pub(crate) fn commit_fixture(
    owner: &mut SpellPresentationOwner,
    runtime: &mut ChannelRuntimeV1,
    caster: ExactActorRef,
    session: GameSessionId,
    command: u64,
) {
    let prepared = prepared(owner, runtime, caster, session, command, 1);
    owner
        .hold_prepared_before_sql(runtime, &prepared, &prepared.batch)
        .unwrap();
    let staged = runtime.stage_spell_batch(&prepared.batch).unwrap();
    let receipt = runtime.commit_spell_batch(staged).unwrap();
    owner.install_preflighted(prepared, &receipt);
}
