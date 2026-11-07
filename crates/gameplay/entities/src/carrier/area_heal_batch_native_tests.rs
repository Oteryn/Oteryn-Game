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
#[test]
fn all_target_preflight_rejects_second_before_first_hp_and_commits_caps() {
    let mut r = fixture();
    let a = ExactActorRef(
        r.carrier
            .admit_creature(&r.continuity, ActorState(0), "oteryn:creature.rat", 5)
            .expect("valid native fixture operation must succeed"),
    );
    let b = ExactActorRef(
        r.carrier
            .admit_creature(&r.continuity, ActorState(0), "oteryn:creature.rat", 10)
            .expect("valid native fixture operation must succeed"),
    );
    let mut q = vec![
        (a, "oteryn:creature.rat".to_string(), 20, 10),
        (b, "oteryn:creature.demon".to_string(), 20, 10),
    ];
    assert!(r.commit_source_creature_heal_batch([1; 32], &q).is_err());
    let ai = r
        .carrier
        .validate_ref(&r.continuity, a.0)
        .expect("valid native fixture operation must succeed");
    assert!(matches!(
        r.carrier.slots[ai],
        Slot::CreatureOccupied { health: 5, .. }
    ));
    q[1].1 = "oteryn:creature.rat".into();
    q[1].3 = 100;
    let receipts = r
        .commit_source_creature_heal_batch([1; 32], &q)
        .expect("valid native fixture operation must succeed");
    assert_eq!(
        (receipts[0].health_after, receipts[1].health_after),
        (15, 20)
    );
    let after = receipts[0].health_after;
    assert!(r.commit_source_creature_heal_batch([2; 32], &q).is_err());
    assert!(matches!(r.carrier.slots[ai],Slot::CreatureOccupied{health,..}if health==after));
}
#[test]
fn stale_target_generation_and_duplicates_refuse_whole_batch() {
    let mut r = fixture();
    let a = ExactActorRef(
        r.carrier
            .admit_creature(&r.continuity, ActorState(0), "oteryn:creature.rat", 5)
            .expect("valid native fixture operation must succeed"),
    );
    let q = (a, "oteryn:creature.rat".to_string(), 20, 10);
    assert_eq!(
        r.commit_source_creature_heal_batch([1; 32], &[q.clone(), q.clone()])
            .expect_err("fixture must reject this invalid operation"),
        CarrierError::PlanConflict
    );
    let i = r
        .carrier
        .validate_ref(&r.continuity, a.0)
        .expect("valid native fixture operation must succeed");
    if let Slot::CreatureOccupied { generation, .. } = &mut r.carrier.slots[i] {
        *generation += 1
    } else {
        assert!(
            matches!(r.carrier.slots[i], Slot::CreatureOccupied { .. }),
            "fixture must contain a native creature"
        )
    };
    assert_eq!(
        r.commit_source_creature_heal_batch([1; 32], &[q])
            .expect_err("fixture must reject this invalid operation"),
        CarrierError::StaleActorGeneration
    );
    assert!(matches!(
        r.carrier.slots[i],
        Slot::CreatureOccupied { health: 5, .. }
    ));
}
