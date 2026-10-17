//! ACH batch PostgreSQL cases mounted only by the existing PG roots.
//! No PG tests are included through the production owner or library unit lane.
#![allow(clippy::expect_used)]

use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, TestResult, configured_admin, debug, fence, id, runtime,
};
use crate::domain::CharacterId;
use crate::durability::DurabilityRoot;
use crate::durability::account_achievement::granting_harness::begin_test_root_locked_grants;
use crate::durability::account_achievement::{
    AchievementCatalogueLookup, AchievementGrantError, AchievementGrantOutcome,
    AchievementGrantRequest, AchievementSourceEvent, MAX_ACHIEVEMENT_GRANTS,
};

const COOKIES: &str = "oteryn:achievement/allow_cookies";
const NUT: &str = "oteryn:achievement/the_professor_s_nut";

fn grant(key: &str, event: u8) -> AchievementGrantRequest {
    AchievementGrantRequest {
        achievement_key: key.into(),
        catalogue: AchievementCatalogueLookup::Earnable {
            revision: "r1".into(),
        },
        source: AchievementSourceEvent {
            kind: "oteryn:test-granter".into(),
            event_id: vec![event],
        },
    }
}

async fn counts(harness: &Harness) -> TestResult<(i64, i64)> {
    Ok((
        harness
            .count("game_account_achievement_grant_requests")
            .await?,
        harness.count("game_account_achievements").await?,
    ))
}

fn run(
    tag: &'static str,
    case: impl AsyncFnOnce(&Harness, &DurabilityRoot) -> TestResult,
) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Err("canonical PostgreSQL17.6 must be configured".into());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, tag, false).await?;
        let server: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&harness.pool)
            .await?;
        assert_eq!(
            server, "170006",
            "canonical qualification is PostgreSQL17.6"
        );
        eprintln!("ACHIEVEMENT-BATCH-NATIVE-PG server_version_num={server} fixture={tag}");
        let root = harness.root.clone();
        let result = case(&harness, &root).await;
        let cleanup = harness.cleanup().await;
        result?;
        cleanup
    })
}

#[test]
fn achievement_batch_processes_source_order_and_returns_input_order() -> TestResult {
    run("ach_batch_order", async |harness, root| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        let requests = vec![grant(COOKIES, 90), grant(NUT, 70), grant(COOKIES, 60)];
        let outcomes = root
            .commit_test_achievement_grants(&authority, &harness.node, fence(1)?, requests.clone())
            .await
            .map_err(debug)?;
        let [
            AchievementGrantOutcome::AlreadyHeld(held),
            AchievementGrantOutcome::Granted(nut),
            AchievementGrantOutcome::Granted(first),
        ] = outcomes.as_slice()
        else {
            return Err(format!("unexpected outcomes: {outcomes:?}").into());
        };
        assert_eq!(held, first);
        assert_eq!(first.source, requests[2].source);
        assert_eq!(nut.source, requests[1].source);
        assert_eq!(counts(harness).await?, (3, 2));
        assert_eq!(harness.root_revision().await?, "1");
        Ok(())
    })
}

#[test]
fn achievement_batch_bounds_and_duplicate_sources_fail_without_writes() -> TestResult {
    run("ach_batch_bounds", async |harness, root| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        let mut changed_revision = grant(COOKIES, 60);
        changed_revision.catalogue = AchievementCatalogueLookup::Earnable {
            revision: "r2".into(),
        };
        let oversized = (0..=MAX_ACHIEVEMENT_GRANTS)
            .map(|index| {
                let mut request = grant(COOKIES, 60);
                request.source.event_id = index.to_be_bytes().to_vec();
                request
            })
            .collect();
        for requests in [
            vec![],
            oversized,
            vec![grant(COOKIES, 60), changed_revision],
        ] {
            let result = root
                .commit_test_achievement_grants(&authority, &harness.node, fence(1)?, requests)
                .await;
            assert!(
                matches!(result, Err(AchievementGrantError::InvalidInput)),
                "{result:?}"
            );
            assert_eq!(counts(harness).await?, (0, 0));
        }
        let maximal = (0..MAX_ACHIEVEMENT_GRANTS)
            .map(|index| {
                let mut request = grant(COOKIES, 60);
                request.source.event_id = index.to_be_bytes().to_vec();
                request
            })
            .collect();
        let outcomes = root
            .commit_test_achievement_grants(&authority, &harness.node, fence(1)?, maximal)
            .await
            .map_err(debug)?;
        assert_eq!(outcomes.len(), MAX_ACHIEVEMENT_GRANTS);
        assert!(matches!(
            outcomes.first(),
            Some(AchievementGrantOutcome::Granted(_))
        ));
        assert!(
            outcomes[1..]
                .iter()
                .all(|outcome| matches!(outcome, AchievementGrantOutcome::AlreadyHeld(_)))
        );
        assert_eq!(counts(harness).await?, (64, 1));
        Ok(())
    })
}

#[test]
fn achievement_batch_late_unknown_or_invalid_request_rolls_back_the_earning_call() -> TestResult {
    run("ach_batch_invalid", async |harness, root| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        let mut unknown = grant(NUT, 90);
        unknown.catalogue = AchievementCatalogueLookup::Absent;
        let result = root
            .commit_test_achievement_grants(
                &authority,
                &harness.node,
                fence(1)?,
                vec![grant(COOKIES, 60), unknown],
            )
            .await;
        assert!(
            matches!(result, Err(AchievementGrantError::UnknownAchievement)),
            "{result:?}"
        );
        assert_eq!(counts(harness).await?, (0, 0));
        let mut invalid = grant(NUT, 90);
        invalid.source.event_id.clear();
        let result = root
            .commit_test_achievement_grants(
                &authority,
                &harness.node,
                fence(1)?,
                vec![grant(COOKIES, 60), invalid],
            )
            .await;
        assert!(
            matches!(result, Err(AchievementGrantError::InvalidInput)),
            "{result:?}"
        );
        assert_eq!(counts(harness).await?, (0, 0));
        Ok(())
    })
}

#[test]
fn achievement_batch_late_source_conflict_rolls_back_earlier_insert() -> TestResult {
    run("ach_batch_conflict", async |harness, root| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        root.commit_test_achievement_grants(
            &authority,
            &harness.node,
            fence(1)?,
            vec![grant(COOKIES, 90)],
        )
        .await
        .map_err(debug)?;
        let mut conflict = grant(COOKIES, 90);
        conflict.catalogue = AchievementCatalogueLookup::Earnable {
            revision: "r2".into(),
        };
        let result = root
            .commit_test_achievement_grants(
                &authority,
                &harness.node,
                fence(1)?,
                vec![conflict, grant(NUT, 60)],
            )
            .await;
        assert!(
            matches!(result, Err(AchievementGrantError::ConflictingSourceEvent)),
            "{result:?}"
        );
        // NUT sorts before the conflicting request and really inserted into
        // this transaction; the caller's drop must discard it as well.
        assert_eq!(counts(harness).await?, (1, 1));
        Ok(())
    })
}

#[test]
fn achievement_batch_concurrent_reversed_inputs_keep_one_fact_per_key() -> TestResult {
    run("ach_batch_concurrent", async |harness, root| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        // The native test root has one pool connection. A second root makes
        // these genuine competing DB transactions rather than pool queuing.
        let competing = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
        assert!(competing.maintain_ready_once().await?);
        let competing_authority = competing
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let requests = vec![grant(COOKIES, 60), grant(NUT, 70)];
        let reversed = requests.iter().cloned().rev().collect();
        let current = fence(1)?;
        use std::future::{Future, poll_fn};
        use std::task::Poll;
        let mut left = std::pin::pin!(root.commit_test_achievement_grants(
            &authority,
            &harness.node,
            current,
            requests,
        ));
        let mut right = std::pin::pin!(competing.commit_test_achievement_grants(
            &competing_authority,
            &harness.node,
            current,
            reversed,
        ));
        let (mut left_result, mut right_result) = (None, None);
        let (left, right) = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            poll_fn(|context| {
                if left_result.is_none()
                    && let Poll::Ready(value) = left.as_mut().poll(context)
                {
                    left_result = Some(value);
                }
                if right_result.is_none()
                    && let Poll::Ready(value) = right.as_mut().poll(context)
                {
                    right_result = Some(value);
                }
                if left_result.is_some() && right_result.is_some() {
                    Poll::Ready((left_result.take(), right_result.take()))
                } else {
                    Poll::Pending
                }
            }),
        )
        .await?;
        let left = left.ok_or("left grant did not complete")?.map_err(debug)?;
        let right = right
            .ok_or("right grant did not complete")?
            .map_err(debug)?;
        let outcomes: Vec<_> = left.iter().chain(&right).collect();
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| matches!(outcome, AchievementGrantOutcome::Granted(_)))
                .count(),
            2
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| matches!(outcome, AchievementGrantOutcome::AlreadyHeld(_)))
                .count(),
            2
        );
        assert_eq!(counts(harness).await?, (2, 2));
        Ok(())
    })
}

/// Codex P1 on #2154: two Characters of one account whose batches would
/// insert overlapping fact keys in opposite orders. The second batch must
/// wait on the account's achievement lock before any request or fact row,
/// never on a fact row the first holds (the 40P01 cycle).
#[test]
fn achievement_batch_two_characters_of_one_account_serialize_on_the_account_lock() -> TestResult {
    run("ach_batch_account", async |harness, _root| {
        let first = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let second = CharacterId::from_bytes(id(51)).map_err(debug)?;
        sqlx::query(
            "INSERT INTO game_character_roots(character_id, account_id, world_id, lifecycle, \
               character_revision, profile_revision, ruleset_revision, content_revision, \
               starter_template_revision, name) \
             SELECT encode($1,'hex')::uuid, account_id, world_id, 1, 1, profile_revision, \
                    ruleset_revision, content_revision, starter_template_revision, 'Fixture Second' \
               FROM game_character_roots WHERE character_id = encode($2,'hex')::uuid",
        )
        .bind(second.as_bytes().as_slice())
        .bind(first.as_bytes().as_slice())
        .execute(&harness.pool)
        .await?;

        // Source order writes NUT then COOKIES for the first Character ...
        let (held, outcomes) = begin_test_root_locked_grants(
            &harness.pool,
            first,
            &[grant(NUT, 1), grant(COOKIES, 2)],
        )
        .await
        .map_err(debug)?;
        let outcomes = outcomes.map_err(debug)?;
        assert!(
            outcomes
                .iter()
                .all(|outcome| matches!(outcome, AchievementGrantOutcome::Granted(_))),
            "{outcomes:?}"
        );
        // ... and COOKIES then NUT for the second.
        let pool = harness.pool.clone();
        let competing = tokio::spawn(async move {
            let requests = [grant(NUT, 4), grant(COOKIES, 3)];
            let (tx, outcomes) = begin_test_root_locked_grants(&pool, second, &requests).await?;
            tx.commit().await?;
            Ok::<_, crate::durability::DurabilityError>(outcomes)
        });

        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let waiting: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM pg_locks \
                  WHERE NOT granted AND database = \
                        (SELECT oid FROM pg_database WHERE datname = current_database()) \
                    AND locktype = 'advisory'",
            )
            .fetch_one(&harness.pool)
            .await?;
            if waiting == 1 {
                break;
            }
            if tokio::time::Instant::now() >= deadline || competing.is_finished() {
                return Err("second Character did not wait on the account lock".into());
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        held.commit().await?;

        let outcomes = tokio::time::timeout(std::time::Duration::from_secs(10), competing)
            .await?
            .map_err(debug)?
            .map_err(debug)?
            .map_err(debug)?;
        let [
            AchievementGrantOutcome::AlreadyHeld(nut),
            AchievementGrantOutcome::AlreadyHeld(cookies),
        ] = outcomes.as_slice()
        else {
            return Err(format!("unexpected outcomes: {outcomes:?}").into());
        };
        assert_eq!(nut.character_id, first);
        assert_eq!(cookies.character_id, first);
        assert_eq!(counts(harness).await?, (4, 2));
        Ok(())
    })
}

#[test]
fn achievement_batch_reopens_durable_facts_under_current_connection_generation() -> TestResult {
    run("ach_batch_reload", async |harness, root| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        let requests = vec![grant(COOKIES, 60), grant(NUT, 70)];
        let committed = root
            .commit_test_achievement_grants(&authority, &harness.node, fence(1)?, requests.clone())
            .await
            .map_err(debug)?;
        let reopened = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
        assert!(reopened.maintain_ready_once().await?);
        let current_authority = reopened
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation = 2")
            .execute(&harness.pool)
            .await?;
        let stale = reopened
            .commit_test_achievement_grants(
                &current_authority,
                &harness.node,
                fence(1)?,
                requests.clone(),
            )
            .await;
        assert!(
            matches!(stale, Err(AchievementGrantError::AuthorityRejected)),
            "{stale:?}"
        );
        let mut current = fence(1)?;
        current.connection_generation =
            crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
        let replay = reopened
            .commit_test_achievement_grants(&current_authority, &harness.node, current, requests)
            .await
            .map_err(debug)?;
        let expected: Vec<_> = committed
            .into_iter()
            .map(|outcome| match outcome {
                AchievementGrantOutcome::Granted(fact) => {
                    AchievementGrantOutcome::AlreadyHeld(fact)
                }
                other => other,
            })
            .collect();
        assert_eq!(replay, expected);
        assert_eq!(counts(harness).await?, (2, 2));
        Ok(())
    })
}

#[test]
fn achievement_batch_retired_request_still_rejects_foreign_transaction_witness() -> TestResult {
    run("ach_batch_retired", async |harness, root| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        let mut request = grant(COOKIES, 60);
        request.catalogue = AchievementCatalogueLookup::Retired;
        let result = root
            .commit_test_achievement_grant_with_foreign_token(
                &authority,
                &harness.node,
                fence(1)?,
                request.clone(),
            )
            .await;
        assert!(
            matches!(result, Err(AchievementGrantError::AuthorityRejected)),
            "{result:?}"
        );
        let valid = root
            .commit_test_achievement_grants(&authority, &harness.node, fence(1)?, vec![request])
            .await
            .map_err(debug)?;
        assert_eq!(valid, vec![AchievementGrantOutcome::Retired]);
        assert_eq!(counts(harness).await?, (0, 0));
        Ok(())
    })
}
