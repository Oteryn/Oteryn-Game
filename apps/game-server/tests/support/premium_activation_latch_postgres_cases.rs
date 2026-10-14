// PREM-WIRE-1 durable Premium activation latch (migration 0085,
// `durability::premium_activation_latch`, PREMIUM-ACTIVATION-0 §1.2-§1.3) on the CHARM-2 harness.
// The latch needs no Character.

use crate::bestiary_postgres_harness::{Harness, TestResult, configured_admin, runtime};
use crate::durability::DurabilityRoot;
use crate::durability::premium_activation_latch::{
    EmptyLatchHeld, PREMIUM_ACTIVATION_LOCK_KEY, PreDeliveryBypass, PremiumActivationLatch,
    PremiumActivationRead, PremiumGateTx,
};
use std::future::Future;
use std::pin::Pin;

/// 2026-10-01T00:00:00Z.
const S: i64 = 1_790_812_800_000_000;

/// A bypass that writes nothing and proves it ran in the gate's transaction.
struct Probe;
impl PreDeliveryBypass for Probe {
    type Output = i32;
    fn apply<'a, 't>(
        self,
        _held: &'a EmptyLatchHeld,
        tx: &'a mut PremiumGateTx<'t>,
    ) -> Pin<Box<dyn Future<Output = i32> + Send + 'a>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            sqlx::query_scalar::<_, i32>("SELECT 7")
                .fetch_one(&mut **tx)
                .await
                .unwrap_or(0)
        })
    }
}

fn latch(activation_id: &str, latched_at_us: i64) -> PremiumActivationLatch {
    PremiumActivationLatch {
        activation_id: activation_id.to_owned(),
        switch_over_us: S,
        latched_at_us,
    }
}

fn run(
    tag: &'static str,
    case: impl AsyncFnOnce(&Harness, &DurabilityRoot) -> TestResult,
) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, tag, false).await?;
        let root = harness.root.clone();
        case(&harness, &root).await?;
        harness.cleanup().await
    })
}

#[test]
fn premium_activation_latches_once_and_ends_the_pre_delivery_bypass() -> TestResult {
    run("premium_latch", async |_, root| {
        assert_eq!(
            root.read_premium_activation::<i32>().await?,
            PremiumActivationRead::Empty
        );
        let mut bypass = Some(Probe);
        assert_eq!(
            root.read_premium_activation_with(&mut bypass).await?,
            PremiumActivationRead::PreDelivery(7)
        );
        assert!(bypass.is_none());
        let first = root
            .latch_premium_activation("activation-1", S, S + 10)
            .await?;
        assert_eq!(first, latch("activation-1", S + 10));
        // A second latch, of any activation, returns the winner unchanged.
        let second = root
            .latch_premium_activation("activation-2", S, S + 20)
            .await?;
        assert_eq!(second, first);
        assert_eq!(
            root.read_premium_activation::<i32>().await?,
            PremiumActivationRead::Latched(first.clone())
        );
        // After the latch no bypass runs; it is handed back.
        let mut bypass = Some(Probe);
        assert_eq!(
            root.read_premium_activation_with(&mut bypass).await?,
            PremiumActivationRead::Latched(first)
        );
        assert!(bypass.is_some());
        Ok(())
    })
}

#[test]
fn premium_activation_latch_waits_for_a_bypass_holding_the_shared_lock() -> TestResult {
    run("premium_latch_fence", async |harness, root| {
        // An in-flight pre-delivery bypass: the shared lock, held by an open transaction.
        let mut bypass = harness.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock_shared($1)")
            .bind(PREMIUM_ACTIVATION_LOCK_KEY)
            .execute(&mut *bypass)
            .await?;
        // The latch waits at most 5 s for it, then fails and writes nothing.
        assert!(
            root.latch_premium_activation("activation-1", S, S)
                .await
                .is_err()
        );
        assert_eq!(harness.count("game_premium_activation_latch").await?, 0);
        bypass.commit().await?;
        // The failed pass retired its holder: the root is ready again after one maintenance
        // window, as after any failed pass.
        assert!(root.maintain_ready_once().await?);
        root.latch_premium_activation("activation-1", S, S).await?;
        // A latch holding the lock exclusively blocks a new shared reader until it commits.
        let mut latching = harness.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(PREMIUM_ACTIVATION_LOCK_KEY)
            .execute(&mut *latching)
            .await?;
        let shared: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock_shared($1)")
            .bind(PREMIUM_ACTIVATION_LOCK_KEY)
            .fetch_one(&harness.pool)
            .await?;
        assert!(!shared);
        latching.rollback().await?;
        Ok(())
    })
}

#[test]
fn premium_activation_latch_is_immutable() -> TestResult {
    run("premium_latch_guards", async |harness, root| {
        root.latch_premium_activation("activation-1", S, S).await?;
        for statement in [
            "UPDATE game_premium_activation_latch SET activation_id = 'activation-2'",
            "UPDATE game_premium_activation_latch SET switch_over_us = 0",
            "DELETE FROM game_premium_activation_latch",
            "TRUNCATE game_premium_activation_latch",
            "INSERT INTO game_premium_activation_latch \
             (id, activation_id, switch_over_us, latched_at_us) VALUES (2, 'b', 0, 0)",
            "INSERT INTO game_premium_activation_latch \
             (id, activation_id, switch_over_us, latched_at_us) VALUES (1, 'b', 0, 0)",
        ] {
            let result = sqlx::query(sqlx::AssertSqlSafe(statement))
                .execute(&harness.pool)
                .await;
            assert!(result.is_err(), "{statement}");
        }
        assert_eq!(
            root.read_premium_activation::<i32>().await?,
            PremiumActivationRead::Latched(latch("activation-1", S))
        );
        // The runtime role reads and inserts the latch, never more.
        let privileges: Vec<(String, bool)> = sqlx::query_as(
            "SELECT p, has_table_privilege('oteryn_game_runtime', \
                                           'game_premium_activation_latch', p) \
               FROM unnest(ARRAY['SELECT', 'INSERT', 'UPDATE', 'DELETE', 'TRUNCATE']) AS p",
        )
        .fetch_all(&harness.pool)
        .await?;
        for (privilege, held) in privileges {
            assert_eq!(
                held,
                privilege == "SELECT" || privilege == "INSERT",
                "{privilege}"
            );
        }
        Ok(())
    })
}
