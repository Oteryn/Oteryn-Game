//! The Quest writer reads immutable personal marks with the runtime role.
use super::{TestResult, fixture::{Harness, run}};
#[test]
fn review5_runtime_role_can_lock_personal_map_marks() -> TestResult {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "review5_map_marks", false, false).await?;
            let result = async {
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
                let role: String = sqlx::query_scalar("SELECT current_user").fetch_one(&mut *tx).await?;
                assert_eq!(role, "oteryn_game_runtime");
                sqlx::query("SELECT character_id FROM game_character_quest_map_marks LIMIT 1 FOR SHARE")
                    .fetch_all(&mut *tx).await?;
                tx.rollback().await?;
                Ok::<(), Box<dyn std::error::Error>>(())
            }.await;
            h.cleanup().await?;
            result
        }).map_err(|error| error.to_string())
    })?.join().map_err(|_| "runtime map mark worker panicked")?.map_err(Into::into)
}
