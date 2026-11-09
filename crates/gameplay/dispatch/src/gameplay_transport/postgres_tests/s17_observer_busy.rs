//! Observer admission and moderation share the busy-holder command budget.
use super::{
    TestResult,
    fixture::{Harness, run},
    item_look::large_stack,
};
use oteryn_gameplay_social::{
    cast::{
        CastOwner, Settings,
        preferences::{Preferences, PreferencesStore},
        service::{Operation, Reply, apply_durable},
        test_support::admission,
    },
    spectate::spectate_test_support::facts,
};
use prost::Message as _;
use std::{collections::HashMap, future::Future, task::Poll};

struct Access;
impl PreferencesStore for Access {
    async fn load(&self, _: [u8; 16]) -> Result<Preferences, oteryn_gameplay_social::cast::Error> {
        Ok(Preferences {
            settings: Settings::default(),
            description: "Busy observer".into(),
        })
    }
}

#[test]
fn s17c_observer_postgres_busy_join_renew_and_ban_wait_without_retrying_storage() -> TestResult {
    large_stack(|| {
        run(async |admin| {
            let h = Harness::create(admin, "s17c_observer_busy", true).await?;
            let result = async {
                let facts = facts(1, 1, 1);
                let mut owner = CastOwner::new(8)?;
                let state = owner.start(facts, Settings::default(), 0)?;
                owner.persist_audit(&h.root, &h.node).await?;
                let accounts = HashMap::from([(facts.authority.current_game_session_id(), [1; 16])]);
                let mut observer = None;
                for step in 0..3 {
                    let op = match observer {
                        None => Operation::Join(admission(state, [2; 16], [3; 16], 1)),
                        Some(id) if step == 1 => Operation::Renew(id, admission(state, [2; 16], [3; 16], 1)),
                        Some(id) => Operation::Ban(facts, state.revision, id),
                    };
                    let held = h.root.try_issue_semantic_pass()?;
                    let mut exchange = std::pin::pin!(apply_durable(
                        &mut owner, &h.root, &h.node, &Access, &accounts, op, 25_000, 0,
                    ));
                    let first = std::future::poll_fn(|cx| Poll::Ready(exchange.as_mut().poll(cx))).await;
                    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                    assert!(first.is_pending(), "step {step}: busy holder must wait before storage");
                    if let Reply::Observer(id) = exchange.await? {
                        observer = Some(id);
                    }
                }
                assert!(owner.leave(observer.ok_or("observer")?).is_err(), "ban removes the observer");
                assert!(h.root.cast_account_banned(&h.node, [1; 16], [2; 16]).await?);
                owner.persist_audit(&h.root, &h.node).await?;
                let bans: i64 = sqlx::query_scalar("SELECT count(*) FROM game_cast_account_bans").fetch_one(&h.pool).await?;
                assert_eq!(bans, 1);
                let records: Vec<Vec<u8>> = sqlx::query_scalar("SELECT payload FROM game_cast_audit_outbox LIMIT 64").fetch_all(&h.pool).await?;
                let events = records.iter().map(|bytes| crate::durability::cast_audit::CastAuditRecord::decode(bytes.as_slice())).collect::<Result<Vec<_>, _>>()?;
                assert_eq!(events.iter().filter(|event| event.kind == 12).count(), 1, "busy retries cannot duplicate the ban audit");

                let Reply::Observer(id) = apply_durable(&mut owner, &h.root, &h.node, &Access, &accounts,
                    Operation::Join(admission(state, [4; 16], [5; 16], 1)), 25_000, 0).await? else {
                    return Err("join reply".into());
                };
                sqlx::query("ALTER TABLE game_cast_audit_outbox ADD CONSTRAINT s17c_reject CHECK (ordinal < 0) NOT VALID").execute(&h.pool).await?;
                assert!(apply_durable(&mut owner, &h.root, &h.node, &Access, &accounts,
                    Operation::Ban(facts, state.revision, id), 25_000, 0).await.is_err());
                assert!(!h.root.cast_account_banned(&h.node, [1; 16], [4; 16]).await?, "storage failure rolls back the ban");
                owner.leave(id)?;
                sqlx::query("DROP TABLE game_cast_account_bans").execute(&h.pool).await?;
                assert!(apply_durable(&mut owner, &h.root, &h.node, &Access, &accounts,
                    Operation::Join(admission(state, [6; 16], [7; 16], 1)), 25_000, 0).await.is_err(), "ban read storage errors remain fatal");
                Ok::<(), Box<dyn std::error::Error>>(())
            }.await;
            h.cleanup().await?;
            result
        })
    })
}
