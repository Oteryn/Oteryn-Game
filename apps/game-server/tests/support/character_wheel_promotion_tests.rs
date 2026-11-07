//! Durable promotion and native Premium authority at the Wheel allocation boundary.
use super::*;
use crate::durability::character_wheel::WheelPremiumGate;
use crate::durability::premium_fence::{EntitlementState, PremiumEvidence};
use crate::gameplay_transport::spell_premium_coordinator::{
    PremiumActivation, RegisteredPremiumClock, SpellPremiumCoordinator, clock_registration,
};
use oteryn_gameplay_social::premium::{PremiumConsumer, TrustedNow};
use std::sync::Mutex;

#[test]
fn wheel_not_promoted_refuses_from_durable_state_at_temple() -> TestResult {
    run("wheel_not_promoted", async |h| {
        eligible(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let before = snapshot(&h.pool).await?;
        let outcome = commit(
            h,
            &authority,
            fence(3)?,
            request(2, 0, &[(15, 9)])?,
            &ruleset(R1)?,
            AT_TEMPLE,
        )
        .await?;
        assert!(matches!(
            outcome,
            WheelCommitOutcome::Refused(WheelRefusal::NotEligible(WheelIneligibility::NotPromoted))
        ));
        assert_eq!(snapshot(&h.pool).await?, before);
        Ok(())
    })
}

const NOW: i64 = 1_790_000_000_000_000;
const MINUTE: i64 = 60_000_000;

struct Clock(Mutex<Option<TrustedNow>>);
impl clock_registration::Registered for Clock {}
impl RegisteredPremiumClock for Clock {
    fn read_current_window(&self) -> Option<TrustedNow> {
        *self.0.lock().expect("clock")
    }
}

fn evidence(account: [u8; 16], state: EntitlementState, revision: u64) -> PremiumEvidence {
    PremiumEvidence {
        account_id: account,
        producer_revision: "1".repeat(40),
        producer_profile: oteryn_gameplay_social::premium::PRODUCER_PROFILE.into(),
        product_id: oteryn_gameplay_social::premium::PRODUCT_ID.into(),
        product_version: oteryn_gameplay_social::premium::PRODUCT_VERSION,
        entitlement_id: Some("01926c1e-8a40-7c3b-9f2e-5a1d3c4b6e70".into()),
        state,
        lifecycle_revision: revision,
        authority_revision: revision,
        effective_from_us: NOW - MINUTE,
        effective_until_us: NOW + 60 * MINUTE,
        authority_issued_at_us: NOW,
        authority_valid_until_us: NOW + 60 * MINUTE,
        refresh_after_us: NOW + 30 * MINUTE,
    }
}

#[test]
fn wheel_premium_free_unknown_expired_unavailable_refuse_and_current_commits() -> TestResult {
    run("wheel_premium", async |h| {
        eligible(&h.pool).await?;
        crate::character_promotion_fixture::purchase(h, 3).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let rules = ruleset(R1)?;
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let activation = PremiumActivation {
            activation_id: "wheel-premium-test".into(),
            switch_over_us: NOW,
        };
        let consumer = Arc::new(PremiumConsumer::default());
        let clock = Arc::new(Clock(Mutex::new(TrustedNow::new(NOW + MINUTE, 1))));
        let coordinator = SpellPremiumCoordinator::new(
            Arc::clone(&consumer),
            Some(clock.clone()),
            Some(activation),
            Some(h.root.clone()),
        );
        let sequencer = CharacterRevisionSequencer::new();
        let mut slot = sequencer.acquire(character).await;
        let before = snapshot(&h.pool).await?;
        // Even an empty latch cannot grant the exception once the configured start is reached.
        let unlatched = slot
            .commit_wheel(
                &h.root,
                &authority,
                &h.node,
                fence(4)?,
                request(2, 0, &[(15, 9)])?,
                Arc::clone(&rules),
                AT_TEMPLE,
                &coordinator,
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            unlatched,
            WheelCommitOutcome::Refused(WheelRefusal::NotEligible(WheelIneligibility::NotPremium))
        );
        assert_eq!(snapshot(&h.pool).await?, before);
        h.root
            .latch_premium_activation("wheel-premium-test", NOW, NOW)
            .await
            .map_err(debug)?;
        // Unknown, free, expired and revoked accounts all use the same refusal, and retrying
        // the refused occurrence after a real Premium grant must still be able to commit.
        for (revision, state) in (1_u64..).zip([
            None,
            Some(EntitlementState::None),
            Some(EntitlementState::Expired),
            Some(EntitlementState::Revoked),
        ]) {
            if let Some(state) = state {
                consumer.prove_for_test(evidence(id(40), state, revision));
            }
            let outcome = slot
                .commit_wheel(
                    &h.root,
                    &authority,
                    &h.node,
                    fence(4)?,
                    request(2, 0, &[(15, 9)])?,
                    Arc::clone(&rules),
                    AT_TEMPLE,
                    &coordinator,
                )
                .await
                .map_err(debug)?;
            assert_eq!(
                outcome,
                WheelCommitOutcome::Refused(WheelRefusal::NotEligible(
                    WheelIneligibility::NotPremium
                )),
                "{state:?}"
            );
            assert_eq!(snapshot(&h.pool).await?, before);
        }
        // Proven Premium on a different account cannot authorize this Character.
        consumer.prove_for_test(evidence(id(99), EntitlementState::Active, 1));
        let refused = h
            .root
            .commit_character_wheel(
                &authority,
                &h.node,
                fence(4)?,
                request(2, 0, &[(15, 9)])?,
                Arc::clone(&rules),
                AT_TEMPLE,
                &coordinator,
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            refused,
            WheelCommitOutcome::Refused(WheelRefusal::NotEligible(WheelIneligibility::NotPremium))
        );
        assert_eq!(snapshot(&h.pool).await?, before);
        // Each lifecycle transition has a newer Platform revision; conflicting evidence must
        // never be used as a fixture for current Premium.
        consumer.prove_for_test(evidence(id(40), EntitlementState::Active, 5));
        assert!(consumer.premium_current(id(40), clock.read_current_window()));
        let change = committed(
            slot.commit_wheel(
                &h.root,
                &authority,
                &h.node,
                fence(4)?,
                request(2, 0, &[(15, 9)])?,
                Arc::clone(&rules),
                AT_TEMPLE,
                &coordinator,
            )
            .await,
        )?;
        assert_eq!(change.after, slots(&[(15, 9)])?);
        assert_eq!(change.committed_character_revision.get(), 5);

        let allocated = snapshot(&h.pool).await?;
        // Loss of trusted time or a failed pull closes an already-issued coordinator.
        // A lapse blocks removal as well as addition, even at a temple.
        for (now, failed_pull) in [
            (None, false),
            (TrustedNow::new(NOW + MINUTE, 1), true),
            (TrustedNow::new(NOW + 60 * MINUTE, 1), false),
        ] {
            *clock.0.lock().expect("clock") = now;
            consumer.prove_for_test(evidence(id(40), EntitlementState::Active, 5));
            if failed_pull {
                consumer.pull_failed(id(40), consumer.ticket());
            }
            let outcome = slot
                .commit_wheel(
                    &h.root,
                    &authority,
                    &h.node,
                    fence(5)?,
                    request(3, 1, &[])?,
                    Arc::clone(&rules),
                    AT_TEMPLE,
                    &coordinator,
                )
                .await
                .map_err(debug)?;
            assert_eq!(
                outcome,
                WheelCommitOutcome::Refused(WheelRefusal::NotEligible(
                    WheelIneligibility::NotPremium
                ))
            );
            assert_eq!(snapshot(&h.pool).await?, allocated);
            // Reading the existing allocation remains allowed after Premium is lost.
            let read = h
                .root
                .read_character_wheel(&authority, character, Arc::clone(&rules))
                .await
                .map_err(debug)?;
            assert_eq!(read.slots, change.after);
            assert_eq!(read.wheel_revision, 1);
        }
        // Receipt replay does not reacquire Premium or create a new mutation.
        assert!(matches!(
            slot.commit_wheel(
                &h.root,
                &authority,
                &h.node,
                fence(5)?,
                request(2, 0, &[(15, 9)])?,
                Arc::clone(&rules),
                AT_TEMPLE,
                &coordinator
            )
            .await
            .map_err(debug)?,
            WheelCommitOutcome::AlreadyCommitted(_)
        ));
        assert_eq!(snapshot(&h.pool).await?, allocated);
        Ok(())
    })
}

/// Observe the lock from an independent physical connection while the allocation gate runs.
struct ObserveLatch<'a> {
    coordinator: SpellPremiumCoordinator,
    pool: &'a sqlx::PgPool,
}
impl WheelPremiumGate for ObserveLatch<'_> {
    async fn allocation_permitted(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        account: [u8; 16],
    ) -> Result<bool, crate::durability::DurabilityError> {
        let permitted = self.coordinator.allocation_permitted(tx, account).await?;
        let mut independent = self.pool.begin().await?;
        let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
            .bind(crate::durability::premium_activation_latch::PREMIUM_ACTIVATION_LOCK_KEY)
            .fetch_one(&mut *independent)
            .await?;
        assert!(
            !acquired,
            "the pre-delivery witness must hold the allocation transaction's lock"
        );
        // The gate must precede the Character lock, matching promotion's order.
        sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE NOWAIT")
            .bind(id(CHARACTER).as_slice())
            .fetch_one(&mut *independent)
            .await?;
        independent.rollback().await?;
        Ok(permitted)
    }
}

#[test]
fn wheel_premium_pre_delivery_holds_allocation_lock_and_latch_ends_exception() -> TestResult {
    run("wheel_premium_latch", async |h| {
        eligible(&h.pool).await?;
        crate::character_promotion_fixture::purchase(h, 3).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let rules = ruleset(R1)?;
        let gate = ObserveLatch {
            coordinator: pre_delivery(h),
            pool: &h.pool,
        };
        let before = snapshot(&h.pool).await?;
        // Missing Premium wiring is unavailable, never a pre-delivery authorization.
        let missing = SpellPremiumCoordinator::new(Default::default(), None, None, None);
        let refused = h
            .root
            .commit_character_wheel(
                &authority,
                &h.node,
                fence(4)?,
                request(2, 0, &[(15, 9)])?,
                Arc::clone(&rules),
                AT_TEMPLE,
                &missing,
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            refused,
            WheelCommitOutcome::Refused(WheelRefusal::NotEligible(WheelIneligibility::NotPremium))
        );
        assert_eq!(snapshot(&h.pool).await?, before);
        committed(
            h.root
                .commit_character_wheel(
                    &authority,
                    &h.node,
                    fence(4)?,
                    request(2, 0, &[(15, 9)])?,
                    Arc::clone(&rules),
                    AT_TEMPLE,
                    &gate,
                )
                .await,
        )?;
        h.root
            .latch_premium_activation("wheel-premium-test", NOW, NOW)
            .await
            .map_err(debug)?;
        // Reuse the very coordinator that granted the bypass: the new latch ends it.
        let allocated = snapshot(&h.pool).await?;
        let refused = h
            .root
            .commit_character_wheel(
                &authority,
                &h.node,
                fence(5)?,
                request(3, 1, &[(15, 10)])?,
                Arc::clone(&rules),
                AT_TEMPLE,
                &gate.coordinator,
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            refused,
            WheelCommitOutcome::Refused(WheelRefusal::NotEligible(WheelIneligibility::NotPremium))
        );
        assert_eq!(snapshot(&h.pool).await?, allocated);
        Ok(())
    })
}
