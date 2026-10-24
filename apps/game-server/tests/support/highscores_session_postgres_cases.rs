//! RANK-1: a real fresh-admission commit reaches the dispatched session highscores handler.

use crate::authority_matrix::{TestResult, checked};
use crate::durability::account_achievement::EarnedAchievement;
use crate::durability::admission_authority_guards::AdmissionGuardStore;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::highscore_snapshot::reply::{HighscoresReply, answer_session_highscores};
use crate::durability::highscore_snapshot::{AchievementPoints, SnapshotCatalogues};
use crate::durability::{DurabilityRoot, MigrationExecutor};
use crate::foundation::admission_authority_publication::AdmissionAuthorityPublicationV1;
use crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
use crate::{authority_matrix, postgres, postgres_clock, postgres_e2e_is_configured};
use oteryn_protocol_oteryn::highscores::{HighscoresQuery, decode_highscores_result};
use std::sync::Arc;

/// The empty cut has no earned facts; an unexpected achievement fails closed.
struct EmptyAchievementPoints;

impl AchievementPoints for EmptyAchievementPoints {
    fn total_points(&self, earned: &[EarnedAchievement]) -> Option<u32> {
        earned.is_empty().then_some(0)
    }
}

#[test]
fn committed_fresh_session_reads_its_world_highscores_through_the_session_handler() -> TestResult<()>
{
    if !postgres_e2e_is_configured()? {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let database = postgres::IsolatedPostgres::create("highscores_session").await?;
            let result = async {
                let url = database.database_url()?;
                MigrationExecutor::connect_migration(&url)
                    .await?
                    .apply_embedded_ledger()
                    .await?;
                let pool = sqlx::PgPool::connect(&url).await?;
                let now = postgres_clock(&pool).await?;
                let owner = postgres::fresh::Source::new(now)?;
                let guards = AdmissionGuardStore::connect_runtime(&url, 8192).await?;
                guards
                    .publish(&checked(AdmissionAuthorityPublicationV1::prepare(
                        &owner, now,
                    ))?)
                    .await?;
                let fresh = FreshAdmissionStore::connect_runtime(&url, 65536, 8192).await?;
                let request = owner.request()?;
                let FreshAdmissionDurableOutcomeV1::Committed(receipt) =
                    fresh.commit(&request).await?
                else {
                    return Err("fresh session was not committed".into());
                };
                let committed = checked(receipt.binding().initial_commit())?;
                let world = committed.world_id();
                let session = committed.game_session_id();
                assert_eq!(committed.character_id(), owner.current.character_id);
                assert_eq!(world, owner.current.world_id);

                let root = DurabilityRoot::connect_test_runtime(&url)?;
                assert!(root.maintain_ready_once().await?);
                let catalogues = SnapshotCatalogues::activated(Arc::new(EmptyAchievementPoints))
                    .map_err(|error| format!("{error:?}"))?;
                let published = root
                    .refresh_highscore_schedule(world, catalogues, true)
                    .await
                    .map_err(|error| format!("{error:?}"))?;
                let snapshot = published
                    .snapshots
                    .iter()
                    .find(|snapshot| snapshot.category == 1)
                    .ok_or("experience snapshot was not published")?;
                let query = || HighscoresQuery {
                    category: 1,
                    vocation: 0,
                    page: 1,
                    snapshot_id: 0,
                };
                let HighscoresReply::Result(payload) =
                    answer_session_highscores(&root, world, world, session, query()).await
                else {
                    return Err("committed session highscores query was not accepted".into());
                };
                let decoded =
                    decode_highscores_result(&payload).map_err(|error| format!("{error:?}"))?;
                assert_ne!(decoded.snapshot_id, 0);
                assert_eq!(decoded.snapshot_id, snapshot.snapshot_id);
                assert_eq!(decoded.category, 1);
                let foreign_world = authority_matrix::world(90)?;
                assert_ne!(foreign_world, world);
                assert_eq!(
                    answer_session_highscores(&root, foreign_world, world, session, query()).await,
                    HighscoresReply::Rejected
                );
                pool.close().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            database.cleanup().await?;
            result
        })
}
