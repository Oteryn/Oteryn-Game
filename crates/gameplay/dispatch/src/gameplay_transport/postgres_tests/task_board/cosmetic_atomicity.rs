//! The paid receipt cannot commit if its whole appearance participant is omitted.
use super::*;

pub(super) async fn omission_case(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let points: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(id(41).as_slice()).bind(task_board::POINTS).fetch_one(&harness.pool).await?;
    let board: Value = sqlx::query_scalar(
        "SELECT body FROM game_character_task_boards WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&harness.pool)
    .await?;
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_task_board_receipts WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&harness.pool)
    .await?;
    let grants = receipt_count(harness).await?;
    // A missing participant can omit all three rows. Skipping only its source row
    // would exercise the appearance->source guard, not receipt->appearance.
    rollback_cases::start_fault_witness(harness).await?;
    sqlx::raw_sql("CREATE FUNCTION task_board_test_omit_cosmetic() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('task_board_test_fault_hits'); RETURN NULL; END $$; CREATE TRIGGER task_board_test_omit_cosmetic_source BEFORE INSERT ON game_task_board_cosmetic_grants FOR EACH ROW WHEN(NEW.offer_id=0) EXECUTE FUNCTION task_board_test_omit_cosmetic(); CREATE TRIGGER task_board_test_omit_cosmetic_receipt BEFORE INSERT ON game_account_appearance_unlock_receipts FOR EACH ROW WHEN(NEW.source_kind='task_board' AND NEW.unlock_key='oteryn:taskboard/shop/offer-0') EXECUTE FUNCTION task_board_test_omit_cosmetic(); CREATE TRIGGER task_board_test_omit_cosmetic_fact BEFORE INSERT ON game_account_appearance_unlocks FOR EACH ROW WHEN(NEW.unlock_key='oteryn:taskboard/shop/offer-0') EXECUTE FUNCTION task_board_test_omit_cosmetic();")
        .execute(&harness.pool).await?;
    let command = admitted.continuity.next_command_id;
    let attempted = serve_attempt(owner, admitted, command, &[full_request(16, 0, "")]).await;
    sqlx::raw_sql("DROP TRIGGER task_board_test_omit_cosmetic_source ON game_task_board_cosmetic_grants; DROP TRIGGER task_board_test_omit_cosmetic_receipt ON game_account_appearance_unlock_receipts; DROP TRIGGER task_board_test_omit_cosmetic_fact ON game_account_appearance_unlocks; DROP FUNCTION task_board_test_omit_cosmetic();")
        .execute(&harness.pool).await?;
    rollback_cases::finish_fault_witness(harness, 3).await?;
    let (end, replies) = attempted?;
    assert_unavailable(owner, end, &replies, admitted, command);
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(id(41).as_slice()).bind(task_board::POINTS).fetch_one(&harness.pool).await?, points);
    assert_eq!(
        sqlx::query_scalar::<_, Value>(
            "SELECT body FROM game_character_task_boards WHERE character_id=encode($1,'hex')::uuid"
        )
        .bind(id(41).as_slice())
        .fetch_one(&harness.pool)
        .await?,
        board
    );
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_task_board_receipts WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&harness.pool).await?, receipts);
    assert_eq!(receipt_count(harness).await?, grants);
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_task_board_cosmetic_grants WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&harness.pool).await?, 0);
    let unchanged = appearance(owner, admitted).await?;
    assert!(
        !unchanged
            .catalogue
            .outfits
            .iter()
            .any(|entry| entry.key == OUTFIT)
    );
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: cosmetic_receipt_requires_actual_appearance_participant"
    );
    Ok(())
}
