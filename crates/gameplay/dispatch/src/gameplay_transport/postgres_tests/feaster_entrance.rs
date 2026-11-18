//! Crystal's level-only entrance through the ordinary movement owner.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{CharacterId, CharacterLease, GameSessionId, MovementLocalPosition};
use crate::map::{boot, overlay::TilePos};
use oteryn_gameplay_quests::quest::world_callbacks::Callback;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

#[test]
fn feaster_entrance_checks_level_without_quest_state() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run_async(|admin| exercise(admin, None));
    eprintln!(
        "POSTGRES_CASE_COMPLETED feaster_entrance duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn feaster_timed_room_admits_two_players_and_ejects_both() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run_async(|admin| exercise(admin, Some(5)))
}

#[test]
fn feaster_solo_room_refuses_second_player_and_ejects_entrant() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run_async(|admin| exercise(admin, Some(1)))
}

async fn exercise(admin: String, capacity: Option<u8>) -> TestResult {
    let timed = capacity.is_some();
    let solo = capacity == Some(1);
    let h = Harness::create_with_admission_seed(admin, "feaster_level", true, false).await?;
    prepare_current_session(&h).await?;
    if timed {
        h.seed_appearance_observer().await?;
        super::appearance_change_postgres::observer_visibility_tests::admit(&h).await?;
    }
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let initial = Dependencies::new()?;
    let world_id = initial.admitted.world_id;
    let (controller, activated) =
        super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
    let active = controller.active().ok_or("active generation")?;
    let room = activated.qualified_room();
    let (bytes, pins) = boot::tests::bundle();
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let world = boot::boot(
        &bytes,
        &boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: TilePos {
                x: 1,
                y: 0,
                floor: -7,
            },
        },
        world_id,
        initial.admitted.channel_id,
        boot::tests::items,
    )
    .map_err(debug)?;
    let cells = world.movement_cells(room.movement_cells()).map_err(debug)?;
    let cases: &[(i64, i32)] = if timed {
        &[(250, 3)]
    } else {
        &[(249, 1), (250, 3)]
    };
    for &(level, expected_x) in cases {
        // Seed boundary levels using the same isolated transaction as the boss activity cases.
        let mut seed = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *seed)
            .await?;
        let changed = sqlx::query(
            "UPDATE game_character_progression_state SET level=$1,total_experience=$2 \
             WHERE character_id=encode($3,'hex')::uuid OR ($4 AND character_id=encode($5,'hex')::uuid)",
        )
        .bind(level)
        .bind((level - 1) * (50 * level * level - 250 * level + 600) / 3)
        .bind(id(41).as_slice())
        .bind(timed)
        .bind(id(101).as_slice())
        .execute(&mut *seed)
        .await?;
        assert_eq!(changed.rows_affected(), if timed { 2 } else { 1 });
        seed.commit().await?;
        let pin = ChannelContentPin::from_activation(
            world_id,
            1,
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            world.frame_binding_digest(world_id),
            world.map_revision_digest(),
            (1, 0, 7),
        );
        if timed {
            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,$5,$2,$3,$4,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-11')")
                .bind(world_id.as_bytes().as_slice())
                .bind(pin.server_artifact_digest().as_slice())
                .bind(pin.client_artifact_digest().as_slice())
                .bind(pin.frame_binding_digest().as_slice())
                .bind(crate::map::overlay::map_revision(&base))
                .execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
                .bind(world_id.as_bytes().as_slice())
                .execute(&h.pool)
                .await?;
        }
        let deps = Dependencies::with_content_and_capacity(
            pin,
            room.clone(),
            initial.spells.clone(),
            if timed { 3 } else { 2 },
        )?;
        let origin = deps
            .runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        assert!(
            owner
                .states
                .quest_callbacks
                .fixture
                .set(vec![portal(capacity)?])
                .is_ok()
        );
        let session = deps.admitted.game_session_id;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "session fence")?
            .ok_or("current session fence")?;
        deps.runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                session,
                CharacterLease::new(
                    CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        deps.initialize(&owner).await;
        let (step, _) = owner
            .ordinary_paced_step(
                deps.actor,
                deps.admitted.game_session_id,
                StepDirection::East,
                Some(origin),
                None,
            )
            .await;
        assert_eq!(step.disposition, StepDisposition::Moved);
        let position = deps
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?;
        assert_eq!(position.position().x, expected_x, "level {level}");
        if timed {
            let observer =
                super::appearance_change_postgres::observer_visibility_tests::initialize(
                    &deps, &owner,
                )
                .await?;
            let observer_session = GameSessionId::decode(&id(110)).map_err(debug)?;
            let observer_before = deps
                .runtime
                .lock()
                .await
                .read_actor_position(observer)
                .map_err(debug)?;
            let (step, _) = owner
                .ordinary_paced_step(
                    observer,
                    observer_session,
                    StepDirection::East,
                    Some(observer_before),
                    None,
                )
                .await;
            assert_eq!(step.disposition, StepDisposition::Moved);
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(observer)
                    .map_err(debug)?
                    .position()
                    .x,
                if solo { 1 } else { 3 }
            );
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_millis(50))
                .ok_or("spawn clock")?;
            owner.drain_quest_encounter_deadlines().await;
            {
                let runtime = deps.runtime.lock().await;
                assert_eq!(
                    runtime.current_live_creature_census().map_err(debug)?.len(),
                    1
                );
            }
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(301))
                .ok_or("owner clock")?;
            owner.drain_quest_encounter_deadlines().await;
            let runtime = deps.runtime.lock().await;
            assert!(
                runtime
                    .current_live_creature_census()
                    .map_err(debug)?
                    .is_empty()
            );
            for player in [deps.actor, observer] {
                assert_eq!(
                    runtime
                        .read_actor_position(player)
                        .map_err(debug)?
                        .position()
                        .x,
                    if solo && player == observer { 1 } else { 7 }
                );
            }
        }
    }
    let rows: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_quest_states), \
                (SELECT count(*) FROM game_character_quest_receipts)",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(rows, (0, 0));
    if timed {
        let cooldowns: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_boss_cooldowns WHERE boss_key=$1",
        )
        .bind(if solo {
            "oteryn:boss.the_fear_feaster"
        } else {
            "oteryn:boss.brain_head"
        })
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(cooldowns, if solo { 1 } else { 2 });
    }
    drop(authority);
    drop(seal);
    h.cleanup().await
}

fn portal(capacity: Option<u8>) -> TestResult<Callback> {
    let mut callback = serde_json::json!({
        "key": "oteryn:quest-callback/feaster-level-entrance-fixture",
        "quest": "oteryn:quest.feaster_of_souls_quest",
        "trigger": "step_in",
        "selector": {"position":{"x":2,"y":0,"floor":-7},"ordinal":0},
        "lever": null,
        "reversible": false,
        "passages": [],
        "storage": [],
        "teleport": {"x":3,"y":0,"floor":-7},
        "effects": {"teleport_effect": 11},
        "offerings": [],
        "minimum_level": 250,
        "return_on_failure": true,
        "failure_message": "You need at least level 250 to enter.",
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/feaster_of_souls/actions_entrances.lua:17-33"]
    });
    if let Some(capacity) = capacity {
        let solo = capacity == 1;
        let defender = if solo {
            "oteryn:creature.the_fear_feaster"
        } else {
            "oteryn:creature.brain_head"
        };
        callback["effects"]["encounter"] = serde_json::json!({
            "minimum_level": 250,
            "players": [],
            "monsters": [{"position":{"x":1,"y":0,"floor":-7},"creature":defender}],
            "room_min":{"x":1,"y":0,"floor":-7},
            "room_max":{"x":3,"y":0,"floor":-7},
            "timing":{
                "spawn_after_ms":if solo { 0 } else { 50 },"reset_after_ms":300_000,
                "admission":{"capacity":capacity,"join_for_ms":if solo { 0 } else { 30_000 },"cooldown_s":if solo { 72_000 } else { 36_000 },"exit":{"x":7,"y":0,"floor":-7}}
            }
        });
    }
    Ok(serde_json::from_value(callback)?)
}

#[path = "feaster_entrance/unwelcome.rs"]
mod unwelcome;
#[path = "feaster_entrance/zarganash.rs"]
mod zarganash;
