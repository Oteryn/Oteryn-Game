//! Accepted steps drive Crystal's discovery milestones through the fenced map writer.
use super::cyclopedia_fixtures::support;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, maintain_ready, prepare_current_session, reset};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{AdmittedSession, FirstEntryOutcome};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::world_spatial::{self, StepDirection, StepDisposition};
use std::time::Duration;

#[test]
fn measuring_tibia_steps_activate_discovery_and_grant_only_completed_main_area_milestones()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_case(false)
}

#[test]
fn measuring_tibia_failed_grant_rolls_back_discovery_and_retries_on_step() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_case(true)
}

fn run_case(refuse_grant: bool) -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(refuse_grant).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Measuring Tibia scenario panicked")??;
    Ok(())
}

fn scenario(refuse_grant: bool) -> TestResult {
    run(async move |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "measuring_tibia", true, false).await?;
        eprintln!("POSTGRES_STARTED measuring_tibia");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activation_pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let world = support::world_with_two_main_areas(&deps, generation)?;
        let room = activation_pin.qualified_room();
        let cells = world.movement_cells(room.movement_cells()).map_err(debug)?;
        support::seed(&h, world.base().digest()).await?;
        let identity = generation.identity();
        let frame = activation_pin.frame_binding().digest();
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE game_world_reset_epochs SET server_digest=$1,client_digest=$2,frame_digest=$3",
        )
        .bind(identity.server_artifact_digest().as_slice())
        .bind(identity.client_artifact_digest().as_slice())
        .bind(frame.as_slice())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        let combat_room = room
            .bundle_view(
                cells.clone(),
                frame,
                world.map_revision_digest(),
                crate::content::NativeEntryStart { x: 1, y: 0, floor: 7 },
            )
            .ok_or("Cyclopedia combat room")?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            identity.server_artifact_digest(),
            identity.client_artifact_digest(),
            frame,
            world.map_revision_digest(),
            (1, 0, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime.initialize_first_entry_position(deps.actor).map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);

        let mut owner = deps.owner(&h, &authority);
        owner.movement_cells = (&cells).into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&combat_room));
        owner.active_generation = Some(generation);
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(deps.admitted.game_session_id)
            .await?.0;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::NONE;

        // Movement samples the admission equipment owner for this active generation.
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(identity.server_artifact_digest().as_slice())
            .bind(identity.client_artifact_digest().as_slice())
            .bind(frame.as_slice()).execute(&h.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &admitted.item_fence.ok_or("equipment fence")?,
                identity.server_artifact_digest(),
                fresh.operation(),
            )
            .await.map_err(debug)?;
        step(&mut owner, &mut admitted, &deps, &h, StepDirection::East).await?;
        let active: serde_json::Value =
            sqlx::query_scalar("SELECT progress->'subareas'->'2'->'exploration' FROM game_character_cyclopedia")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(active["points"].as_array().map(Vec::len), Some(7));
        assert_eq!(active["found"], 0);
        assert_rewards(&h, &[], &[]).await?;

        // Prepare each run one point short, on actual tiles inside its pinned subarea.
        // The durable completion and grants below are written only by the accepted STEP.
        for (subarea, y) in [(2, 0), (3, 8), (5, 9)] {
            let points: Vec<_> = (11..18)
                .map(|x| serde_json::json!({"x": x, "y": y, "floor": -7}))
                .collect();
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "UPDATE game_character_cyclopedia SET progress=jsonb_set(progress,$1,$2,true)",
            )
            .bind(vec!["subareas".to_owned(), subarea.to_string()])
            .bind(serde_json::json!({"discovered": false, "exploration": {"points": points, "found": 63}}))
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            {
                let mut runtime = owner.runtime.lock().await;
                let mut movement = runtime.borrow_movement_position();
                let mut current = movement.read(deps.actor).map_err(debug)?;
                let target = MovementLocalPosition { x: 16, y, floor: 7 };
                while current.position() != target {
                    let at = current.position();
                    let next = if at.x != target.x {
                        MovementLocalPosition { x: at.x + (target.x - at.x).signum(), ..at }
                    } else {
                        MovementLocalPosition { y: at.y + (target.y - at.y).signum(), ..at }
                    };
                    current = movement.commit_cardinal(current, next).map_err(debug)?;
                }
            }
            if subarea == 5 && refuse_grant {
                sqlx::query(
                    "ALTER TABLE game_account_achievement_grant_requests                      ADD CONSTRAINT measuring_tibia_test_refuse_world                      CHECK (achievement_key <> 'oteryn:achievement/measuring_the_world')",
                )
                .execute(&h.pool)
                .await?;
                step(&mut owner, &mut admitted, &deps, &h, StepDirection::East).await?;
                assert!(!discovered(&h, subarea).await?);
                assert_rewards(&h, &["widely_travelled"], &[0, 1]).await?;
                sqlx::query(
                    "ALTER TABLE game_account_achievement_grant_requests                      DROP CONSTRAINT measuring_tibia_test_refuse_world",
                )
                .execute(&h.pool)
                .await?;
                step(&mut owner, &mut admitted, &deps, &h, StepDirection::West).await?;
            }
            step(&mut owner, &mut admitted, &deps, &h, StepDirection::East).await?;
            assert!(discovered(&h, subarea).await?, "step completes subarea {subarea}");
            match subarea {
                2 => assert_rewards(&h, &[], &[]).await?,
                3 => assert_rewards(&h, &["widely_travelled"], &[0, 1]).await?,
                5 => assert_rewards(&h, &["measuring_the_world", "widely_travelled"], &[0, 1, 2]).await?,
                _ => return Err("unexpected fixture subarea".into()),
            }
        }
        step(&mut owner, &mut admitted, &deps, &h, StepDirection::West).await?;
        step(&mut owner, &mut admitted, &deps, &h, StepDirection::East).await?;
        assert_rewards(&h, &["measuring_the_world", "widely_travelled"], &[0, 1, 2]).await?;
        Ok(())
    })
}

async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    deps: &Dependencies,
    h: &Harness,
    direction: StepDirection,
) -> TestResult {
    owner.clock_origin -= Duration::from_secs(2);
    deps.initialize(owner).await;
    maintain_ready(&h.root).await?;
    let step_id = admitted.continuity.next_command_id;
    let (status, result) = support::exchange(
        owner,
        admitted,
        step_id,
        world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        &world_spatial::encode_step_intent(direction),
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(result, world_spatial::encode_step_result(StepDisposition::Moved));
    Ok(())
}

async fn discovered(h: &Harness, subarea: u32) -> TestResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT (progress->'subareas'->$1->>'discovered')::boolean FROM game_character_cyclopedia",
    )
    .bind(subarea.to_string())
    .fetch_one(&h.pool)
    .await?)
}

async fn assert_rewards(h: &Harness, achievements: &[&str], addons: &[u8]) -> TestResult {
    let earned: Vec<String> = sqlx::query_scalar(
        "SELECT split_part(achievement_key,'/',2) FROM game_account_achievement_grant_requests          WHERE achievement_key IN ('oteryn:achievement/widely_travelled','oteryn:achievement/measuring_the_world')          ORDER BY achievement_key",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(earned, achievements);
    let held: Vec<String> = sqlx::query_scalar(
        "SELECT split_part(achievement_key,'/',2) FROM game_account_achievements
         WHERE achievement_key IN ('oteryn:achievement/widely_travelled','oteryn:achievement/measuring_the_world')
         ORDER BY achievement_key",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(held, achievements);
    let granted: Vec<String> = sqlx::query_scalar(
        "SELECT unlock_key FROM game_account_appearance_unlock_receipts          WHERE unlock_key LIKE 'oteryn:outfit.discoverer/quest-grant/%' ORDER BY unlock_key",
    )
    .fetch_all(&h.pool)
    .await?;
    let expected: Vec<_> = addons
        .iter()
        .map(|addon| format!("oteryn:outfit.discoverer/quest-grant/{addon}"))
        .collect();
    assert_eq!(granted, expected);
    let unlocked: Vec<String> = sqlx::query_scalar(
        "SELECT unlock_key FROM game_account_appearance_unlocks
         WHERE unlock_key LIKE 'oteryn:outfit.discoverer/quest-grant/%' ORDER BY unlock_key",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(unlocked, expected);
    Ok(())
}
