//! Native-only private continuation tests. The canonical Character-authority target supplies
//! the real library modules and item_transfer_postgres_cases harness. The root runs these
//! only through the authorized shared PostgreSQL route (D1329), never an agent-owned server.
use crate::domain::CharacterRevision;
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_forge_dust::{
    ForgeDustGainCause, ForgeDustLimitCandidate, ForgeDustLimitDisposition, ForgeDustLimitError,
    ForgeDustLimitRequest, ForgeDustWriteIds, gain_forge_dust_in_transaction, read_forge_dust,
};
use crate::durability::character_forge_dust_audit::{
    audit_forge_dust_ledger, decode_forge_dust_limit_envelope,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::{
    ChannelId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration,
    WorldId,
};
use crate::item_transfer_postgres_cases as native;
use native::{Harness, TestResult, debug};
use std::{future::Future, pin::Pin};

const CONTENT: [u8; 32] = [0x37; 32];
// Exact copy of the unnumbered proposal. Publication replaces this fixture include with the
// finally allocated installed migration; until then it is applied only to this fresh test DB.
const APPEND_SQL: &str = include_str!("forge_dust_limit_migration.sql");

fn occurrence(family: u8, index: u16) -> [u8; 16] {
    let mut value = native::id(family);
    value[14..16].copy_from_slice(&index.to_be_bytes());
    value
}
fn request(index: u16, stage: u16) -> ForgeDustLimitRequest {
    ForgeDustLimitRequest {
        occurrence_id: occurrence(91, index),
        expected_upgrade_count: stage,
    }
}

fn run_case<F>(tag: &str, case: F) -> TestResult
where
    F: for<'a> FnOnce(&'a Harness) -> Pin<Box<dyn Future<Output = TestResult> + 'a>>,
{
    run_native_case(tag, true, case)
}

fn run_native_case<F>(tag: &str, install: bool, case: F) -> TestResult
where
    F: for<'a> FnOnce(&'a Harness) -> Pin<Box<dyn Future<Output = TestResult> + 'a>>,
{
    let Some(admin) = native::configured_admin() else {
        return Ok(());
    };
    native::runtime()?.block_on(async move {
        let harness = Harness::create(admin, tag).await?;
        let result = async {
            let version: String = sqlx::query_scalar("SHOW server_version_num")
                .fetch_one(&harness.pool)
                .await?;
            assert_eq!(
                version, "170006",
                "exact PostgreSQL17.6 qualification required"
            );
            eprintln!("FORGE-LIMIT-PG: case={tag} server_version_num={version}");
            if install {
                let mut tx = harness.pool.begin().await?;
                sqlx::raw_sql(APPEND_SQL).execute(&mut *tx).await?;
                tx.commit().await?;
            }
            activate(&harness, 1, None, CONTENT).await?;
            case(&harness).await
        }
        .await;
        let cleanup = harness.cleanup().await;
        match (result, cleanup) {
            (Err(error), Err(cleanup)) => {
                Err(format!("{error}; test database cleanup also failed: {cleanup}").into())
            }
            (Err(error), Ok(())) => Err(error),
            (Ok(()), cleanup) => cleanup,
        }
    })
}

async fn activate(
    h: &Harness,
    sequence: i64,
    previous: Option<i64>,
    digest: [u8; 32],
) -> TestResult {
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
        .bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::bigint::numeric,$4::bigint::numeric,$5,$6,$7)")
        .bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHANNEL).as_slice()).bind(sequence).bind(previous)
        .bind(digest.as_slice()).bind([2u8;32].as_slice()).bind([3u8;32].as_slice()).execute(&h.pool).await?;
    Ok(())
}

/// Asset funding fixture through the unchanged FORGE-1a Gain writer. It is not a declaration
/// that the still separate creature-kill reward admission has shipped.
async fn fund(h: &Harness, index: u16, amount: u32) -> TestResult {
    let fence = native::fence()?;
    let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let gameplay = CurrentCharacterGameplayFence {
        character_id: fence.character_id,
        game_session_id: fence.game_session_id,
        connection_generation: fence.connection_generation,
        character_lease_generation: fence.character_lease_generation,
        runtime_scope: fence.runtime_scope,
        scope_ownership_generation: fence.scope_ownership_generation,
        expected_character_revision: CharacterRevision::new(revision.parse()?).map_err(debug)?,
    };
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    gain_forge_dust_in_transaction(
        &mut tx,
        &gameplay,
        ForgeDustGainCause::CreatureKill {
            occurrence: occurrence(94, index),
        },
        amount,
        ForgeDustWriteIds {
            entry_id: occurrence(95, index),
            transaction_id: occurrence(96, index),
        },
    )
    .await
    .map_err(debug)?;
    tx.commit().await?;
    Ok(())
}

async fn reserve(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    index: u16,
    stage: u16,
) -> TestResult<ForgeDustLimitCandidate> {
    h.root
        .freeze_forge_dust_limit(
            authority,
            &h.node,
            native::fence()?,
            native::command(u64::from(index) + 1)?,
            request(index, stage),
        )
        .await
        .map_err(|error| debug(error).into())
}
async fn effect_counts(h: &Harness) -> TestResult<Vec<i64>> {
    let mut result = Vec::new();
    for table in [
        "game_character_forge_dust_entries",
        "game_forge_dust_limit_receipts",
        "game_forge_dust_limit_history",
        "game_item_audit_outbox",
    ] {
        result.push(h.count(table).await?);
    }
    Ok(result)
}
async fn revision(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(native::id(native::CHARACTER).as_slice()).fetch_one(&h.pool).await?)
}

#[test]
fn native_limit_raise_atomic_replay_new_command_and_ledger_audit() -> TestResult {
    run_case("forge_limit_replay", |h| {
        Box::pin(async move {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            fund(h, 1, 100).await?;
            let before_revision = revision(h).await?;
            let candidate = reserve(h, &authority, 1, 0).await?;
            let first = h
                .root
                .commit_forge_dust_limit(
                    &authority,
                    &h.node,
                    native::fence()?,
                    native::command(2)?,
                    &candidate,
                )
                .await
                .map_err(debug)?;
            assert!(!first.replayed);
            assert_eq!(first.receipt.disposition, ForgeDustLimitDisposition::Raised);
            let entry = first.receipt.entry.ok_or("no committed dust entry")?;
            assert_eq!(
                (
                    entry.before.balance(),
                    entry.after.balance(),
                    entry.after.limit().get(),
                    entry.amount
                ),
                (100, 75, 120, 25)
            );
            assert_eq!(effect_counts(h).await?, [2, 1, 1, 1]);
            assert_eq!(revision(h).await?, before_revision);
            let envelope: Vec<u8> = sqlx::query_scalar(
                "SELECT envelope FROM game_item_audit_outbox WHERE event_id=encode($1,'hex')::uuid",
            )
            .bind(first.receipt.event_id.as_slice())
            .fetch_one(&h.pool)
            .await?;
            let (_, audit) = decode_forge_dust_limit_envelope(&envelope).map_err(debug)?;
            assert_eq!(audit.dust_burned, 25);
            assert_eq!(audit.content_digest, CONTENT);
            assert_eq!(audit.command_id, 2);
            let recreated = reserve(h, &authority, 1, 0).await?;
            let replay = h
                .root
                .commit_forge_dust_limit(
                    &authority,
                    &h.node,
                    native::fence()?,
                    native::command(999)?,
                    &recreated,
                )
                .await
                .map_err(debug)?;
            assert!(replay.replayed);
            assert_eq!(replay.receipt, first.receipt);
            assert_eq!(
                h.root
                    .reconcile_forge_dust_limit(&authority, &candidate)
                    .await
                    .map_err(debug)?,
                Some(first.receipt)
            );
            assert_eq!(effect_counts(h).await?, [2, 1, 1, 1]);
            // A retained successful history row is not ordinary mutable cache data.
            let mut deletion = h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *deletion)
                .await?;
            let deleted = sqlx::query("DELETE FROM game_forge_dust_limit_history")
                .execute(&mut *deletion)
                .await;
            assert!(
                matches!(deleted,Err(sqlx::Error::Database(ref error)) if error.code().as_deref()==Some("23514") && error.message().contains("oldest overflow"))
            );
            deletion.rollback().await?;
            let mut connection = h.pool.acquire().await?;
            let audit = audit_forge_dust_ledger(&mut connection, native::fence()?.character_id, 2)
                .await
                .map_err(debug)?;
            assert_eq!(
                (
                    audit.entries,
                    audit.credited,
                    audit.spent,
                    audit.balance.limit().get()
                ),
                (2, 100, 25, 120)
            );
            Ok(())
        })
    })
}

#[test]
fn native_limit_raise_insufficient_and_conflicting_quote_write_nothing() -> TestResult {
    run_case("forge_limit_refusals", |h| {
        Box::pin(async move {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            for (index, amount) in [(1, 0), (2, 24)] {
                if amount > 0 {
                    fund(h, index, amount).await?;
                }
                let before = effect_counts(h).await?;
                assert!(matches!(
                    h.root
                        .freeze_forge_dust_limit(
                            &authority,
                            &h.node,
                            native::fence()?,
                            native::command(1)?,
                            request(index, 0)
                        )
                        .await,
                    Err(ForgeDustLimitError::InsufficientDust)
                ));
                assert_eq!(effect_counts(h).await?, before);
                assert_eq!(h.count("game_forge_dust_limit_reservations").await?, 0);
            }
            fund(h, 3, 100).await?;
            let _candidate = reserve(h, &authority, 3, 0).await?;
            assert!(matches!(
                h.root
                    .freeze_forge_dust_limit(
                        &authority,
                        &h.node,
                        native::fence()?,
                        native::command(2)?,
                        request(3, 1)
                    )
                    .await,
                Err(ForgeDustLimitError::ConflictingOccurrence)
            ));
            assert_eq!(effect_counts(h).await?, [2, 0, 0, 0]);
            Ok(())
        })
    })
}

#[test]
fn native_limit_raise_revision_change_is_receipt_only_and_remains_terminal_after_rollback()
-> TestResult {
    run_case("forge_limit_revisions", |h| {
        Box::pin(async move {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            fund(h, 1, 100).await?;
            let candidate = reserve(h, &authority, 1, 0).await?;
            activate(h, 2, Some(1), [0x49; 32]).await?;
            let first = h
                .root
                .commit_forge_dust_limit(
                    &authority,
                    &h.node,
                    native::fence()?,
                    native::command(2)?,
                    &candidate,
                )
                .await
                .map_err(debug)?;
            assert_eq!(
                first.receipt.disposition,
                ForgeDustLimitDisposition::RevisionChanged
            );
            assert!(first.receipt.entry.is_none());
            assert!(first.receipt.envelope_sha256.is_none());
            assert_eq!(effect_counts(h).await?, [1, 1, 0, 0]);
            // A receipt from another physical transaction, and especially a terminal rejection,
            // cannot be borrowed to create a successful history record.
            let mut history = h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *history)
                .await?;
            sqlx::query("INSERT INTO game_forge_dust_limit_history(character_id,occurrence_id,history_ordinal) SELECT character_id,occurrence_id,1 FROM game_forge_dust_limit_receipts")
            .execute(&mut *history).await?;
            let committed = history.commit().await;
            assert!(
                matches!(committed,Err(sqlx::Error::Database(ref error)) if error.code().as_deref()==Some("23514") && error.message().contains("same-transaction successful receipt"))
            );
            activate(h, 3, Some(2), CONTENT).await?;
            let replay = h
                .root
                .commit_forge_dust_limit(
                    &authority,
                    &h.node,
                    native::fence()?,
                    native::command(99)?,
                    &candidate,
                )
                .await
                .map_err(debug)?;
            assert!(replay.replayed);
            assert_eq!(replay.receipt, first.receipt);
            assert_eq!(effect_counts(h).await?, [1, 1, 0, 0]);
            let mut connection = h.pool.acquire().await?;
            let balance = read_forge_dust(&mut connection, native::fence()?.character_id)
                .await
                .map_err(debug)?;
            assert_eq!((balance.balance(), balance.limit().get()), (100, 100));
            Ok(())
        })
    })
}

#[test]
fn native_limit_raise_current_fence_matrix_and_revoked_incarnation_have_no_effect() -> TestResult {
    run_case("forge_limit_fences", |h| {
        Box::pin(async move {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            fund(h, 1, 100).await?;
            for variant in 0..7 {
                let candidate = reserve(h, &authority, variant + 1, 0).await?;
                let mut changed = native::fence()?;
                match variant {
                    0 => {
                        changed.game_session_id =
                            GameSessionId::decode(&native::id(151)).map_err(debug)?
                    }
                    1 => {
                        changed.connection_generation =
                            ConnectionGeneration::new(2).map_err(debug)?
                    }
                    2 => changed.character_lease_generation = 2,
                    3 => {
                        changed.scope_ownership_generation =
                            ScopeOwnershipGeneration::new(2).map_err(debug)?
                    }
                    4 => {
                        changed.runtime_scope = RuntimeScopeRefV1::channel(
                            WorldId::decode(&native::id(native::WORLD)).map_err(debug)?,
                            ChannelId::decode(&native::id(152)).map_err(debug)?,
                        )
                    }
                    5 => {
                        changed.runtime_scope = RuntimeScopeRefV1::channel(
                            WorldId::decode(&native::id(153)).map_err(debug)?,
                            ChannelId::decode(&native::id(native::CHANNEL)).map_err(debug)?,
                        )
                    }
                    6 => {
                        changed.character_id =
                            crate::domain::CharacterId::from_bytes(native::id(154))
                                .map_err(debug)?
                    }
                    _ => unreachable!(),
                }
                assert!(
                    matches!(
                        h.root
                            .commit_forge_dust_limit(
                                &authority,
                                &h.node,
                                changed,
                                native::command(2)?,
                                &candidate
                            )
                            .await,
                        Err(ForgeDustLimitError::AuthorityRejected)
                    ),
                    "fence variant{variant}"
                );
                assert_eq!(effect_counts(h).await?, [1, 0, 0, 0]);
            }
            let candidate = reserve(h, &authority, 50, 0).await?;
            h.root
                .revoke_node_registration(h.node.fact())
                .await
                .map_err(debug)?;
            assert!(matches!(
                h.root
                    .commit_forge_dust_limit(
                        &authority,
                        &h.node,
                        native::fence()?,
                        native::command(2)?,
                        &candidate
                    )
                    .await,
                Err(ForgeDustLimitError::AuthorityRejected)
            ));
            assert_eq!(effect_counts(h).await?, [1, 0, 0, 0]);
            Ok(())
        })
    })
}

#[test]
fn native_limit_raise_revalidates_persisted_session_lease_readiness_and_root() -> TestResult {
    run_case("forge_limit_durable_fences", |h| {
        Box::pin(async move {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            fund(h, 1, 100).await?;
            let facts = [
                (
                    "session",
                    "UPDATE game_durability_reconnect_sessions SET session_state=3",
                    "UPDATE game_durability_reconnect_sessions SET session_state=1",
                ),
                (
                    "eligibility",
                    "UPDATE game_durability_admission_character_guards SET eligible=false",
                    "UPDATE game_durability_admission_character_guards SET eligible=true",
                ),
                (
                    "lease",
                    "UPDATE game_durability_admission_character_guards SET lease_generation=2",
                    "UPDATE game_durability_admission_character_guards SET lease_generation=1",
                ),
                (
                    "readiness",
                    "UPDATE game_durability_admission_runtime_guards SET ready=false",
                    "UPDATE game_durability_admission_runtime_guards SET ready=true",
                ),
                (
                    "scope",
                    "UPDATE game_runtime_scope_assignments SET ownership_generation=2",
                    "UPDATE game_runtime_scope_assignments SET ownership_generation=1",
                ),
            ];
            for (index, (label, break_fact, restore)) in facts.into_iter().enumerate() {
                let index = u16::try_from(index)? + 1;
                let candidate = reserve(h, &authority, index, 0).await?;
                // Negative stale-state fixture only. The candidate was issued by the real native
                // owner before invalidation; no replacement positive authority is fabricated.
                h.tamper(break_fact).await?;
                let frozen = h
                    .root
                    .freeze_forge_dust_limit(
                        &authority,
                        &h.node,
                        native::fence()?,
                        native::command(2)?,
                        request(100 + index, 0),
                    )
                    .await;
                let committed = h
                    .root
                    .commit_forge_dust_limit(
                        &authority,
                        &h.node,
                        native::fence()?,
                        native::command(3)?,
                        &candidate,
                    )
                    .await;
                h.tamper(restore).await?;
                if !matches!(frozen, Err(ForgeDustLimitError::AuthorityRejected)) {
                    return Err(format!("freeze {label}: {frozen:?}").into());
                }
                if !matches!(committed, Err(ForgeDustLimitError::AuthorityRejected)) {
                    return Err(format!("commit {label}: {committed:?}").into());
                }
                let effects = effect_counts(h).await?;
                if effects != [1, 0, 0, 0] {
                    return Err(
                        format!("{label}: refused authority changed effects: {effects:?}").into(),
                    );
                }
                let reservations = h.count("game_forge_dust_limit_reservations").await?;
                if reservations != i64::from(index) {
                    return Err(format!("{label}: unexpected reservations: {reservations}").into());
                }
            }

            // The native root schema admits lifecycle=1 only, and the current root revision
            // guard keeps lifecycle/account/world immutable. A lifecycle=2 root cannot be a
            // reachable stale-authority fixture. Prove the actual forbidden mutation is refused
            // with every trigger and CHECK enabled, then verify the valid root was untouched.
            let candidate = reserve(h, &authority, 6, 0).await?;
            let root_before: String = sqlx::query_scalar(
                "SELECT to_jsonb(r)::text FROM game_character_roots r \
                 WHERE character_id=encode($1,'hex')::uuid",
            )
            .bind(native::id(native::CHARACTER).as_slice())
            .fetch_one(&h.pool)
            .await?;
            let mut mutation = h.pool.begin().await?;
            let changed = sqlx::query(
                "UPDATE game_character_roots SET lifecycle=2, \
                 character_revision=character_revision+1 \
                 WHERE character_id=encode($1,'hex')::uuid",
            )
            .bind(native::id(native::CHARACTER).as_slice())
            .execute(&mut *mutation)
            .await;
            // Roll back even an unexpected success before propagating a test failure.
            let rolled_back = mutation.rollback().await;
            if !matches!(changed, Err(sqlx::Error::Database(ref error))
                if error.code().as_deref() == Some("23514")
                    && error.message() == "Character root permits only one exact revision successor")
            {
                return Err(
                    format!("root mutation: {changed:?}; rollback: {rolled_back:?}").into(),
                );
            }
            rolled_back?;
            let root_after: String = sqlx::query_scalar(
                "SELECT to_jsonb(r)::text FROM game_character_roots r \
                 WHERE character_id=encode($1,'hex')::uuid",
            )
            .bind(native::id(native::CHARACTER).as_slice())
            .fetch_one(&h.pool)
            .await?;
            if root_after != root_before {
                return Err("forbidden lifecycle mutation changed the native root".into());
            }
            let reconciled = h
                .root
                .reconcile_forge_dust_limit(&authority, &candidate)
                .await
                .map_err(debug)?;
            let effects = effect_counts(h).await?;
            let reservations = h.count("game_forge_dust_limit_reservations").await?;
            if reconciled.is_some() || effects != [1, 0, 0, 0] || reservations != 6 {
                return Err(format!("root refusal changed Forge state: {reconciled:?}, {effects:?}, reservations={reservations}").into());
            }
            Ok(())
        })
    })
}

#[test]
fn native_limit_raise_two_occurrences_serialize_and_loser_keeps_quoted_stage() -> TestResult {
    run_case("forge_limit_concurrent", |h| {
        Box::pin(async move {
            // Each real root owns one ready database holder. Racing two operations on a single
            // root tests its nonblocking holder admission, not PostgreSQL row serialization.
            let second_root = DurabilityRoot::connect_test_runtime(&h.database.url)?;
            if !second_root.maintain_ready_once().await? {
                return Err("second native DurabilityRoot did not become ready".into());
            }
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let second_authority = second_root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            fund(h, 1, 100).await?;
            let before_revision = revision(h).await?;
            let first = reserve(h, &authority, 1, 0).await?;
            let second = second_root
                .freeze_forge_dust_limit(
                    &second_authority,
                    &h.node,
                    native::fence()?,
                    native::command(3)?,
                    request(2, 0),
                )
                .await
                .map_err(debug)?;
            let (a, b) = native::join_two(
                h.root.commit_forge_dust_limit(
                    &authority,
                    &h.node,
                    native::fence()?,
                    native::command(2)?,
                    &first,
                ),
                second_root.commit_forge_dust_limit(
                    &second_authority,
                    &h.node,
                    native::fence()?,
                    native::command(3)?,
                    &second,
                ),
            )
            .await;
            let (winner, first_won) = match (a, b) {
                (Ok(winner), Err(ForgeDustLimitError::StageChanged)) => (winner, true),
                (Err(ForgeDustLimitError::StageChanged), Ok(winner)) => (winner, false),
                other => {
                    return Err(format!("expected one Raise and one stale quote: {other:?}").into());
                }
            };
            if winner.replayed || winner.receipt.disposition != ForgeDustLimitDisposition::Raised {
                return Err(
                    format!("concurrent winner was not one fresh Raise: {winner:?}").into(),
                );
            }
            let entry = winner
                .receipt
                .entry
                .ok_or("winner has no paid dust entry")?;
            if (
                entry.before.balance(),
                entry.after.balance(),
                entry.before.limit().get(),
                entry.after.limit().get(),
                entry.amount,
            ) != (100, 75, 100, 120, 25)
            {
                return Err(
                    format!("concurrent winner did not pay its exact quote: {entry:?}").into(),
                );
            }
            // A new CommandRef does not requote the losing retained occurrence at stage 1.
            let retry = if first_won {
                second_root
                    .commit_forge_dust_limit(
                        &second_authority,
                        &h.node,
                        native::fence()?,
                        native::command(4)?,
                        &second,
                    )
                    .await
            } else {
                h.root
                    .commit_forge_dust_limit(
                        &authority,
                        &h.node,
                        native::fence()?,
                        native::command(4)?,
                        &first,
                    )
                    .await
            };
            if !matches!(retry, Err(ForgeDustLimitError::StageChanged)) {
                return Err(
                    format!("losing occurrence changed its retained quote: {retry:?}").into(),
                );
            }
            let effects = effect_counts(h).await?;
            let reservations = h.count("game_forge_dust_limit_reservations").await?;
            let after_revision = revision(h).await?;
            if effects != [2, 1, 1, 1] || reservations != 2 || after_revision != before_revision {
                return Err(format!("concurrent effects: {effects:?}, reservations={reservations}, revision={before_revision}->{after_revision}").into());
            }
            Ok(())
        })
    })
}

#[test]
fn native_limit_raise_all_stages_are_paid_and_the_226th_is_refused() -> TestResult {
    run_case("forge_limit_max", |h| {
        Box::pin(async move {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let original_revision = revision(h).await?;
            for stage in 0..225u16 {
                // Every high limit is reached through all preceding paid native commits. No
                // tampered 4580/4600 fixture stands in for historical unlock authority.
                fund(h, stage + 1, u32::MAX).await?;
                let candidate = reserve(h, &authority, stage + 1, stage).await?;
                let result = h
                    .root
                    .commit_forge_dust_limit(
                        &authority,
                        &h.node,
                        native::fence()?,
                        native::command(u64::from(stage) + 1)?,
                        &candidate,
                    )
                    .await
                    .map_err(debug)?;
                let entry = result.receipt.entry.ok_or("missing paid stage")?;
                assert_eq!(entry.amount, 25 + u32::from(stage));
                assert_eq!(entry.after.limit().get(), 100 + 20 * (stage + 1));
            }
            let before = effect_counts(h).await?;
            assert_eq!(before, [450, 225, 225, 225]);
            assert!(matches!(
                h.root
                    .freeze_forge_dust_limit(
                        &authority,
                        &h.node,
                        native::fence()?,
                        native::command(999)?,
                        request(500, 225)
                    )
                    .await,
                Err(ForgeDustLimitError::MaximumLimitReached)
            ));
            assert_eq!(effect_counts(h).await?, before);
            assert_eq!(revision(h).await?, original_revision);
            let mut connection = h.pool.acquire().await?;
            let ledger =
                audit_forge_dust_ledger(&mut connection, native::fence()?.character_id, 450)
                    .await
                    .map_err(debug)?;
            assert_eq!(ledger.balance.limit().get(), 4600);
            assert_eq!(ledger.spent, (25u64 + 249) * 225 / 2);
            assert_eq!(
                ledger.credited - ledger.spent,
                u64::from(ledger.balance.balance())
            );
            Ok(())
        })
    })
}

#[test]
fn native_limit_raise_partial_ledger_and_forged_terminal_receipt_cannot_commit() -> TestResult {
    run_case("forge_limit_sql_proofs", |h| {
        Box::pin(async move {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            fund(h, 1, 100).await?;
            let _candidate = reserve(h, &authority, 1, 0).await?;
            let before = effect_counts(h).await?;
            // Legal D1230 arithmetic and a correct chain are not sufficient without the exact
            // same-TX receipt, event and history. This direct runtime-role forgery must roll back.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO game_character_forge_dust_entries(entry_id,character_id,previous_entry_id,kind,cause,cause_occurrence_id,transaction_id,amount,lost_amount,balance_before,balance_after,dust_limit_before,dust_limit_after) SELECT r.entry_id,r.character_id,d.last_entry_id,'LIMIT_RAISE','forge_dust_limit',r.occurrence_id,r.transaction_id,25,0,100,75,100,120 FROM game_forge_dust_limit_reservations r JOIN game_character_forge_dust d USING(character_id)").execute(&mut *tx).await?;
            sqlx::query("UPDATE game_character_forge_dust d SET balance=75,dust_limit=120,last_entry_id=r.entry_id FROM game_forge_dust_limit_reservations r WHERE r.character_id=d.character_id").execute(&mut *tx).await?;
            assert!(tx.commit().await.is_err());
            assert_eq!(effect_counts(h).await?, before);
            // Existing PROF-SHAPE-1b's closed SPEND/line pairing remains active after this append.
            // Correct old arithmetic alone cannot bypass its retained 0060 receipt proof.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO game_character_forge_dust_entries(entry_id,character_id,previous_entry_id,kind,cause,cause_occurrence_id,transaction_id,amount,lost_amount,balance_before,balance_after,dust_limit_before,dust_limit_after) SELECT encode($1,'hex')::uuid,character_id,last_entry_id,'SPEND','proficiency',encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,0,100,99,100,100 FROM game_character_forge_dust")
            .bind(occurrence(221,1).as_slice()).bind(occurrence(222,1).as_slice()).bind(occurrence(223,1).as_slice()).execute(&mut *tx).await?;
            sqlx::query("UPDATE game_character_forge_dust SET balance=99,last_entry_id=encode($1,'hex')::uuid")
            .bind(occurrence(221,1).as_slice()).execute(&mut *tx).await?;
            let committed = tx.commit().await;
            assert!(
                matches!(committed,Err(sqlx::Error::Database(ref error)) if error.code().as_deref()==Some("23514") && error.message().contains("proficiency modification dust does not equal"))
            );
            assert_eq!(effect_counts(h).await?, before);
            // Fabricated REVISION_CHANGED while the actual active binding is unchanged is refused.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "UPDATE game_forge_dust_limit_reservations SET work_units_used=work_units_used+1",
            )
            .execute(&mut *tx)
            .await?;
            sqlx::query("INSERT INTO game_forge_dust_limit_receipts(character_id,occurrence_id,transaction_id,event_id,entry_id,outcome) SELECT character_id,occurrence_id,transaction_id,event_id,entry_id,2 FROM game_forge_dust_limit_reservations").execute(&mut *tx).await?;
            assert!(tx.commit().await.is_err());
            assert_eq!(effect_counts(h).await?, before);
            Ok(())
        })
    })
}

#[test]
fn native_limit_raise_failed_commit_rolls_back_then_same_candidate_reconciles_and_retries()
-> TestResult {
    run_case("forge_limit_abort", |h| {
        Box::pin(async move {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            fund(h, 1, 100).await?;
            let candidate = reserve(h, &authority, 1, 0).await?;
            // Test-only fault at the actual final receipt constraint; all other real guards stay on.
            sqlx::raw_sql("CREATE FUNCTION fixture_reject_forge_limit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'fixture forced noncommit' USING ERRCODE='23514'; END $$; CREATE CONSTRAINT TRIGGER fixture_reject_forge_limit AFTER INSERT ON game_forge_dust_limit_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION fixture_reject_forge_limit();").execute(&h.pool).await?;
            assert!(matches!(
                h.root
                    .commit_forge_dust_limit(
                        &authority,
                        &h.node,
                        native::fence()?,
                        native::command(2)?,
                        &candidate
                    )
                    .await,
                Err(ForgeDustLimitError::Unavailable(_))
            ));
            assert_eq!(effect_counts(h).await?, [1, 0, 0, 0]);
            assert!(
                h.root
                    .reconcile_forge_dust_limit(&authority, &candidate)
                    .await
                    .map_err(debug)?
                    .is_none()
            );
            sqlx::query(
                "DROP TRIGGER fixture_reject_forge_limit ON game_forge_dust_limit_receipts",
            )
            .execute(&h.pool)
            .await?;
            let retry = h
                .root
                .commit_forge_dust_limit(
                    &authority,
                    &h.node,
                    native::fence()?,
                    native::command(3)?,
                    &candidate,
                )
                .await
                .map_err(debug)?;
            assert_eq!(retry.receipt.entry.ok_or("retry entry")?.amount, 25);
            assert_eq!(effect_counts(h).await?, [2, 1, 1, 1]);
            assert_eq!(
                h.root
                    .reconcile_forge_dust_limit(&authority, &candidate)
                    .await
                    .map_err(debug)?,
                Some(retry.receipt)
            );
            Ok(())
        })
    })
}

#[test]
fn native_limit_raise_retry_budget_exhaustion_keeps_reconciliation_available() -> TestResult {
    run_case("forge_limit_retry_budget", |h| {
        Box::pin(async move {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            fund(h, 1, 100).await?;
            let candidate = reserve(h, &authority, 1, 0).await?;
            sqlx::raw_sql("CREATE FUNCTION fixture_reject_forge_limit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'fixture forced noncommit' USING ERRCODE='23514'; END $$; CREATE CONSTRAINT TRIGGER fixture_reject_forge_limit AFTER INSERT ON game_forge_dust_limit_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION fixture_reject_forge_limit();").execute(&h.pool).await?;
            for attempt in 1..=3u64 {
                let outcome = h
                    .root
                    .commit_forge_dust_limit(
                        &authority,
                        &h.node,
                        native::fence()?,
                        native::command(attempt + 1)?,
                        &candidate,
                    )
                    .await;
                assert!(
                    matches!(outcome, Err(ForgeDustLimitError::Unavailable(_))),
                    "attempt {attempt}: {outcome:?}"
                );
                let used: i16 = sqlx::query_scalar(
                    "SELECT work_units_used FROM game_forge_dust_limit_reservations",
                )
                .fetch_one(&h.pool)
                .await?;
                assert_eq!(u64::try_from(used)?, attempt);
            }
            sqlx::query(
                "DROP TRIGGER fixture_reject_forge_limit ON game_forge_dust_limit_receipts",
            )
            .execute(&h.pool)
            .await?;
            let fourth = h
                .root
                .commit_forge_dust_limit(
                    &authority,
                    &h.node,
                    native::fence()?,
                    native::command(9)?,
                    &candidate,
                )
                .await;
            assert!(matches!(fourth, Err(ForgeDustLimitError::CapacityExceeded)));
            assert!(
                h.root
                    .reconcile_forge_dust_limit(&authority, &candidate)
                    .await
                    .map_err(debug)?
                    .is_none()
            );
            assert_eq!(effect_counts(h).await?, [1, 0, 0, 0]);
            Ok(())
        })
    })
}

#[test]
fn native_limit_raise_migration_rejects_legacy_225_and_disconnected_history() -> TestResult {
    run_native_case("forge_limit_legacy", false, |h| {
        Box::pin(async move {
            fund(h, 1, 100).await?;
            // This negative fixture represents unexplained pre-existing data. It never stands in
            // for a positive unlock or authority proof: the migration must refuse it unchanged.
            h.tamper("UPDATE game_character_forge_dust SET dust_limit=225")
                .await?;
            let mut tx = h.pool.begin().await?;
            let result = sqlx::raw_sql(APPEND_SQL).execute(&mut *tx).await;
            assert!(
                matches!(result,Err(sqlx::Error::Database(ref error)) if error.code().as_deref()==Some("23514") && error.message().contains("proven legacy limit-100"))
            );
            tx.rollback().await?;
            h.tamper("UPDATE game_character_forge_dust SET dust_limit=100")
                .await?;
            // Both records have valid old Gain arithmetic and limit100, but form a detached cycle.
            let a = native::uuid_text(occurrence(211, 1));
            let b = native::uuid_text(occurrence(212, 1));
            let character = native::uuid_text(native::id(native::CHARACTER));
            h.tamper(&format!("INSERT INTO game_character_forge_dust_entries(entry_id,character_id,previous_entry_id,kind,cause,cause_occurrence_id,transaction_id,amount,lost_amount,balance_before,balance_after,dust_limit_before,dust_limit_after) VALUES ('{a}','{character}','{b}','GAIN','creature_kill','{a}','{a}',1,1,100,100,100,100),('{b}','{character}','{a}','GAIN','creature_kill','{b}','{b}',1,1,100,100,100,100)")).await?;
            let mut tx = h.pool.begin().await?;
            let result = sqlx::raw_sql(APPEND_SQL).execute(&mut *tx).await;
            assert!(
                matches!(result,Err(sqlx::Error::Database(ref error)) if error.code().as_deref()==Some("23514") && error.message().contains("unreachable component"))
            );
            tx.rollback().await?;
            let legacy_limit:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_constraint WHERE conname='game_character_forge_dust_limit_range' AND pg_get_constraintdef(oid) LIKE '%225%')").fetch_one(&h.pool).await?;
            assert!(legacy_limit, "failed migration did not alter the old bound");
            Ok(())
        })
    })
}
