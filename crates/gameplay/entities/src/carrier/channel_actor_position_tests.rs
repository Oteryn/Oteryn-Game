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
        runtime_scope: RuntimeScopeRefV1::channel(
            oteryn_gameplay_foundation::foundation::WorldId::decode(&uuid_v7(seed))
                .expect("world fixture"),
            oteryn_gameplay_foundation::foundation::ChannelId::decode(&uuid_v7(seed + 1))
                .expect("channel fixture"),
        ),
        scope_generation: oteryn_gameplay_foundation::foundation::ScopeOwnershipGeneration::new(
            generation,
        )
        .expect("nonzero scope"),
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
        scope: positions::PositionScope::new(continuity.runtime_scope)
            .expect("valid scoped context"),
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
        Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PositionUnavailable)
    );
    let scope = context(&continuity);
    let initial = carrier
        .initialize_position(&continuity, actor, scope, position(4))
        .expect("initialize");
    assert_eq!(initial.version.revision, 1);
    assert_eq!(carrier.read_position(&continuity, actor), Ok(initial));
    assert_eq!(
        carrier.initialize_position(&continuity, actor, scope, position(7)),
        Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PositionAlreadyInitialized)
    );
    let committed = carrier
        .compare_commit_position(&continuity, initial, scope, position(5))
        .expect("commit");
    assert_eq!(committed.version.revision, 2);
    assert_eq!(committed.version.position, position(5));
    assert_eq!(carrier.read_position(&continuity, actor), Ok(committed));
    assert_eq!(
        carrier.compare_commit_position(&continuity, initial, scope, position(6)),
        Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PositionSnapshotMismatch)
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
    wrong_world.runtime_scope = oteryn_gameplay_foundation::foundation::RuntimeScopeRefV1::channel(
        grant(30, 1).runtime_scope.world_id(),
        oteryn_gameplay_foundation::foundation::ExactActorRef(wrong_world)
            .channel_id()
            .expect("Channel actor"),
    );
    let mut wrong_channel = actor;
    wrong_channel.runtime_scope =
        oteryn_gameplay_foundation::foundation::RuntimeScopeRefV1::channel(
            oteryn_gameplay_foundation::foundation::ExactActorRef(wrong_channel).world_id(),
            grant(30, 1).channel_id().expect("Channel fixture"),
        );
    let mut wrong_scope = actor;
    wrong_scope.set_scope_generation(
        oteryn_gameplay_foundation::foundation::ScopeOwnershipGeneration::new(2).expect("scope"),
    );
    for wrong in [wrong_world, wrong_channel, wrong_scope] {
        assert_eq!(
            carrier.read_position(&continuity, wrong),
            Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::WrongScope)
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
            Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::WrongScope)
        );
        assert_eq!(carrier.slots, before);
    }

    for changed in 0..6 {
        let mut wrong = scope;
        match changed {
            0 => {
                wrong.scope = positions::PositionScope::channel(
                    grant(31, 1).runtime_scope.world_id(),
                    continuity.channel_id().expect("Channel fixture"),
                )
            }
            1 => {
                wrong.scope = positions::PositionScope::channel(
                    continuity.runtime_scope.world_id(),
                    grant(31, 1).channel_id().expect("Channel fixture"),
                )
            }
            2 => {
                wrong.scope_generation =
                    oteryn_gameplay_foundation::foundation::ScopeOwnershipGeneration::new(2)
                        .expect("scope")
            }
            3 => wrong.coordinate_frame_marker += 1,
            4 => wrong.map_revision_marker += 1,
            _ => wrong.content_generation_marker += 1,
        }
        let expected_error = if changed < 3 {
            oteryn_gameplay_domain::domain::carrier_values::CarrierError::InvalidPreProductionPositionContext
        } else {
            oteryn_gameplay_domain::domain::carrier_values::CarrierError::PositionContextMismatch
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
            Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::InvalidPreProductionPositionContext)
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
        Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PositionSnapshotMismatch)
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
        Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::StaleActorGeneration)
    );
    assert_eq!(
        carrier.compare_commit_position(&continuity, initial, scope, position(6)),
        Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PositionSnapshotMismatch)
    );
    assert_eq!(
        carrier.read_position(&continuity, recycled),
        Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PositionUnavailable)
    );
    assert_eq!(carrier.slots, after_recycle);

    continuity.advance(grant(40, 2)).expect("owner advance");
    assert_eq!(
        carrier.read_position(&continuity, recycled),
        Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::WrongScope)
    );
    assert_eq!(
        carrier.compare_commit_position(&continuity, initial, scope, position(7)),
        Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::WrongScope)
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
        Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PositionSnapshotMismatch)
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
        Err(
            oteryn_gameplay_domain::domain::carrier_values::CarrierError::PositionRevisionExhausted
        )
    );
    assert_eq!(carrier.slots, before);
}

#[test]
fn translation_stamp_ignores_turns_and_same_cell_writes() {
    let (continuity, mut carrier) = fixture(60, 1);
    let actor = carrier.admit(&continuity, ActorState(1)).expect("admit");
    let scope = context(&continuity);
    let initial = carrier
        .initialize_position(&continuity, actor, scope, position(4))
        .expect("initialize");
    let mut current = MovementPositionSnapshot(initial);
    assert_eq!(current.last_translation_revision(), 0);
    for facing in [
        MovementFacing::North,
        MovementFacing::West,
        MovementFacing::South,
    ] {
        let before = current;
        current = carrier
            .current_owner_movement_position(&continuity)
            .commit_turn(before, facing)
            .expect("real turn");
        assert_eq!(current.position(), before.position());
        assert_eq!(current.facing(), Some(facing));
        assert!(current.revision() > before.revision());
        assert_eq!(current.last_translation_revision(), 0);
    }
    let next = MovementLocalPosition {
        x: 5,
        ..current.position()
    };
    let moved = carrier
        .current_owner_movement_position(&continuity)
        .commit_cardinal(current, next)
        .expect("real translation");
    assert_eq!(moved.last_translation_revision(), moved.revision());
    let turned = carrier
        .current_owner_movement_position(&continuity)
        .commit_turn(moved, MovementFacing::East)
        .expect("turn after movement");
    assert_eq!(
        turned.last_translation_revision(),
        moved.last_translation_revision()
    );
    let same_cell = MovementPositionSnapshot(
        carrier
            .compare_commit_position(&continuity, turned.0, scope, turned.0.version.position)
            .expect("same-cell owner write"),
    );
    assert!(same_cell.revision() > turned.revision());
    assert_eq!(same_cell.position(), moved.position());
    assert_eq!(
        same_cell.last_translation_revision(),
        moved.last_translation_revision()
    );
}

#[test]
fn translation_stamp_observes_out_and_back_and_rejects_stale_snapshots() {
    let (continuity, mut carrier) = fixture(70, 1);
    let actor = carrier.admit(&continuity, ActorState(1)).expect("admit");
    let scope = context(&continuity);
    let initial = MovementPositionSnapshot(
        carrier
            .initialize_position(&continuity, actor, scope, position(4))
            .expect("initialize"),
    );
    let away = carrier
        .current_owner_movement_position(&continuity)
        .commit_cardinal(
            initial,
            MovementLocalPosition {
                x: 5,
                ..initial.position()
            },
        )
        .expect("step out");
    let returned = carrier
        .current_owner_movement_position(&continuity)
        .commit_cardinal(away, initial.position())
        .expect("step back");
    assert_eq!(returned.position(), initial.position());
    assert!(returned.last_translation_revision() > away.last_translation_revision());
    assert_eq!(returned.last_translation_revision(), returned.revision());
    let before = carrier.slots.clone();
    assert_eq!(
        carrier
            .current_owner_movement_position(&continuity)
            .commit_cardinal(initial, away.position()),
        Err(CarrierError::PositionSnapshotMismatch)
    );
    // The readonly marker remains part of the owner's exact snapshot comparison.
    let mut altered = returned;
    altered.0.version.last_translation_revision = away.last_translation_revision();
    assert_eq!(
        carrier
            .current_owner_movement_position(&continuity)
            .commit_cardinal(altered, away.position()),
        Err(CarrierError::PositionSnapshotMismatch)
    );
    assert_eq!(carrier.slots, before);
}

#[test]
fn translation_stamp_tracks_relocation_in_the_same_position_owner() {
    // This sealed fixture exercises the position owner only. Production relocation
    // eligibility still comes from the existing World/SQL proof producer.
    struct Proof(
        ExactActorRef,
        MovementPositionSnapshot,
        MovementLocalPosition,
    );
    impl relocation_seal::Sealed for Proof {}
    impl SpellRelocationProof for Proof {
        fn parts(
            &self,
        ) -> (
            ExactActorRef,
            MovementPositionSnapshot,
            MovementLocalPosition,
        ) {
            (self.0, self.1, self.2)
        }
    }
    let (continuity, mut carrier) = fixture(80, 1);
    let actor = carrier.admit(&continuity, ActorState(1)).expect("admit");
    let scope = context(&continuity);
    let initial = MovementPositionSnapshot(
        carrier
            .initialize_position(&continuity, actor, scope, position(4))
            .expect("initialize"),
    );
    let destination = MovementLocalPosition {
        x: 20,
        y: 5,
        floor: 8,
    };
    let moved = carrier
        .current_owner_movement_position(&continuity)
        .commit_spell_relocation(Proof(ExactActorRef(actor), initial, destination))
        .expect("same-slot floor relocation");
    assert_eq!(moved.position(), destination);
    assert_eq!(moved.last_translation_revision(), moved.revision());
    let same_cell = carrier
        .current_owner_movement_position(&continuity)
        .commit_spell_relocation(Proof(ExactActorRef(actor), moved, destination))
        .expect("same-cell relocation owner write");
    assert_eq!(
        same_cell.last_translation_revision(),
        moved.last_translation_revision()
    );
    let returned = carrier
        .current_owner_movement_position(&continuity)
        .commit_spell_relocation(Proof(ExactActorRef(actor), same_cell, initial.position()))
        .expect("relocation back");
    assert_eq!(returned.position(), initial.position());
    assert!(returned.last_translation_revision() > moved.last_translation_revision());
    assert_eq!(returned.last_translation_revision(), returned.revision());
}

#[test]
fn translation_stamp_is_scoped_to_the_exact_actor_and_current_owner() {
    let (mut continuity, mut carrier) = fixture(90, 1);
    let actor = carrier.admit(&continuity, ActorState(1)).expect("admit");
    let scope = context(&continuity);
    let initial = MovementPositionSnapshot(
        carrier
            .initialize_position(&continuity, actor, scope, position(4))
            .expect("initialize"),
    );
    let old = carrier
        .current_owner_movement_position(&continuity)
        .commit_cardinal(
            initial,
            MovementLocalPosition {
                x: 5,
                ..initial.position()
            },
        )
        .expect("old actor moved");
    carrier
        .remove(&continuity, actor)
        .expect("remove old actor");
    let recycled = carrier
        .admit(&continuity, ActorState(2))
        .expect("reuse slot");
    assert_eq!(recycled.actor_local_id, actor.actor_local_id);
    assert_ne!(
        recycled.actor_local_generation,
        actor.actor_local_generation
    );
    let fresh = MovementPositionSnapshot(
        carrier
            .initialize_position(&continuity, recycled, scope, position(5))
            .expect("new actor position"),
    );
    assert_eq!(fresh.position(), old.position());
    assert_eq!(fresh.last_translation_revision(), 0);
    let before = carrier.slots.clone();
    assert!(
        carrier
            .current_owner_movement_position(&continuity)
            .commit_cardinal(old, initial.position())
            .is_err()
    );
    assert_eq!(carrier.slots, before);
    continuity.advance(grant(90, 2)).expect("owner retired");
    assert_eq!(
        carrier
            .current_owner_movement_position(&continuity)
            .commit_cardinal(fresh, initial.position()),
        Err(CarrierError::WrongScope)
    );
    assert_eq!(carrier.slots, before);
}
