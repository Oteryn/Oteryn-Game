#![allow(clippy::expect_used)]
// D1092 APP-1 native authority target. Every case requires an explicitly routed
// PostgreSQL 17.6 service and the installed migration 0093; absence is a failure.
extern crate oteryn_game_server as production_server;
extern crate self as oteryn_game_server;
pub use production_server::admission_evidence;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_recovery_fence.rs"]
pub mod character_recovery_fence;
pub use production_server::domain;
#[allow(dead_code, unused_imports)]
#[path = "../src/durability/mod.rs"]
mod durability;
#[allow(dead_code, unused_imports)]
#[path = "../src/foundation/mod.rs"]
pub mod foundation;
#[allow(dead_code, unused_imports)]
#[path = "../src/native_admission_source/mod.rs"]
pub mod native_admission_source;
#[allow(dead_code, unused_imports)]
#[path = "../src/premium/mod.rs"]
mod premium;

// Real native owner/harness composition. Module registration and migration 0093
// are CP slot changes; private qualification mounts them explicitly before that slot.
#[allow(
    dead_code,
    reason = "shared native harness includes helpers for other receipt families"
)]
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;

use bestiary_postgres_harness::{
    CHARACTER, Harness, SESSION, TestResult, debug, fence, id, runtime,
};
use domain::appearance::{AppearanceContent, AppearanceRejection, AppearanceSelection, Sex};
use domain::{CharacterId, CharacterRevision};
use durability::character_appearance::{
    AppearanceChangeOccurrence, AppearanceChangeOutcome, AppearanceChangeRequest,
    AppearanceValidationContext, AppearanceWriteError, DurableAppearanceSelection,
};
use durability::character_progression::CharacterProgressionError;
use foundation::{ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration};

// Explicit TEST-ONLY content and sex. These are NOT authoring facts, an owner port or an
// accepted active catalogue. They isolate durable/fence behavior from the blocked content lane.
struct TestContent;
impl AppearanceContent<String> for TestContent {
    fn palette_size(&self) -> u16 {
        133
    }
    fn outfit_addon_count(&self, key: &String) -> Option<u8> {
        matches!(key.as_str(), "starter-a" | "starter-b" | "earned" | "store").then_some(2)
    }
    fn outfit_is_starter(&self, key: &String, sex: Sex) -> bool {
        matches!(key.as_str(), "starter-a" | "starter-b") && sex == Sex::Male
    }
    fn addon_is_starter(&self, key: &String, _: Sex, addon: u8) -> bool {
        key == "starter-b" && addon == 1
    }
    fn mount_exists(&self, key: &String) -> bool {
        key == "horse"
    }
}
fn validation() -> AppearanceValidationContext<TestContent> {
    AppearanceValidationContext {
        sex: Sex::Male,
        content: TestContent,
        content_revision: "content-1".into(),
    }
}
fn selection(key: &str) -> AppearanceSelection<String> {
    AppearanceSelection {
        outfit_key: key.into(),
        colours: [0, 1, 2, 132],
        addons: 0,
        mount_key: None,
    }
}
fn request(
    tag: u8,
    before: Option<DurableAppearanceSelection>,
    key: &str,
) -> TestResult<AppearanceChangeRequest> {
    Ok(AppearanceChangeRequest {
        occurrence: AppearanceChangeOccurrence::from_bytes(id(tag)).map_err(debug)?,
        before,
        after: selection(key),
        content_revision: "content-1".into(),
    })
}
fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    // Unlike ordinary optional PostgreSQL suites, this qualification must fail when unrouted.
    let admin = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
    runtime()?.block_on(async move {
        let h = Harness::create_appearance(admin, tag).await?;
        let outcome = body(&h).await;
        h.cleanup().await?;
        outcome
    })
}
async fn snapshot(h: &Harness) -> TestResult<Vec<Vec<String>>> {
    let mut snapshot = Vec::new();
    for table in [
        "game_character_roots",
        "game_character_progression_state",
        "game_character_appearance_receipts",
        "game_character_appearance",
        "game_account_appearance_unlock_receipts",
        "game_account_appearance_unlocks",
    ] {
        snapshot.push(
            sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT row_to_json(t)::text FROM {table} t ORDER BY row_to_json(t)::text"
            )))
            .fetch_all(&h.pool)
            .await?,
        );
    }
    Ok(snapshot)
}

#[test]
fn appearance_native_commit_replay_reconnect_and_restarted_reload() -> TestResult {
    run("appearance_native_reload", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        assert_eq!(
            h.root
                .read_character_appearance(&authority, character)
                .await
                .map_err(debug)?,
            None
        );
        let first = request(80, None, "starter-a")?;
        let outcome = h
            .root
            .commit_character_appearance(
                &authority,
                &h.node,
                fence(1)?,
                first.clone(),
                validation(),
            )
            .await
            .map_err(debug)?;
        let AppearanceChangeOutcome::Committed(receipt) = outcome else {
            return Err(format!("first: {outcome:?}").into());
        };
        assert_eq!(receipt.original_character_revision.get(), 1);
        assert_eq!(receipt.committed_character_revision.get(), 2);
        assert!(receipt.matches_request(&fence(1)?, &first, Sex::Male));
        assert_eq!(h.root_revision().await?, "2");
        assert_eq!(h.count("game_character_appearance_receipts").await?, 1);
        let committed = snapshot(h).await?;
        // Current independently persisted connection generation changes; no fact is inferred
        // from the retained appearance receipt. Historical retry is still disposition only.
        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=2 WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(id(SESSION).as_slice()).execute(&h.pool).await?;
        assert_eq!(
            h.root
                .commit_character_appearance(
                    &authority,
                    &h.node,
                    fence(1)?,
                    first.clone(),
                    validation()
                )
                .await
                .map_err(debug)?,
            AppearanceChangeOutcome::AlreadyCommitted(receipt.clone())
        );
        assert_eq!(
            h.root
                .reconcile_character_appearance(&authority, first.occurrence)
                .await
                .map_err(debug)?,
            Some(receipt)
        );
        let before = h
            .root
            .read_character_appearance(&authority, character)
            .await
            .map_err(debug)?;
        let stale = h
            .root
            .commit_character_appearance(
                &authority,
                &h.node,
                fence(2)?,
                request(81, before.clone(), "starter-b")?,
                validation(),
            )
            .await;
        assert!(
            matches!(
                stale,
                Err(AppearanceWriteError::Fence(
                    CharacterProgressionError::AuthorityRejected
                ))
            ),
            "{stale:?}"
        );
        assert_eq!(snapshot(h).await?, committed);
        // A separate native root/pool reopens recovery and reads persisted state. This is a
        // genuine storage/recovery restart, not proof of the still-unwired client login flow.
        let restarted = durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let reopened = restarted
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert_eq!(
            restarted
                .read_character_appearance(&reopened, character)
                .await
                .map_err(debug)?,
            before
        );
        let mut current = fence(2)?;
        current.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        let outcome = restarted
            .commit_character_appearance(
                &reopened,
                &h.node,
                current,
                request(82, before, "starter-b")?,
                validation(),
            )
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, AppearanceChangeOutcome::Committed(_)));
        assert_eq!(h.root_revision().await?, "3");
        assert_eq!(h.count("game_character_appearance_receipts").await?, 2);
        let loaded = restarted
            .read_character_appearance(&reopened, character)
            .await
            .map_err(debug)?
            .ok_or("missing reload")?;
        assert_eq!(loaded.selection.outfit_key, "starter-b");
        assert_eq!(loaded.committed_character_revision.get(), 3);
        let committed = snapshot(h).await?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(id(SESSION).as_slice()).execute(&h.pool).await?;
        assert!(matches!(
            restarted
                .commit_character_appearance(&reopened, &h.node, fence(1)?, first, validation())
                .await
                .map_err(debug)?,
            AppearanceChangeOutcome::AlreadyCommitted(_)
        ));
        let mut closed = fence(3)?;
        closed.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        let result = restarted
            .commit_character_appearance(
                &reopened,
                &h.node,
                closed,
                request(83, Some(loaded), "starter-a")?,
                validation(),
            )
            .await;
        assert!(
            matches!(
                result,
                Err(AppearanceWriteError::Fence(
                    CharacterProgressionError::AuthorityRejected
                ))
            ),
            "closed session: {result:?}"
        );
        assert_eq!(snapshot(h).await?, committed);
        Ok(())
    })
}

#[test]
fn appearance_native_stale_fences_and_current_context_write_nothing() -> TestResult {
    run("appearance_native_fences", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let initial = snapshot(h).await?;
        let mut connection = fence(1)?;
        connection.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        let mut lease = fence(1)?;
        lease.character_lease_generation = 2;
        let mut session = fence(1)?;
        session.game_session_id = GameSessionId::decode(&id(99)).map_err(debug)?;
        let mut scope = fence(1)?;
        scope.scope_ownership_generation = ScopeOwnershipGeneration::new(2).map_err(debug)?;
        let mut character = fence(1)?;
        character.character_id = CharacterId::from_bytes(id(98)).map_err(debug)?;
        for (tag, changed, label) in [
            (90, connection, "connection"),
            (91, lease, "lease"),
            (92, session, "session"),
            (93, scope, "scope"),
            (94, character, "character"),
        ] {
            let result = h
                .root
                .commit_character_appearance(
                    &authority,
                    &h.node,
                    changed,
                    request(tag, None, "starter-a")?,
                    validation(),
                )
                .await;
            assert!(
                matches!(
                    result,
                    Err(AppearanceWriteError::Fence(
                        CharacterProgressionError::AuthorityRejected
                    ))
                ),
                "{label}: {result:?}"
            );
            assert_eq!(snapshot(h).await?, initial, "{label}");
        }
        let other_node = bestiary_postgres_harness::register(&h.root, 2).await?;
        let result = h
            .root
            .commit_character_appearance(
                &authority,
                &other_node,
                fence(1)?,
                request(95, None, "starter-a")?,
                validation(),
            )
            .await;
        assert!(
            matches!(
                result,
                Err(AppearanceWriteError::Fence(
                    CharacterProgressionError::AuthorityRejected
                ))
            ),
            "node: {result:?}"
        );
        assert_eq!(snapshot(h).await?, initial);
        let result = h
            .root
            .commit_character_appearance(
                &authority,
                &h.node,
                fence(2)?,
                request(96, None, "starter-a")?,
                validation(),
            )
            .await;
        assert!(
            matches!(
                result,
                Err(AppearanceWriteError::Fence(
                    CharacterProgressionError::CharacterRevisionMismatch
                ))
            ),
            "revision: {result:?}"
        );
        assert_eq!(snapshot(h).await?, initial);
        // A separately supplied active interpretation must match the independently locked root,
        // even when the request itself names the expected original content revision.
        let mut wrong_content = validation();
        wrong_content.content_revision = "content-2".into();
        let result = h
            .root
            .commit_character_appearance(
                &authority,
                &h.node,
                fence(1)?,
                request(97, None, "starter-a")?,
                wrong_content,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(AppearanceWriteError::Fence(
                    CharacterProgressionError::ProgressionContextMismatch
                ))
            ),
            "content: {result:?}"
        );
        assert_eq!(snapshot(h).await?, initial);
        Ok(())
    })
}

#[test]
fn appearance_native_validation_and_expected_prior_state_fail_closed() -> TestResult {
    run("appearance_native_rules", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let initial = snapshot(h).await?;
        for (tag, mut command, rejection) in [
            (
                100,
                request(100, None, "earned")?,
                AppearanceRejection::OutfitNotAuthorized,
            ),
            (
                101,
                request(101, None, "store")?,
                AppearanceRejection::OutfitNotAuthorized,
            ),
            (
                102,
                request(102, None, "starter-a")?,
                AppearanceRejection::AddonNotAuthorized { addon: 1 },
            ),
            (
                103,
                request(103, None, "starter-a")?,
                AppearanceRejection::ColourOutOfPalette { slot: 0 },
            ),
            (
                104,
                request(104, None, "starter-a")?,
                AppearanceRejection::MountNotAuthorized,
            ),
        ] {
            match tag {
                102 => command.after.addons = 1,
                103 => command.after.colours[0] = 133,
                104 => command.after.mount_key = Some("horse".into()),
                _ => {}
            }
            let result = h
                .root
                .commit_character_appearance(&authority, &h.node, fence(1)?, command, validation())
                .await;
            assert!(
                matches!(result,Err(AppearanceWriteError::Rejected(found)) if found==rejection),
                "{result:?}"
            );
            assert_eq!(snapshot(h).await?, initial);
        }
        let before = Some(DurableAppearanceSelection {
            selection: selection("starter-a"),
            content_revision: "content-1".into(),
            committed_character_revision: CharacterRevision::new(2).map_err(debug)?,
        });
        let result = h
            .root
            .commit_character_appearance(
                &authority,
                &h.node,
                fence(1)?,
                request(105, before, "starter-b")?,
                validation(),
            )
            .await;
        assert!(
            matches!(result, Err(AppearanceWriteError::StateMismatch)),
            "{result:?}"
        );
        assert_eq!(snapshot(h).await?, initial);
        let mut first = request(106, None, "starter-b")?;
        first.after.addons = 1;
        assert!(matches!(
            h.root
                .commit_character_appearance(&authority, &h.node, fence(1)?, first, validation())
                .await
                .map_err(debug)?,
            AppearanceChangeOutcome::Committed(_)
        ));
        Ok(())
    })
}

#[test]
fn appearance_native_occurrence_binds_complete_intent_and_replay_is_historical() -> TestResult {
    run("appearance_native_binding", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let first = request(110, None, "starter-a")?;
        h.root
            .commit_character_appearance(
                &authority,
                &h.node,
                fence(1)?,
                first.clone(),
                validation(),
            )
            .await
            .map_err(debug)?;
        let committed = snapshot(h).await?;
        for field in [
            "outfit", "colour", "addon", "mount", "content", "revision", "sex",
        ] {
            let mut changed = first.clone();
            let mut current = fence(1)?;
            let mut context = validation();
            match field {
                "outfit" => changed.after.outfit_key = "starter-b".into(),
                "colour" => changed.after.colours[0] = 1,
                "addon" => changed.after.addons = 1,
                "mount" => changed.after.mount_key = Some("horse".into()),
                "content" => changed.content_revision = "content-2".into(),
                "revision" => current = fence(2)?,
                "sex" => context.sex = Sex::Female,
                _ => return Err("unknown binding case".into()),
            }
            let result = h
                .root
                .commit_character_appearance(&authority, &h.node, current, changed, context)
                .await;
            assert!(
                matches!(
                    result,
                    Err(AppearanceWriteError::Fence(
                        CharacterProgressionError::ConflictingOccurrence
                    ))
                ),
                "{field}: {result:?}"
            );
            assert_eq!(snapshot(h).await?, committed, "{field}");
        }
        Ok(())
    })
}

// Poll both actual native transactions, using separate ready roots rather than competing for
// one holder. No custom lock or fake receipt is used to make the concurrent case pass.
async fn join_two<A: std::future::Future, B: std::future::Future>(
    first: A,
    second: B,
) -> (A::Output, B::Output) {
    let mut first = std::pin::pin!(first);
    let mut second = std::pin::pin!(second);
    let mut a = None;
    let mut b = None;
    std::future::poll_fn(move |cx| {
        if a.is_none()
            && let std::task::Poll::Ready(value) = first.as_mut().poll(cx)
        {
            a = Some(value);
        }
        if b.is_none()
            && let std::task::Poll::Ready(value) = second.as_mut().poll(cx)
        {
            b = Some(value);
        }
        match (a.take(), b.take()) {
            (Some(a), Some(b)) => std::task::Poll::Ready((a, b)),
            (x, y) => {
                a = x;
                b = y;
                std::task::Poll::Pending
            }
        }
    })
    .await
}
#[test]
fn appearance_native_serializes_same_occurrence_and_competing_selection() -> TestResult {
    run("appearance_native_race", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let other = durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let other_authority = other.open_character_authority(&seal).await.map_err(debug)?;
        let first = request(120, None, "starter-a")?;
        let (a, b) = join_two(
            h.root.commit_character_appearance(
                &authority,
                &h.node,
                fence(1)?,
                first.clone(),
                validation(),
            ),
            other.commit_character_appearance(
                &other_authority,
                &h.node,
                fence(1)?,
                first,
                validation(),
            ),
        )
        .await;
        match (a.map_err(debug)?, b.map_err(debug)?) {
            (
                AppearanceChangeOutcome::Committed(a),
                AppearanceChangeOutcome::AlreadyCommitted(b),
            )
            | (
                AppearanceChangeOutcome::AlreadyCommitted(b),
                AppearanceChangeOutcome::Committed(a),
            ) => assert_eq!(a, b),
            other => return Err(format!("same occurrence: {other:?}").into()),
        }
        assert_eq!(h.root_revision().await?, "2");
        assert_eq!(h.count("game_character_appearance_receipts").await?, 1);
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let before = h
            .root
            .read_character_appearance(&authority, character)
            .await
            .map_err(debug)?;
        let a = request(121, before.clone(), "starter-b")?;
        let mut b = request(122, before, "starter-b")?;
        b.after.colours[0] = 2;
        let (a, b) = join_two(
            h.root
                .commit_character_appearance(&authority, &h.node, fence(2)?, a, validation()),
            other.commit_character_appearance(
                &other_authority,
                &h.node,
                fence(2)?,
                b,
                validation(),
            ),
        )
        .await;
        match (a, b) {
            (
                Ok(AppearanceChangeOutcome::Committed(_)),
                Err(AppearanceWriteError::Fence(
                    CharacterProgressionError::CharacterRevisionMismatch,
                )),
            )
            | (
                Err(AppearanceWriteError::Fence(
                    CharacterProgressionError::CharacterRevisionMismatch,
                )),
                Ok(AppearanceChangeOutcome::Committed(_)),
            ) => {}
            other => return Err(format!("competing occurrence: {other:?}").into()),
        }
        assert_eq!(h.root_revision().await?, "3");
        assert_eq!(h.count("game_character_appearance_receipts").await?, 2);
        Ok(())
    })
}

#[test]
fn appearance_native_foreign_root_and_real_scope_replacement_cannot_write() -> TestResult {
    use durability::runtime_scope_assignment::{
        AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
    };
    run("appearance_native_replacement", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let initial = snapshot(h).await?;
        let foreign = durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(foreign.maintain_ready_once().await?);
        let result = foreign
            .commit_character_appearance(
                &authority,
                &h.node,
                fence(1)?,
                request(130, None, "starter-a")?,
                validation(),
            )
            .await;
        assert!(
            matches!(
                result,
                Err(AppearanceWriteError::Fence(
                    CharacterProgressionError::AuthorityRejected
                ))
            ),
            "foreign root: {result:?}"
        );
        assert_eq!(snapshot(h).await?, initial);
        // Resolve the predecessor from the current owner AFTER readiness publication, then
        // replace its real registered holder. The old authentic Node proof becomes insufficient.
        let scope = bestiary_postgres_harness::scope()?;
        let predecessor = h
            .root
            .read_runtime_scope_predecessor(scope)
            .await
            .map_err(debug)?
            .ok_or("missing predecessor")?;
        let next_node = bestiary_postgres_harness::register(&h.root, 2).await?;
        let replacement = h
            .writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([8; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Replace {
                    scope,
                    predecessor,
                    target: next_node.fact(),
                },
            })
            .await
            .map_err(debug)?;
        assert!(matches!(replacement, AssignmentOutcome::Committed(_)));
        let result = h
            .root
            .commit_character_appearance(
                &authority,
                &h.node,
                fence(1)?,
                request(131, None, "starter-a")?,
                validation(),
            )
            .await;
        assert!(
            matches!(
                result,
                Err(AppearanceWriteError::Fence(
                    CharacterProgressionError::AuthorityRejected
                ))
            ),
            "replaced owner: {result:?}"
        );
        assert_eq!(snapshot(h).await?, initial);
        Ok(())
    })
}

#[test]
fn appearance_native_preserves_other_receipt_families_in_shared_revision_chain() -> TestResult {
    use durability::character_stance::{
        StanceChangeOccurrence, StanceChangeOutcome, StanceChangeRequest,
    };
    run("appearance_native_mixed", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        h.root
            .commit_character_appearance(
                &authority,
                &h.node,
                fence(1)?,
                request(140, None, "starter-a")?,
                validation(),
            )
            .await
            .map_err(debug)?;
        let stance = h
            .root
            .commit_character_stance(
                &authority,
                &h.node,
                fence(2)?,
                StanceChangeRequest {
                    occurrence: StanceChangeOccurrence::from_bytes(id(141)).map_err(debug)?,
                    before: None,
                    after: Some("protector".into()),
                    content_revision: "content-1".into(),
                    policy_revision: "policy-1".into(),
                    policy_digest: [1; 32],
                },
            )
            .await
            .map_err(debug)?;
        assert!(matches!(stance, StanceChangeOutcome::Committed(_)));
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let before = h
            .root
            .read_character_appearance(&authority, character)
            .await
            .map_err(debug)?;
        assert_eq!(
            before
                .as_ref()
                .map(|a| a.committed_character_revision.get()),
            Some(2)
        );
        h.root
            .commit_character_appearance(
                &authority,
                &h.node,
                fence(3)?,
                request(142, before, "starter-b")?,
                validation(),
            )
            .await
            .map_err(debug)?;
        assert_eq!(h.root_revision().await?, "4");
        assert_eq!(h.count("game_character_appearance_receipts").await?, 2);
        assert_eq!(h.count("game_character_stance_receipts").await?, 1);
        let reopened = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let loaded = h
            .root
            .read_character_appearance(&reopened, character)
            .await
            .map_err(debug)?
            .ok_or("missing mixed-chain selection")?;
        assert_eq!(loaded.selection.outfit_key, "starter-b");
        assert_eq!(loaded.committed_character_revision.get(), 4);
        assert_eq!(
            h.root
                .read_character_stance(&reopened, character)
                .await
                .map_err(debug)?
                .key(),
            Some("protector")
        );
        let state:(String,i64,i64)=sqlx::query_as("SELECT character_revision::text,level,total_experience FROM game_character_progression_state")
            .fetch_one(&h.pool).await?;
        assert_eq!(state, ("4".into(), 50, 1000));
        Ok(())
    })
}

#[test]
fn appearance_native_current_recovery_provenance_corruption_rejects_all_consumers() -> TestResult {
    use durability::DurabilityError;
    run("appearance_native_recovery_fault", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let first = request(150, None, "starter-a")?;
        h.root
            .commit_character_appearance(
                &authority,
                &h.node,
                fence(1)?,
                first.clone(),
                validation(),
            )
            .await
            .map_err(debug)?;
        let before = h
            .root
            .read_character_appearance(&authority, character)
            .await
            .map_err(debug)?;
        let unchanged = snapshot(h).await?;
        // NEGATIVE DB CORRUPTION ONLY, following the existing native proficiency boundary
        // test. The authentic sealed register/proof is untouched. This is not a legal native
        // recovery successor: its exclusive guard cannot coexist with this same-store seal.
        let mut fault = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *fault)
            .await?;
        sqlx::query("UPDATE game_character_recovery_admissions SET issued_at=issued_at+1")
            .execute(&mut *fault)
            .await?;
        fault.commit().await?;
        let admissions:Vec<String> = sqlx::query_scalar("SELECT row_to_json(a)::text FROM game_character_recovery_admissions a ORDER BY recovery_generation")
            .fetch_all(&h.pool).await?;
        for boundary in ["new commit", "exact replay", "load", "reconcile"] {
            let outcome = match boundary {
                "new commit" => h
                    .root
                    .commit_character_appearance(
                        &authority,
                        &h.node,
                        fence(2)?,
                        request(151, before.clone(), "starter-b")?,
                        validation(),
                    )
                    .await
                    .map(|_| ()),
                "exact replay" => h
                    .root
                    .commit_character_appearance(
                        &authority,
                        &h.node,
                        fence(1)?,
                        first.clone(),
                        validation(),
                    )
                    .await
                    .map(|_| ()),
                "load" => h
                    .root
                    .read_character_appearance(&authority, character)
                    .await
                    .map(|_| ()),
                "reconcile" => h
                    .root
                    .reconcile_character_appearance(&authority, first.occurrence)
                    .await
                    .map(|_| ()),
                _ => return Err("unknown recovery consumer".into()),
            };
            assert!(
                matches!(
                    outcome,
                    Err(AppearanceWriteError::Unavailable(
                        DurabilityError::Unavailable
                    ))
                ),
                "{boundary}: {outcome:?}"
            );
            assert_eq!(
                snapshot(h).await?,
                unchanged,
                "{boundary}: appearance mutation after recovery substitution"
            );
            let after:Vec<String> = sqlx::query_scalar("SELECT row_to_json(a)::text FROM game_character_recovery_admissions a ORDER BY recovery_generation")
                .fetch_all(&h.pool).await?;
            assert_eq!(after, admissions, "{boundary}: recovery evidence changed");
        }
        let mut restore = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *restore)
            .await?;
        sqlx::query("UPDATE game_character_recovery_admissions SET issued_at=issued_at-1")
            .execute(&mut *restore)
            .await?;
        restore.commit().await?;
        assert_eq!(
            h.root
                .read_character_appearance(&authority, character)
                .await
                .map_err(debug)?,
            before
        );
        assert_eq!(snapshot(h).await?, unchanged);
        Ok(())
    })
}

#[test]
fn appearance_native_receipt_insert_fault_rolls_back_prior_root_and_state_updates() -> TestResult {
    use durability::{DurabilityError, SqlxFailureClass};
    run("appearance_native_rollback", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        h.root
            .commit_character_appearance(
                &authority,
                &h.node,
                fence(1)?,
                request(160, None, "starter-a")?,
                validation(),
            )
            .await
            .map_err(debug)?;
        let before = h
            .root
            .read_character_appearance(&authority, character)
            .await
            .map_err(debug)?;
        let unchanged = snapshot(h).await?;
        // Real database fault at the writer's receipt INSERT, after its actual root/state
        // UPDATEs. nextval is intentionally nontransactional test instrumentation: it proves
        // this trigger observed both successors before aborting their transaction. No gameplay
        // fence, content validator, current authority or transaction API is replaced or weakened.
        sqlx::raw_sql(r#"
            CREATE SEQUENCE app1_receipt_fault_hits;
            CREATE FUNCTION app1_receipt_insert_fault() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN
                IF NOT EXISTS (
                    SELECT 1 FROM game_character_roots r
                    JOIN game_character_progression_state s USING(character_id)
                    WHERE r.character_id=NEW.character_id
                      AND r.character_revision=NEW.committed_character_revision
                      AND s.character_revision=NEW.committed_character_revision
                ) THEN
                    RAISE EXCEPTION 'receipt fault did not observe actual prior updates' USING ERRCODE='XX000';
                END IF;
                PERFORM nextval('app1_receipt_fault_hits');
                RAISE EXCEPTION 'isolated APP-1 receipt insertion fault' USING ERRCODE='23514';
            END;
            $$;
            CREATE TRIGGER app1_receipt_insert_fault BEFORE INSERT
                ON game_character_appearance_receipts FOR EACH ROW
                EXECUTE FUNCTION app1_receipt_insert_fault();
        "#).execute(&h.pool).await?;
        let second = request(161, before, "starter-b")?;
        let outcome = h
            .root
            .commit_character_appearance(
                &authority,
                &h.node,
                fence(2)?,
                second.clone(),
                validation(),
            )
            .await;
        assert!(
            matches!(
                outcome,
                Err(AppearanceWriteError::Unavailable(
                    DurabilityError::Database(SqlxFailureClass::Database(None))
                ))
            ),
            "{outcome:?}"
        );
        let hits: i64 = sqlx::query_scalar(
            "SELECT CASE WHEN is_called THEN last_value ELSE 0 END FROM app1_receipt_fault_hits",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            hits, 1,
            "intended post-update receipt fault was not reached exactly once"
        );
        assert_eq!(
            snapshot(h).await?,
            unchanged,
            "root/state/selection/receipt/fact rollback incomplete"
        );
        sqlx::raw_sql("DROP TRIGGER app1_receipt_insert_fault ON game_character_appearance_receipts; DROP FUNCTION app1_receipt_insert_fault();")
            .execute(&h.pool).await?;
        // Exactly the same occurrence can now commit once: the failed transaction retained no
        // receipt, no revision advance and no replacement projection.
        assert!(matches!(
            h.root
                .commit_character_appearance(&authority, &h.node, fence(2)?, second, validation())
                .await
                .map_err(debug)?,
            AppearanceChangeOutcome::Committed(_)
        ));
        assert_eq!(h.root_revision().await?, "3");
        assert_eq!(h.count("game_character_appearance_receipts").await?, 2);
        let reopened = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert_eq!(
            h.root
                .read_character_appearance(&reopened, character)
                .await
                .map_err(debug)?
                .ok_or("missing retried selection")?
                .selection
                .outfit_key,
            "starter-b"
        );
        Ok(())
    })
}
