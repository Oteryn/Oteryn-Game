//! Publish the source room views from the same World bundle used by the Channel.
use super::TestResult;
use super::fixture::{Harness, debug, run};
use crate::content::bosses::activity_template::captured_boss_levers;
use crate::durability::activity_handoff::ActivityHandoffOwner;
use crate::foundation::MovementLocalPosition;

#[test]
fn ddepths_three_warzone_rooms_publish_with_one_landing_and_individual_capacity_postgres()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e::scenario_thread::run("ddepths-qualified-rooms", || {
        run(async |admin| {
            let scene = super::srx3_soul_war_world_fixture::source_world(MovementLocalPosition {
                x: 33461,
                y: 32267,
                floor: 15,
            })?;
            let candidates = captured_boss_levers().map_err(debug)?;
            let rows: Vec<_> = candidates
                .into_iter()
                .filter(|candidate| candidate.path.starts_with("content/encounters/bosses/warzone-"))
                .collect();
            assert_eq!(rows.len(), 3);
            let mut templates = Vec::with_capacity(rows.len());
            for candidate in rows {
                let template = scene
                    .controller
                    .qualify_activity_template(&scene.room, &scene.room, "content-1", &candidate)
                    .map_err(|error| format!("{}: {error:?}", candidate.path))?;
                assert_eq!(template.entry_positions().len(), 1);
                assert_eq!(template.minimum_players(), 1);
                assert!(template.maximum_players() >= 2);
                assert_eq!(template.cooldown_seconds(), 36_000);
                assert_eq!(template.time_limit_seconds(), 1_800);
                assert_eq!(template.after_kill_seconds(), 1_200);
                assert!(template.exit_teleporter().is_some());
                templates.push(template);
            }

            let harness = Harness::create_with_admission_seed(
                admin, "ddepths_qualified_rooms", false, false,
            ).await?;
            let result: TestResult = async {
                ActivityHandoffOwner::new(harness.root.clone())
                    .publish_qualified_boss_rooms(&harness.node, &templates)
                    .await?;
                let published: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM game_boss_room_publications WHERE boss_key = ANY($1)",
                )
                .bind(templates.iter().map(|room| room.boss_key()).collect::<Vec<_>>())
                .fetch_one(&harness.pool)
                .await?;
                assert_eq!(published, 3);
                Ok(())
            }.await;
            let cleanup = harness.cleanup().await;
            result?;
            cleanup
        })
    })
}
