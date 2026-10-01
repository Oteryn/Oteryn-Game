// PREM-1a Premium consumer fence (migration 0029, `durability::premium_fence`, `premium`).
// Every wrapper provides the same path-loaded crate root, the `premium` module and the
// `bestiary_postgres_harness` module; the fence needs no Character.

use crate::bestiary_postgres_harness::{Harness, TestResult, configured_admin, runtime};
use crate::durability::DurabilityRoot;
use crate::durability::premium_fence::{EntitlementState, PremiumEvidence, PremiumFenceOutcome};
use crate::premium::snapshot::canonical_uuid;
use crate::premium::{
    IngestOutcome, PRODUCER_PROFILE, PRODUCT_ID, PremiumClass, PremiumConsumer, TrustedNow,
};

const ACCOUNT: [u8; 16] = [0x61; 16];
const HOUR: i64 = 3_600_000_000;
/// 2026-09-30T12:00:00Z.
const T0: i64 = 1_790_769_600_000_000;

fn evidence(authority: u64, lifecycle: u64, state: EntitlementState) -> PremiumEvidence {
    PremiumEvidence {
        account_id: ACCOUNT,
        producer_revision: "c914564".into(),
        producer_profile: PRODUCER_PROFILE.into(),
        product_id: PRODUCT_ID.into(),
        product_version: 1,
        entitlement_id: (state != EntitlementState::None).then(|| "ent-1".into()),
        state,
        lifecycle_revision: lifecycle,
        authority_revision: authority,
        effective_from_us: T0 - 24 * HOUR,
        effective_until_us: T0 + 24 * HOUR,
        authority_issued_at_us: T0,
        authority_valid_until_us: T0 + HOUR,
        refresh_after_us: T0 + HOUR * 2 / 3,
    }
}

fn outcome_of(outcome: PremiumFenceOutcome) -> (&'static str, u64, EntitlementState, bool) {
    let (kind, view) = match outcome {
        PremiumFenceOutcome::Accepted(view) => ("accepted", view),
        PremiumFenceOutcome::Replayed(view) => ("replayed", view),
        PremiumFenceOutcome::Stale(view) => ("stale", view),
        PremiumFenceOutcome::Conflict(view) => ("conflict", view),
    };
    (
        kind,
        view.latest.authority_revision,
        view.latest.state,
        view.conflicting,
    )
}

async fn accept(
    root: &DurabilityRoot,
    e: &PremiumEvidence,
) -> TestResult<(&'static str, u64, EntitlementState, bool)> {
    Ok(outcome_of(root.accept_premium_evidence(e).await?))
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
fn premium_fence_is_monotonic_and_idempotent() -> TestResult {
    use EntitlementState::{Active, None as NoEntitlement, Revoked};
    run("premium_order", async |harness, root| {
        assert_eq!(root.load_premium_fence(ACCOUNT).await?, None);
        assert_eq!(
            accept(root, &evidence(5, 1, Active)).await?,
            ("accepted", 5, Active, false)
        );
        assert_eq!(
            accept(root, &evidence(5, 1, Active)).await?,
            ("replayed", 5, Active, false)
        );
        // A renewal: same lifecycle, strictly higher authority revision.
        assert_eq!(
            accept(root, &evidence(6, 1, Active)).await?,
            ("accepted", 6, Active, false)
        );
        // Never-seen lower revision: stale, the fence keeps revision 6.
        assert_eq!(
            accept(root, &evidence(4, 1, Active)).await?,
            ("stale", 6, Active, false)
        );
        assert_eq!(
            accept(root, &evidence(8, 2, Revoked)).await?,
            ("accepted", 8, Revoked, false)
        );
        // Delayed older ACTIVE after a newer revoke: stale, the revoke stays.
        assert_eq!(
            accept(root, &evidence(6, 1, Active)).await?,
            ("stale", 8, Revoked, false)
        );
        // NONE advances the account high water; no older ACTIVE comes back.
        assert_eq!(
            accept(root, &evidence(9, 0, NoEntitlement)).await?,
            ("accepted", 9, NoEntitlement, false)
        );
        assert_eq!(
            accept(root, &evidence(7, 1, Active)).await?,
            ("stale", 9, NoEntitlement, false)
        );
        // Another entitlement opens its own row.
        let mut other = evidence(10, 1, Active);
        other.entitlement_id = Some("ent-2".into());
        assert_eq!(accept(root, &other).await?, ("accepted", 10, Active, false));
        assert_eq!(harness.count("game_premium_evidence").await?, 5);
        assert_eq!(harness.count("game_premium_entitlement_fence").await?, 2);
        let loaded = root.load_premium_fence(ACCOUNT).await?.ok_or("fence")?;
        assert_eq!(loaded.latest, other);
        Ok(())
    })
}

#[test]
fn premium_equivocation_fails_closed_and_sticks() -> TestResult {
    use EntitlementState::Active;
    run("premium_conflict", async |harness, root| {
        accept(root, &evidence(5, 1, Active)).await?;
        accept(root, &evidence(6, 1, Active)).await?;
        // Same current revision, other content.
        let mut contradiction = evidence(6, 1, Active);
        contradiction.authority_valid_until_us += 1;
        assert_eq!(
            accept(root, &contradiction).await?,
            ("conflict", 6, Active, true)
        );
        // Build provenance and scheduling are not authority: still an exact replay.
        let mut rebuilt = evidence(6, 1, Active);
        rebuilt.producer_revision = "d00d".into();
        rebuilt.refresh_after_us += 1;
        assert_eq!(accept(root, &rebuilt).await?, ("replayed", 6, Active, true));
        // Newer evidence is still fenced, the conflict stays.
        assert_eq!(
            accept(root, &evidence(7, 1, Active)).await?,
            ("accepted", 7, Active, true)
        );
        assert_eq!(harness.count("game_premium_evidence").await?, 3);
        Ok(())
    })?;
    run("premium_history", async |_, root| {
        accept(root, &evidence(5, 1, Active)).await?;
        accept(root, &evidence(6, 1, Active)).await?;
        // Same historical revision, other content: detected below the high water.
        let mut historical = evidence(5, 1, Active);
        historical.effective_until_us -= 1;
        assert_eq!(
            accept(root, &historical).await?,
            ("conflict", 6, Active, true)
        );
        Ok(())
    })?;
    run("premium_lifecycle", async |harness, root| {
        accept(root, &evidence(5, 2, Active)).await?;
        // A higher authority revision may not lower the lifecycle revision...
        assert_eq!(
            accept(root, &evidence(6, 1, Active)).await?,
            ("conflict", 5, Active, true)
        );
        // ...nor change lifecycle facts under the same one.
        let mut changed = evidence(7, 2, Active);
        changed.effective_until_us += HOUR;
        assert_eq!(accept(root, &changed).await?, ("conflict", 5, Active, true));
        assert_eq!(harness.count("game_premium_evidence").await?, 1);
        Ok(())
    })
}

#[test]
fn premium_time_derived_state_is_not_a_lifecycle_change() -> TestResult {
    use EntitlementState::{Active, Expired, NotYetEffective};
    run("premium_state_time", async |harness, root| {
        let mut pending = evidence(5, 1, NotYetEffective);
        pending.effective_from_us = T0 + HOUR;
        pending.effective_until_us = T0 + 4 * HOUR;
        assert_eq!(
            accept(root, &pending).await?,
            ("accepted", 5, NotYetEffective, false)
        );
        // `effective_from` passed: the producer reports ACTIVE under the same lifecycle revision.
        let mut active = pending.clone();
        active.state = Active;
        active.authority_revision = 6;
        active.authority_issued_at_us = T0 + 2 * HOUR;
        active.authority_valid_until_us = T0 + 3 * HOUR;
        assert_eq!(accept(root, &active).await?, ("accepted", 6, Active, false));
        // `effective_until` passed: EXPIRED under the same lifecycle revision.
        let mut expired = active.clone();
        expired.state = Expired;
        expired.authority_revision = 7;
        expired.authority_issued_at_us = T0 + 5 * HOUR;
        expired.authority_valid_until_us = T0 + 6 * HOUR;
        assert_eq!(
            accept(root, &expired).await?,
            ("accepted", 7, Expired, false)
        );
        // The older ACTIVE stays stale behind the newer EXPIRED.
        assert_eq!(accept(root, &active).await?, ("stale", 7, Expired, false));
        // The same authority revision with another state is still equivocation.
        let mut contradiction = expired.clone();
        contradiction.state = Active;
        assert_eq!(
            accept(root, &contradiction).await?,
            ("conflict", 7, Expired, true)
        );
        assert_eq!(harness.count("game_premium_evidence").await?, 3);
        assert_eq!(harness.count("game_premium_entitlement_fence").await?, 1);
        Ok(())
    })?;
    run("premium_lifecycle_facts", async |harness, root| {
        accept(root, &evidence(5, 1, Active)).await?;
        // Under the same lifecycle revision the product and effective window may not change.
        let mut product = evidence(6, 1, Active);
        product.product_version = 2;
        assert_eq!(accept(root, &product).await?, ("conflict", 5, Active, true));
        let mut start = evidence(7, 1, Active);
        start.effective_from_us -= 1;
        assert_eq!(accept(root, &start).await?, ("conflict", 5, Active, true));
        let mut end = evidence(8, 1, Expired);
        end.effective_until_us = T0 - 1;
        assert_eq!(accept(root, &end).await?, ("conflict", 5, Active, true));
        assert_eq!(harness.count("game_premium_evidence").await?, 1);
        Ok(())
    })?;
    run("premium_revocation_facts", async |harness, root| {
        use EntitlementState::Revoked;
        accept(root, &evidence(5, 1, Active)).await?;
        assert_eq!(
            accept(root, &evidence(6, 2, Revoked)).await?,
            ("accepted", 6, Revoked, false)
        );
        // A revocation change needs a new lifecycle revision (PREMIUM-DELIVERY-0 §4): ACTIVE
        // again under the revoked one does not re-authorize.
        assert_eq!(
            accept(root, &evidence(7, 2, Active)).await?,
            ("conflict", 6, Revoked, true)
        );
        assert_eq!(harness.count("game_premium_evidence").await?, 2);
        Ok(())
    })?;
    run("premium_expiry_rollback", async |harness, root| {
        accept(root, &evidence(5, 1, Active)).await?;
        // EXPIRED while `effective_until` is still ahead: a producer expiry, not elapsed time.
        assert_eq!(
            accept(root, &evidence(6, 1, Expired)).await?,
            ("accepted", 6, Expired, false)
        );
        // ACTIVE again under the same lifecycle revision is a step back: no re-authorization.
        assert_eq!(
            accept(root, &evidence(7, 1, Active)).await?,
            ("conflict", 6, Expired, true)
        );
        assert_eq!(harness.count("game_premium_evidence").await?, 2);
        Ok(())
    })?;
    run("premium_start_rollback", async |_, root| {
        accept(root, &evidence(5, 1, Active)).await?;
        assert_eq!(
            accept(root, &evidence(6, 1, NotYetEffective)).await?,
            ("conflict", 5, Active, true)
        );
        Ok(())
    })?;
    run("premium_withdrawn_entitlement", async |harness, root| {
        use EntitlementState::None as NoEntitlement;
        accept(root, &evidence(5, 1, Active)).await?;
        assert_eq!(
            accept(root, &evidence(6, 0, NoEntitlement)).await?,
            ("accepted", 6, NoEntitlement, false)
        );
        // The old grant again after NONE, without a new lifecycle revision: no re-grant.
        assert_eq!(
            accept(root, &evidence(7, 1, Active)).await?,
            ("conflict", 6, NoEntitlement, true)
        );
        assert_eq!(harness.count("game_premium_evidence").await?, 2);
        Ok(())
    })?;
    run("premium_regranted_entitlement", async |_, root| {
        use EntitlementState::None as NoEntitlement;
        accept(root, &evidence(5, 1, Active)).await?;
        accept(root, &evidence(6, 0, NoEntitlement)).await?;
        // A new grant raises the lifecycle revision and reopens the entitlement.
        assert_eq!(
            accept(root, &evidence(7, 2, Active)).await?,
            ("accepted", 7, Active, false)
        );
        // Renewals of that grant stay accepted.
        assert_eq!(
            accept(root, &evidence(8, 2, Active)).await?,
            ("accepted", 8, Active, false)
        );
        Ok(())
    })?;
    run("premium_unversioned_revoke", async |_, root| {
        use EntitlementState::Revoked;
        accept(root, &evidence(5, 1, Active)).await?;
        // A revoke under the same lifecycle revision contradicts the ACTIVE one: fail closed.
        assert_eq!(
            accept(root, &evidence(6, 1, Revoked)).await?,
            ("conflict", 5, Active, true)
        );
        Ok(())
    })
}

#[test]
fn premium_fence_rows_cannot_be_rolled_back() -> TestResult {
    use EntitlementState::Active;
    run("premium_guards", async |harness, root| {
        accept(root, &evidence(5, 1, Active)).await?;
        accept(root, &evidence(6, 2, Active)).await?;
        let mut contradiction = evidence(6, 2, Active);
        contradiction.authority_issued_at_us += 1;
        accept(root, &contradiction).await?;
        for statement in [
            "UPDATE game_premium_account_fence SET authority_revision = 5",
            "UPDATE game_premium_account_fence SET conflict_authority_revision = NULL",
            "UPDATE game_premium_entitlement_fence SET lifecycle_revision = 1, authority_revision = 5",
            "UPDATE game_premium_evidence SET entitlement_state = 4",
            "DELETE FROM game_premium_evidence",
            "DELETE FROM game_premium_account_fence",
            "DELETE FROM game_premium_entitlement_fence",
            "TRUNCATE game_premium_evidence CASCADE",
        ] {
            let result = sqlx::query(sqlx::AssertSqlSafe(statement))
                .execute(&harness.pool)
                .await;
            assert!(result.is_err(), "{statement}");
        }
        // The runtime role inserts evidence and inserts or advances fences, never more.
        let privileges: Vec<(String, bool)> = sqlx::query_as(
            "SELECT t || ':' || p, has_table_privilege('oteryn_game_runtime', t, p) \
               FROM unnest(ARRAY['game_premium_evidence', 'game_premium_account_fence', \
                                 'game_premium_entitlement_fence']) AS t, \
                    unnest(ARRAY['SELECT', 'INSERT', 'UPDATE', 'DELETE', 'TRUNCATE']) AS p",
        )
        .fetch_all(&harness.pool)
        .await?;
        for (privilege, held) in privileges {
            let expected = privilege.ends_with(":SELECT")
                || privilege.ends_with(":INSERT")
                || (privilege.ends_with(":UPDATE")
                    && !privilege.starts_with("game_premium_evidence"));
            assert_eq!(held, expected, "{privilege}");
        }
        let fence = root.load_premium_fence(ACCOUNT).await?.ok_or("fence")?;
        assert_eq!(
            (fence.latest.authority_revision, fence.conflicting),
            (6, true)
        );
        Ok(())
    })
}

fn body(authority: u64, nonce: &str) -> Vec<u8> {
    format!(
        concat!(
            "{{\"schema\":\"oteryn.premium_snapshot.v1\",\"producer_revision\":\"c914564\",",
            "\"producer_profile\":\"{}\",\"nonce\":\"{}\",\"account_id\":\"{}\",",
            "\"product_id\":\"{}\",\"product_version\":1,\"entitlement_id\":\"ent-1\",",
            "\"entitlement_state\":\"ACTIVE\",\"lifecycle_revision\":1,\"authority_revision\":{},",
            "\"effective_from\":\"2026-09-29T12:00:00Z\",\"effective_until\":\"2026-10-01T12:00:00Z\",",
            "\"authority_issued_at\":\"2026-09-30T12:00:00Z\",",
            "\"authority_valid_until\":\"2026-09-30T13:00:00Z\",",
            "\"refresh_after\":\"2026-09-30T12:40:00Z\"}}"
        ),
        PRODUCER_PROFILE,
        nonce,
        canonical_uuid(ACCOUNT),
        PRODUCT_ID,
        authority
    )
    .into_bytes()
}

#[test]
fn premium_restart_reproves_before_benefit() -> TestResult {
    run("premium_restart", async |_, root| {
        let now = TrustedNow::new(T0 + 60_000_000, 1_000_000);
        let first = PremiumConsumer::default();
        assert_eq!(
            first.ingest(root, ACCOUNT, "n1", &body(5, "n1")).await,
            IngestOutcome::Accepted
        );
        assert!(first.premium_current(ACCOUNT, now));
        // A response bound to another request changes nothing.
        assert!(matches!(
            first.ingest(root, ACCOUNT, "n2", &body(6, "n1")).await,
            IngestOutcome::Rejected(_)
        ));
        assert!(first.premium_current(ACCOUNT, now));
        // After a restart the durable ACTIVE evidence is not yet current authority.
        let restarted = PremiumConsumer::default();
        restarted.load(root, ACCOUNT).await?;
        assert_eq!(
            restarted.class(ACCOUNT, now),
            PremiumClass::AuthorityUnavailable
        );
        assert!(!restarted.premium_entitlement_ended(ACCOUNT, now));
        // A stale pull does not re-prove it; the high water replayed or advanced does.
        assert_eq!(
            restarted.ingest(root, ACCOUNT, "n3", &body(4, "n3")).await,
            IngestOutcome::Stale
        );
        assert!(!restarted.premium_current(ACCOUNT, now));
        assert_eq!(
            restarted.ingest(root, ACCOUNT, "n4", &body(5, "n4")).await,
            IngestOutcome::Replayed
        );
        assert!(restarted.premium_current(ACCOUNT, now));
        Ok(())
    })
}
