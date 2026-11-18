//! The Edron chamber's actual switches use the PostgreSQL-backed game command owner.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::ChannelRuntimeV1;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::world_callbacks;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn espher2_lord_switch_use_closes_its_wall_without_starting_early_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("espher2_lord_switch", || {
        run(async |admin| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "espher2_lord_switch", true, false)
                .await?;
            prepare_current_session(&h).await?;
            let callback = world_callbacks::load()?
                .into_iter()
                .find(|row| row.key == "oteryn:quest-callback/elemental-spheres/lord-switch-1")
                .ok_or("Lord chamber switch must be registered")?;
            let encounter = callback.effects.encounter.as_ref().ok_or("encounter")?;
            let station = &encounter.switches.as_ref().ok_or("four switches")?.stations[0];
            let world_tile = |p: world_callbacks::Position| TilePos {
                x: p.x,
                y: p.y,
                floor: p.floor,
            };
            let start = world_tile(station.stand);
            let mut deps = Dependencies::new()?;
            let (controller, activated) =
                super::death_ml_loss_composed::production_generation_with_pin(
                    deps.admitted.world_id,
                    "content-1",
                )?;
            let generation = controller.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native profiles")?;
            let world = super::source_world_crop::world(
                world_tile(encounter.room_min),
                TilePos {
                    x: 33279,
                    ..world_tile(encounter.room_max)
                },
                start,
                native,
            )?;
            let cells = world
                .movement_cells(activated.qualified_room().movement_cells())
                .map_err(debug)?;
            let frame = world.frame_binding_digest(deps.admitted.world_id);
            let room = activated
                .qualified_room()
                .bundle_view(
                    cells.clone(),
                    frame,
                    world.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: i32::from(start.x),
                        y: i32::from(start.y),
                        floor: 12,
                    },
                )
                .ok_or("chamber room")?;
            let pin = crate::content::channel_content_pin::ChannelContentPin::from_activation(
                deps.admitted.world_id,
                1,
                generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(),
                frame,
                world.map_revision_digest(),
                (i32::from(start.x), i32::from(start.y), 12),
            );
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                deps.admitted.world_id,
                deps.admitted.channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                64,
                pin,
            )
            .map_err(debug)?;
            let session = deps.admitted.game_session_id;
            let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            deps.runtime = tokio::sync::Mutex::new(runtime);
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&room), Some(&room));
            let mut admitted = deps.admitted;
            admitted.runtime_actor = Some(deps.actor);
            admitted.first_entry = FirstEntryOutcome::Positioned;
            let fence = owner
                .current_quest_fence(session)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            admitted.item_fence = Some(fence.item_fence());
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            let map = owner.bundle_map().ok_or("map")?;
            let at = world_tile(station.lever);
            let key = (0..=8)
                .filter_map(|ordinal| crate::map::view::placement_key(at, ordinal))
                .find(|key| {
                    map.facts
                        .quest_callback_matches(*key, None, None, Some(2772))
                })
                .ok_or("source lever placement")?;
            let mut placement = world.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            let walls: Vec<_> = station
                .walls
                .map(world_tile)
                .into_iter()
                .filter(|wall| {
                    !map.facts
                        .quest_callback_blocked(map.overlay.base(), *wall, false)
                })
                .collect();
            assert!(
                !walls.is_empty(),
                "at least one door tile is initially open"
            );
            let outcome = owner
                .use_object(
                    deps.actor,
                    UseCommand {
                        game_session_id: session,
                        command_id: 1,
                        item_fence: admitted.item_fence,
                    },
                    WorldObjectTarget {
                        placement,
                        expected_revision: 0,
                    },
                )
                .await;
            assert_eq!(outcome.disposition, UseDisposition::Committed);
            assert_eq!(map.facts.object_revision(key), 1);
            assert!(
                walls
                    .into_iter()
                    .all(|wall| map
                        .facts
                        .quest_callback_blocked(map.overlay.base(), wall, false))
            );
            owner.drain_quest_encounter_deadlines().await;
            assert!(
                owner
                    .runtime
                    .lock()
                    .await
                    .current_live_creature_census()
                    .map_err(debug)?
                    .is_empty()
            );
            Ok(())
        })
    })
}
