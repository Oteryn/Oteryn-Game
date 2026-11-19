use super::TestResult;

pub(super) async fn lifetime_highscore_job(
    root: std::sync::Arc<crate::durability::DurabilityRoot>,
    pool: &sqlx::PgPool,
    world: crate::foundation::WorldId,
    character: oteryn_gameplay_domain::domain::CharacterId,
) -> TestResult {
    use crate::durability::highscore_snapshot::{
        self as hs, AchievementPoints, SnapshotCatalogues,
    };
    struct NoEarnedAchievements;
    impl AchievementPoints for NoEarnedAchievements {
        fn total_points(
            &self,
            earned: &[crate::durability::account_achievement::EarnedAchievement],
        ) -> Option<u32> {
            earned.is_empty().then_some(0)
        }
    }
    let started = std::time::Instant::now();
    let (allowed, legacy, cleanup): (bool, bool, bool) = sqlx::query_as("SELECT has_table_privilege('oteryn_game_highscore_job','game_tibiadrome_scores','SELECT'),has_table_privilege('oteryn_game_highscore_job','game_character_tibiadrome','SELECT'),EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version=2170 AND success)")
        .fetch_one(pool).await?;
    assert!(allowed, "the isolated job can read its actual score source");
    assert!(!legacy, "the job has no unused legacy-table grant");
    assert!(cleanup, "the real forward cleanup migration executed");
    let constraints: Vec<String> = sqlx::query_scalar("SELECT pg_get_constraintdef(oid) FROM pg_constraint WHERE conrelid IN ('game_world_highscore_snapshots'::regclass,'game_world_highscore_heads'::regclass,'game_public_highscores'::regclass) AND conname LIKE '%category_check' ORDER BY conname")
        .fetch_all(pool).await?;
    eprintln!(
        "SRX3_DROME ACTUAL_HIGHSCORE_DATABASE_PROOF source_select={allowed} legacy_select={legacy} forward_2170={cleanup} category_constraints={constraints:?}"
    );
    let catalogue = SnapshotCatalogues::activated(std::sync::Arc::new(NoEarnedAchievements))
        .map_err(|error| format!("{error:?}"))?;
    let job = std::sync::Arc::new(hs::job::HighscoreWorldJob::new(
        root.clone(),
        world,
        catalogue,
    ));
    let (resets, receiver) = tokio::sync::mpsc::channel(1);
    let runner_job = job.clone();
    let runner = tokio::spawn(async move { runner_job.run(receiver).await });
    let completed = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while job.completed_passes() == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await;
    drop(resets);
    runner.await?;
    completed?;
    let character = oteryn_protocol_oteryn::CharacterId::decode(character.as_bytes())
        .map_err(|error| format!("{error:?}"))?;
    let page = root
        .read_highscore_page(world, 14, None, 1, 0, character)
        .await
        .map_err(|error| format!("Drome lifetime job/query: {error:?}"))?;
    assert_eq!(page.rows.len(), 1);
    assert_eq!(
        (page.rows[0].character_id, page.rows[0].value),
        (character, (1, 0))
    );
    eprintln!(
        "SRX3_DROME POSTGRES_LIFETIME_REAL_WORLD_JOB_COMPLETED duration={:?}",
        started.elapsed()
    );
    Ok(())
}
