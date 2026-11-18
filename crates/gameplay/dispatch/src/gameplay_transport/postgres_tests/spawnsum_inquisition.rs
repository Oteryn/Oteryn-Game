use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult};
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, MovementLocalPosition, NodeId,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::map::{
    overlay::TilePos,
    view::{EntrySource, MapFacts, compose_tile, placement_key},
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
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

const TRACK: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline";
const QUEST: &str = "oteryn:quest.the_inquisition_quest";

#[derive(Clone, Copy)]
enum Scene {
    HauntedRuin,
    ShadowNexus,
    VampireHunt,
}

impl Scene {
    fn crop(self) -> (world::NativeCrop, TilePos, u8, &'static [u16]) {
        match self {
            Self::HauntedRuin => (
                world::NativeCrop {
                    bytes: include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../content/world/placements/region-z07-x126-y128.b3"
                    )),
                    region: (7, 126, 128),
                    bounds: (32254, 32788, 32262, 32794),
                    start: (32256, 32789),
                    expected: vec![],
                },
                TilePos {
                    x: 32256,
                    y: 32790,
                    floor: -7,
                },
                0,
                &[7868],
            ),
            Self::VampireHunt => (
                world::NativeCrop {
                    bytes: include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../content/world/placements/region-z09-x128-y124.b3"
                    )),
                    region: (9, 128, 124),
                    bounds: (32776, 31976, 32783, 31984),
                    start: (32779, 31978),
                    expected: vec![],
                },
                TilePos {
                    x: 32779,
                    y: 31979,
                    floor: -9,
                },
                0,
                &[],
            ),
            Self::ShadowNexus => (
                world::NativeCrop {
                    bytes: include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../content/world/placements/region-z12-x129-y123.b3"
                    )),
                    region: (12, 129, 123),
                    bounds: (33112, 31700, 33118, 31705),
                    start: (33115, 31703),
                    expected: vec![],
                },
                NEXUS,
                3,
                &[7925, 7926, 7927, 7928, 7929, 7930, 7931],
            ),
        }
    }

    fn stage(self) -> Option<i64> {
        match self {
            Self::HauntedRuin => Some(12),
            Self::VampireHunt => Some(8),
            Self::ShadowNexus => None,
        }
    }
}

#[test]
fn spawnsum_inquisition_haunted_ruin_consumes_water_and_temporarily_closes_door() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, "spawnsum_haunted_ruin", Scene::HauntedRuin)
}

#[test]
fn spawnsum_inquisition_vampire_hunt_use_spawns_the_count() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, "spawnsum_vampire_hunt", Scene::VampireHunt)
}

#[test]
fn spawnsum_inquisition_holy_water_use_advances_shared_decay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, "spawnsum_inquisition", Scene::ShadowNexus)
}

fn run_case(admin: String, case: &'static str, scene: Scene) -> TestResult {
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                super::reset();
                let h = Harness::create_with_admission_seed(admin, case, false, false).await?;
                let result = exercise(&h, scene).await;
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

async fn exercise(h: &Harness, scene: Scene) -> TestResult {
    super::prepare_current_session(h).await?;
    super::item_look::seed_main_backpack(h, &[(WATER, 1)]).await?;
    if let Some(stage) = scene.stage() {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        for advance in [
            "UPDATE game_character_roots SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid AND character_revision=1",
            "UPDATE game_character_progression_state SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid AND character_revision=1",
        ] {
            sqlx::query(advance)
                .bind(id(41).as_slice())
                .execute(&mut *tx)
                .await?;
        }
        let seeded = sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND character_revision=2")
            .bind(id(41).as_slice()).bind(TRACK).bind(QUEST).bind(stage)
            .execute(&mut *tx).await?;
        assert_eq!(seeded.rows_affected(), 1);
        tx.commit().await?;
    }
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
    let (crop, target, ordinal, appearances) = scene.crop();
    let start = MovementLocalPosition {
        x: i32::from(crop.start.0),
        y: i32::from(crop.start.1),
        floor: i16::from(crop.region.0),
    };
    let world = world::native_scene_with_appearances(native, vec![crop], &[], appearances)?;
    let cells = world
        .movement_cells(pin.qualified_room().movement_cells())
        .map_err(debug)?;
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
        8,
        content_pin,
    )
    .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
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
    owner.quest_catalogue = Some(std::sync::Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
    owner.movement_cells = (&cells).into();
    deps.initialize(&owner).await;

    let map = owner.bundle_map().ok_or("bundle")?;
    let key = placement_key(target, ordinal).ok_or("source placement")?;
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
    let lease = CharacterLease::new(
        CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
        fence.character_lease_generation,
    )
    .map_err(debug)?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(deps.actor, session, lease)
        .map_err(debug)?;
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
    let assert_water_consumed = || async {
        let inventory = h
            .root
            .read_item_use_inventory(&authority, &h.node, fence)
            .await
            .map_err(debug)?;
        assert!(
            !inventory
                .backpack
                .iter()
                .flat_map(|bag| &bag.entries)
                .any(|entry| { entry.item.definition.production_key == WATER })
        );
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    if let Some(stage) = scene.stage() {
        owner.states.test_owner_time.store(101, Ordering::Relaxed);
        let (disposition, creature) = match scene {
            Scene::HauntedRuin => (
                use_water(1)
                    .await
                    .map_err(debug)?
                    .ok_or("holy water USE route")?
                    .disposition,
                "oteryn:creature.pirate_ghost",
            ),
            Scene::VampireHunt => (
                owner
                    .use_world_object(
                        deps.actor,
                        UseCommand {
                            game_session_id: session,
                            command_id: 1,
                            item_fence: Some(fence),
                        },
                        WorldObjectTarget {
                            placement: placement.clone(),
                            expected_revision: map.facts.object_revision(key),
                        },
                    )
                    .await
                    .disposition,
                "oteryn:creature.the_count",
            ),
            Scene::ShadowNexus => return Err("Nexus is exercised separately".into()),
        };
        assert_eq!(disposition, UseDisposition::Committed);
        let runtime = owner.runtime.lock().await;
        assert!(
            runtime
                .positioned_melee_census()
                .map_err(debug)?
                .iter()
                .any(|(actor, _, session)| {
                    session.is_none()
                        && runtime.matches_live_creature_identity(*actor, creature.as_bytes())
                })
        );
        drop(runtime);
        let state = h
            .root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(state.tracks().get(TRACK), Some(&(stage + 1)));
        if matches!(scene, Scene::HauntedRuin) {
            assert_water_consumed().await?;
            let door = TilePos {
                x: 32260,
                y: 32791,
                floor: -7,
            };
            let appearance = || {
                compose_tile(&map.overlay, &map.facts, door)
                    .ok()
                    .flatten()
                    .and_then(|tile| tile.entries.get(1).map(|entry| entry.facts.appearance_id))
            };
            assert_eq!(appearance(), Some(7868));
            assert!(
                map.facts
                    .quest_callback_blocked(map.overlay.base(), door, false)
            );

            assert_eq!(map.facts.expire_movement_items(10_000_100), 1);
            assert_eq!(appearance(), Some(7869));
        }
        return Ok(());
    }
    let appearance = || {
        compose_tile(&map.overlay, &map.facts, NEXUS).ok().flatten()?.entries
            .into_iter().find_map(|entry| {
                matches!(entry.source, EntrySource::Base { placement_key, .. } if placement_key == key)
                    .then_some(entry.facts.appearance_id)
            })
    };
    owner.states.test_owner_time.store(101, Ordering::Relaxed);
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
        .store(30_000_102, Ordering::Relaxed);
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
    owner
        .states
        .test_owner_time
        .store(60_000_103, Ordering::Relaxed);
    assert_eq!(
        use_water(4)
            .await
            .map_err(debug)?
            .ok_or("USE not routed")?
            .disposition,
        UseDisposition::Committed
    );
    assert_eq!(appearance(), Some(7930));
    assert_eq!(map.facts.expire_movement_items(90_000_102), 1);
    assert_eq!(appearance(), Some(7931));
    owner
        .states
        .test_owner_time
        .store(90_000_104, Ordering::Relaxed);
    assert_eq!(
        use_water(5)
            .await
            .map_err(debug)?
            .ok_or("USE not routed")?
            .disposition,
        UseDisposition::Committed
    );
    let saved = h
        .root
        .read_character_quest_state(&authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(
        saved
            .tracks()
            .get("oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline"),
        Some(&22)
    );
    assert_eq!(
        saved
            .tracks()
            .get("oteryn:quest-progress/quest/u8_2/the_inquisition_quest/mission07"),
        Some(&2)
    );
    assert_water_consumed().await?;
    assert_eq!(map.facts.expire_movement_items(110_000_102), 1);
    assert_eq!(appearance(), Some(7925));
    Ok(())
}
