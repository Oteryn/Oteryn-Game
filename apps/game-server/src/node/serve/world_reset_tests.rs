use super::*;

#[test]
fn reset_drain_exit_is_retryable_and_retains_the_observed_reset()
-> Result<(), Box<dyn std::error::Error>> {
    let state = ResetWatch::default();
    assert!(classify_exit(&state, None).is_ok());
    assert!(matches!(
        classify_exit(
            &state,
            Some(crate::gameplay_transport::GameplayServeError::InvalidLimits)
        ),
        Err(BootError::Serve)
    ));
    state.requested.store(true, Ordering::Release);
    let error = classify_exit(&state, None)
        .err()
        .ok_or("a completed reset drain must restart")?;
    assert_eq!(error.exit_code(), 17);
    assert!(matches!(error, BootError::Readiness("World reset drain")));
    Ok(())
}

#[test]
fn planned_reset_observation_forces_a_complete_recent_highscore_cut()
-> crate::bestiary_postgres_harness::TestResult {
    use crate::bestiary_postgres_harness::{
        Harness, TestResult, configured_admin, debug, runtime, scope,
    };
    use crate::durability::highscore_snapshot::SnapshotCatalogues;
    use sqlx::Executor;
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        runtime().map_err(debug)?.block_on(async {
            let h = Harness::create(admin, "reset_highscore", true).await?;
            let result: TestResult = async {
                let world = scope()?.world_id();
                let achievements = crate::achievement_catalogue::AchievementCatalogue::embedded().map_err(debug)?;
                let catalogues = SnapshotCatalogues::activated(std::sync::Arc::new(achievements.clone())).map_err(debug)?;
                let first = h.root.refresh_highscore_schedule(world, catalogues.clone(), false).await.map_err(debug)?;
                assert!(!first.snapshots.is_empty());
                h.root.await_ready().await?;
                let recent = h.root.refresh_highscore_schedule(world, catalogues, false).await.map_err(debug)?;
                assert!(recent.snapshots.is_empty(), "the normal schedule must skip this recent cut");
                let state = ResetWatch::default();
                let drain = GameplayDrain::at_reset_epoch(0);
                h.root.await_ready().await?;
                assert!(!observe_reset(&h.root, world, &state, &drain, &achievements).await);
                sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(123),'1970-01')")
                    .bind(world.as_bytes().as_slice()).bind([1u8;32].as_slice()).execute(&h.pool).await?;
                sqlx::query("INSERT INTO game_world_reset_jobs(world_id,reset_epoch,target_digest,retirement_binding,target_map_revision,target_server_digest,target_client_digest,target_frame_digest,target_house_tiles,calendar_revision,phase) SELECT world_id,reset_epoch,target_digest,sha256(convert_to('oteryn:world-reset-retirement:v1','UTF8') || target_digest),map_revision,server_digest,client_digest,frame_digest,'[]'::jsonb,calendar_revision,'RETIRING' FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid")
                    .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
                h.pool.execute("CREATE SEQUENCE reset_cut_failure; GRANT USAGE ON SEQUENCE reset_cut_failure TO oteryn_game_highscore_job; CREATE FUNCTION reset_cut_fail_once() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF nextval('reset_cut_failure')=1 THEN RAISE EXCEPTION 'transient reset cut failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reset_cut_fail_once BEFORE INSERT ON game_world_highscore_snapshots FOR EACH ROW EXECUTE FUNCTION reset_cut_fail_once()").await?;
                h.root.await_ready().await?;
                assert!(!observe_reset(&h.root, world, &state, &drain, &achievements).await);
                assert!(!state.requested.load(Ordering::Acquire), "a failed cut cannot release the drain");
                h.root.await_ready().await?;
                assert!(observe_reset(&h.root, world, &state, &drain, &achievements).await);
                assert!(state.requested.load(Ordering::Acquire));
                let heads: Vec<(i32, i64)> = sqlx::query_as("SELECT category,current_snapshot_id FROM game_world_highscore_heads WHERE world_id=encode($1,'hex')::uuid ORDER BY category")
                    .bind(world.as_bytes().as_slice()).fetch_all(&h.pool).await?;
                assert_eq!(heads.len(), first.snapshots.len(), "every supported category is retained");
                for snapshot in first.snapshots {
                    let head = heads.iter().find(|(category, _)| *category == i32::from(snapshot.category)).ok_or("missing head")?;
                    assert!(u64::try_from(head.1)? > snapshot.snapshot_id, "the reset must force every category before requesting drain");
                }
                Ok(())
            }.await;
            h.cleanup().await?;
            result
        }).map_err(debug)
    })?.join().map_err(|_| "reset snapshot thread failed")?.map_err(Into::into)
}
