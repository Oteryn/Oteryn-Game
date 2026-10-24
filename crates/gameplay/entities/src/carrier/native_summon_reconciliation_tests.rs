use super::*;
fn fixture() -> ChannelRuntimeV1 {
    let id = |t: u8| [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, t];
    let w = WorldId::decode(&id(1)).expect("valid native fixture operation must succeed");
    ChannelRuntimeV1::from_committed_assignment(
        w,
        ChannelId::decode(&id(2)).expect("valid native fixture operation must succeed"),
        NodeId::decode(&id(3)).expect("valid native fixture operation must succeed"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        ChannelContentPin::test(w),
    )
    .expect("valid native fixture operation must succeed")
}
fn parent(r: &mut ChannelRuntimeV1) -> ExactActorRef {
    let a = r
        .carrier
        .admit_creature(&r.continuity, ActorState(0), "oteryn:creature.rat", 20)
        .expect("valid native fixture operation must succeed");
    r.carrier
        .initialize_position(
            &r.continuity,
            a,
            r.pinned_position_context(),
            LocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
        )
        .expect("valid native fixture operation must succeed");
    ExactActorRef(a)
}
fn child(r: &mut ChannelRuntimeV1, p: ExactActorRef) -> ExactActorRef {
    let b = r.binding();
    let scope = oteryn_gameplay_foundation::foundation::RuntimeScopeRefV1::channel(
        b.world_id(),
        b.channel_id(),
    );
    let mut f =
        oteryn_gameplay_foundation::foundation::scoped_fence_fixture(scope, b.scope_generation());
    let n = f
        .accept_input(b.scope_generation())
        .expect("valid native fixture operation must succeed");
    let stamp = f.stamp(n);
    let spec = NativeSummonAdmissionSpec::qualified(
        "oteryn:creature.rat",
        "oteryn:creature.rat",
        10,
        0,
        2,
        2,
        r.content.server_artifact_digest(),
    )
    .expect("valid native fixture operation must succeed");
    r.admit_native_summon(
        &f,
        stamp,
        p,
        &spec,
        MovementLocalPosition {
            x: 101,
            y: 100,
            floor: 7,
        },
    )
    .expect("valid native fixture operation must succeed")
}
#[test]
fn separated_summon_late_reservation_failure_restores_occupied_index_and_visibility() {
    use super::super::{SourceActorReservationProof, source_reservation_seal};
    struct Proof {
        owner: ExactActorRef,
        session: GameSessionId,
        position: MovementPositionSnapshot,
        affected: [ExactActorRef; 1],
    }
    impl source_reservation_seal::Sealed for Proof {}
    impl SourceActorReservationProof for Proof {
        fn owner(&self) -> (ExactActorRef, GameSessionId) {
            (self.owner, self.session)
        }
        fn event_identity(&self) -> [u8; 16] {
            [7; 16]
        }
        fn source_binding_digest(&self) -> [u8; 32] {
            [8; 32]
        }
        fn affected_actors(&self) -> &[ExactActorRef] {
            &self.affected
        }
        fn current_for(&self, r: &ChannelRuntimeV1) -> bool {
            r.owner_fence().is_ok()
                && r.read_actor_position(self.owner) == Ok(self.position)
                && r.player_control_facts(self.owner, self.session)
                    .is_ok_and(|f| f.control_loss.is_none())
        }
    }
    let mut r = fixture();
    let p = parent(&mut r);
    let a = child(&mut r, p);
    let at = r
        .carrier
        .read_position(&r.continuity, a.0)
        .expect("valid native fixture operation must succeed");
    let context = r.pinned_position_context();
    r.carrier
        .compare_commit_position(
            &r.continuity,
            at,
            context,
            LocalPosition {
                x: 102,
                y: 100,
                floor: 7,
            },
        )
        .expect("valid native fixture operation must succeed");
    let b = child(&mut r, p);
    let session = GameSessionId::decode(&[1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 4])
        .expect("valid native fixture operation must succeed");
    let reserved = r
        .reserve_fresh_session(session)
        .expect("valid native fixture operation must succeed");
    let owner = r
        .commit_fresh_session(reserved)
        .expect("valid native fixture operation must succeed");
    let position = match r
        .initialize_first_entry_position(owner)
        .expect("valid native fixture operation must succeed")
    {
        super::super::FirstEntryPosition::Initialized(p)
        | super::super::FirstEntryPosition::Reconciled(p) => p,
    };
    let proof = Proof {
        owner,
        session,
        position,
        affected: [b],
    };
    let mut reserved = r
        .prepare_source_actor_reservation(&proof)
        .expect("valid native fixture operation must succeed");
    r.reserve_source_actors(&mut reserved, &proof)
        .expect("valid native fixture operation must succeed");
    let slots = r.carrier.slots.clone();
    let free = r.carrier.free_head;
    let occupied = r.carrier.occupied.clone();
    let visible = r.visible_entities();
    let before = r
        .carrier
        .read_position(&r.continuity, p.0)
        .expect("valid native fixture operation must succeed");
    assert_eq!(
        r.carrier.compare_commit_position(
            &r.continuity,
            before,
            context,
            LocalPosition {
                x: 200,
                y: 100,
                floor: 7
            }
        ),
        Err(CarrierError::PlanConflict)
    );
    assert_eq!(r.carrier.slots, slots);
    assert_eq!(r.carrier.free_head, free);
    assert_eq!(r.carrier.occupied, occupied);
    assert_eq!(r.visible_entities(), visible);
    assert!(r.contains_live_creature(a));
    assert!(r.contains_live_creature(b));
    // A later ordinary removal/re-admission uses a slot exactly once, with a new generation.
    r.carrier
        .remove(&r.continuity, a.0)
        .expect("valid native fixture operation must succeed");
    let reused = parent(&mut r);
    assert_eq!(reused.0.actor_local_id, a.0.actor_local_id);
    assert_ne!(reused.0.actor_local_generation, a.0.actor_local_generation);
    let index = u32::try_from(
        r.carrier
            .validate_ref(&r.continuity, reused.0)
            .expect("valid native fixture operation must succeed"),
    )
    .expect("valid native fixture operation must succeed");
    assert_eq!(
        r.carrier.occupied.iter().filter(|i| **i == index).count(),
        1
    );
}
#[test]
fn parent_projection_cascades_native_death_and_preserves_main_condition_death_hook() {
    let mut r = fixture();
    let p = parent(&mut r);
    let a = child(&mut r, p);
    let i = r
        .carrier
        .validate_ref(&r.continuity, a.0)
        .expect("valid native fixture operation must succeed");
    let before = match &r.carrier.slots[i] {
        Slot::CreatureOccupied { committed, .. } => Some(committed.conditions.clone()),
        _ => None,
    }
    .expect("fixture must contain a native creature");
    r.carrier
        .current_owner_exact_commit(&r.continuity)
        .commit_damage(
            p,
            OwnerDamageCommand {
                target: b"oteryn:creature.rat",
                occurrence: b"master-lethal",
                binding: b"master-lethal\0fixture",
                damage: 20,
            },
        )
        .expect("valid native fixture operation must succeed");
    let mut death = r.borrow_combat_death();
    let receipt = death
        .committed_lethal_receipt(p)
        .expect("valid native fixture operation must succeed");
    death
        .project_committed_lethal(receipt)
        .expect("valid native fixture operation must succeed");
    assert!(!r.contains_live_creature(a));
    let after = match &r.carrier.slots[i] {
        Slot::CreatureOccupied { committed, .. } => Some(&committed.conditions),
        _ => None,
    }
    .expect("fixture must contain a native creature");
    assert_ne!(before, *after);
    let child_death = r
        .project_native_summon_death(a)
        .expect("valid native fixture operation must succeed");
    assert_eq!(child_death.child, a);
    assert_eq!(r.carrier.corpse_projections.len(), 1);
    assert_eq!(
        r.borrow_combat_death()
            .reward_occurrence(a, [0; 16])
            .expect_err("fixture must reject this invalid operation"),
        CarrierError::SummonHasNoRewards
    );
}
#[test]
fn parent_administrative_remove_retires_child_without_corpse_and_generation_reuse_is_not_related() {
    let mut r = fixture();
    let p = parent(&mut r);
    let a = child(&mut r, p);
    r.carrier
        .remove(&r.continuity, p.0)
        .expect("valid native fixture operation must succeed");
    assert!(!r.contains_live_creature(a));
    assert!(r.carrier.native_summons.is_empty());
    assert!(r.carrier.corpse_projections.is_empty());
    assert!(r.carrier.death_reward_occurrences.is_empty());
    let replacement = parent(&mut r);
    assert_ne!(replacement, p);
    assert_eq!(r.native_summon_master(replacement), None);
}
