//! STANCE-1 real writer, read/reconcile and mixed Character chain on PostgreSQL 17.6.
//! Parent protected harness provides migrations and independently seeded current authority.
use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, SESSION, TestResult, configured_admin, context, debug, fence, id, runtime,
};
use crate::domain::CharacterId;
use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
use crate::durability::character_progression::{
    CharacterProgressionError, ExperienceAwardRequest, ExperienceCommitOutcome,
    ExperienceRewardOccurrence,
};
use crate::durability::character_stance::{
    DurableCharacterStance, StanceChangeOccurrence, StanceChangeOutcome, StanceChangeRequest,
};
use crate::durability::monk_state::{
    DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveRequest,
};
use crate::foundation::{ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};

fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, tag, true).await?;
        let outcome = body(&harness).await;
        harness.cleanup().await?;
        outcome
    })
}
fn request(tag: u8, before: Option<&str>, after: Option<&str>) -> TestResult<StanceChangeRequest> {
    Ok(StanceChangeRequest {
        occurrence: StanceChangeOccurrence::from_bytes(id(tag)).map_err(debug)?,
        before: before.map(str::to_owned),
        after: after.map(str::to_owned),
        content_revision: "content-1".into(),
        policy_revision: "policy-1".into(),
        policy_digest: [1; 32],
    })
}
type Snapshot = (String, String, i64, i64, i16, i64, Option<String>, i64);
async fn snapshot(harness: &Harness) -> TestResult<Snapshot> {
    Ok(sqlx::query_as("SELECT r.character_revision::text,s.character_revision::text,s.level,s.total_experience,s.harmony,s.serene_forced_remaining_micros,t.stance_key,(SELECT count(*) FROM game_character_stance_receipts) FROM game_character_roots r JOIN game_character_progression_state s USING(character_id) LEFT JOIN game_character_stance t USING(character_id) WHERE r.character_id=encode($1,'hex')::uuid")
        .bind(id(CHARACTER).as_slice()).fetch_one(&harness.pool).await?)
}

#[test]
fn stance_writer_binds_history_and_loads_across_other_character_writers() -> TestResult {
    run("stance_writer_chain", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        assert_eq!(
            h.root
                .read_character_stance(&authority, character)
                .await
                .map_err(debug)?,
            DurableCharacterStance::default()
        );
        let toggle = request(80, None, Some("protector"))?;
        let first = h
            .root
            .commit_character_stance(&authority, &h.node, fence(1)?, toggle.clone())
            .await
            .map_err(debug)?;
        let StanceChangeOutcome::Committed(receipt) = first else {
            return Err(format!("first: {first:?}").into());
        };
        assert_eq!(receipt.original_character_revision().get(), 1);
        assert_eq!(receipt.committed_character_revision().get(), 2);
        assert_eq!(receipt.before(), None);
        assert_eq!(receipt.after(), Some("protector"));
        assert_eq!(receipt.policy_digest(), &[1; 32]);
        assert!(receipt.matches_request(&fence(1)?, &toggle));
        assert_eq!(
            snapshot(h).await?,
            (
                "2".into(),
                "2".into(),
                50,
                1000,
                0,
                0,
                Some("protector".into()),
                1
            )
        );
        // Lost-response history survives a superseded connection. A fresh write is separately
        // fenced below; historical replay/reconcile do not grant that connection authority.
        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=2 WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(id(SESSION).as_slice()).execute(&h.pool).await?;
        let replay = h
            .root
            .commit_character_stance(&authority, &h.node, fence(1)?, toggle.clone())
            .await
            .map_err(debug)?;
        assert_eq!(
            replay,
            StanceChangeOutcome::AlreadyCommitted(receipt.clone())
        );
        assert_eq!(
            h.root
                .reconcile_character_stance(&authority, toggle.occurrence)
                .await
                .map_err(debug)?,
            Some(receipt.clone())
        );
        assert_eq!(
            h.root
                .reconcile_character_stance(
                    &authority,
                    StanceChangeOccurrence::from_bytes(id(89)).map_err(debug)?
                )
                .await
                .map_err(debug)?,
            None
        );
        let stale = h
            .root
            .commit_character_stance(
                &authority,
                &h.node,
                fence(2)?,
                request(81, Some("protector"), None)?,
            )
            .await;
        assert!(
            matches!(stale, Err(CharacterProgressionError::AuthorityRejected)),
            "{stale:?}"
        );
        assert_eq!(
            snapshot(h).await?,
            (
                "2".into(),
                "2".into(),
                50,
                1000,
                0,
                0,
                Some("protector".into()),
                1
            )
        );
        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=1 WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(id(SESSION).as_slice()).execute(&h.pool).await?;
        // Other Character writers may advance revision without rewriting the standard slot.
        h.root
            .commit_character_monk_state_save(
                &authority,
                &h.node,
                fence(2)?,
                MonkStateSaveRequest {
                    occurrence: MonkStateSaveOccurrence::from_bytes(id(82)).map_err(debug)?,
                    state: DurableMonkState::new(3, 2_500_000).map_err(debug)?,
                },
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            snapshot(h).await?,
            (
                "3".into(),
                "3".into(),
                50,
                1000,
                3,
                2_500_000,
                Some("protector".into()),
                1
            )
        );
        let switch = request(83, Some("protector"), Some("blood-rage"))?;
        let second = h
            .root
            .commit_character_stance(&authority, &h.node, fence(3)?, switch)
            .await
            .map_err(debug)?;
        assert!(matches!(second, StanceChangeOutcome::Committed(_)));
        assert_eq!(
            snapshot(h).await?,
            (
                "4".into(),
                "4".into(),
                50,
                1000,
                3,
                2_500_000,
                Some("blood-rage".into()),
                2
            )
        );
        // A freshly opened authority reads the durable projection without a runtime cache.
        let reopened = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let loaded = h
            .root
            .read_character_stance(&reopened, character)
            .await
            .map_err(debug)?;
        assert_eq!(loaded.key(), Some("blood-rage"));
        assert_eq!(
            loaded.committed_character_revision().map(|r| r.get()),
            Some(4)
        );
        let off = request(84, Some("blood-rage"), None)?;
        h.root
            .commit_character_stance(&reopened, &h.node, fence(4)?, off)
            .await
            .map_err(debug)?;
        let loaded = h
            .root
            .read_character_stance(&reopened, character)
            .await
            .map_err(debug)?;
        assert_eq!(loaded.key(), None);
        assert_eq!(
            loaded.committed_character_revision().map(|r| r.get()),
            Some(5)
        );
        assert_eq!(
            snapshot(h).await?,
            ("5".into(), "5".into(), 50, 1000, 3, 2_500_000, None, 3)
        );
        assert_eq!(h.count("game_character_build_receipts").await?, 0);
        assert_eq!(h.count("game_character_xp_receipts").await?, 0);
        Ok(())
    })
}

#[test]
fn stance_writer_rejects_mutated_intent_and_independent_current_authority() -> TestResult {
    run("stance_writer_reject", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let initial = snapshot(h).await?;
        let mut stale_connection = fence(1)?;
        stale_connection.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        let mut stale_lease = fence(1)?;
        stale_lease.character_lease_generation = 2;
        let mut stale_session = fence(1)?;
        stale_session.game_session_id = GameSessionId::decode(&id(51)).map_err(debug)?;
        let mut stale_scope = fence(1)?;
        stale_scope.scope_ownership_generation = ScopeOwnershipGeneration::new(2).map_err(debug)?;
        for (tag, current, case) in [
            (90, stale_connection, "connection"),
            (91, stale_lease, "lease"),
            (92, stale_session, "session"),
            (93, stale_scope, "scope ownership"),
        ] {
            let outcome = h
                .root
                .commit_character_stance(
                    &authority,
                    &h.node,
                    current,
                    request(tag, None, Some("protector"))?,
                )
                .await;
            assert!(
                matches!(outcome, Err(CharacterProgressionError::AuthorityRejected)),
                "{case}: {outcome:?}"
            );
            assert_eq!(snapshot(h).await?, initial, "{case}");
        }
        let mut wrong_before = request(94, Some("protector"), Some("blood-rage"))?;
        let outcome = h
            .root
            .commit_character_stance(&authority, &h.node, fence(1)?, wrong_before.clone())
            .await;
        assert!(
            matches!(outcome, Err(CharacterProgressionError::StanceStateMismatch)),
            "{outcome:?}"
        );
        assert_eq!(snapshot(h).await?, initial);
        wrong_before.before = None;
        wrong_before.after = None;
        let outcome = h
            .root
            .commit_character_stance(&authority, &h.node, fence(1)?, wrong_before)
            .await;
        assert!(
            matches!(outcome, Err(CharacterProgressionError::InvalidInput)),
            "{outcome:?}"
        );
        for (tag, field) in [(95, "content"), (96, "policy")] {
            let mut changed = request(tag, None, Some("protector"))?;
            if field == "content" {
                changed.content_revision = "content-2".into();
            } else {
                changed.policy_revision = "policy-2".into();
            }
            let outcome = h
                .root
                .commit_character_stance(&authority, &h.node, fence(1)?, changed)
                .await;
            assert!(
                matches!(
                    outcome,
                    Err(CharacterProgressionError::ProgressionContextMismatch)
                ),
                "{field}: {outcome:?}"
            );
            assert_eq!(snapshot(h).await?, initial);
        }
        let toggle = request(97, None, Some("protector"))?;
        let first = h
            .root
            .commit_character_stance(&authority, &h.node, fence(1)?, toggle.clone())
            .await
            .map_err(debug)?;
        let StanceChangeOutcome::Committed(receipt) = first else {
            return Err(format!("first: {first:?}").into());
        };
        let before = snapshot(h).await?;
        for field in ["before", "after", "content", "policy", "digest", "revision"] {
            let mut changed = toggle.clone();
            let mut expected = fence(1)?;
            match field {
                "before" => changed.before = Some("blood-rage".into()),
                "after" => changed.after = Some("sharp-mind".into()),
                "content" => changed.content_revision = "content-2".into(),
                "policy" => changed.policy_revision = "policy-2".into(),
                "digest" => changed.policy_digest = [2; 32],
                "revision" => expected = fence(2)?,
                _ => return Err("unknown conflict field".into()),
            }
            assert!(!receipt.matches_request(&expected, &changed));
            let outcome = h
                .root
                .commit_character_stance(&authority, &h.node, expected, changed)
                .await;
            assert!(
                matches!(
                    outcome,
                    Err(CharacterProgressionError::ConflictingOccurrence)
                ),
                "{field}: {outcome:?}"
            );
            assert_eq!(snapshot(h).await?, before, "{field}");
        }
        let stale_revision = h
            .root
            .commit_character_stance(
                &authority,
                &h.node,
                fence(1)?,
                request(98, Some("protector"), None)?,
            )
            .await;
        assert!(
            matches!(
                stale_revision,
                Err(CharacterProgressionError::CharacterRevisionMismatch)
            ),
            "{stale_revision:?}"
        );
        assert_eq!(snapshot(h).await?, before);
        Ok(())
    })
}

// Poll both real database transactions without requiring Tokio's optional macros feature.
async fn join_two<A: std::future::Future, B: std::future::Future>(
    first: A,
    second: B,
) -> (A::Output, B::Output) {
    let mut first = std::pin::pin!(first);
    let mut second = std::pin::pin!(second);
    let mut first_output = None;
    let mut second_output = None;
    std::future::poll_fn(move |cx| {
        if first_output.is_none()
            && let std::task::Poll::Ready(output) = first.as_mut().poll(cx)
        {
            first_output = Some(output);
        }
        if second_output.is_none()
            && let std::task::Poll::Ready(output) = second.as_mut().poll(cx)
        {
            second_output = Some(output);
        }
        match (first_output.take(), second_output.take()) {
            (Some(a), Some(b)) => std::task::Poll::Ready((a, b)),
            (a, b) => {
                first_output = a;
                second_output = b;
                std::task::Poll::Pending
            }
        }
    })
    .await
}
fn award(tag: u8) -> TestResult<ExperienceAwardRequest<2>> {
    let context = context();
    Ok(ExperienceAwardRequest {
        occurrence: ExperienceRewardOccurrence::from_bytes(id(tag)).map_err(debug)?,
        amount: ExactI64::new(5),
        context: context.clone(),
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        policy: FiniteProgressionPolicy {
            context,
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            death_policy_revision: "death-1".into(),
            declared_difference_revision: "declaration-1".into(),
            thresholds: [
                LevelThreshold {
                    level: 50,
                    minimum_experience: ExactI64::new(1000),
                },
                LevelThreshold {
                    level: 51,
                    minimum_experience: ExactI64::new(1100),
                },
            ],
            terminal_exclusive_experience: ExactI64::new(1200),
            death_loss_numerator: 1,
            death_loss_denominator: 10,
            death_loss_rounding: RoundingMode::Floor,
        },
    })
}
#[test]
fn stance_writer_serializes_same_occurrence_and_competing_xp_revision() -> TestResult {
    run("stance_writer_concurrent", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        // Each test root intentionally owns only one ready SQL holder. Use two
        // independently ready roots so this exercises the database writer locks,
        // rather than rejecting the second checkout before it reaches PostgreSQL.
        let concurrent = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(concurrent.maintain_ready_once().await?);
        let concurrent_authority = concurrent
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let first = request(100, None, Some("protector"))?;
        let (a, b) = join_two(
            concurrent.commit_character_stance(
                &concurrent_authority,
                &h.node,
                fence(1)?,
                first.clone(),
            ),
            h.root
                .commit_character_stance(&authority, &h.node, fence(1)?, first.clone()),
        )
        .await;
        let a = a.map_err(debug)?;
        let b = b.map_err(debug)?;
        let (committed, replayed) = match (a, b) {
            (StanceChangeOutcome::Committed(a), StanceChangeOutcome::AlreadyCommitted(b))
            | (StanceChangeOutcome::AlreadyCommitted(b), StanceChangeOutcome::Committed(a)) => {
                (a, b)
            }
            other => return Err(format!("same-occurrence concurrency: {other:?}").into()),
        };
        assert_eq!(committed, replayed);
        assert_eq!(h.count("game_character_stance_receipts").await?, 1);
        let second = request(101, Some("protector"), None)?;
        let (stance, xp) = join_two(
            h.root
                .commit_character_stance(&authority, &h.node, fence(2)?, second.clone()),
            concurrent.commit_character_experience(
                &concurrent_authority,
                &h.node,
                fence(2)?,
                award(102)?,
            ),
        )
        .await;
        let final_stance_revision = match (stance, xp) {
            (
                Ok(StanceChangeOutcome::Committed(_)),
                Err(CharacterProgressionError::CharacterRevisionMismatch),
            ) => {
                assert_eq!(
                    snapshot(h).await?,
                    ("3".into(), "3".into(), 50, 1000, 0, 0, None, 2)
                );
                h.root
                    .commit_character_experience(&authority, &h.node, fence(3)?, award(102)?)
                    .await
                    .map_err(debug)?;
                3
            }
            (
                Err(CharacterProgressionError::CharacterRevisionMismatch),
                Ok(ExperienceCommitOutcome::Committed(_)),
            ) => {
                assert_eq!(
                    snapshot(h).await?,
                    (
                        "3".into(),
                        "3".into(),
                        50,
                        1005,
                        0,
                        0,
                        Some("protector".into()),
                        1
                    )
                );
                h.root
                    .commit_character_stance(&authority, &h.node, fence(3)?, second)
                    .await
                    .map_err(debug)?;
                4
            }
            other => return Err(format!("stance/XP revision race: {other:?}").into()),
        };
        assert_eq!(
            snapshot(h).await?,
            ("4".into(), "4".into(), 50, 1005, 0, 0, None, 2)
        );
        assert_eq!(h.count("game_character_xp_receipts").await?, 1);
        // Restart is a separate root/pool, not an actor-local cache lookup.
        let restarted = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let reopened = restarted
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let loaded = restarted
            .read_character_stance(
                &reopened,
                CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?,
            )
            .await
            .map_err(debug)?;
        assert_eq!(loaded.key(), None);
        assert_eq!(
            loaded.committed_character_revision().map(|r| r.get()),
            Some(final_stance_revision)
        );
        Ok(())
    })
}
