use crate::runtime_actor_carrier::runtime_actor_source_step::{
    SourceSceneMove, SourceSceneRelocationProof,
};

struct SceneMoves(Vec<SourceSceneMove>);
impl crate::runtime_actor_carrier::runtime_actor_source_step::source_step_seal::Sealed
    for SceneMoves
{
}
impl SourceSceneRelocationProof for SceneMoves {
    fn validate_current(&self, _: &ChannelRuntimeV1) -> Result<(), CarrierError> {
        Ok(())
    }
    fn moves(&self) -> &[SourceSceneMove] {
        &self.0
    }
}
fn scene_batch_actors() -> (ChannelRuntimeV1, ExactActorRef, ExactActorRef) {
    let mut world = runtime(2);
    let reservation = world.reserve_fresh_session(session(801)).expect("reserve");
    let a = world.commit_fresh_session(reservation).expect("actor");
    let reservation = world.reserve_fresh_session(session(802)).expect("reserve");
    let b = world.commit_fresh_session(reservation).expect("actor");
    world
        .initialize_pinned_test_position(
            a,
            MovementLocalPosition {
                x: 1,
                y: 1,
                floor: 7,
            },
        )
        .expect("position");
    world
        .initialize_pinned_test_position(
            b,
            MovementLocalPosition {
                x: 2,
                y: 1,
                floor: 7,
            },
        )
        .expect("position");
    (world, a, b)
}
fn scene_batch_moves(world: &ChannelRuntimeV1, a: ExactActorRef, b: ExactActorRef) -> SceneMoves {
    SceneMoves(
        [a, b]
            .into_iter()
            .map(|actor| SourceSceneMove {
                actor,
                expected: world.read_actor_position(actor).expect("snapshot"),
                destination: MovementLocalPosition {
                    x: 3,
                    y: 2,
                    floor: 7,
                },
            })
            .collect(),
    )
}
#[test]
fn source_scene_batch_late_stale_actor_restores_first_and_never_projects() {
    let (mut world, a, b) = scene_batch_actors();
    let proof = scene_batch_moves(&world, a, b);
    let before_a = world.read_actor_position(a).expect("a");
    let old_b = world.read_actor_position(b).expect("b");
    world
        .borrow_movement_position()
        .commit_turn(old_b, MovementFacing::East)
        .expect("real newer position revision");
    let before_b = world.read_actor_position(b).expect("b");
    let projected = std::cell::Cell::new(false);
    assert!(
        world
            .commit_source_scene_relocations(proof, || {
                projected.set(true);
                true
            })
            .is_err()
    );
    assert!(!projected.get());
    assert_eq!(world.read_actor_position(a), Ok(before_a));
    assert_eq!(world.read_actor_position(b), Ok(before_b));
}
#[test]
fn source_scene_batch_rejected_projection_restores_every_position() {
    let (mut world, a, b) = scene_batch_actors();
    let proof = scene_batch_moves(&world, a, b);
    let before = [
        world.read_actor_position(a).expect("a"),
        world.read_actor_position(b).expect("b"),
    ];
    assert!(
        world
            .commit_source_scene_relocations(proof, || false)
            .is_err()
    );
    assert_eq!(world.read_actor_position(a), Ok(before[0]));
    assert_eq!(world.read_actor_position(b), Ok(before[1]));
}
#[test]
fn source_scene_batch_authorized_shared_landing_commits_all() {
    let (mut world, a, b) = scene_batch_actors();
    let proof = scene_batch_moves(&world, a, b);
    let committed = world
        .commit_source_scene_relocations(proof, || true)
        .expect("one projection");
    assert_eq!(committed.len(), 2);
    assert_eq!(committed[0].position(), committed[1].position());
    assert_eq!(world.read_actor_position(a), Ok(committed[0]));
    assert_eq!(world.read_actor_position(b), Ok(committed[1]));
}

#[test]
fn source_scene_npc_rollback_preserves_identity_and_other_placement_of_same_key() {
    use crate::runtime_actor_carrier::npc_actor::{NpcPlacement, NpcPlacementTable, NpcPlacements};
    let mut world = runtime(2);
    let actors = world
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: "oteryn:npc.scene".into(),
                    placements: vec![
                        NpcPlacement {
                            x: 10,
                            y: 20,
                            floor: -7,
                            direction: MovementFacing::West,
                        },
                        NpcPlacement {
                            x: 10,
                            y: 24,
                            floor: -7,
                            direction: MovementFacing::North,
                        },
                    ],
                }],
            },
            |_| true,
            true,
        )
        .expect("native NPC slots")
        .actors;
    let actor = actors[0];
    let other = actors[1];
    let before = world
        .read_scene_actor_position(actor)
        .expect("NPC snapshot");
    let other_before = world
        .read_scene_actor_position(other)
        .expect("other NPC snapshot");
    let change = SourceSceneMove {
        actor,
        expected: before,
        destination: MovementLocalPosition {
            x: 12,
            y: 20,
            floor: 7,
        },
    };
    assert!(
        world
            .commit_source_scene_relocations(SceneMoves(vec![change]), || false)
            .is_err()
    );
    assert_eq!(world.read_scene_actor_position(actor), Ok(before));
    let moved = world
        .commit_source_scene_relocations(SceneMoves(vec![change]), || true)
        .expect("one scene owner");
    assert_eq!(moved[0].position(), change.destination);
    assert_eq!(moved[0].facing(), before.facing());
    assert_eq!(world.npc_key(actor), Some("oteryn:npc.scene"));
    assert_eq!(world.read_scene_actor_position(other), Ok(other_before));
    assert_eq!(
        world
            .scene_actor_census()
            .expect("complete NPC census")
            .len(),
        2
    );
}
