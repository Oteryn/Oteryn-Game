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

struct ResetTestBuildFormula;

impl crate::durability::character_build::BuildFormula for ResetTestBuildFormula {
    fn required(&self, _: &str, _: usize, _: u16) -> Option<u64> {
        Some(1_000_000)
    }
    fn digest(&self) -> [u8; 32] {
        [1; 32]
    }
    fn content_revision(&self) -> &str {
        "content-1"
    }
}

#[test]
fn planned_reset_drains_before_a_single_post_activation_highscore_cut()
-> crate::bestiary_postgres_harness::TestResult {
    reset_cut_retries(ResetCutFailure::Write)
}

#[test]
fn planned_reset_retries_unknown_highscore_commit() -> crate::bestiary_postgres_harness::TestResult
{
    reset_cut_retries(ResetCutFailure::CommitTimeout)
}

#[test]
fn planned_reset_retries_rejected_highscore_commit() -> crate::bestiary_postgres_harness::TestResult
{
    reset_cut_retries(ResetCutFailure::TransactionTimeout)
}

#[test]
fn planned_reset_retries_highscore_deadline() -> crate::bestiary_postgres_harness::TestResult {
    reset_cut_retries(ResetCutFailure::PassDeadline)
}

enum ResetCutFailure {
    Write,
    CommitTimeout,
    TransactionTimeout,
    PassDeadline,
}

fn reset_cut_retries(failure: ResetCutFailure) -> crate::bestiary_postgres_harness::TestResult {
    let clear_timeouts = matches!(failure, ResetCutFailure::PassDeadline);
    let (trigger, fault) = match failure {
        ResetCutFailure::CommitTimeout => (
            "CREATE CONSTRAINT TRIGGER reset_cut_fail_once AFTER INSERT ON game_world_highscore_snapshots DEFERRABLE INITIALLY DEFERRED",
            "PERFORM set_config('transaction_timeout','0',true); PERFORM pg_sleep(2.3); RAISE EXCEPTION 'late commit rollback';",
        ),
        ResetCutFailure::TransactionTimeout => (
            "CREATE CONSTRAINT TRIGGER reset_cut_fail_once AFTER INSERT ON game_world_highscore_snapshots DEFERRABLE INITIALLY DEFERRED",
            // Rearm the active transaction timer so PostgreSQL expires it inside COMMIT.
            "PERFORM set_config('transaction_timeout','0',true); PERFORM set_config('transaction_timeout','100ms',true); PERFORM pg_sleep(4);",
        ),
        ResetCutFailure::PassDeadline => (
            "CREATE TRIGGER reset_cut_fail_once BEFORE INSERT ON game_world_highscore_snapshots",
            "PERFORM pg_sleep(4);",
        ),
        ResetCutFailure::Write => (
            "CREATE TRIGGER reset_cut_fail_once BEFORE INSERT ON game_world_highscore_snapshots",
            "RAISE EXCEPTION 'transient reset cut failure';",
        ),
    };
    use crate::bestiary_postgres_harness::{
        CHARACTER, Harness, TestResult, configured_admin, context, debug, fence, id, runtime, scope,
    };
    use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState,
    };
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    use crate::durability::highscore_snapshot::SnapshotCatalogues;
    use crate::foundation::CharacterId;
    use oteryn_simulation_determinism::{ExactI64, RoundingMode};
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
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                h.root.commit_character_build(
                    &authority, &h.node, fence(1)?,
                    BuildChangeRequest {
                        occurrence: BuildOccurrence::from_bytes(id(60)).map_err(debug)?,
                        cause: BuildCause::VocationChoice,
                        before: DurableBuildState::default(),
                        after: DurableBuildState::new("knight", (0, 0), [(10, 0); 7]).map_err(debug)?,
                        pruned_stance: None,
                    },
                    &ResetTestBuildFormula,
                ).await.map_err(debug)?;
                h.root.await_ready().await?;
                let first = h.root.refresh_highscore_schedule(world, catalogues.clone(), false).await.map_err(debug)?;
                assert!(!first.snapshots.is_empty());
                h.root.await_ready().await?;
                let recent = h.root.refresh_highscore_schedule(world, catalogues, false).await.map_err(debug)?;
                assert!(recent.snapshots.is_empty(), "the normal schedule must skip this recent cut");
                let state = ResetWatch::default();
                let drain = GameplayDrain::at_reset_epoch(0);
                h.root.await_ready().await?;
                assert!(!observe_reset(&h.root, world, &state, &drain).await);
                sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(123),'1970-01')")
                    .bind(world.as_bytes().as_slice()).bind([1u8;32].as_slice()).execute(&h.pool).await?;
                sqlx::query("INSERT INTO game_world_reset_jobs(world_id,reset_epoch,target_digest,retirement_binding,target_map_revision,target_server_digest,target_client_digest,target_frame_digest,target_house_tiles,calendar_revision,phase) SELECT world_id,reset_epoch,target_digest,sha256(convert_to('oteryn:world-reset-retirement:v1','UTF8') || target_digest),map_revision,server_digest,client_digest,frame_digest,'[]'::jsonb,calendar_revision,'RETIRING' FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid")
                    .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
                sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE SEQUENCE reset_cut_failure; GRANT USAGE ON SEQUENCE reset_cut_failure TO oteryn_game_highscore_job; CREATE FUNCTION reset_cut_fail_once() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF nextval('reset_cut_failure')=1 THEN {fault} END IF; RETURN NEW; END $$; {trigger} FOR EACH ROW EXECUTE FUNCTION reset_cut_fail_once()"))).execute(&h.pool).await?;
                if clear_timeouts {
                    // Disable server timers in a preceding statement so the Root owns the timeout.
                    h.pool.execute("CREATE FUNCTION reset_cut_disable_timeouts() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM set_config('transaction_timeout','0',true); PERFORM set_config('statement_timeout','0',true); RETURN NEW; END $$; CREATE TRIGGER reset_cut_disable_timeouts BEFORE INSERT ON game_world_highscore_heads FOR EACH ROW EXECUTE FUNCTION reset_cut_disable_timeouts()").await?;
                }
                h.root.await_ready().await?;
                assert!(observe_reset(&h.root, world, &state, &drain).await,
                    "publication failure cannot prevent the reset drain");
                assert!(state.requested.load(Ordering::Acquire));
                let second_watch = ResetWatch::default();
                h.root.await_ready().await?;
                assert!(observe_reset(&h.root, world, &second_watch, &GameplayDrain::at_reset_epoch(0)).await);
                let attempted: bool = sqlx::query_scalar("SELECT is_called FROM reset_cut_failure").fetch_one(&h.pool).await?;
                assert!(!attempted, "neither observer may publish while sessions can still commit");
                h.root.await_ready().await?;
                let award = h.root.commit_character_experience(&authority, &h.node, fence(2)?,
                    ExperienceAwardRequest {
                        occurrence: ExperienceRewardOccurrence::from_bytes(id(80)).map_err(debug)?,
                        amount: ExactI64::new(5),
                        context: context(),
                        policy_revision: "policy-1".into(),
                        reward_revision: "reward-1".into(),
                        policy: FiniteProgressionPolicy {
                            context: context(),
                            policy_revision: "policy-1".into(),
                            reward_revision: "reward-1".into(),
                            death_policy_revision: "death-1".into(),
                            declared_difference_revision: "declaration-1".into(),
                            thresholds: [
                                LevelThreshold { level: 50, minimum_experience: ExactI64::new(1000) },
                                LevelThreshold { level: 51, minimum_experience: ExactI64::new(1100) },
                            ],
                            terminal_exclusive_experience: ExactI64::new(1200),
                            death_loss_numerator: 1,
                            death_loss_denominator: 10,
                            death_loss_rounding: RoundingMode::Floor,
                        },
                    },
                ).await.map_err(debug)?;
                assert!(matches!(award, ExperienceCommitOutcome::Committed(_)));
                // Simulate the durable reset activation transaction after all channels drain.
                h.pool.execute("UPDATE game_world_reset_jobs SET phase='ACTIVATED' WHERE reset_epoch=0; INSERT INTO game_world_reset_epochs SELECT world_id,1,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,clock_timestamp(),reset_month FROM game_world_reset_epochs WHERE reset_epoch=0; INSERT INTO game_world_reset_active SELECT world_id,1 FROM game_world_reset_epochs WHERE reset_epoch=1 ON CONFLICT(world_id) DO UPDATE SET reset_epoch=EXCLUDED.reset_epoch").await?;
                let catalogues = SnapshotCatalogues::activated(std::sync::Arc::new(achievements.clone())).map_err(debug)?;
                h.root.await_ready().await?;
                let stop = CancellationToken::new();
                let observe = async {
                    let published = async {
                        loop {
                            // Mirror the node's maintenance pump if the failed pass retired its holder.
                            if h.root.has_ready_demand() {
                                h.root.maintain_ready_once().await?;
                            }
                            let current: i64 = sqlx::query_scalar(
                                "SELECT count(*) FROM game_world_highscore_heads WHERE published_reset_epoch=1",
                            ).fetch_one(&h.pool).await?;
                            if usize::try_from(current)? == first.snapshots.len() {
                                break;
                            }
                            tokio::time::sleep(Duration::from_millis(20)).await;
                        }
                        Ok::<_, Box<dyn std::error::Error>>(())
                    };
                    let result = tokio::time::timeout(Duration::from_secs(15), published).await;
                    stop.cancel();
                    result
                };
                let ((), result) = tokio::join!(highscores_loop(&h.root, world, &achievements, &stop), observe);
                let attempted: bool = sqlx::query_scalar("SELECT is_called FROM reset_cut_failure").fetch_one(&h.pool).await?;
                assert!(attempted, "the job must attempt the first post-activation cut");
                result.map_err(|_| "the job waited too long to retry the failed reset cut")??;
                let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
                other.await_ready().await?;
                let repeated = other.refresh_highscore_schedule(world, catalogues, false).await.map_err(debug)?;
                assert!(repeated.snapshots.is_empty(), "another node must observe the already published epoch");
                let heads: Vec<(i32, i64, i64)> = sqlx::query_as("SELECT category,current_snapshot_id,previous_snapshot_id FROM game_world_highscore_heads WHERE world_id=encode($1,'hex')::uuid ORDER BY category")
                    .bind(world.as_bytes().as_slice()).fetch_all(&h.pool).await?;
                assert_eq!(heads.len(), first.snapshots.len(), "every supported category is retained");
                for snapshot in first.snapshots {
                    let head = heads.iter().find(|(category, _, _)| *category == i32::from(snapshot.category)).ok_or("missing head")?;
                    assert_eq!(u64::try_from(head.2)?, snapshot.snapshot_id);
                    assert!(u64::try_from(head.1)? > snapshot.snapshot_id, "the activated epoch must refresh every category");
                }
                let character = CharacterId::decode(&id(CHARACTER)).map_err(debug)?;
                h.root.await_ready().await?;
                let page = h.root.read_highscore_page(world, 1, None, 1, 0, character).await.map_err(debug)?;
                assert_eq!(page.own_row.ok_or("missing post-reset rank")?.value, (1005, 0));
                Ok(())
            }.await;
            h.cleanup().await?;
            result
        }).map_err(debug)
    })?.join().map_err(|_| "reset snapshot thread failed")?.map_err(Into::into)
}

#[test]
fn highscore_epoch_upgrade_replaces_unproven_cut_and_retains_history()
-> crate::bestiary_postgres_harness::TestResult {
    use crate::bestiary_postgres_harness::{
        CHARACTER, Harness, TestResult, configured_admin, debug, id, runtime, scope,
    };
    use crate::durability::highscore_snapshot::{SnapshotCatalogues, publish_in_transaction};
    use crate::foundation::CharacterId;
    use sqlx::Executor;
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        runtime().map_err(debug)?.block_on(async {
            let h = Harness::create(admin, "highscore_epoch_upgrade", true).await?;
            let result: TestResult = async {
                let world = scope()?.world_id();
                let achievements = crate::achievement_catalogue::AchievementCatalogue::embedded().map_err(debug)?;
                let catalogues = SnapshotCatalogues::activated(std::sync::Arc::new(achievements)).map_err(debug)?;
                let character = CharacterId::decode(&id(CHARACTER)).map_err(debug)?;
                h.root.await_ready().await?;
                let first = h.root.refresh_highscore_schedule(world, catalogues.clone(), true).await.map_err(debug)?;
                let previous = first.snapshots.iter().find(|s| s.category == 1).ok_or("missing experience snapshot")?.snapshot_id;
                let mut legacy = h.pool.begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ").await?;
                let _: String = sqlx::query_scalar("SELECT pg_current_snapshot()::text").fetch_one(&mut *legacy).await?;
                h.pool.execute("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) SELECT world_id,1,sha256('epoch'::bytea),'map-1',sha256('server'::bytea),sha256('client'::bytea),sha256('frame'::bytea),'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-11' FROM game_character_roots; INSERT INTO game_world_reset_active SELECT world_id,reset_epoch FROM game_world_reset_epochs").await?;
                let visible_epochs: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_reset_active").fetch_one(&mut *legacy).await?;
                assert_eq!(visible_epochs, 0, "the legacy read snapshot predates activation");
                let stale = publish_in_transaction(&mut legacy, world, &catalogues).await.map_err(debug)?;
                let stale_xp = stale.iter().find(|s| s.category == 1).ok_or("missing stale cut")?;
                legacy.commit().await?;
                let activation_ms: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM activated_at)*1000)::bigint FROM game_world_reset_epochs").fetch_one(&h.pool).await?;
                assert!(stale_xp.computed_at_ms >= activation_ms);

                // Retain both snapshots but recreate the pre-2903 head shape before applying
                // the actual upgrade SQL in its own transaction, as the migrator does.
                h.pool.execute("ALTER TABLE game_world_highscore_heads DROP COLUMN published_reset_epoch").await?;
                let mut migration = h.pool.begin().await?;
                sqlx::raw_sql(include_str!("../../../migrations/2903_highscore_reset_epoch.sql")).execute(&mut *migration).await?;
                migration.commit().await?;

                h.root.await_ready().await?;
                let page = h.root.read_highscore_page(world, 1, None, 1, previous, character).await.map_err(debug)?;
                assert_eq!(page.snapshot.snapshot_id, previous);
                h.root.await_ready().await?;
                let refresh = h.root.refresh_highscore_schedule(world, catalogues.clone(), false).await.map_err(debug)?;
                assert_eq!(refresh.snapshots.len(), stale.len(), "the first scheduled pass must replace the unproven cut");
                h.root.await_ready().await?;
                let retained = h.root.read_highscore_page(world, 1, None, 1, stale_xp.snapshot_id, character).await.map_err(debug)?;
                assert_eq!(retained.snapshot.snapshot_id, stale_xp.snapshot_id);
                h.root.await_ready().await?;
                let repeat = h.root.refresh_highscore_schedule(world, catalogues, false).await.map_err(debug)?;
                assert!(repeat.snapshots.is_empty(), "the proven epoch needs only one cut");
                Ok(())
            }.await;
            h.cleanup().await?;
            result
        }).map_err(debug)
    })?.join().map_err(|_| "upgrade snapshot thread failed")?.map_err(Into::into)
}
