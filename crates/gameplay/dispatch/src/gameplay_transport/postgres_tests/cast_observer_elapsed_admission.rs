//! Observer expiry is checked after both holder waits and an actual ban-table read.
use super::TestResult;
use super::fixture::{Harness, join_two};
use super::kill_reward_e2e::scenario_thread::run_async as run;
use oteryn_gameplay_social::{
    cast::{
        CastOwner, Error, Settings,
        preferences::{Preferences, PreferencesStore},
        service::{Operation, apply_durable},
        test_support::admission,
    },
    spectate::spectate_test_support::facts,
};
use std::{collections::HashMap, future::Future, task::Poll, time::Duration};

const REQUEST_NOW_MS: u64 = 29_900;
// The existing verified-admission fixture expires at exactly 30 seconds.
const PROOF_EXPIRES_MS: u64 = 30_000;

struct Access;
impl PreferencesStore for Access {
    async fn load(&self, _: [u8; 16]) -> Result<Preferences, Error> {
        Ok(Preferences {
            settings: Settings::default(),
            description: "Expiry boundary".into(),
        })
    }
}

#[test]
fn cast_observer_join_and_renew_reject_expiry_across_holder_and_sql_waits_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run(async |admin| {
        let h = Harness::create(admin, "cast_observer_elapsed_admission", true).await?;
        let result = async {
            for in_sql in [false, true] {
                for renew in [false, true] {
                    let facts = facts(1, 1, 1);
                    let mut owner = CastOwner::new(8)?;
                    let state = owner.start(facts, Settings::default(), 0)?;
                    let accounts = HashMap::from([
                        (facts.authority.current_game_session_id(), [1; 16]),
                    ]);
                    let proof = admission(state, [2; 16], [3; 16], 1);
                    let operation = if renew {
                        let id = owner.join(&proof, 29_000)?;
                        Operation::Renew(id, proof)
                    } else {
                        Operation::Join(proof)
                    };
                    let reply = if in_sql {
                        let mut blocker = h.pool.begin().await?;
                        sqlx::query("LOCK TABLE game_cast_account_bans IN ACCESS EXCLUSIVE MODE")
                            .execute(&mut *blocker)
                            .await?;
                        let exchange = apply_durable(
                            &mut owner, &h.root, &h.node, &Access, &accounts,
                            operation, REQUEST_NOW_MS, 0,
                        );
                        let release = async {
                            tokio::time::timeout(Duration::from_secs(1), async {
                                loop {
                                    let waiting: bool = sqlx::query_scalar(
                                        "SELECT EXISTS(SELECT 1 FROM pg_locks \
                                         WHERE NOT granted AND relation='game_cast_account_bans'::regclass)",
                                    )
                                    .fetch_one(&h.pool)
                                    .await?;
                                    if waiting {
                                        return Ok::<(), Box<dyn std::error::Error>>(());
                                    }
                                    tokio::task::yield_now().await;
                                }
                            })
                            .await??;
                            // The real query is waiting, so the operation has
                            // already sampled its clock. Cross its exact expiry.
                            cross_expiry().await;
                            blocker.rollback().await?;
                            Ok::<(), Box<dyn std::error::Error>>(())
                        };
                        let (reply, released) = join_two(exchange, release).await;
                        released?;
                        reply
                    } else {
                        let held = h.root.try_issue_semantic_pass()?;
                        let mut exchange = Box::pin(apply_durable(
                            &mut owner, &h.root, &h.node, &Access, &accounts,
                            operation, REQUEST_NOW_MS, 0,
                        ));
                        assert!(
                            std::future::poll_fn(|cx| Poll::Ready(exchange.as_mut().poll(cx)))
                                .await
                                .is_pending()
                        );
                        cross_expiry().await;
                        held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                        exchange.await
                    };
                    assert!(
                        matches!(reply, Err(Error::Unavailable)),
                        "in_sql={in_sql}, renew={renew}: expired admission must refuse"
                    );
                    // A storage error before the owner decision cannot satisfy
                    // this: the real join/renew must have ticked past expiry.
                    assert!(owner.poll(PROOF_EXPIRES_MS - 1).is_err());
                    assert_eq!(
                        owner.state(facts.authority.current_game_session_id())
                            .ok_or("cast state")?
                            .viewers,
                        0
                    );
                    assert!(!h.root.cast_account_banned(&h.node, [1; 16], [2; 16]).await?);
                }
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        }
        .await;
        h.cleanup().await?;
        result
    })
}

async fn cross_expiry() {
    let expiry =
        tokio::time::Instant::now() + Duration::from_millis(PROOF_EXPIRES_MS - REQUEST_NOW_MS);
    tokio::time::sleep_until(expiry).await;
}
