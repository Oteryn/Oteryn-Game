use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition, NodeId};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::map::{
    overlay::TilePos,
    view::{EntrySource, MapFacts, compose_tile, placement_key},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};
use std::sync::atomic::Ordering;

#[path = "l6_world_pickups/world.rs"]
mod world;

const NEXUS: TilePos = TilePos {
    x: 33115,
    y: 31702,
    floor: -12,
};
const WATER: &str = "oteryn:item.tibia.i133";

#[test]
fn spawnsum_inquisition_holy_water_use_advances_shared_decay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                super::reset();
                let h = Harness::create_with_admission_seed(
                    admin,
                    "spawnsum_inquisition",
                    false,
                    false,
                )
                .await?;
                let result = exercise(&h).await;
                h.cleanup().await?;
                result
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Inquisition PG test panicked")?;
    eprintln!(
        "INQUISITION_POSTGRES_CASE_COMPLETED duration={:?}",
        started.elapsed()
    );
    result.map_err(Into::into)
}

async fn exercise(h: &Harness) -> TestResult {
    super::prepare_current_session(h).await?;
    super::item_look::seed_main_backpack(h, &[(WATER, 1)]).await?;
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
                "/../../../content/world/placements/region-z12-x129-y123.b3"
            )),
            region: (12, 129, 123),
            bounds: (33112, 31700, 33118, 31705),
            start: (33115, 31703),
            expected: vec![],
        }],
        &[],
        &[7925, 7926, 7927, 7928, 7929, 7930, 7931],
    )?;
    let cells = world
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let start = MovementLocalPosition {
        x: 33115,
        y: 31703,
        floor: 12,
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
        "runtime-scope-assignment:1",
        2,
        content_pin,
    )
    .map_err(debug)?;
    let session = deps.admitted.game_session_id;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(deps.actor, start)
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    deps.initialize(&owner).await;

    let map = owner.bundle_map().ok_or("bundle")?;
    let key = placement_key(NEXUS, 3).ok_or("Nexus placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let item = native
        .item_index()
        .and_then(|index| index.definition_ref("Item", WATER, "definition-r1"))
        .ok_or("holy water definition")?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("current USE fence")?
        .item_fence();
    let use_water = |command_id| {
        owner.tame_mount(
            deps.actor,
            UseCommand {
                game_session_id: session,
                command_id,
                item_fence: Some(fence),
            },
            UseIntent {
                target: UseTarget::ItemByDefinition(item),
                use_with: None,
                use_with_position: None,
                use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                    placement: placement.clone(),
                    expected_revision: map.facts.object_revision(key),
                })),
            },
            None,
            None,
            false,
        )
    };
    let appearance = || {
        compose_tile(&map.overlay, &map.facts, NEXUS).ok().flatten()?.entries
            .into_iter().find_map(|entry| {
                matches!(entry.source, EntrySource::Base { placement_key, .. } if placement_key == key)
                    .then_some(entry.facts.appearance_id)
            })
    };
    owner.states.test_owner_time.store(100, Ordering::Relaxed);
    assert_eq!(
        use_water(1)
            .await
            .map_err(debug)?
            .ok_or("USE not routed")?
            .disposition,
        UseDisposition::Committed
    );
    assert_eq!(appearance(), Some(7926));
    assert_eq!(
        use_water(2)
            .await
            .map_err(debug)?
            .ok_or("USE not routed")?
            .disposition,
        UseDisposition::Rejected
    );
    assert_eq!(map.facts.expire_movement_items(30_000_099), 0);
    assert_eq!(map.facts.expire_movement_items(30_000_100), 1);
    assert_eq!(appearance(), Some(7927));
    owner
        .states
        .test_owner_time
        .store(30_000_101, Ordering::Relaxed);
    assert_eq!(
        use_water(3)
            .await
            .map_err(debug)?
            .ok_or("USE not routed")?
            .disposition,
        UseDisposition::Committed
    );
    assert_eq!(appearance(), Some(7928));
    assert_eq!(map.facts.expire_movement_items(50_000_100), 0);
    assert_eq!(map.facts.expire_movement_items(60_000_101), 1);
    assert_eq!(appearance(), Some(7929));
    assert_eq!(map.facts.expire_movement_items(80_000_101), 1);
    assert_eq!(appearance(), Some(7925));
    Ok(())
}
