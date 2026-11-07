//! A successful claim cannot acknowledge a token without the existing ledger.
use super::*;
use oteryn_gameplay_quests::quest::task_board::full::{BountyStatus, BountyTask};

pub(super) async fn missing_participant(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &mut AdmittedSession,
) -> TestResult {
    full_board(owner, admitted, 4, 0, "").await?;
    let before = value_snapshot(harness).await?;
    let receipts = quest_receipts(harness).await?;
    let board_receipts = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_task_board_receipts")
        .fetch_one(&harness.pool)
        .await?;
    rollback_cases::start_fault_witness(harness).await?;
    sqlx::raw_sql("CREATE FUNCTION task_board_test_skip_token() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('task_board_test_fault_hits'); RETURN NULL; END $$;
        CREATE TRIGGER task_board_test_skip_token BEFORE INSERT ON game_task_board_token_sources FOR EACH ROW EXECUTE FUNCTION task_board_test_skip_token();
        CREATE TRIGGER task_board_test_skip_token BEFORE INSERT ON game_character_bounty_reroll_token_entries FOR EACH ROW EXECUTE FUNCTION task_board_test_skip_token();
        CREATE TRIGGER task_board_test_skip_token BEFORE INSERT OR UPDATE ON game_character_bounty_reroll_tokens FOR EACH ROW EXECUTE FUNCTION task_board_test_skip_token();")
        .execute(&harness.pool).await?;
    let command = admitted.continuity.next_command_id;
    let result = serve_attempt(owner, admitted, command, &[full_request(12, 0, "")]).await;
    sqlx::raw_sql(
        "DROP TRIGGER task_board_test_skip_token ON game_task_board_token_sources;
        DROP TRIGGER task_board_test_skip_token ON game_character_bounty_reroll_token_entries;
        DROP TRIGGER task_board_test_skip_token ON game_character_bounty_reroll_tokens;
        DROP FUNCTION task_board_test_skip_token();",
    )
    .execute(&harness.pool)
    .await?;
    rollback_cases::finish_fault_witness(harness, 3).await?;
    let (end, replies) = result?;
    assert_unavailable(owner, end, &replies, admitted, command);
    assert_eq!(quest_receipts(harness).await?, receipts);
    // An unavailable command remains outstanding. Read the durable owners directly;
    // this isolated scenario ends here instead of fabricating a resumable stream.
    assert_eq!(value_snapshot(harness).await?, before);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_task_board_receipts")
            .fetch_one(&harness.pool)
            .await?,
        board_receipts
    );
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: token_receipt_requires_existing_ledger_participant"
    );
    Ok(())
}

/// Stage an already completed offer from the production generator. For the boundary case
/// only, the historical ledger tail and its balance are staged together; every
/// subsequent claim runs with the normal source, continuity and receipt guards.
pub(super) async fn completed_fixture(
    harness: &Harness,
    balance: Option<i64>,
) -> TestResult<BountyTask> {
    let mut data = fixture_board_data(harness).await?;
    let mut task = data
        .bounty
        .offers
        .first()
        .cloned()
        .ok_or("generated bounty offer")?;
    task.current = task.required;
    data.bounty.offers.clear();
    data.bounty.active = Some(task.clone());
    data.bounty.state = BountyStatus::Completed;
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE game_character_task_boards SET body=$2 WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .bind(serde_json::to_value(data)?)
    .execute(&mut *tx)
    .await?;
    if let Some(balance) = balance {
        assert_eq!(sqlx::query("UPDATE game_character_bounty_reroll_token_entries e SET balance_before=$2-e.amount::bigint,balance_after=$2 FROM game_character_bounty_reroll_tokens b WHERE b.character_id=encode($1,'hex')::uuid AND b.world_id=encode($3,'hex')::uuid AND e.claim_id=b.last_claim_id AND e.previous_claim_id IS NOT NULL")
            .bind(id(41).as_slice()).bind(balance).bind(id(42).as_slice()).execute(&mut *tx).await?.rows_affected(),1);
        assert_eq!(sqlx::query("UPDATE game_character_bounty_reroll_tokens SET balance=$2 WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($3,'hex')::uuid")
            .bind(id(41).as_slice()).bind(balance).bind(id(42).as_slice()).execute(&mut *tx).await?.rows_affected(),1);
    }
    tx.commit().await?;
    Ok(task)
}

/// Compare actual value owners, including their append-only participants. Command
/// refusals may add their own receipt, but cannot change any of these values.
pub(super) async fn value_snapshot(harness: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('character_revision',(SELECT character_revision FROM game_character_roots WHERE character_id=p.character_id),'progression',to_jsonb(p),'board',b.body,'tracks',(SELECT jsonb_object_agg(track_key,value) FROM game_character_quest_tracks WHERE character_id=p.character_id),'tokens',(SELECT to_jsonb(t) FROM game_character_bounty_reroll_tokens t WHERE t.character_id=p.character_id AND t.world_id=b.world_id),'token_entries',(SELECT count(*) FROM game_character_bounty_reroll_token_entries WHERE character_id=p.character_id),'token_sources',(SELECT count(*) FROM game_task_board_token_sources WHERE character_id=p.character_id),'quest_receipts',(SELECT count(*) FROM game_character_quest_receipts WHERE character_id=p.character_id),'xp_receipts',(SELECT count(*) FROM game_character_xp_receipts WHERE character_id=p.character_id),'xp_obligations',(SELECT count(*) FROM game_character_quest_xp_obligations WHERE character_id=p.character_id)) FROM game_character_progression_state p JOIN game_character_task_boards b USING(character_id) WHERE p.character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&harness.pool).await?)
}

pub(super) async fn overflow_and_replay(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &mut AdmittedSession,
    action: u8,
) -> TestResult<wire::full::Reply> {
    let unchanged = value_snapshot(harness).await?;
    let command = admitted.continuity.next_command_id;
    let refused = full_board(owner, admitted, action, 0, "").await?;
    assert_eq!(refused.refusal, 11); // Rule::ArithmeticOverflow, a durable semantic refusal.
    assert_eq!(value_snapshot(harness).await?, unchanged);
    let stored:serde_json::Value=sqlx::query_scalar("SELECT refusal FROM game_task_board_receipts WHERE character_id=encode($1,'hex')::uuid AND cause_id=encode($2,'hex')::uuid AND cause_ordinal=$3::text::numeric AND cause_kind='command' AND committed_character_revision IS NULL AND effects='null'::jsonb")
        .bind(id(41).as_slice()).bind(admitted.game_session_id.as_bytes().as_slice()).bind(command.to_string())
        .fetch_one(&harness.pool).await?;
    assert_eq!(stored, serde_json::json!({"Rule":"ArithmeticOverflow"}));
    let replay = serve_from(owner, admitted, command, &[full_request(action, 0, "")]).await?;
    assert_eq!(replay[0].1, CommandStatus::DuplicateReplay);
    assert_eq!(
        wire::full::decode_reply(&replay[0].2).map_err(debug)?,
        refused
    );
    assert_eq!(value_snapshot(harness).await?, unchanged);
    Ok(refused)
}

pub(super) async fn bigint_boundary(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let task = completed_fixture(harness, Some(i64::MAX - 1)).await?;
    let before = full_board(owner, admitted, 4, 0, "").await?;
    assert_eq!(before.bounty.rerolls, u64::try_from(i64::MAX - 1)?);
    assert_eq!(before.bounty.state, 3);
    let before_xp = experience(harness).await?;
    let command = admitted.continuity.next_command_id;
    let claimed = full_board(owner, admitted, 9, 0, "").await?;
    assert_eq!(
        (
            claimed.refusal,
            claimed.bounty.state,
            claimed.bounty.rerolls
        ),
        (0, 1, i64::MAX.unsigned_abs())
    );
    assert_eq!(
        claimed.bounty_points,
        before.bounty_points + u64::from(task.points)
    );
    assert_eq!(
        (claimed.points, claimed.soulseals),
        (before.points, before.soulseals)
    );
    assert_eq!(
        experience(harness).await?,
        before_xp + i64::try_from(task.experience)?
    );
    let balance:i64=sqlx::query_scalar("SELECT balance FROM game_character_bounty_reroll_tokens WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).fetch_one(&harness.pool).await?;
    assert_eq!(balance, i64::MAX);
    let committed = value_snapshot(harness).await?;
    let replay = serve_from(owner, admitted, command, &[full_request(9, 0, "")]).await?;
    assert_eq!(replay[0].1, CommandStatus::DuplicateReplay);
    assert_eq!(
        wire::full::decode_reply(&replay[0].2).map_err(debug)?,
        claimed
    );
    assert_eq!(value_snapshot(harness).await?, committed);

    completed_fixture(harness, None).await?;
    let before = full_board(owner, admitted, 4, 0, "").await?;
    assert_eq!(
        (before.bounty.state, before.bounty.rerolls),
        (3, i64::MAX.unsigned_abs())
    );
    let mut expected = before;
    expected.refusal = 11;
    let refused = overflow_and_replay(owner, harness, admitted, 9).await?;
    assert_eq!(refused, expected);
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: bounty_token_bigint_boundary_and_overflow_refusal_replay"
    );
    Ok(())
}
