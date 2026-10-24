use super::*;
use crate::bestiary_postgres_harness as harness;
use crate::foundation::{CommandId, ConnectionGeneration};
use harness::{Harness, TestResult, WORLD, debug, id};

fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let Some(admin) = harness::configured_admin() else {
        return Ok(());
    };
    harness::runtime()?.block_on(async move {
        let h = Harness::create(admin, tag, true).await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&h.pool)
            .await?;
        let database: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(version, crate::CANONICAL_POSTGRES_SERVER_VERSION_NUM);
        eprintln!("DAILY-REWARD-PG database={database} server_version_num={version} test={tag}");
        let result = body(&h).await;
        h.cleanup().await?;
        eprintln!("DAILY-REWARD-PG cleanup test={tag}");
        result
    })
}

fn request(epoch: u64, transaction: u8) -> TestResult<DailyRewardClaimRequest> {
    Ok(DailyRewardClaimRequest {
        command: CommandRef::new(
            harness::item_fence()?.game_session_id,
            CommandId::new(u64::from(transaction)).map_err(debug)?,
        ),
        reset_epoch: epoch,
        transaction_id: id(transaction),
        request_binding: [transaction; 32],
    })
}

// Migration-owner fixtures provide committed history, never a runtime epoch producer.
async fn reset(h: &Harness, epoch: u64, timestamp: &str) -> TestResult {
    sqlx::query("INSERT INTO public.game_world_reset_history(world_id,reset_epoch,reset_day,reset_at_unix_ms,committed_at_unix_ms) SELECT encode($1,'hex')::uuid,$2::text::numeric(20,0),((t AT TIME ZONE 'Europe/Berlin')-INTERVAL '10 hours')::date-DATE '1970-01-01',floor(extract(epoch FROM t)*1000)::bigint,floor(extract(epoch FROM t)*1000)::bigint FROM (SELECT $3::text::timestamptz t) x")
        .bind(id(WORLD).as_slice()).bind(epoch.to_string()).bind(timestamp).execute(&h.pool).await?;
    Ok(())
}

async fn claim(
    h: &Harness,
    request: DailyRewardClaimRequest,
) -> TestResult<DailyRewardClaimOutcome> {
    claim_on(&h.root, h, request).await
}

async fn claim_on(
    root: &DurabilityRoot,
    h: &Harness,
    request: DailyRewardClaimRequest,
) -> TestResult<DailyRewardClaimOutcome> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = root.open_character_authority(&seal).await.map_err(debug)?;
    root.commit_daily_reward_core(&authority, &h.node, harness::item_fence()?, request)
        .await
        .map_err(|e| debug(e).into())
}

async fn projection(h: &Harness) -> TestResult<(i16, i32, String, i64)> {
    Ok(sqlx::query_as("SELECT lane_day,streak,last_claim_epoch::text,last_claim_day FROM game_character_daily_rewards WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(id(harness::CHARACTER).as_slice()).bind(id(WORLD).as_slice()).fetch_one(&h.pool).await?)
}

#[test]
fn daily_reward_postgres_claim_replay_gap_and_restart() -> TestResult {
    run("daily_claim", async |h| {
        reset(h, 1, "2026-10-22T08:00:00Z").await?;
        let first = request(1, 61)?;
        let DailyRewardClaimOutcome::Committed(receipt) = claim(h, first).await? else {
            return Err("first claim was not fresh".into());
        };
        assert_eq!(
            (
                receipt.lane_day,
                receipt.streak_before,
                receipt.streak_after,
                receipt.jokers_spent
            ),
            (1, 0, 1, 0)
        );
        let before = projection(h).await?;
        let mut retry = first;
        retry.transaction_id = id(62);
        retry.command = request(1, 62)?.command;
        assert_eq!(
            claim(h, retry).await?,
            DailyRewardClaimOutcome::AlreadyClaimed(receipt.clone())
        );
        assert_eq!(projection(h).await?, before);
        assert_eq!(h.count("game_character_daily_reward_claims").await?, 1);
        // A new Root and independently reconciled recovery recover the original receipt.
        let reopened = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(reopened.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = reopened
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert_eq!(
            reopened
                .commit_daily_reward_core(&authority, &h.node, harness::item_fence()?, retry)
                .await
                .map_err(debug)?,
            DailyRewardClaimOutcome::AlreadyClaimed(receipt)
        );
        reset(h, 2, "2026-10-23T08:00:00Z").await?;
        let DailyRewardClaimOutcome::Committed(second) = claim(h, request(2, 63)?).await? else {
            return Err("second claim was not fresh".into());
        };
        assert_eq!((second.lane_day, second.streak_after), (2, 2));
        reset(h, 5, "2026-10-26T09:00:00Z").await?;
        let DailyRewardClaimOutcome::Committed(gap) = claim(h, request(5, 64)?).await? else {
            return Err("gap claim was not fresh".into());
        };
        assert_eq!(
            (
                gap.lane_day,
                gap.streak_before,
                gap.streak_after,
                gap.jokers_spent
            ),
            (3, 2, 1, 0)
        );
        assert_eq!(projection(h).await?.0, 4);
        assert_eq!(h.root_revision().await?, "1");
        Ok(())
    })
}

#[test]
fn daily_reward_postgres_refusals_write_nothing() -> TestResult {
    run("daily_refusals", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let valid = harness::item_fence()?;
        // No history: restart or session generation cannot invent a window.
        assert!(matches!(
            h.root
                .commit_daily_reward_core(&authority, &h.node, valid, request(1, 61)?)
                .await,
            Err(DailyRewardClaimError::ResetWindowUnavailable)
        ));
        reset(h, 2, "2026-10-23T08:00:00Z").await?;
        for epoch in [1, 3, u64::MAX] {
            assert!(matches!(
                h.root
                    .commit_daily_reward_core(&authority, &h.node, valid, request(epoch, 61)?)
                    .await,
                Err(DailyRewardClaimError::ResetWindowUnavailable)
            ));
        }
        let mut stale = valid;
        stale.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        assert!(matches!(
            h.root
                .commit_daily_reward_core(&authority, &h.node, stale, request(2, 61)?)
                .await,
            Err(DailyRewardClaimError::AuthorityRejected)
        ));
        let mut wrong_session = request(2, 61)?;
        wrong_session.command = CommandRef::new(
            crate::foundation::GameSessionId::decode(&id(71)).map_err(debug)?,
            wrong_session.command.command_id(),
        );
        assert!(matches!(
            h.root
                .commit_daily_reward_core(&authority, &h.node, valid, wrong_session)
                .await,
            Err(DailyRewardClaimError::AuthorityRejected)
        ));
        assert_eq!(h.count("game_character_daily_rewards").await?, 0);
        assert_eq!(h.count("game_character_daily_reward_claims").await?, 0);
        let first = request(2, 61)?;
        claim(h, first).await?;
        let before = projection(h).await?;
        let mut different = first;
        different.request_binding[0] ^= 1;
        assert!(matches!(
            h.root
                .commit_daily_reward_core(&authority, &h.node, valid, different)
                .await,
            Err(DailyRewardClaimError::BindingMismatch)
        ));
        assert!(matches!(
            h.root
                .commit_daily_reward_core(&authority, &h.node, stale, first)
                .await,
            Err(DailyRewardClaimError::AuthorityRejected)
        ));
        assert_eq!(projection(h).await?, before);
        assert_eq!(h.count("game_character_daily_reward_claims").await?, 1);
        // Independently replace the persisted current generation. The original
        // otherwise-valid fence now names stale authority, including on replay.
        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=2 WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(valid.game_session_id.as_bytes().as_slice()).execute(&h.pool).await?;
        for candidate in [first, request(3, 62)?] {
            assert!(matches!(
                h.root
                    .commit_daily_reward_core(&authority, &h.node, valid, candidate)
                    .await,
                Err(DailyRewardClaimError::AuthorityRejected)
            ));
        }
        assert_eq!(projection(h).await?, before);
        assert_eq!(h.count("game_character_daily_reward_claims").await?, 1);
        Ok(())
    })
}

#[test]
fn daily_reward_postgres_concurrent_claim_serializes() -> TestResult {
    run("daily_race", async |h| {
        reset(h, 1, "2026-10-22T08:00:00Z").await?;
        // Each writer gets its own native Root/connection. One Root intentionally
        // refuses overlapping semantic passes before PostgreSQL is reached.
        let second_root = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(second_root.maintain_ready_once().await?);
        let first = request(1, 61)?;
        let mut retry = first;
        retry.transaction_id = id(62);
        retry.command = request(1, 62)?.command;
        use std::future::Future;
        use std::task::Poll;
        let mut left = std::pin::pin!(claim(h, first));
        let mut right = std::pin::pin!(claim_on(&second_root, h, retry));
        let (mut left_result, mut right_result) = (None, None);
        let (left, right) = std::future::poll_fn(|cx| {
            if left_result.is_none()
                && let Poll::Ready(value) = left.as_mut().poll(cx)
            {
                left_result = Some(value);
            }
            if right_result.is_none()
                && let Poll::Ready(value) = right.as_mut().poll(cx)
            {
                right_result = Some(value);
            }
            if left_result.is_some() && right_result.is_some() {
                Poll::Ready((left_result.take(), right_result.take()))
            } else {
                Poll::Pending
            }
        })
        .await;
        let (left, right) = (
            left.ok_or("missing left result")?,
            right.ok_or("missing right result")?,
        );
        let (left, right) = (left?, right?);
        let receipt = match (left, right) {
            (DailyRewardClaimOutcome::Committed(a), DailyRewardClaimOutcome::AlreadyClaimed(b))
            | (DailyRewardClaimOutcome::AlreadyClaimed(b), DailyRewardClaimOutcome::Committed(a)) =>
            {
                assert_eq!(a, b);
                a
            }
            other => return Err(format!("unexpected race: {other:?}").into()),
        };
        assert!([first.transaction_id, retry.transaction_id].contains(&receipt.transaction_id));
        assert_eq!(h.count("game_character_daily_reward_claims").await?, 1);
        assert_eq!(projection(h).await?.1, 1);
        Ok(())
    })
}

#[test]
fn daily_reward_postgres_insert_failure_rolls_back_state() -> TestResult {
    run("daily_rollback", async |h| {
        reset(h, 1, "2026-10-22T08:00:00Z").await?;
        claim(h, request(1, 61)?).await?;
        let before = projection(h).await?;
        reset(h, 2, "2026-10-23T08:00:00Z").await?;
        // Reusing a TransactionId at a different epoch fails *after* the attempted
        // state update; the native runner must roll that update back as well.
        assert!(claim(h, request(2, 61)?).await.is_err());
        assert_eq!(projection(h).await?, before);
        assert_eq!(h.count("game_character_daily_reward_claims").await?, 1);
        assert!(matches!(
            claim(h, request(2, 62)?).await?,
            DailyRewardClaimOutcome::Committed(_)
        ));
        Ok(())
    })
}

#[test]
fn daily_reward_postgres_schema_constraints_privileges_and_dst() -> TestResult {
    run("daily_schema", async |h| {
        // PostgreSQL's timezone database resolves both DST transitions before
        // the pure civil-day rule runs. The UTC reset hour changes with Berlin.
        for (timestamp, delta) in [
            ("2026-03-28T08:59:59.999Z", -1_i64),
            ("2026-03-28T09:00:00Z", 0),
            ("2026-03-29T07:59:59.999Z", -1),
            ("2026-03-29T08:00:00Z", 0),
            ("2026-10-24T07:59:59.999Z", -1),
            ("2026-10-24T08:00:00Z", 0),
            ("2026-10-25T08:59:59.999Z", -1),
            ("2026-10-25T09:00:00Z", 0),
        ] {
            let (day, seconds): (i64, i64) = sqlx::query_as("SELECT (t::date-DATE '1970-01-01')::bigint,floor(extract(epoch FROM t::time))::bigint FROM (SELECT $1::text::timestamptz AT TIME ZONE 'Europe/Berlin' t) x")
                .bind(timestamp).fetch_one(&h.pool).await?;
            assert_eq!(
                daily_reward_reset_day(day, u32::try_from(seconds)?).map_err(debug)?,
                day + delta
            );
        }
        reset(h, 1, "2026-10-22T08:00:00Z").await?;
        assert!(reset(h, 2, "2026-10-22T09:00:00Z").await.is_err());
        assert!(reset(h, 0, "2026-10-23T08:00:00Z").await.is_err());
        claim(h, request(1, 61)?).await?;
        let before = projection(h).await?;
        for query in [
            "UPDATE game_character_daily_rewards SET lane_day=8",
            "UPDATE game_character_daily_rewards SET streak=100001",
            "UPDATE game_character_daily_rewards SET streak=0",
            "UPDATE game_character_daily_rewards SET last_claim_epoch=NULL,last_claim_day=NULL,streak=1",
            "UPDATE game_character_daily_rewards SET last_claim_epoch=0",
            "UPDATE game_character_daily_rewards SET last_claim_day=last_claim_day+1",
            "UPDATE game_character_daily_rewards SET streak=2",
            "UPDATE game_character_daily_reward_claims SET streak_after=2",
            "DELETE FROM game_character_daily_reward_claims",
            "TRUNCATE game_character_daily_reward_claims CASCADE",
            "DELETE FROM game_character_daily_rewards",
            "UPDATE game_world_reset_history SET reset_epoch=2",
            "DELETE FROM game_world_reset_history",
        ] {
            assert!(
                sqlx::query(query).execute(&h.pool).await.is_err(),
                "accepted forbidden mutation: {query}"
            );
        }
        assert_eq!(projection(h).await?, before);
        // Runtime has only the prescribed grants; the reset owner has not yet
        // been composed, so there is no runtime history publication privilege.
        for query in [
            "DELETE FROM game_character_daily_rewards",
            "UPDATE game_character_daily_reward_claims SET streak_after=2",
            "INSERT INTO game_world_reset_history SELECT world_id,2,reset_day+1,reset_at_unix_ms+86400000,committed_at_unix_ms+86400000 FROM game_world_reset_history",
        ] {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *tx)
                .await?;
            let error = sqlx::query(query)
                .execute(&mut *tx)
                .await
                .err()
                .ok_or("missing runtime privilege refusal")?;
            assert_eq!(
                error
                    .as_database_error()
                    .and_then(|error| error.code())
                    .as_deref(),
                Some("42501")
            );
            tx.rollback().await?;
        }
        reset(h, u64::MAX, "2026-10-23T08:00:00Z").await?;
        let DailyRewardClaimOutcome::Committed(last) = claim(h, request(u64::MAX, 62)?).await?
        else {
            return Err("full-u64 claim was not fresh".into());
        };
        assert_eq!(
            (last.reset_epoch, last.streak_after, last.jokers_spent),
            (u64::MAX, 1, 0)
        );
        Ok(())
    })
}
