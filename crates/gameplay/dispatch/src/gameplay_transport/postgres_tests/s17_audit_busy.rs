//! A busy shared holder must not turn an unwritten audit into a storage failure.
use super::{
    TestResult,
    fixture::{Harness, run},
    item_look::large_stack,
};
use oteryn_gameplay_social::{
    cast::{
        CastOwner, Error, Settings,
        preferences::{Preferences, PreferencesStore},
        service::{Operation, Reply, apply_durable},
        test_support::admission,
    },
    spectate::spectate_test_support::facts,
};
use std::{future::Future, task::Poll};

#[test]
fn s17_audit_postgres_busy_holder_preserves_cast_and_storage_failure_closes_it() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            let h = Harness::create(admin, "s17_audit_busy", true).await?;
            let result = async {
                let target = facts(1, 1, 1);
                let session = target.authority.current_game_session_id();
                let mut cast = CastOwner::new(8)?;
                let state = cast.start(target, Settings::default(), 0)?;
                let held = h.root.try_issue_semantic_pass()?;
                let (pending, persisted) = {
                    let mut persist = std::pin::pin!(cast.persist_audit(&h.root, &h.node));
                    let first = std::future::poll_fn(|cx| {
                        Poll::Ready(persist.as_mut().poll(cx))
                    })
                    .await;
                    // Release only after the audit has actually attempted acquisition.
                    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                    let pending = first.is_pending();
                    let result = match first {
                        Poll::Pending => persist.await,
                        Poll::Ready(result) => result,
                    };
                    (pending, result)
                };
                assert!(pending, "a busy holder must wait without closing the cast");
                persisted?;
                assert_eq!(cast.state(session), Some(state));
                let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_cast_audit_outbox")
                    .fetch_one(&h.pool)
                    .await?;
                assert_eq!(count, 1, "the pending prefix commits exactly once");

                sqlx::query("ALTER TABLE game_cast_audit_outbox ADD CONSTRAINT s17_reject CHECK (ordinal < 0) NOT VALID")
                    .execute(&h.pool).await?;
                cast.stop(target, state.revision)?;
                let next = cast.start(target, Settings::default(), 0)?;
                assert!(cast.persist_audit(&h.root, &h.node).await.is_err());
                assert!(cast.state(session).is_none(), "storage failure still closes the cast");
                assert!(next.revision > state.revision);
                let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_cast_audit_outbox")
                    .fetch_one(&h.pool)
                    .await?;
                assert_eq!(count, 1, "failed storage cannot acknowledge a prefix");
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            h.cleanup().await?;
            result
        })
    })
}

struct StoredPreferences;
impl PreferencesStore for StoredPreferences {
    async fn load(&self, _: [u8; 16]) -> Result<Preferences, Error> {
        Ok(Preferences {
            settings: Settings::default(),
            description: "A cast".into(),
        })
    }
}

async fn after_holder_release<T>(
    root: &crate::durability::DurabilityRoot,
    operation: impl Future<Output = Result<T, Error>>,
) -> TestResult<Result<T, Error>> {
    let held = root.try_issue_semantic_pass()?;
    let mut operation = std::pin::pin!(operation);
    let first = std::future::poll_fn(|cx| Poll::Ready(operation.as_mut().poll(cx))).await;
    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
    assert!(
        first.is_pending(),
        "an observer ban check must wait for the busy holder"
    );
    Ok(operation.await)
}

#[test]
fn s17_observer_postgres_busy_ban_read_preserves_admission_and_fail_closed_checks() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            let h = Harness::create(admin, "s17_observer_ban_busy", true).await?;
            let result = async {
                let target = facts(1, 1, 1);
                let session = target.authority.current_game_session_id();
                let mut cast = CastOwner::new(8)?;
                let state = cast.start(target, Settings::default(), 0)?;
                let accounts = std::collections::HashMap::from([(session, [1; 16])]);
                cast.persist_audit(&h.root, &h.node).await?;
                let Reply::Observer(observer) = after_holder_release(
                    &h.root,
                    apply_durable(
                        &mut cast,
                        &h.root,
                        &h.node,
                        &StoredPreferences,
                        &accounts,
                        Operation::Join(admission(state, [2; 16], [2; 16], 1)),
                        0,
                        0,
                    ),
                )
                .await??
                else {
                    return Err("observer reply".into());
                };
                assert_eq!(cast.state(session).map(|s| s.viewers), Some(1));
                let renewed = after_holder_release(
                    &h.root,
                    apply_durable(
                        &mut cast,
                        &h.root,
                        &h.node,
                        &StoredPreferences,
                        &accounts,
                        Operation::Renew(observer, admission(state, [2; 16], [2; 16], 1)),
                        25_000,
                        0,
                    ),
                )
                .await??;
                assert!(matches!(renewed, Reply::Applied));

                let record = crate::durability::cast_audit::CastAuditRecord {
                    ordinal: 1,
                    kind: 12,
                    target_session: session.as_bytes().to_vec(),
                    cast_id: state.cast_id.to_vec(),
                    revision: state.revision,
                    epoch: state.epoch,
                    subject_account: Some(vec![3; 16]),
                    ..Default::default()
                };
                h.root
                    .ban_cast_account(&h.node, [1; 16], [3; 16], record)
                    .await?;
                let banned = after_holder_release(
                    &h.root,
                    apply_durable(
                        &mut cast,
                        &h.root,
                        &h.node,
                        &StoredPreferences,
                        &accounts,
                        Operation::Join(admission(state, [3; 16], [3; 16], 1)),
                        29_000,
                        0,
                    ),
                )
                .await?;
                assert!(matches!(banned, Err(Error::Unavailable)));
                assert_eq!(cast.state(session).map(|s| s.viewers), Some(1));

                // The fixture grant expires at 30_000. The retry's 2 ms floor
                // crosses both grant and current-observer expiry before renewal.
                let expired = after_holder_release(
                    &h.root,
                    apply_durable(
                        &mut cast,
                        &h.root,
                        &h.node,
                        &StoredPreferences,
                        &accounts,
                        Operation::Renew(observer, admission(state, [2; 16], [2; 16], 1)),
                        29_999,
                        0,
                    ),
                )
                .await?;
                assert!(matches!(expired, Err(Error::Unavailable)));
                assert_eq!(cast.state(session).map(|s| s.viewers), Some(0));

                let mut unavailable = CastOwner::new(8)?;
                let fresh = unavailable.start(target, Settings::default(), 0)?;
                sqlx::query("ALTER TABLE game_cast_account_bans RENAME TO s17_unavailable_bans")
                    .execute(&h.pool)
                    .await?;
                let storage_failed = apply_durable(
                    &mut unavailable,
                    &h.root,
                    &h.node,
                    &StoredPreferences,
                    &accounts,
                    Operation::Join(admission(fresh, [4; 16], [4; 16], 1)),
                    0,
                    0,
                )
                .await;
                sqlx::query("ALTER TABLE s17_unavailable_bans RENAME TO game_cast_account_bans")
                    .execute(&h.pool)
                    .await?;
                assert!(matches!(storage_failed, Err(Error::Unavailable)));
                assert_eq!(
                    unavailable.state(session),
                    Some(fresh),
                    "storage failure cannot admit an observer"
                );
                Ok::<(), Box<dyn std::error::Error>>(())
            }
            .await;
            h.cleanup().await?;
            result
        })
    })
}
