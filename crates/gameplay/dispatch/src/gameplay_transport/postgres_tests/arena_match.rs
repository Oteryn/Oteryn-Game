//! SV15: real command-436 connection, production combat, fee and complete backpack MINT.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority,
};
use oteryn_protocol_oteryn::arena::{self as wire, ArenaAction as A, ArenaDisposition as D};
#[path = "arena_match/fixture.rs"]
pub(super) mod fixture;

#[test]
fn arena_match_real_connection_status_and_full_match() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            scenario(false, false).map_err(|e| e.to_string())?;
            scenario(true, false).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "arena match scenario panicked")??;
    eprintln!("M51 POSTGRES_CASE_COMPLETED: 30 real opponents, fee, reward, replay, exit, reload");
    Ok(())
}
#[test]
fn arena_match_credited_portal_timeout_restores_pillar() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true, true).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "arena timeout scenario panicked")??;
    eprintln!("M51 POSTGRES_CASE_COMPLETED credited_portal_timeout duration={:?}", started.elapsed());
    Ok(())
}

async fn command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    action: A,
) -> TestResult<wire::ArenaResult> {
    let id = admitted.continuity.next_command_id;
    let (status, bytes) = super::guild_wire::exchange(
        owner,
        admitted,
        id,
        wire::COMMAND_TYPE_ARENA_MATCH,
        &wire::encode_match_request(action).map_err(debug)?,
    )
    .await?;
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
    wire::decode_match_result(&bytes).map_err(debug)
}
#[test]
fn m51_postgres_native_portals_complete_all_ten_pits_and_reward() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_path(true, true).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "M51 native portal run panicked")??;
    eprintln!(
        "M51 REAL POSTGRES native ten-pit portal run duration={:?}",
        started.elapsed()
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}

#[test]
fn m51_postgres_native_recreated_pillars_complete_all_ten_pits_and_reward() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            scenario_route(true, Some(NativeScenario::RecreatedPillars)).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "M51 recreated pillar worker panicked")??;
    eprintln!(
        "M51 REAL POSTGRES recreated pillars duration={:?}",
        started.elapsed()
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}

fn scenario(funded: bool, expire: bool) -> TestResult {
    scenario_route_with_expiry(funded, None, expire)
}

#[test]
fn m51_postgres_native_departure_cancels_without_a_leave_command() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_route(true, Some(NativeScenario::Departure)).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M51 native departure worker panicked")??;
    eprintln!(
        "M51 REAL POSTGRES native departure duration={:?}",
        started.elapsed()
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}

#[derive(Clone, Copy)]
enum NativeScenario {
    Portals,
    RecreatedPillars,
    Departure,
    Timeout,
    ResetMonsters,
}

#[test]
fn m51_postgres_native_timeout_kicks_and_persists_without_a_status_command() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_route(true, Some(NativeScenario::Timeout)).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M51 native timeout worker panicked")??;
    eprintln!(
        "M51 REAL POSTGRES native timeout duration={:?}",
        started.elapsed()
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}
#[test]
fn m51_postgres_native_entry_resets_only_monsters_inside_source_box() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            scenario_route(true, Some(NativeScenario::ResetMonsters)).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "M51 native reset worker panicked")??;
    eprintln!(
        "M51 REAL POSTGRES native monster reset duration={:?}",
        started.elapsed()
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}

fn scenario_path(funded: bool, native_portals: bool) -> TestResult {
    scenario_route(funded, native_portals.then_some(NativeScenario::Portals))
}
fn scenario_route(funded: bool, route: Option<NativeScenario>) -> TestResult {
    scenario_route_with_expiry(funded, route, false)
}
fn scenario_route_with_expiry(funded: bool, route: Option<NativeScenario>, expire: bool) -> TestResult {
    let native_portals = route.is_some();
    let leave_early = matches!(route, Some(NativeScenario::Departure));
    let timeout_early = matches!(route, Some(NativeScenario::Timeout));
    let reset_monsters = matches!(route, Some(NativeScenario::ResetMonsters));
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "arena_match", false, false).await?;
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let (bytes, pins) = if matches!(route, Some(NativeScenario::RecreatedPillars)) {
            fixture::bundle_with_pillar_floors(native)?
        } else if native_portals {
            fixture::bundle_with_pillars(native)?
        } else {
            fixture::bundle(native)?
        };
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let world = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                bundle: pins,
                map_revision: crate::map::overlay::map_revision(&base),
                start: crate::map::overlay::TilePos {
                    x: 32251,
                    y: 31098,
                    floor: -6,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                let index = native.item_index()?;
                Some(crate::map::facts::ItemDefinition {
                    reference: index.definition_ref("Item", key, "definition-r1")?,
                    solid: Some(key.ends_with(".i1841")),
                    blocks_projectile: false,
                    pickupable: false,
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        fixture::runtime(&mut deps, active, &pin, &world)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        let catalogue = std::sync::Arc::new(oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?.catalogue().clone());
        owner.quest_catalogue = Some(std::sync::Arc::clone(&catalogue));
        {
            let runtime = deps.runtime.lock().await;
            owner
                .states
                .spell_states
                .lock()
                .await
                .initialize(
                    &runtime,
                    deps.actor,
                    deps.admitted.game_session_id,
                    crate::spell::cast::CharacterCastFacts {
                        vocation: crate::spell::Vocation::Knight,
                        magic_level: 0,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now(),
                )
                .ok_or("player spell initialization")?;
        }
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let mut admitted = deps.admitted;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[4, 6, 17, 170],
        )
        .ok_or("capability selection")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(active.identity().server_artifact_digest().as_slice()).bind(active.identity().client_artifact_digest().as_slice()).bind(world.frame_binding_digest(deps.admitted.world_id).as_slice()).execute(&harness.pool).await?;
        let coins = if funded {
            "oteryn:item.tibia.i3043"
        } else {
            "oteryn:item.tibia.i3035"
        };
        super::item_look::seed_main_backpack(&harness, &[(coins, 2)]).await?;
        fixture::equip_sword(&harness).await?;
        owner.admit_progression_session(&mut admitted).await;
        assert!(
            owner
                .progression_binding(admitted.game_session_id)
                .is_some(),
            "production progression binding"
        );
        fixture::train_character(&owner, admitted.game_session_id).await?;
        let build = harness
            .root
            .read_character_build_state(
                &authority,
                admitted.item_fence.ok_or("fence")?.character_id,
            )
            .await?;
        {
            let runtime = deps.runtime.lock().await;
            assert!(owner.states.spell_states.lock().await.load_owned_build(
                &runtime,
                deps.actor,
                admitted.game_session_id,
                &build
            ));
        }
        assert_eq!(
            command(&owner, &mut admitted, A::Status)
                .await?
                .state
                .completed_rank,
            0
        );
        assert_eq!(
            command(&owner, &mut admitted, A::Advance)
                .await?
                .disposition,
            D::WrongState
        );
        let status_payload = wire::encode_match_request(A::Status).map_err(debug)?;
        let mut stale = admitted;
        stale
            .item_fence
            .as_mut()
            .ok_or("item fence")?
            .character_lease_generation += 1;
        assert!(
            owner
                .arena_match_command(stale, admitted.continuity.next_command_id, &status_payload)
                .await
                .is_none()
        );
        let mut no_capability = admitted;
        no_capability.continuity.selected_capabilities = SelectedCapabilities::NONE;
        assert!(
            owner
                .arena_match_command(
                    no_capability,
                    admitted.continuity.next_command_id,
                    &status_payload
                )
                .await
                .is_none()
        );
        if timeout_early {
            owner
                .regression_arena_pit_time(5000)
                .ok_or("fixture timer")?;
        }
        let reset_observations = if reset_monsters {
            let mut runtime = deps.runtime.lock().await;
            let mut spawn = |at| {
                runtime
                    .spawn_arena_opponent("oteryn:creature.frostfur", at, |runtime, at| {
                        Some(crate::gameplay_transport::arena_match::walkable(
                            &owner, runtime, at,
                        ))
                    })
                    .map_err(debug)
            };
            let inside = spawn(crate::foundation::MovementLocalPosition {
                x: 32205,
                y: 31094,
                floor: 7,
            })?;
            let outside = spawn(crate::foundation::MovementLocalPosition {
                x: 32215,
                y: 31098,
                floor: 7,
            })?;
            let npc = runtime
                .place_dynamic_npc(
                    "oteryn:npc.halvar",
                    crate::foundation::NpcPlacement {
                        x: 32206,
                        y: 31094,
                        floor: -7,
                        direction: crate::foundation::MovementFacing::South,
                    },
                )
                .map_err(debug)?;
            Some((inside, outside, npc))
        } else {
            None
        };
        let entry_id = admitted.continuity.next_command_id;
        let entered = command(&owner, &mut admitted, A::Enter).await?;
        if !funded {
            assert_eq!(entered.disposition, D::InsufficientGold);
            assert_eq!(entered.state.active_rank, 0);
            let runtime = deps.runtime.lock().await;
            let at = runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            assert_eq!((at.x, at.y, at.floor), (32251, 31098, 6));
            assert!(runtime.visible_entities().creatures.is_empty());
            eprintln!("SV15 POSTGRES_INSUFFICIENT_FEE_PROOF: no opponent or relocation");
            return Ok(());
        }
        assert_eq!(entered.disposition, D::Applied, "{entered:?}");
        assert_eq!(entered.state.pit, 1);
        let fee_count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=17")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(fee_count, 1);
        let fee: i64 = sqlx::query_scalar(
            "SELECT fee_gold_units FROM game_item_fee_burns WHERE cause_kind=17",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(fee, 1000);
        let replay = owner
            .arena_match_command(
                admitted,
                entry_id,
                &wire::encode_match_request(A::Enter).map_err(debug)?,
            )
            .await
            .ok_or("entry replay")?;
        assert_eq!(wire::decode_match_result(&replay).map_err(debug)?, entered);
        if let Some((inside, outside, npc)) = reset_observations {
            let runtime = deps.runtime.lock().await;
            assert!(
                !runtime.contains_live_creature(inside),
                "entry clears an unregistered Monster inside the source box"
            );
            assert!(
                runtime.contains_live_creature(outside),
                "source reset must preserve Monsters outside its box"
            );
            assert_eq!(
                runtime.npc_key(npc),
                Some("oteryn:npc.halvar"),
                "source reset removes Monsters, never NPCs"
            );
            assert_eq!(
                runtime.visible_entities().creatures.len(),
                2,
                "outside Monster and the newly spawned opponent remain"
            );
            drop(runtime);
            let victories: i64 = sqlx::query_scalar("SELECT count(*) FROM game_arena_victories")
                .fetch_one(&harness.pool)
                .await?;
            assert_eq!(victories, 0, "administrative reset is not a kill source");
            return Ok(());
        }
        if timeout_early {
            let target = {
                let runtime = deps.runtime.lock().await;
                runtime
                    .visible_entities()
                    .creatures
                    .iter()
                    .find(|c| runtime.contains_live_creature(c.actor))
                    .ok_or("live arena opponent")?
                    .actor
            };
            let deadline:i64 = sqlx::query_scalar("SELECT (body->>'deadline_ms')::bigint FROM game_character_arena_matches WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
            sqlx::query("SELECT pg_sleep(GREATEST(0,($1-floor(extract(epoch FROM clock_timestamp())*1000))/1000.0)+0.05)")
                .bind(deadline).execute(&harness.pool).await?;
            let mut stored = (1, 1);
            for _ in 0..100 {
                owner.clock_origin -= std::time::Duration::from_secs(1);
                owner.drain_monster_melee().await;
                stored=sqlx::query_as("SELECT (body->>'rank')::int,(body->>'phase')::int FROM game_character_arena_matches WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
                if stored == (0, 0) {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
            assert_eq!(stored, (0, 0), "silent timeout must persist without Status");
            let leases: i64 = sqlx::query_scalar("SELECT count(*) FROM game_svargrond_pit_leases")
                .fetch_one(&harness.pool)
                .await?;
            assert_eq!(leases, 0);
            let runtime = deps.runtime.lock().await;
            assert!(!runtime.contains_live_creature(target));
            let at = runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            assert_eq!(
                (at.x, at.y, at.floor),
                (32251, 31098, 6),
                "native timer kicks to the source return"
            );
            drop(runtime);
            let native:i64=sqlx::query_scalar("SELECT count(*) FROM game_arena_match_receipts WHERE substring(native_origin_binding FROM 1 FOR $1)=$2")
                .bind(i32::try_from(b"oteryn:svargrond-native-timeout:v1".len())?)
                .bind(b"oteryn:svargrond-native-timeout:v1".as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(native, 1);
            return Ok(());
        }
        if leave_early {
            let target = {
                let mut runtime = deps.runtime.lock().await;
                let target = runtime
                    .visible_entities()
                    .creatures
                    .iter()
                    .find(|c| runtime.contains_live_creature(c.actor))
                    .ok_or("live arena opponent")?
                    .actor;
                let before = runtime.read_actor_position(deps.actor).map_err(debug)?;
                runtime
                    .relocate_arena_player(
                        deps.actor,
                        admitted.game_session_id,
                        before,
                        crate::foundation::MovementLocalPosition {
                            x: 32251,
                            y: 31098,
                            floor: 6,
                        },
                    )
                    .map_err(debug)?;
                target
            };
            owner.clock_origin -= std::time::Duration::from_secs(3);
            owner.drain_monster_melee().await;
            let stored: (i32, i32) = sqlx::query_as("SELECT (body->>'rank')::int,(body->>'phase')::int FROM game_character_arena_matches WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(
                stored,
                (0, 0),
                "native departure must cancel without client Leave"
            );
            let leases: i64 = sqlx::query_scalar("SELECT count(*) FROM game_svargrond_pit_leases")
                .fetch_one(&harness.pool)
                .await?;
            assert_eq!(
                leases, 0,
                "departure releases durable custody before the timer expires"
            );
            assert!(
                !deps.runtime.lock().await.contains_live_creature(target),
                "departure removes its old opponent"
            );
            let native: i64 = sqlx::query_scalar("SELECT count(*) FROM game_arena_match_receipts WHERE substring(native_origin_binding FROM 1 FOR $1)=$2")
                .bind(i32::try_from(b"oteryn:svargrond-native-departure:v1".len())?)
                .bind(b"oteryn:svargrond-native-departure:v1".as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(native, 1);
            let victories: i64 = sqlx::query_scalar("SELECT count(*) FROM game_arena_victories")
                .fetch_one(&harness.pool)
                .await?;
            assert_eq!(
                victories, 0,
                "leaving cannot create a kill or completed rank"
            );
            return Ok(());
        }
        for rank in 1..=3 {
            if rank > 1 {
                assert_eq!(command(&owner, &mut admitted, A::Leave).await?.disposition, D::Applied);
                let next = command(&owner, &mut admitted, A::Enter).await?;
                assert_eq!(next.disposition, D::Applied);
                assert_eq!(next.state.active_rank, rank);
            }
        for pit in 1..=10 {
            assert_eq!(
                command(&owner, &mut admitted, A::Advance)
                    .await?
                    .disposition,
                D::WrongState,
                "client cannot report a kill"
            );
            let target = {
                let runtime = deps.runtime.lock().await;
                runtime
                    .visible_entities()
                    .creatures
                    .iter()
                    .find(|c| runtime.contains_live_creature(c.actor))
                    .ok_or("live opponent")?
                    .actor
            };
            fixture::fight(&mut owner, &mut admitted, deps.actor, target).await?;
            // Exercise the production cadence, including its spell-lane and runtime fencing.
            // A direct arena tick would miss a disconnected scheduler registration.
            owner.drain_monster_melee().await;
            assert!(
                crate::gameplay_transport::arena_match::pit_exit_open(&owner, admitted.game_session_id),
                "credited death opens the source pit without another client command"
            );
            let (x, y) = crate::durability::arena_match::donor::CENTERS[usize::from(pit - 1)];
            owner.clock_origin -= std::time::Duration::from_secs(3);
            let map = owner.bundle_map().ok_or("arena map")?;
            let pos = crate::map::overlay::TilePos { x: (x - 6) as u16, y: y as u16, floor: -7 };
            let mut projected = Vec::new();
            crate::map::view::compose_into(map.overlay.base(), map.overlay.tile(pos), &map.facts, pos, &mut projected).map_err(debug)?;
            assert!(projected.iter().any(|entry| entry.facts.appearance_id == 5022), "credited death creates the native next-pit portal");
            assert!(!projected.iter().any(|entry| entry.facts.appearance_id == 1841), "credited death removes the source pillar");
            if expire {
                crate::gameplay_transport::arena_match::expire_pit(&owner, admitted.game_session_id);
                owner.drain_monster_melee().await;
                {
                    let runtime = deps.runtime.lock().await;
                    let at = runtime.read_actor_position(deps.actor).map_err(debug)?.position();
                    assert_eq!((at.x, at.y, at.floor), crate::durability::arena_match::donor::KICK);
                    assert!(runtime.companion_snapshot_including_dead(target).is_err());
                }
                assert!(!crate::gameplay_transport::arena_match::pit_exit_open(&owner, admitted.game_session_id));
                projected.clear();
                crate::map::view::compose_into(map.overlay.base(), map.overlay.tile(pos), &map.facts, pos, &mut projected).map_err(debug)?;
                assert!(projected.iter().any(|entry| entry.facts.appearance_id == 1841));
                assert!(!projected.iter().any(|entry| entry.facts.appearance_id == 5022));
                return Ok(());
            }
            let start = {
                let mut runtime = deps.runtime.lock().await;
                let before = runtime.read_actor_position(deps.actor).map_err(debug)?;
                runtime.relocate_arena_player(deps.actor, admitted.game_session_id, before,
                    crate::foundation::MovementLocalPosition { x: x - 5, y, floor: 7 }).map_err(debug)?;
                runtime.read_actor_position(deps.actor).map_err(debug)?
            };
            let _ = owner.ordinary_paced_step(deps.actor, admitted.game_session_id,
                oteryn_protocol_oteryn::world_spatial::StepDirection::West, Some(start), None).await;
            let advanced = if native_portals {
                owner.drain_monster_melee().await;
                let stored: (i32, i32) = sqlx::query_as(
                    "SELECT (body->>'pit')::int,(body->>'phase')::int FROM game_character_arena_matches WHERE character_id=encode($1,'hex')::uuid",
                ).bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
                assert_eq!(
                    stored,
                    (i32::from(pit), 2),
                    "credited death must persist without an arena Status command"
                );
                let layout = oteryn_gameplay_quests::quest::barbarian_arena::Pit::new(pit)
                    .ok_or("source pit")?
                    .layout();
                let [x, y, floor] = layout.pillar;
                {
                    let mut runtime = deps.runtime.lock().await;
                    let before = runtime.read_actor_position(deps.actor).map_err(debug)?;
                    runtime
                        .relocate_arena_player(
                            deps.actor,
                            admitted.game_session_id,
                            before,
                            crate::foundation::MovementLocalPosition {
                                x: x + 1,
                                y,
                                floor: -floor as i16,
                            },
                        )
                        .map_err(debug)?;
                }
                let moved = crate::gameplay_transport::connection::regression_routed_step(
                    &owner,
                    admitted,
                    deps.actor,
                    oteryn_protocol_oteryn::world_spatial::StepDirection::West,
                )
                .await?;
                assert_eq!(
                    moved.disposition,
                    oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved,
                    "native portal STEP in pit {pit}"
                );
                let at = deps
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position();
                let expected = if pit == 10 {
                    crate::durability::arena_match::donor::REWARD_ROOM
                } else {
                    let (x, y) = crate::durability::arena_match::donor::CENTERS[usize::from(pit)];
                    (x, y, 7)
                };
                assert_eq!(
                    (at.x, at.y, at.floor),
                    expected,
                    "portal must use its native progression owner"
                );
                command(&owner, &mut admitted, A::Status).await?
            } else {
                command(&owner, &mut admitted, A::Status).await?
            };
            assert_eq!(advanced.disposition, D::Applied, "pit {pit}: {advanced:?}");
            assert_eq!(advanced.state.pit, if pit == 10 { 0 } else { pit + 1 });
            projected.clear();
            crate::map::view::compose_into(map.overlay.base(), map.overlay.tile(pos), &map.facts, pos, &mut projected).map_err(debug)?;
            assert!(projected.iter().any(|entry| entry.facts.appearance_id == 1841), "leaving resets the source pillar");
            assert!(!projected.iter().any(|entry| entry.facts.appearance_id == 5022));

        }
        }
        let victories: i64 = sqlx::query_scalar("SELECT count(*) FROM game_arena_victories")
            .fetch_one(&harness.pool)
            .await?;
        assert_eq!(victories, 30);
        let steps: i64 = sqlx::query_scalar("SELECT count(*) FROM game_arena_match_receipts WHERE command_id > 18446744073709551615")
            .fetch_one(&harness.pool).await?;
        assert_eq!(steps, 30, "native steps have their own durable receipt namespace");
        if native_portals {
            for (prefix, callback) in [
                (b"oteryn:svargrond-native-step:v1".as_slice(), "STEP"),
                (b"oteryn:svargrond-native-death:v1".as_slice(), "DEATH"),
            ] {
                let native_receipts: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM game_arena_match_receipts WHERE substring(native_origin_binding FROM 1 FOR $1)=$2",
                ).bind(i32::try_from(prefix.len())?).bind(prefix).fetch_one(&harness.pool).await?;
                assert_eq!(native_receipts, 10, "one native {callback} receipt per pit");
            }
        }
        let achievements:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/greenhorn'").fetch_one(&harness.pool).await?;
        assert_eq!(achievements, 1);
        let status = command(&owner, &mut admitted, A::Status).await?;
        assert_eq!(status.state.completed_rank, 3);
        assert_eq!(status.state.phase, 3);
        let reward_id = admitted.continuity.next_command_id;
        let claim = A::ClaimReward {
            rank: 1,
            category: 3,
            choice: 1,
        };
        let rewarded = command(&owner, &mut admitted, claim).await?;
        assert_eq!(rewarded.disposition, D::Applied, "{rewarded:?}");
        assert_eq!(rewarded.state.claimed_masks[0], 4);
        let replay = owner
            .arena_match_command(
                admitted,
                reward_id,
                &wire::encode_match_request(claim).map_err(debug)?,
            )
            .await
            .ok_or("reward replay")?;
        assert_eq!(wire::decode_match_result(&replay).map_err(debug)?, rewarded);
        assert_eq!(
            command(
                &owner,
                &mut admitted,
                A::ClaimReward {
                    rank: 1,
                    category: 3,
                    choice: 2
                }
            )
            .await?
            .disposition,
            D::AlreadyClaimed
        );
        let rewards:i64=sqlx::query_scalar("SELECT count(*) FROM game_reward_claims WHERE claim_production_key='oteryn:reward.svargrond-arena.1.3'").fetch_one(&harness.pool).await?;
        assert_eq!(rewards, 1);
        let items:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i7392' AND lifecycle=1").fetch_one(&harness.pool).await?;
        assert_eq!(items, 1);
        use crate::gameplay_transport::spell_premium_coordinator::{SpellPremiumCoordinator, purchase_fixture};
        let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
        harness.root.latch_premium_activation(&activation.activation_id, activation.switch_over_us,
            activation.switch_over_us + 60_000_000).await.map_err(debug)?;
        let premium = SpellPremiumCoordinator::new(consumer, Some(clock), Some(activation), Some(harness.root.clone()));
        owner.premium_coordinator = Some(&premium);
        owner.admit_premium(id(40), admitted.game_session_id);
        for rank in 1..=3 {
            assert_eq!(command(&owner, &mut admitted, A::ClaimReward { rank, category: 4, choice: 1 }).await?.disposition, D::Applied);
            let completed: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE quest_key='oteryn:quest.barbarian_arena_quest' AND completed_character_revision IS NOT NULL")
                .fetch_one(&harness.pool).await?;
            assert_eq!(completed, 0, "three trophies and the prerequisite are mandatory");
        }
        owner.premium_coordinator = None;
        finish_barbarian_test(&owner, admitted.game_session_id).await?;
        assert_eq!(command(&owner, &mut admitted, A::Status).await?.disposition, D::Applied);
        let missing_premium: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE quest_key='oteryn:quest.barbarian_arena_quest' AND completed_character_revision IS NOT NULL")
            .fetch_one(&harness.pool).await?;
        assert_eq!(missing_premium, 0, "missing native Premium authority fails closed");
        owner.premium_coordinator = Some(&premium);
        assert_eq!(command(&owner, &mut admitted, A::Status).await?.disposition, D::Applied);
        let completed: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE quest_key='oteryn:quest.barbarian_arena_quest' AND completed_character_revision IS NOT NULL")
            .fetch_one(&harness.pool).await?;
        assert_eq!(completed, 1, "all native goals complete the canonical quest through the real caller");
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key='oteryn:quest.barbarian_arena_quest' AND completes")
            .fetch_one(&harness.pool).await?;
        assert_eq!(receipts, 1);
        assert_eq!(command(&owner, &mut admitted, A::Status).await?.disposition, D::Applied);
        let replay_count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key='oteryn:quest.barbarian_arena_quest' AND completes")
            .fetch_one(&harness.pool).await?;
        assert_eq!(replay_count, 1, "canonical completion is idempotent");
        eprintln!("M51 CANONICAL_POSTGRES_PROOF: rank=3 victories=30 trophies=3 completions={completed} receipts={receipts}");

        assert_eq!(
            command(&owner, &mut admitted, A::Leave).await?.state.phase,
            0
        );
        let position = deps
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        assert_eq!((position.x, position.y, position.floor), (32251, 31098, 6));
        let restarted = deps.owner(&harness, &authority);
        let result = restarted
            .arena_match_command(
                admitted,
                admitted.continuity.next_command_id,
                &wire::encode_match_request(A::Status).map_err(debug)?,
            )
            .await
            .ok_or("reload")?;
        let reloaded = wire::decode_match_result(&result).map_err(debug)?;
        assert_eq!(reloaded.state.completed_rank, 3);
        assert_eq!(reloaded.state.claimed_masks[0], 12);
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&harness.pool)
            .await?;
        assert_eq!(
            version,
            oteryn_gameplay_durability::CANONICAL_POSTGRES_SERVER_VERSION_NUM
        );
        eprintln!(
            "SV15 POSTGRES_PROOF: server_version_num={version}, entry_fee=1000, completed=3, weapon_claims={rewards}, reward_items={items}"
        );
        let _ = native;
        Ok(())
    })
}


// Positive setup uses the same fenced quest writer, never a raw state insert.
async fn finish_barbarian_test(owner: &ComposedFreshAdmission<'_, '_, '_>, session: crate::foundation::GameSessionId) -> TestResult {
    use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack, QuestTransition, QuestEffect, QuestComparison, QuestEffectKind};
    use crate::durability::quest_state::{QuestCause, QuestTransitionRequest, QuestTransitionOutcome};
    let quest = "oteryn:quest.barbarian_test";
    let track = "oteryn:quest-progress/fixture/barbarian-test";
    let key = "oteryn:quest-transition/fixture/barbarian-test";
    let catalogue = std::sync::Arc::new(QuestStateCatalogue::new("content-1", vec![QuestTrack {
        key: track.into(), quest: quest.into(), initial: 0, min: 0, max: 1,
    }], vec![QuestTransition { key: key.into(), quest: quest.into(), completes: true, experience: None, world_change: None,
        effects: vec![QuestEffect { track: track.into(), from: QuestComparison::Eq(0), effect: QuestEffectKind::Set(1) }] }]).map_err(debug)?);
    let fence = owner.current_quest_fence(session).await.map_err(|_| "fence")?.ok_or("session")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let cause = QuestCause::Command(crate::foundation::CommandRef::new(session, crate::foundation::CommandId::new(900051).map_err(debug)?));
    let result = slot.commit_quest_transition(owner.root, owner.character, owner.holder, fence,
        QuestTransitionRequest { transition_key: key.into(), cause }, catalogue).await.map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    Ok(())
}
