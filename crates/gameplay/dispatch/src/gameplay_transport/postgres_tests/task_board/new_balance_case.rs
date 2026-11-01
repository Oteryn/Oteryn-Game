//! Accepted balance widths are exercised through the actual claim/reset writer.
use super::token_case::{completed_fixture, overflow_and_replay, value_snapshot};
use super::*;
use oteryn_gameplay_quests::quest::task_board::full::{
    MAX_BOUNTY_POINTS, MAX_SOULSEALS, SOULSEALS, TaskBoardData,
};

async fn historical_board(harness: &Harness, data: &TaskBoardData) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    assert_eq!(sqlx::query("UPDATE game_character_task_boards SET body=$2 WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).bind(serde_json::to_value(data)?).execute(&mut *tx).await?.rows_affected(),1);
    tx.commit().await?;
    Ok(())
}

async fn points_and_seals(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &mut AdmittedSession,
    original: &TaskBoardData,
) -> TestResult {
    let period: i64 = sqlx::query_scalar("SELECT game_task_board_period(encode($1,'hex')::uuid)")
        .bind(id(42).as_slice())
        .fetch_one(&harness.pool)
        .await?;
    let mut pending = original.clone();
    let weekly = pending.weekly.as_mut().ok_or("generated weekly board")?;
    weekly.period = period - 7;
    weekly.finished = false;
    weekly.difficulty = 0;
    weekly.any_kills = weekly.any_required;
    for task in &mut weekly.kills {
        task.current = 0;
    }
    for task in &mut weekly.deliveries {
        task.delivered = false;
    }
    assert_eq!(weekly.rewards().map_err(debug)?, (25, 1));
    historical_board(harness, &pending).await?;
    fixture_track(harness, task_board::POINTS, 999_999_990).await?;
    fixture_track(harness, SOULSEALS, i64::from(MAX_SOULSEALS) - 1).await?;
    let before_xp = experience(harness).await?;
    let command = admitted.continuity.next_command_id;
    let settled = full_board(owner, admitted, 4, 0, "").await?;
    assert_eq!(
        (settled.refusal, settled.points, settled.soulseals),
        (0, 1_000_000_015, u64::from(MAX_SOULSEALS))
    );
    assert!(!settled.weekly.started);
    assert_eq!(
        experience(harness).await?,
        before_xp,
        "reset pays points and soulseals, not XP again"
    );
    let committed = value_snapshot(harness).await?;
    assert_eq!(
        committed["tracks"][task_board::POINTS].as_i64(),
        Some(1_000_000_015)
    );
    assert_eq!(
        committed["tracks"][SOULSEALS].as_i64(),
        Some(i64::from(MAX_SOULSEALS))
    );
    let replay = serve_from(owner, admitted, command, &[full_request(4, 0, "")]).await?;
    assert_eq!(replay[0].1, CommandStatus::DuplicateReplay);
    assert_eq!(
        wire::full::decode_reply(&replay[0].2).map_err(debug)?,
        settled
    );
    assert_eq!(value_snapshot(harness).await?, committed);

    // The next committed epoch has one more earned seal. The failed seal credit
    // must also roll back its otherwise affordable twenty-five hunting points.
    pending.weekly.as_mut().ok_or("weekly board")?.period = period;
    historical_board(harness, &pending).await?;
    fixture_next_reset(harness).await?;
    let refused = overflow_and_replay(owner, harness, admitted, 4).await?;
    assert_eq!(
        (refused.points, refused.soulseals),
        (settled.points, u64::from(MAX_SOULSEALS))
    );
    assert!(refused.weekly.started);
    assert_eq!(
        (
            refused.weekly.pending_points,
            refused.weekly.pending_soulseals
        ),
        (25, 1)
    );
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: hunting_points_cross_billion_and_soulseal_u32_boundary"
    );
    Ok(())
}

async fn bounty_points(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let task = completed_fixture(harness, None).await?;
    assert!(task.points > 0);
    fixture_track(
        harness,
        task_board::BOUNTY_POINTS,
        i64::from(MAX_BOUNTY_POINTS) - i64::from(task.points),
    )
    .await?;
    let before = full_board(owner, admitted, 4, 0, "").await?;
    assert!(
        before.bounty.rerolls < i64::MAX.unsigned_abs() - 1,
        "this case isolates the bounty-points bound"
    );
    let before_xp = experience(harness).await?;
    let command = admitted.continuity.next_command_id;
    let claimed = full_board(owner, admitted, 9, 0, "").await?;
    assert_eq!(
        (claimed.refusal, claimed.bounty_points),
        (0, u64::from(MAX_BOUNTY_POINTS))
    );
    assert_eq!(claimed.bounty.rerolls, before.bounty.rerolls + 1);
    assert_eq!(
        (claimed.points, claimed.soulseals),
        (before.points, before.soulseals)
    );
    assert_eq!(
        experience(harness).await?,
        before_xp + i64::try_from(task.experience)?
    );
    let committed = value_snapshot(harness).await?;
    assert_eq!(
        committed["tracks"][task_board::BOUNTY_POINTS].as_i64(),
        Some(i64::from(MAX_BOUNTY_POINTS))
    );
    let replay = serve_from(owner, admitted, command, &[full_request(9, 0, "")]).await?;
    assert_eq!(replay[0].1, CommandStatus::DuplicateReplay);
    assert_eq!(
        wire::full::decode_reply(&replay[0].2).map_err(debug)?,
        claimed
    );
    assert_eq!(value_snapshot(harness).await?, committed);

    completed_fixture(harness, None).await?;
    let mut expected = full_board(owner, admitted, 4, 0, "").await?;
    assert_eq!(
        (expected.bounty.state, expected.bounty_points),
        (3, u64::from(MAX_BOUNTY_POINTS))
    );
    expected.refusal = 11;
    assert_eq!(
        overflow_and_replay(owner, harness, admitted, 9).await?,
        expected
    );
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: bounty_points_u32_boundary_and_overflow_refusal_replay"
    );
    Ok(())
}

pub(super) async fn currency_boundaries(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let original = fixture_board_data(harness).await?;
    let balances = full_board(owner, admitted, 4, 0, "").await?;
    points_and_seals(owner, harness, admitted, &original).await?;
    // Restore only historical fixture inputs. Successful awards above and below
    // remain receipts; no production migration or pin is rewritten by this test.
    historical_board(harness, &original).await?;
    fixture_track(harness, task_board::POINTS, i64::try_from(balances.points)?).await?;
    fixture_track(harness, SOULSEALS, i64::try_from(balances.soulseals)?).await?;
    bounty_points(owner, harness, admitted).await?;
    historical_board(harness, &original).await?;
    fixture_track(
        harness,
        task_board::BOUNTY_POINTS,
        i64::try_from(balances.bounty_points)?,
    )
    .await?;
    Ok(())
}
