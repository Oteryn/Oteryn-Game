//! MAINRED-NB: measure the real held pass over a channel-sized idle census.
use super::*;
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_durability::durability::native_world_read::qualified_combat_tile;
use oteryn_gameplay_entities::carrier::{NativeRaidSpawnSpec, RaidSpawnIdentity, RaidSpawnOutcome};

#[test]
fn source_owner_cycle_drives_monsters_without_connection_vitals() -> TestResult {
    run_case(async |harness| {
        prepare_current_session(harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
        let active = controller.active().ok_or("active generation")?;
        let (deps, nearby) = dependencies(active, &pin)?;
        let mut owner = deps.owner(harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = deps.room.movement_cells().into();
        owner.clock_origin -= std::time::Duration::from_secs(10);
        let stop = oteryn_foundation::CancellationToken::new();
        let observe = async {
            loop {
                if owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .monster_melee
                    .creature_ai
                    .last_report_for_test(nearby)
                    .is_some()
                {
                    stop.cancel();
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        };
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            super::super::fixture::join_two(owner.run_source_owner_cycles(&stop), observe),
        )
        .await
        .map_err(|_| "source owner did not think and stop without a connection vitals tick")?;
        Ok(())
    })
}

#[test]
fn monster_cadence_admits_only_nearby_creatures_in_a_spread_world() -> TestResult {
    run_case(async |harness| {
        prepare_current_session(harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let (deps, nearby) = spread_dependencies(active, &pin)?;
        let mut owner = deps.owner(harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = deps.room.movement_cells().into();
        owner.clock_origin -= std::time::Duration::from_secs(10);
        // Drive the actual connection cadence's monster owner, first with one creature,
        // then with the same local area and 511 creatures in distant World chunks.
        owner.drain_monster_melee().await;
        assert_eq!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .monster_melee
                .creature_ai
                .len(),
            1
        );
        let mut runtime = deps.runtime.lock().await;
        let policy = runtime
            .companion_snapshot(nearby)
            .map_err(debug)?
            .state
            .policy;
        let spec = NativeRaidSpawnSpec::qualified(
            &policy.definition_key,
            &policy.definition_revision,
            native.source_digest(),
            false,
        )
        .map_err(debug)?;
        let objects = deps.door.lock().await;
        let mut distant = Vec::new();
        for index in 1..512_u16 {
            let stamp = runtime.issue_owner_work().map_err(debug)?;
            let outcome = runtime
                .realize_raid_spawn(
                    stamp,
                    RaidSpawnIdentity {
                        firing_id: *world.as_bytes(),
                        wave: 0,
                        entry: 0,
                        index,
                    },
                    &spec,
                    Some(MovementLocalPosition {
                        x: i32::from(index) * 16 + 1,
                        y: 1,
                        floor: 7,
                    }),
                    |runtime, at| {
                        Some(
                            qualified_combat_tile(&deps.room, runtime, &*objects, at).is_ok_and(
                                |tile| {
                                    tile.ground_present()
                                        && !tile.block_solid()
                                        && !tile.floor_change()
                                },
                            ),
                        )
                    },
                )
                .map_err(debug)?;
            let RaidSpawnOutcome::Placed(actor) = outcome.outcome else {
                return Err("distant creature placement refused".into());
            };
            distant.push(actor);
        }
        drop(objects);
        drop(runtime);
        owner.clock_origin -= std::time::Duration::from_secs(1);
        owner.drain_monster_melee().await;
        let states = owner.states.spell_states.lock().await;
        assert_eq!(distant.len(), 511);
        assert_eq!(
            states.monster_melee.creature_ai.len(),
            1,
            "far-away monsters must not enter the connection's think owner"
        );
        assert!(states.monster_melee.creature_ai.state(nearby).is_some());
        assert!(
            distant
                .iter()
                .all(|actor| states.monster_melee.creature_ai.state(*actor).is_none())
        );
        Ok(())
    })
}

#[test]
fn live_join_visibility_projects_only_the_observers_interest_area_postgres() -> TestResult {
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    use crate::gameplay_transport::world_spatial::SessionVisibility;

    run_case(async |harness| {
        prepare_current_session(harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
        let active = controller.active().ok_or("active generation")?;
        let (deps, rat) = spread_dependencies(active, &pin)?;
        let mut owner = deps.owner(harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = deps.room.movement_cells().into();
        let near = owner
            .observe_visible_entities(deps.actor)
            .await
            .ok_or("near visibility")?;
        assert_eq!(near.entities.len(), 2);
        let mut view = SessionVisibility::default();
        let snapshot = view.snapshot(&near, &mut |_| Ok(())).map_err(debug)?;
        assert_eq!(snapshot.entities.len(), 2);
        {
            let mut runtime = deps.runtime.lock().await;
            let position = runtime.read_actor_position(rat).map_err(debug)?;
            runtime
                .borrow_movement_position()
                .commit_creature_relocation(
                    rat,
                    position,
                    MovementLocalPosition {
                        x: 4096,
                        y: 1,
                        floor: 7,
                    },
                )
                .map_err(debug)?;
        }
        let far = owner
            .observe_visible_entities(deps.actor)
            .await
            .ok_or("far visibility")?;
        let snapshot = SessionVisibility::default()
            .snapshot(&far, &mut |_| Ok(()))
            .map_err(debug)?;
        assert_eq!(snapshot.entities.len(), 1, "wire interest remains local");
        assert_eq!(
            far.entities.len(),
            1,
            "distant monsters must not incur health, facing and condition projection on every connection"
        );
        eprintln!("PG_EXECUTED live_join_visibility local=2 distant_projection=0 wire=1");
        Ok(())
    })
}

#[test]
fn monster_pass_cost_idle_channel_does_not_query_every_empty_tile() -> TestResult {
    pass_cost(false, false)
}

#[test]
fn monster_pass_cost_spread_channel_batches_exact_empty_tiles() -> TestResult {
    pass_cost(true, false)
}

#[test]
fn monster_pass_cost_spread_passive_census_batches_field_reads() -> TestResult {
    pass_cost(true, true)
}

fn pass_cost(spread: bool, passive: bool) -> TestResult {
    run_case(async move |harness| {
        prepare_current_session(harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let (deps, rat) = if spread {
            spread_dependencies(active, &pin)?
        } else {
            super::fixture::dependencies_with_capacity(active, &pin, 1024)?
        };
        let mut owner = deps.owner(harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = deps.room.movement_cells().into();
        let permit = owner.spell_lane_permit().await.ok_or("lane")?;
        let mut runtime = deps.runtime.lock().await;
        let objects = deps.door.lock().await;
        let policy = if passive {
            runtime
                .companion_policy("oteryn:creature.deer")
                .map_err(debug)?
        } else {
            runtime.companion_snapshot(rat).map_err(debug)?.state.policy
        };
        let spec = NativeRaidSpawnSpec::qualified(
            &policy.definition_key,
            &policy.definition_revision,
            native.source_digest(),
            false,
        )
        .map_err(debug)?;
        let mut actors = vec![rat];
        let positions = if spread {
            // One real native creature per spatial chunk. A dense 32x32 fixture
            // hides the node's thousands of independent 16x16 empty-cell reads.
            (1..512)
                .map(|index| MovementLocalPosition {
                    x: index * 16 + 1,
                    y: 1,
                    floor: 7,
                })
                .collect::<Vec<_>>()
        } else {
            (-14..18)
                .flat_map(|x| (-15..17).map(move |y| MovementLocalPosition { x, y, floor: -7 }))
                .collect()
        };
        for at in positions {
            let admits = |runtime: &crate::foundation::ChannelRuntimeV1, at| {
                Some(
                    qualified_combat_tile(&deps.room, runtime, &*objects, at).is_ok_and(|tile| {
                        tile.ground_present() && !tile.block_solid() && !tile.floor_change()
                    }),
                )
            };
            if admits(&runtime, at) != Some(true)
                || runtime.position_occupied_by_other(rat, at).map_err(debug)?
                || runtime.read_actor_position(rat).map_err(debug)?.position() == at
            {
                continue;
            }
            let stamp = runtime.issue_owner_work().map_err(debug)?;
            let receipt = runtime
                .realize_raid_spawn(
                    stamp,
                    RaidSpawnIdentity {
                        firing_id: *world.as_bytes(),
                        wave: 0,
                        entry: 0,
                        index: u16::try_from(actors.len())?,
                    },
                    &spec,
                    Some(at),
                    admits,
                )
                .map_err(debug)?;
            let RaidSpawnOutcome::Placed(actor) = receipt.outcome else {
                return Err("qualified creature placement refused".into());
            };
            actors.push(actor);
            if actors.len() == 512 {
                break;
            }
        }
        assert_eq!(
            actors.len(),
            512,
            "real source-qualified positioned creatures"
        );
        drop(objects);
        let mut states = owner.states.spell_states.lock().await;
        for actor in &actors {
            let policy = runtime
                .companion_snapshot(*actor)
                .map_err(debug)?
                .state
                .policy;
            let creature = crate::content::ProjectV2DefinitionRef {
                family: crate::content::ProjectV2Family::Creature,
                key: policy.definition_key.clone(),
                revision: policy.definition_revision.clone(),
            };
            let admission = states
                .monster_melee
                .creature_ai
                .admit_native_creature(&runtime, native, *actor, &creature, 0);
            if passive && *actor != rat {
                assert_eq!(creature.key, "oteryn:creature.deer");
                assert_eq!(
                    admission,
                    Err(crate::ai_think::behaviour_profile::ProfileRefusal::Invalid)
                );
                assert!(states.monster_melee.creature_ai.state(*actor).is_none());
            } else {
                admission.map_err(debug)?;
                assert!(states.monster_melee.creature_ai.state(*actor).is_some());
            }
        }
        let mut attack = owner.states.attack.lock().await;
        let revisions = crate::ability::RevisionSet::new(
            "rules-1",
            "content-1",
            "world-1",
            "formula-1",
            "sim-1",
        )
        .map_err(debug)?;
        for now in [10_000_000, 10_050_000] {
            let before = ground_scans(owner.root).await?;
            let started = std::time::Instant::now();
            owner
                .drive_monster_profiles(
                    &mut runtime,
                    &mut states,
                    &mut attack,
                    &deps.room,
                    native,
                    &permit,
                    &revisions,
                    &[],
                    now,
                )
                .await
                .map_err(debug)?;
            let elapsed = started.elapsed();
            let scans = ground_scans(owner.root).await? - before;
            eprintln!(
                "MONSTER-PASS-COST spread={spread} passive={passive} creatures={} now={now} elapsed_ms={} ground_scans={scans}",
                actors.len(),
                elapsed.as_millis()
            );
            assert!(
                scans <= 64,
                "held monster pass scanned Ground {scans} times in {elapsed:?}"
            );
        }
        assert!(
            actors
                .iter()
                .all(|actor| runtime.contains_live_creature(*actor))
        );
        Ok(())
    })
}

// The existing bundle fixture supplies a thin walkable World, with the same
// native Content and owner pins as production. Terrain is explicit test DATA.
fn spread_dependencies(
    active: &crate::content::ActiveGeneration,
    pin: &crate::content::NativeEntryContentPin,
) -> TestResult<(Dependencies, ExactActorRef)> {
    use crate::content::channel_content_pin::ChannelContentPin;
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    let mut deps = Dependencies::new()?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let entry = pin.qualified_room();
    let world = crate::map::spell_tiles::tests::movement_arena::rectangle((0, 0), (8191, 2), -7);
    let cells = world
        .movement_cells(entry.movement_cells())
        .map_err(debug)?;
    let room = entry
        .bundle_view(
            cells,
            world.frame_binding_digest(deps.admitted.world_id),
            world.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: 0,
                y: 0,
                floor: 7,
            },
        )
        .ok_or("spread bundle combat view")?;
    let channel_pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        pin.activation_sequence(),
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (0, 0, 7),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        1024,
        channel_pin,
    )
    .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let policy = runtime
        .companion_policy("oteryn:creature.rat")
        .map_err(debug)?;
    let spec = NativeRaidSpawnSpec::qualified(
        &policy.definition_key,
        &policy.definition_revision,
        native.source_digest(),
        false,
    )
    .map_err(debug)?;
    let stamp = runtime.issue_owner_work().map_err(debug)?;
    let objects = deps.door.get_mut();
    let receipt = runtime
        .realize_raid_spawn(
            stamp,
            RaidSpawnIdentity {
                firing_id: *deps.admitted.world_id.as_bytes(),
                wave: 0,
                entry: 0,
                index: 0,
            },
            &spec,
            Some(MovementLocalPosition {
                x: 1,
                y: 1,
                floor: 7,
            }),
            |runtime, at| {
                Some(
                    qualified_combat_tile(&room, runtime, objects, at).is_ok_and(|tile| {
                        tile.ground_present() && !tile.block_solid() && !tile.floor_change()
                    }),
                )
            },
        )
        .map_err(debug)?;
    let RaidSpawnOutcome::Placed(rat) = receipt.outcome else {
        return Err("spread first creature placement refused".into());
    };
    deps.room = room;
    deps.combat_room = Some(deps.room.clone());
    deps.runtime = tokio::sync::Mutex::new(runtime);
    Ok((deps, rat))
}

async fn ground_scans(root: &crate::durability::DurabilityRoot) -> TestResult<i64> {
    root.try_issue_semantic_pass().map_err(debug)?.run(|holder, _| {
        Box::pin(async move {
            // The root has one semantic connection. Flush its previous pass's counters,
            // then read in a separate autocommit statement (no cached stats snapshot).
            sqlx::query("SELECT pg_stat_force_next_flush()")
                .execute(&mut **holder).await?;
            let scans = sqlx::query_scalar::<_, i64>(
                "SELECT seq_scan + COALESCE(idx_scan,0) FROM pg_stat_user_tables WHERE relname='game_item_ground_locations'",
            ).fetch_one(&mut **holder).await?;
            Ok(scans)
        })
    }).await.map_err(debug)
}
