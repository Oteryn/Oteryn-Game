//! Node activation and transport owner-cycle integration.
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
/// End to end through the activation path: the bundle's spawn family is realized by
/// `realize_bundle_spawns` against the production generation's served rat, a real lethal
/// native hit kills the creature, and the owner-turn drive (no session involved) warns at
/// the respawn deadline and admits the replacement 4,200 ms later.
#[test]
#[allow(clippy::expect_used)]
fn a_bundle_spawned_rat_killed_by_a_lethal_hit_respawns_on_the_owner_turn() {
    use crate::foundation::{ChannelContentPin, ChannelId, NodeId, WorldId};
    use oteryn_world_bundle::spawn::{Creature, Direction, Period, Point, Source, Table};
    let world =
        WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]).expect("world");
    let channel = ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
        .expect("channel");
    let node =
        NodeId::decode(&[1, 0, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 3]).expect("node");
    // The production generation on the Thalom source world; its rat is the served policy.
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(
            world,
            Some(include_bytes!(
                "../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            )),
        );
    let active = controller.active().expect("activated generation");
    let native = active.native_gameplay().expect("native gameplay state");
    let room = pin.qualified_room().clone();
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        room.map_revision_digest(),
        (2, 11, -7),
    );
    let mut runtime = crate::foundation::ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        node,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        channel_pin,
    )
    .expect("runtime");
    native
        .install_companion_policies(&mut runtime)
        .expect("policies");

    // One bundle source with one rat point on an open Thalom cell (native floor 7 is z -7).
    let table = Table {
        creatures: vec![Creature {
            key: "oteryn:creature.rat".into(),
            period: Period::All,
        }],
        sources: vec![Source {
            key: "oteryn:spawn/e2e".into(),
            floor: 7,
            x: 1,
            y: 11,
            points: vec![Point {
                creature: 0,
                x: 1,
                y: 11,
                direction: Direction::South,
                respawn_ms: 30_000,
            }],
        }],
    };
    let bundle = crate::map::boot::tests::door_world()
        .expect("bundle world")
        .with_spawns_for_test(table);
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("executor");
    let realized = executor.block_on(crate::node::serve::realize_bundle_spawns(
        &mut runtime,
        &bundle,
        &oteryn_foundation::CancellationToken::new(),
    ));
    assert_eq!(realized.ok(), Some(Some((1, 1))));
    let (point, _, live) = runtime.spawn_point_creatures()[0];
    let rat = live.expect("the realized rat is live");
    let health = runtime
        .companion_policy("oteryn:creature.rat")
        .expect("rat policy")
        .maximum_health;

    // A real lethal native hit (the target identity is read from the slot).
    runtime
        .commit_monster_lab_damage(rat, b"e2e-lethal-hit", health)
        .expect("lethal hit");
    assert!(runtime.spawn_point_creatures()[0].2.is_none());

    crate::gameplay_transport::drive_owner_spawns(&mut runtime, 0);
    let due = runtime.pending_respawn(point).expect("pending").due_micros;
    assert_eq!(due, 30_000_000);
    crate::gameplay_transport::drive_owner_spawns(&mut runtime, due);
    assert!(runtime.spawn_point_creatures()[0].2.is_none());
    crate::gameplay_transport::drive_owner_spawns(&mut runtime, due + 4_200_000);
    let replacement = runtime.spawn_point_creatures()[0].2.expect("respawned rat");
    assert_ne!(replacement, rat);
}
