//! Preproduction owner seam tests; marker values do not assert active Reference.
use super::*;

fn uuid_v7(seed: u64) -> [u8; 16] {
    let mut value = [0; 16];
    value[8..].copy_from_slice(&seed.to_be_bytes());
    value[6] = 0x70;
    value[8] = (value[8] & 0x3f) | 0x80;
    value
}

fn grant(seed: u64, generation: u64) -> PreProductionContinuityGrant {
    PreProductionContinuityGrant {
        world_id: WorldId::decode(&uuid_v7(seed)).expect("world fixture"),
        channel_id: ChannelId::decode(&uuid_v7(seed + 1)).expect("channel fixture"),
        scope_generation: ScopeOwnershipGeneration::new(generation).expect("nonzero scope"),
    }
}

fn fixture(seed: u64, capacity: usize) -> (NamespaceContinuityGuard, ChannelActorCarrier) {
    let mut continuity = NamespaceContinuityGuard::from_pre_production_grant(grant(seed, 1));
    let carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, capacity)
        .expect("preproduction bootstrap");
    (continuity, carrier)
}

fn context(continuity: &NamespaceContinuityGuard) -> PreProductionPositionContext {
    PreProductionPositionContext {
        world_id: continuity.world_id,
        channel_id: continuity.channel_id,
        scope_generation: continuity.current_generation,
        coordinate_frame_marker: 11,
        map_revision_marker: 12,
        content_generation_marker: 13,
    }
}

fn position(x: i32) -> LocalPosition {
    LocalPosition { x, y: 5, floor: 7 }
}

#[test]
fn absent_then_initialized_single_slot_reads_and_commits_with_monotone_revision() {
    let (continuity, mut carrier) = fixture(10, 1);
    let actor = carrier.admit(&continuity, ActorState(1)).expect("admit");
    assert_eq!(
        carrier.read_position(&continuity, actor),
        Err(CarrierError::PositionUnavailable)
    );
    let scope = context(&continuity);
    let initial = carrier
        .initialize_position(&continuity, actor, scope, position(4))
        .expect("initialize");
    assert_eq!(initial.version.revision, 1);
    assert_eq!(carrier.read_position(&continuity, actor), Ok(initial));
    assert_eq!(
        carrier.initialize_position(&continuity, actor, scope, position(7)),
        Err(CarrierError::PositionAlreadyInitialized)
    );
    let committed = carrier
        .compare_commit_position(&continuity, initial, scope, position(5))
        .expect("commit");
    assert_eq!(committed.version.revision, 2);
    assert_eq!(committed.version.position, position(5));
    assert_eq!(carrier.read_position(&continuity, actor), Ok(committed));
    assert_eq!(
        carrier.compare_commit_position(&continuity, initial, scope, position(6)),
        Err(CarrierError::PositionSnapshotMismatch)
    );
    assert_eq!(carrier.read_position(&continuity, actor), Ok(committed));
}

#[test]
fn scope_and_context_fields_fail_independently_without_mutation() {
    let (continuity, mut carrier) = fixture(20, 1);
    let actor = carrier.admit(&continuity, ActorState(2)).expect("admit");
    let scope = context(&continuity);
    let initial = carrier
        .initialize_position(&continuity, actor, scope, position(4))
        .expect("initialize");
    let before = carrier.slots.clone();

    let mut wrong_world = actor;
    wrong_world.world_id = grant(30, 1).world_id;
    let mut wrong_channel = actor;
    wrong_channel.channel_id = grant(30, 1).channel_id;
    let mut wrong_scope = actor;
    wrong_scope.scope_generation = ScopeOwnershipGeneration::new(2).expect("scope");
    for wrong in [wrong_world, wrong_channel, wrong_scope] {
        assert_eq!(
            carrier.read_position(&continuity, wrong),
            Err(CarrierError::WrongScope)
        );
        assert_eq!(
            carrier.compare_commit_position(
                &continuity,
                PositionSnapshot {
                    actor_ref: wrong,
                    ..initial
                },
                scope,
                position(5)
            ),
            Err(CarrierError::WrongScope)
        );
        assert_eq!(carrier.slots, before);
    }

    for changed in 0..6 {
        let mut wrong = scope;
        match changed {
            0 => wrong.world_id = grant(31, 1).world_id,
            1 => wrong.channel_id = grant(31, 1).channel_id,
            2 => wrong.scope_generation = ScopeOwnershipGeneration::new(2).expect("scope"),
            3 => wrong.coordinate_frame_marker += 1,
            4 => wrong.map_revision_marker += 1,
            _ => wrong.content_generation_marker += 1,
        }
        let expected_error = if changed < 3 {
            CarrierError::InvalidPreProductionPositionContext
        } else {
            CarrierError::PositionContextMismatch
        };
        assert_eq!(
            carrier.compare_commit_position(&continuity, initial, wrong, position(5)),
            Err(expected_error)
        );
        assert_eq!(carrier.slots, before);
    }
    for changed in 0..3 {
        let mut invalid = scope;
        match changed {
            0 => invalid.coordinate_frame_marker = 0,
            1 => invalid.map_revision_marker = 0,
            _ => invalid.content_generation_marker = 0,
        }
        assert_eq!(
            carrier.compare_commit_position(&continuity, initial, invalid, position(5)),
            Err(CarrierError::InvalidPreProductionPositionContext)
        );
        assert_eq!(carrier.slots, before);
    }
}

#[test]
fn recycled_wrong_actor_and_stale_owner_cannot_read_or_commit() {
    let (mut continuity, mut carrier) = fixture(40, 2);
    let first = carrier.admit(&continuity, ActorState(1)).expect("first");
    let other = carrier.admit(&continuity, ActorState(2)).expect("other");
    let scope = context(&continuity);
    let initial = carrier
        .initialize_position(&continuity, first, scope, position(4))
        .expect("initialize");
    carrier
        .initialize_position(&continuity, other, scope, position(4))
        .expect("identical independent position");
    let before = carrier.slots.clone();
    let mut changed = initial;
    changed.actor_ref = other;
    assert_eq!(
        carrier.compare_commit_position(&continuity, changed, scope, position(6)),
        Err(CarrierError::PositionSnapshotMismatch)
    );
    assert_eq!(carrier.slots, before);

    carrier.remove(&continuity, first).expect("remove");
    let recycled = carrier.admit(&continuity, ActorState(3)).expect("reuse");
    assert_eq!(first.actor_local_id, recycled.actor_local_id);
    assert_ne!(
        first.actor_local_generation,
        recycled.actor_local_generation
    );
    let after_recycle = carrier.slots.clone();
    assert_eq!(
        carrier.read_position(&continuity, first),
        Err(CarrierError::StaleActorGeneration)
    );
    assert_eq!(
        carrier.compare_commit_position(&continuity, initial, scope, position(6)),
        Err(CarrierError::PositionSnapshotMismatch)
    );
    assert_eq!(
        carrier.read_position(&continuity, recycled),
        Err(CarrierError::PositionUnavailable)
    );
    assert_eq!(carrier.slots, after_recycle);

    continuity.advance(grant(40, 2)).expect("owner advance");
    assert_eq!(
        carrier.read_position(&continuity, recycled),
        Err(CarrierError::WrongScope)
    );
    assert_eq!(
        carrier.compare_commit_position(&continuity, initial, scope, position(7)),
        Err(CarrierError::WrongScope)
    );
    assert_eq!(carrier.slots, after_recycle);
}

#[test]
fn stale_position_and_revision_overflow_reject_without_mutation() {
    let (continuity, mut carrier) = fixture(50, 1);
    let actor = carrier.admit(&continuity, ActorState(1)).expect("admit");
    let scope = context(&continuity);
    let initial = carrier
        .initialize_position(&continuity, actor, scope, position(4))
        .expect("initialize");
    let mut wrong = initial;
    wrong.version.position = position(100);
    let before = carrier.slots.clone();
    assert_eq!(
        carrier.compare_commit_position(&continuity, wrong, scope, position(5)),
        Err(CarrierError::PositionSnapshotMismatch)
    );
    assert_eq!(carrier.slots, before);

    if let Slot::Occupied {
        position: Some(version),
        ..
    } = &mut carrier.slots[0]
    {
        version.revision = u64::MAX;
    }
    let overflow_snapshot = carrier.read_position(&continuity, actor).expect("read");
    let before = carrier.slots.clone();
    assert_eq!(
        carrier.compare_commit_position(&continuity, overflow_snapshot, scope, position(5)),
        Err(CarrierError::PositionRevisionExhausted)
    );
    assert_eq!(carrier.slots, before);
}
