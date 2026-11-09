//! A busy shared holder must not turn an unwritten audit into a storage failure.
use super::{
    TestResult,
    fixture::{Harness, run},
    item_look::large_stack,
};
use oteryn_gameplay_social::{
    cast::{CastOwner, Settings},
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
