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
    chamber(false)
}

#[test]
fn espher2_lord_two_players_start_and_exit_at_timeout_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    chamber(true)
}

fn chamber(two_players: bool) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("espher2_lord_switch", move || {
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
            // The compiler loads all floors of the region. The actual USE and
            // relocation assertions below qualify the other physical cells.
            let world = super::source_world_crop::compiled_region(start, start, start, native)?;
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
            let evidence = if two_players {
                Some(super::house_transfer::RecoverySource::start(&h).await?)
            } else {
                None
            };
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            if let Some(source) = &evidence {
                owner.evidence = &source.evidence;
            }
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
            let key = source_lever(map, at)?;
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
            if let Some(source) = &evidence {
                let character =
                    super::party_peer_bootstrap::create_peer(&h, &authority, &source.evidence, 0)
                        .await?;
                let peer = super::activity_deadline_paging::admit_created_member(
                    &h,
                    &source.evidence,
                    0,
                    character,
                )
                .await?;
                let group = encounter.switches.as_ref().ok_or("switch group")?;
                let peer_actor = {
                    let mut runtime = owner.runtime.lock().await;
                    let reservation = runtime.reserve_fresh_session(peer).map_err(debug)?;
                    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                    runtime
                        .initialize_pinned_test_position(actor, local(group.stations[1].stand))
                        .map_err(debug)?;
                    actor
                };
                let mut peer_admission = admitted;
                peer_admission.game_session_id = peer;
                peer_admission.runtime_actor = Some(peer_actor);
                owner.admit_progression_session(&mut peer_admission).await;
                deps.initialize_player(&owner, peer_actor, peer).await;
                let peer_fence = owner
                    .current_quest_fence(peer)
                    .await
                    .map_err(|()| "peer fence")?
                    .ok_or("peer fence")?;
                for (actor, session, fence, station, command_id) in [
                    (
                        peer_actor,
                        peer,
                        peer_fence.item_fence(),
                        &group.stations[1],
                        1,
                    ),
                    (
                        deps.actor,
                        session,
                        fence.item_fence(),
                        &group.stations[2],
                        2,
                    ),
                    (
                        peer_actor,
                        peer,
                        peer_fence.item_fence(),
                        &group.stations[3],
                        2,
                    ),
                ] {
                    let closed = crate::map::floor::BundleDoors(&map.facts);
                    let mut runtime = owner.runtime.lock().await;
                    let before = runtime.read_actor_position(actor).map_err(debug)?;
                    if before.position() != local(station.stand) {
                        let proof = crate::map::floor::prepare_quest_encounter_ejection(
                            &runtime,
                            &owner.movement_cells,
                            actor,
                            session,
                            before,
                            local(station.stand),
                            &closed,
                        )
                        .map_err(debug)?;
                        runtime.commit_source_step(proof).map_err(debug)?;
                    }
                    drop(runtime);
                    let key = source_lever(map, world_tile(station.lever))?;
                    let mut placement = world.base().digest().to_vec();
                    placement.extend_from_slice(&key.to_be_bytes());
                    // The ordinary USE gate measures the real Channel clock.
                    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
                    let outcome = owner
                        .use_object(
                            actor,
                            UseCommand {
                                game_session_id: session,
                                command_id,
                                item_fence: Some(fence),
                            },
                            WorldObjectTarget {
                                placement,
                                expected_revision: 0,
                            },
                        )
                        .await;
                    assert_eq!(
                        outcome.disposition,
                        UseDisposition::Committed,
                        "switch {:?}",
                        station.lever
                    );
                }
                assert!(
                    owner
                        .states
                        .quest_callbacks
                        .next_encounter_due_us()
                        .is_some()
                );
                owner.states.test_owner_time.store(
                    owner.owner_now().get() + 1_000_000,
                    std::sync::atomic::Ordering::Relaxed,
                );
                owner.drain_quest_encounter_deadlines().await;
                assert_eq!(
                    owner
                        .runtime
                        .lock()
                        .await
                        .current_live_creature_census()
                        .map_err(debug)?
                        .len(),
                    1,
                    "all four switches must summon the Lord before ejection"
                );
                owner.states.test_owner_time.store(
                    owner.owner_now().get() + 601_000_001,
                    std::sync::atomic::Ordering::Relaxed,
                );
                owner.drain_quest_encounter_deadlines().await;
                let runtime = owner.runtime.lock().await;
                assert!(
                    runtime
                        .current_live_creature_census()
                        .map_err(debug)?
                        .is_empty(),
                    "timeout must remove the Lord it summoned"
                );
                for actor in [deps.actor, peer_actor] {
                    assert_eq!(
                        runtime
                            .read_actor_position(actor)
                            .map_err(debug)?
                            .position(),
                        local(group.exit)
                    );
                }
                assert!(
                    owner
                        .states
                        .quest_callbacks
                        .next_encounter_due_us()
                        .is_none()
                );
            }
            Ok(())
        })
    })
}

fn local(position: world_callbacks::Position) -> crate::foundation::MovementLocalPosition {
    crate::foundation::MovementLocalPosition {
        x: i32::from(position.x),
        y: i32::from(position.y),
        floor: -i16::from(position.floor),
    }
}

fn source_lever(map: &crate::map::boot::BundleMap, at: TilePos) -> TestResult<u64> {
    let tile = map
        .overlay
        .base()
        .tile(at.x, at.y, at.floor)
        .ok_or("switch tile")?;
    tile.ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .find_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            let facts = map.facts.base_entry(at, ordinal, *id)?;
            (facts.appearance_id == 2772)
                .then(|| crate::map::view::placement_key(at, ordinal))
                .flatten()
        })
        .ok_or_else(|| "source lever placement".into())
}
