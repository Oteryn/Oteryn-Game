//! Creation and upgrade coverage through the native highscore publisher.
use super::*;
use durability::account_achievement::EarnedAchievement;
use durability::character_progression::{INITIAL_CHARACTER_LEVEL, INITIAL_TOTAL_EXPERIENCE};
use durability::highscore_snapshot::{AchievementPoints, SnapshotCatalogues, SnapshotRow};
use foundation::{CharacterId as RankingCharacterId, WorldId as RankingWorldId};
use oteryn_gameplay_durability::GAME_MIGRATOR;
use std::sync::Arc;

const MIGRATION: i64 = 3322;

#[test]
fn freshly_bootstrapped_character_ranks_with_baseline_progression_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "fresh_highscores", None, async |db| {
        let pool = sqlx::PgPool::connect(&db.url).await?;
        let root = DurabilityRoot::connect_test_runtime(&db.url)?;
        assert!(root.maintain_ready_once().await?);
        let retained = fence_parent().join(&db.name);
        std::fs::create_dir(&retained)?;
        let result = async {
            let recovery = CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
                .map_err(|e| format!("{e:?}"))?;
            let fresh = recovery
                .authorize_fresh_store(id(11), 100)
                .map_err(|e| format!("{e:?}"))?;
            root.admit_fresh_character_recovery(&fresh)
                .await
                .map_err(|e| format!("{e:?}"))?;
            drop(fresh);
            let seal = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
            let authority = root
                .open_character_authority(&seal)
                .await
                .map_err(|e| format!("{e:?}"))?;
            let node = register(&root, 1, None).await?;
            initialize_s2(&root, &node).await?;
            assign_world(&pool, &root, &node, 90, 95).await?;
            configure(&pool, ["profile-1", "ruleset-1", "content-1", "starter-1"]).await?;
            allow(&root, &node, 31).await?;
            let created = root
                .bootstrap_character(&authority, &node, &intent(21, 31, 1)?)
                .await
                .map_err(|e| format!("{e:?}"))?;

            assert!(
                root.read_character_progression(&authority, created.character_id)
                    .await?
                    .is_none()
            );
            let ranked = RankingCharacterId::decode(created.character_id.as_bytes())?;
            let rows = publish(&root, ranked).await?;
            assert_eq!(rows.len(), 1);
            assert_baseline(&rows[0], ranked)?;
            Ok(())
        }
        .await;
        std::fs::remove_dir_all(retained)?;
        result
    })
}

#[test]
fn upgraded_highscores_publish_saved_and_never_saved_characters_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "mixed_highscores", Some(MIGRATION - 1), async |db| {
        let mut connection = sqlx::PgConnection::connect(&db.url).await?;
        // Import two revision-one roots using the pre-upgrade creation trigger.
        // Only the saved root has a policy-bound progression row.
        let mut tx = connection.begin().await?;
        sqlx::query(
            "INSERT INTO game_character_account_guards(account_id) VALUES ($1::text::uuid)",
        )
        .bind(uuid(id(31)))
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name,creation_vocation)
            VALUES ($1::text::uuid,$3::text::uuid,$4::text::uuid,1,1,'profile-1','ruleset-1','content-1','starter-1','Saved Hero','knight'),
                   ($2::text::uuid,$3::text::uuid,$4::text::uuid,1,1,'profile-1','ruleset-1','content-1','starter-1','New Hero','druid')")
            .bind(uuid(id(21))).bind(uuid(id(22))).bind(uuid(id(31))).bind(uuid(id(90)))
            .execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_progression_state(character_id,character_revision,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision)
            VALUES ($1::text::uuid,1,2,100,'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1','policy-1','reward-1')")
            .bind(uuid(id(21))).execute(&mut *tx).await?;
        tx.commit().await?;

        GAME_MIGRATOR.run(&mut connection).await?;
        let migration = GAME_MIGRATOR
            .iter()
            .find(|row| row.version == MIGRATION)
            .ok_or("missing creation progression migration")?;
        let mut tx = connection.begin().await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(migration.sql.as_ref()))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        let seeded: Vec<String> = sqlx::query_scalar("SELECT character_id::text FROM game_character_creation_progression ORDER BY character_id LIMIT 3")
            .fetch_all(&mut connection).await?;
        assert_eq!(seeded, vec![uuid(id(22))]);
        connection.close().await?;

        let root = DurabilityRoot::connect_test_runtime(&db.url)?;
        assert!(root.maintain_ready_once().await?);
        let saved = RankingCharacterId::decode(&id(21))?;
        let rows = publish(&root, saved).await?;
        assert_eq!(rows.len(), 2);
        assert_eq!(
            (rows[0].character_id, rows[0].level, rows[0].value),
            (saved, 2, (100, 0))
        );
        assert_baseline(&rows[1], RankingCharacterId::decode(&id(22))?)?;
        Ok(())
    })
}

fn run(
    admin: String,
    tag: &'static str,
    through: Option<i64>,
    exercise: impl AsyncFnOnce(&Database) -> TestResult + Send + 'static,
) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            (|| -> TestResult {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?
                    .block_on(async {
                        let db = Database::create_at(admin, tag, through).await?;
                        let result = exercise(&db).await;
                        db.cleanup().await?;
                        if result.is_ok() {
                            eprintln!("CREATION_HIGHSCORES_PG_EXECUTED fixture={tag}");
                        }
                        result
                    })
            })()
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "creation highscores worker panicked")?
        .map_err(Into::into)
}

struct EmptyAchievements;

impl AchievementPoints for EmptyAchievements {
    fn total_points(&self, earned: &[EarnedAchievement]) -> Option<u32> {
        earned.is_empty().then_some(0)
    }
}

async fn publish(
    root: &DurabilityRoot,
    asking: RankingCharacterId,
) -> TestResult<Vec<SnapshotRow>> {
    let world = RankingWorldId::decode(&id(90))?;
    root.refresh_highscore_schedule(
        world,
        SnapshotCatalogues::activated(Arc::new(EmptyAchievements)).map_err(|e| format!("{e:?}"))?,
        true,
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    Ok(root
        .read_highscore_page(world, 1, None, 1, 0, asking)
        .await
        .map_err(|e| format!("{e:?}"))?
        .rows)
}

fn assert_baseline(row: &SnapshotRow, character_id: RankingCharacterId) -> TestResult {
    assert_eq!(row.character_id, character_id);
    assert_eq!(row.level, INITIAL_CHARACTER_LEVEL);
    assert_eq!(row.value, (u64::try_from(INITIAL_TOTAL_EXPERIENCE)?, 0));
    Ok(())
}
