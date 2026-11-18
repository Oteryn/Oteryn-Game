use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult};
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, GameSessionId, MovementLocalPosition, NodeId,
};
use crate::gameplay_transport::connection::UseCommand;
use crate::map::{
    overlay::TilePos,
    view::{MapFacts, compose_tile, placement_key},
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::atomic::Ordering;

#[path = "l6_world_pickups/world.rs"]
mod world;

const LEVER: TilePos = TilePos {
    x: 33476,
    y: 32700,
    floor: -14,
};
const PORTAL: TilePos = TilePos {
    x: 33477,
    y: 32698,
    floor: -14,
};

#[test]
fn spawnsum_shulgrax_use_checks_flowers_and_grants_individual_timed_access() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                super::reset();
                let h =
                    Harness::create_with_admission_seed(admin, "spawnsum_shulgrax", false, false)
                        .await?;
                let result = exercise(&h).await;
                h.cleanup().await?;
                result
            })
            .map_err(debug)
        })?
        .join()
        .map_err(|_| "Shulgrax PG test panicked")?;
    eprintln!(
        "SHULGRAX_POSTGRES_CASE_COMPLETED duration={:?}",
        started.elapsed()
    );
    result.map_err(Into::into)
}

async fn exercise(h: &Harness) -> TestResult {
    super::prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(
            deps.admitted.world_id,
            None,
        );
    let active = controller.active().ok_or("active content")?;
    let native = active.native_gameplay().ok_or("native item profiles")?;
    let world = world::native_scene_with_appearances(
        native,
        vec![world::NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z14-x130-y127.b3"
            )),
            region: (14, 130, 127),
            bounds: (33455, 32686, 33480, 32709),
            start: (33476, 32701),
            expected: vec![],
        }],
        &[],
        &[3676, 3677, 3678, 5068, 9111],
    )?;
    let cells = world
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let start = MovementLocalPosition {
        x: 33476,
        y: 32701,
        floor: 14,
    };
    let content_pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        pin.qualified_room().map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        content_pin,
    )
    .map_err(debug)?;
    let first = deps.admitted.game_session_id;
    let second = GameSessionId::decode(&id(51)).map_err(debug)?;
    let mut admit = |session, position| -> TestResult<_> {
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(actor, position)
            .map_err(debug)?;
        Ok(actor)
    };
    deps.actor = admit(first, start)?;
    let second_actor = admit(second, MovementLocalPosition { x: 33477, ..start })?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    deps.initialize(&owner).await;
    deps.initialize_player(&owner, second_actor, second).await;

    let map = owner.bundle_map().ok_or("bundle")?;
    let lever = placement_key(LEVER, 2).ok_or("lever")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&lever.to_be_bytes());
    let use_lever = |actor, session, command_id| {
        owner.use_world_object(
            actor,
            UseCommand {
                game_session_id: session,
                command_id,
                item_fence: None,
            },
            WorldObjectTarget {
                placement: placement.clone(),
                expected_revision: map.facts.object_revision(lever),
            },
        )
    };
    let has_item = |pos, id| {
        compose_tile(&map.overlay, &map.facts, pos)
            .ok()
            .flatten()
            .is_some_and(|tile| {
                tile.entries
                    .iter()
                    .any(|entry| entry.facts.appearance_id == id)
            })
    };
    let access = |session, now| {
        owner
            .states
            .quest_callbacks
            .flower_portal_destination(PORTAL, session, now)
    };
    owner.states.test_owner_time.store(101, Ordering::Relaxed);
    assert_eq!(
        use_lever(deps.actor, first, 1).await.disposition,
        UseDisposition::Rejected
    );
    assert!(has_item(LEVER, 9110));
    for (x, y, item) in [
        (33455, 32708, 3676),
        (33459, 32707, 3676),
        (33460, 32708, 3676),
        (33456, 32692, 3678),
        (33458, 32692, 3678),
        (33460, 32693, 3678),
        (33457, 32686, 3677),
        (33459, 32686, 3677),
        (33456, 32687, 3677),
    ] {
        let pos = TilePos { x, y, floor: -14 };
        if !has_item(pos, item) {
            assert!(map.facts.create_movement_item(pos, item, 0, || true));
        }
    }
    assert_eq!(
        use_lever(deps.actor, first, 2).await.disposition,
        UseDisposition::Committed
    );
    assert!(has_item(LEVER, 9111));
    assert!(has_item(PORTAL, 5068));
    let exit = TilePos {
        x: 33436,
        y: 32800,
        floor: -13,
    };
    assert_eq!(access(first, 100), Some(exit));
    assert_ne!(access(second, 100), Some(exit));
    owner
        .states
        .test_owner_time
        .store(1_000_101, Ordering::Relaxed);
    assert_eq!(
        use_lever(second_actor, second, 1).await.disposition,
        UseDisposition::Committed
    );
    assert_eq!(
        use_lever(second_actor, second, 2).await.disposition,
        UseDisposition::Committed
    );
    assert_eq!(access(second, 1_000_100), Some(exit));
    assert_eq!(map.facts.next_movement_decay_time(), Some(30_000_100));
    assert_eq!(map.facts.expire_movement_items(30_000_100), 4);
    assert_ne!(access(first, 30_000_100), Some(exit));
    assert_eq!(access(second, 30_000_100), Some(exit));
    assert_ne!(access(second, 31_000_100), Some(exit));
    assert!(has_item(TilePos { x: 33475, ..PORTAL }, 1635));
    assert!(has_item(TilePos { x: 33476, ..PORTAL }, 7144));
    Ok(())
}
