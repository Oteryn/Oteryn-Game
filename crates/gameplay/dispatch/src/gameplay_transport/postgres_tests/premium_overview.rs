//! T165: real command 113/operation 5 -> actor/account fences -> existing Premium rows.
use super::{
    Dependencies, TestResult,
    fixture::{Harness, debug, id, run},
    prepare_current_session,
};
use crate::durability::{
    fresh_admission::FreshAdmissionStore,
    premium_fence::{EntitlementState, PremiumEvidence},
};
use crate::gameplay_transport::{
    capabilities::{PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities},
    connection::{AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority},
    spell_premium_coordinator::{
        PremiumActivation, RegisteredPremiumClock, SpellPremiumCoordinator, clock_registration,
    },
};
use crate::premium::{PremiumConsumer, TrustedNow};
use oteryn_protocol_oteryn::{
    CommandStatus,
    character_info::CharacterInfoOperation,
    premium_overview::{Overview, State},
};
use std::sync::{Arc, Mutex};
struct Clock(Mutex<Option<TrustedNow>>);
impl clock_registration::Registered for Clock {}
impl RegisteredPremiumClock for Clock {
    fn read_current_window(&self) -> Option<TrustedNow> {
        self.0.lock().ok().and_then(|v| *v)
    }
}
fn evidence(now: i64, account: [u8; 16]) -> PremiumEvidence {
    PremiumEvidence {
        account_id: account,
        producer_revision: "1".repeat(40),
        producer_profile: crate::premium::PRODUCER_PROFILE.into(),
        product_id: crate::premium::PRODUCT_ID.into(),
        product_version: crate::premium::PRODUCT_VERSION,
        entitlement_id: Some("01926c1e-8a40-7c3b-9f2e-5a1d3c4b6e70".into()),
        state: EntitlementState::Active,
        lifecycle_revision: 1,
        authority_revision: 7,
        effective_from_us: now - 60_000_000,
        effective_until_us: now + 3_600_000_000,
        authority_issued_at_us: now,
        authority_valid_until_us: now + 30_000_000,
        refresh_after_us: now + 10_000_000,
    }
}
async fn wire(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    session: &mut AdmittedSession,
    id: u64,
) -> TestResult<Overview> {
    let (status, bytes) =
        super::cyclopedia::support::exchange(owner, session, id, 113, &[5]).await?;
    assert_eq!(status, CommandStatus::Accepted);
    Overview::decode(&bytes).ok_or_else(|| "Premium codec".into())
}
#[test]
fn t165_premium_overview_real_dispatch_account_privacy_expiry_and_leases() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Premium scenario panicked")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        super::reset();
        let h = Harness::create_with_admission_seed(admin, "t165_premium_observation", true, false)
            .await?;
        eprintln!(
            "POSTGRES_STARTED t165_premium_overview_real_dispatch_account_privacy_expiry_and_leases"
        );
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        let now = 1_790_812_800_000_000;
        let clock = Arc::new(Clock(Mutex::new(TrustedNow::new(now, 0))));
        let consumer = Arc::new(PremiumConsumer::default());
        let coordinator = SpellPremiumCoordinator::new(
            consumer.clone(),
            Some(clock.clone()),
            Some(PremiumActivation {
                activation_id: "t165-observation".into(),
                switch_over_us: now - 60_000_000,
            }),
            Some(h.root.clone()),
        );
        owner.premium_coordinator = Some(&coordinator);
        let current = FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(deps.admitted.game_session_id)
            .await?
            .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        deps.runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let mut session = deps.admitted;
        session.item_fence = Some(fence);
        session.first_entry = FirstEntryOutcome::Positioned;
        session.continuity.selected_capabilities =
            SelectedCapabilities::select(PRODUCTION_OFFERED_CAPABILITIES, &[97])
                .ok_or("character info capability")?;
        let account = h
            .root
            .premium_overview_account(&authority, &h.node, fence)
            .await
            .map_err(debug)?;
        // Another payer's paid state must not be used when this actor has no accepted evidence.
        let other = evidence(now, id(41));
        h.root.accept_premium_evidence(&other).await?;
        consumer.prove_for_test(other);
        let empty = wire(&owner, &mut session, 1).await?;
        assert_eq!(empty.state, State::Unavailable);
        assert!(empty.features.iter().all(|f| !f.enabled));
        let paid = evidence(now, account);
        h.root.accept_premium_evidence(&paid).await?;
        consumer.prove_for_test(paid.clone());
        let paid_view = wire(&owner, &mut session, 2).await?;
        assert_eq!(paid_view.state, State::Active);
        assert_eq!(paid_view.authority_revision, 7);
        assert_eq!(paid_view.expires_at_us, Some(paid.effective_until_us));
        assert_eq!(
            paid_view.valid_for_us, 10_000_000,
            "refresh deadline, not subscription expiry"
        );
        assert_eq!(paid_view.features.len(), 5);
        assert!(paid_view.features.iter().all(|f| f.enabled));
        *clock.0.lock().map_err(|_| "clock")? = TrustedNow::new(now + 11_000_000, 0);
        let stale = wire(&owner, &mut session, 3).await?;
        assert_eq!(stale.expires_at_us, Some(paid.effective_until_us));
        assert_eq!(stale.valid_for_us, 0);
        assert!(stale.features.iter().all(|f| !f.enabled));
        let mut revoked = paid.clone();
        revoked.state = EntitlementState::Revoked;
        revoked.authority_revision = 8;
        revoked.lifecycle_revision = 2;
        h.root.accept_premium_evidence(&revoked).await?;
        consumer.prove_for_test(revoked);
        let refused = wire(&owner, &mut session, 4).await?;
        assert_eq!(refused.state, State::Revoked);
        assert!(refused.features.iter().all(|f| !f.enabled));
        let mut stale_session = session;
        stale_session.item_fence = Some(
            crate::durability::item_transfer::CurrentCharacterItemFence {
                character_lease_generation: fence.character_lease_generation + 1,
                ..fence
            },
        );
        assert!(matches!(
            FreshAdmissionAuthority::character_info(
                &owner,
                stale_session,
                CharacterInfoOperation::QueryPremium
            )
            .await,
            crate::gameplay_transport::character_info::CharacterInfoReply::Rejected
        ));
        let (bad, _) =
            super::cyclopedia::support::exchange(&owner, &mut session, 5, 113, &[5, 0]).await?;
        assert_eq!(bad, CommandStatus::Rejected);
        eprintln!(
            "POSTGRES_EXECUTED t165_premium_overview_real_dispatch_account_privacy_expiry_and_leases"
        );
        h.pool.close().await;
        Ok(())
    })
}
