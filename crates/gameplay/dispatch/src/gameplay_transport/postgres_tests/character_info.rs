//! CHARACTER-INFO-1 installed-schema qualification for durable unlock/select.
use super::fixture::{Harness, run};
use super::{TestResult, prepare_current_session};

#[test]
fn character_title_unlock_and_selection_are_durable_and_referential() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "character_title", false, false).await?;
        prepare_current_session(&harness).await?;
        let character: String =
            sqlx::query_scalar("SELECT character_id::text FROM game_character_roots LIMIT 1")
                .fetch_one(&harness.pool)
                .await?;
        sqlx::query("INSERT INTO game_character_titles(character_id,title,earned_at_unix_ms) VALUES($1::uuid,1,100)").bind(&character).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_character_title_selection(character_id,title,revision) VALUES($1::uuid,1,1)").bind(&character).execute(&harness.pool).await?;
        let selected: Option<i16> = sqlx::query_scalar(
            "SELECT title FROM game_character_title_selection WHERE character_id=$1::uuid",
        )
        .bind(&character)
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(selected, Some(1));
        let invalid = sqlx::query(
            "UPDATE game_character_title_selection SET title=2 WHERE character_id=$1::uuid",
        )
        .bind(&character)
        .execute(&harness.pool)
        .await;
        assert!(invalid.is_err(), "selection must reference an earned title");
        Ok(())
    })
}
