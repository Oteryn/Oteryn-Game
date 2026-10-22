//! Reuse the existing real fresh-admission/Channel fixture, mounted once.
use super::*;
use crate::foundation::ControlLossCauseV1;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;

#[test]
fn planned_world_reset_healthy_session_loses_control_and_releases_through_grace() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "planned_reset_loss", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        dependencies.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(dependencies.actor)
            .map_err(debug)?;
        track(&owner, &dependencies)?;
        let world = owner.world_id;
        assert!(harness.root.retiring_world_reset(world).await?.is_none());
        // The retained job is installed by the database owner, not by a test-only
        // witness constructor. The production reader seals the actual durable row.
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_10',1,clock_timestamp(),to_char((clock_timestamp() AT TIME ZONE 'Europe/Berlin')-interval '10 hours','YYYY-MM'))")
            .bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_world_reset_jobs(world_id,reset_epoch,target_digest,retirement_binding,target_map_revision,target_server_digest,target_client_digest,target_frame_digest,target_house_tiles,calendar_revision,phase) VALUES(encode($1,'hex')::uuid,0,$2,sha256(convert_to('oteryn:world-reset-retirement:v1','UTF8') || $2),'map-2',$2,$2,$2,'[]',1,'RETIRING')")
            .bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).execute(&harness.pool).await?;
        let retiring = harness
            .root
            .retiring_world_reset(world)
            .await?
            .ok_or("sealed retiring reset")?;
        let binding: [u8; 32] = sqlx::query_scalar::<_, Vec<u8>>(
            "SELECT retirement_binding FROM game_world_reset_jobs LIMIT 1",
        )
        .fetch_one(&harness.pool)
        .await?
        .try_into()
        .map_err(|_| "binding")?;
        assert_eq!(
            retiring.loss_cause(),
            ControlLossCauseV1::PlannedWorldReset {
                world_id: world,
                reset_epoch: 0,
                retirement_binding: binding,
                target_digest: [7; 32],
            }
        );
        // A retained witness cannot authorize loss after its phase or target changes.
        for (change, restore) in [
            (
                "UPDATE game_world_reset_jobs SET phase='ACTIVATED'",
                "UPDATE game_world_reset_jobs SET phase='RETIRING'",
            ),
            (
                "UPDATE game_world_reset_jobs SET target_digest=decode(repeat('09',32),'hex'),retirement_binding=sha256(convert_to('oteryn:world-reset-retirement:v1','UTF8') || decode(repeat('09',32),'hex'))",
                "UPDATE game_world_reset_jobs SET target_digest=decode(repeat('07',32),'hex'),retirement_binding=sha256(convert_to('oteryn:world-reset-retirement:v1','UTF8') || decode(repeat('07',32),'hex'))",
            ),
        ] {
            sqlx::query(change).execute(&harness.pool).await?;
            assert_eq!(
                owner
                    .lose_control(dependencies.admitted, Duration::ZERO, Some(&retiring))
                    .await,
                crate::gameplay_transport::connection::ControlLossResult::Refused
            );
            assert_eq!(
                FreshAdmissionStore::from_root(harness.root.clone())
                    .current_session(dependencies.admitted.game_session_id)
                    .await?
                    .session_state(),
                crate::foundation::GameSessionState::Active
            );
            sqlx::query(restore).execute(&harness.pool).await?;
        }
        {
            let drain = crate::gameplay_transport::gameplay_drain::GameplayDrain::default();
            drain.request(retiring.clone());
            drain.request(retiring.clone());
            let shutdown = oteryn_foundation::CancellationToken::new();
            let mut lifecycle =
                std::pin::pin!(crate::gameplay_transport::listener::control_loss_lifecycle(
                    &owner,
                    crate::gameplay_transport::listener::LostControl {
                        session: dependencies.admitted,
                        wait: Duration::ZERO,
                        reset: drain.reset(),
                    },
                    &shutdown,
                ));
            let observed_loss = async {
                loop {
                    if owner
                        .runtime
                        .lock()
                        .await
                        .player_control_facts(
                            dependencies.actor,
                            dependencies.admitted.game_session_id,
                        )
                        .map_err(debug)?
                        .control_loss
                        .is_some()
                    {
                        return Ok::<_, Box<dyn std::error::Error>>(());
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            };
            let recorded = tokio::time::timeout(
                Duration::from_secs(10),
                crate::gameplay_transport::first(lifecycle.as_mut(), async {
                    observed_loss.await.expect("current actor loss");
                }),
            )
            .await?;
            assert!(
                recorded.is_none(),
                "the lifecycle must retain the original grace"
            );
            let store = FreshAdmissionStore::from_root(harness.root.clone());
            let current = store
                .current_session(dependencies.admitted.game_session_id)
                .await?;
            assert_eq!(
                current.session_state(),
                crate::foundation::GameSessionState::Reconnectable
            );
            assert!(current.current_transport().is_none());
            let (loss, _) = store
                .owning_loss(
                    dependencies.admitted.game_session_id,
                    current.current_control_loss_epoch().ok_or("loss epoch")?,
                )
                .await?
                .ok_or("durable loss")?;
            assert_eq!(loss.observation.cause, retiring.loss_cause());
            assert_eq!(
                loss.observation.original_grace_deadline - loss.observation.loss_origin,
                60
            );
            tokio::time::timeout(Duration::from_secs(65), lifecycle.as_mut()).await?;
            assert_eq!(
                store
                    .current_session(dependencies.admitted.game_session_id)
                    .await?
                    .session_state(),
                crate::foundation::GameSessionState::Terminal
            );
            assert!(
                owner
                    .runtime
                    .lock()
                    .await
                    .player_control_facts(dependencies.actor, dependencies.admitted.game_session_id)
                    .is_err()
            );
        }
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
