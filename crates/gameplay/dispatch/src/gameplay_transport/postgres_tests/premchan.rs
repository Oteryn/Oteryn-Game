//! PREMCHAN: existing Premium owner, refusal cleanup and the production admitted dispatcher.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::premium_fence::{EntitlementState, PremiumEvidence};
use crate::foundation::{GameSessionId, MessageType};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::{
    PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    AdmissionRefusal, ConnectionEnd, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::spell_premium_coordinator::{
    PremiumActivation, RegisteredPremiumClock, SpellPremiumCoordinator, clock_registration,
};
use crate::premium::{PremiumConsumer, TrustedNow};
use oteryn_protocol_oteryn::{decode_snapshot_chunk, decode_wire_envelope};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct Clock(Mutex<Option<TrustedNow>>);
impl clock_registration::Registered for Clock {}
impl RegisteredPremiumClock for Clock {
    fn read_current_window(&self) -> Option<TrustedNow> {
        *self.0.lock().expect("clock")
    }
}

fn evidence(now: i64, state: EntitlementState) -> PremiumEvidence {
    PremiumEvidence {
        account_id: id(40),
        producer_revision: "1".repeat(40),
        producer_profile: crate::premium::PRODUCER_PROFILE.to_owned(),
        product_id: crate::premium::PRODUCT_ID.to_owned(),
        product_version: crate::premium::PRODUCT_VERSION,
        entitlement_id: Some("01926c1e-8a40-7c3b-9f2e-5a1d3c4b6e70".to_owned()),
        state,
        lifecycle_revision: 1,
        authority_revision: 7,
        effective_from_us: now - 60_000_000,
        effective_until_us: now + 3_600_000_000,
        authority_issued_at_us: now,
        authority_valid_until_us: now + 3_600_000_000,
        refresh_after_us: now + 1_800_000_000,
    }
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_durability_fresh_admission_receipts), (SELECT count(*) FROM game_durability_reconnect_sessions), (SELECT count(*) FROM game_durability_transport_ref_reservations), (SELECT count(*) FROM game_durability_recovery_grant_consumptions)")
        .fetch_one(&h.pool).await?)
}

async fn channel_snapshot(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    selected: bool,
) -> TestResult<Option<Vec<u8>>> {
    let mut admitted = deps.admitted;
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        PRODUCTION_OFFERED_CAPABILITIES,
        if selected { &[83] } else { &[] },
    )
    .ok_or("capability selection")?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client.shutdown().await?;
    let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
    assert!(
        matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
        "{end:?}"
    );
    drop(server);
    let mut bytes = Vec::new();
    client.read_to_end(&mut bytes).await?;
    let mut payload = None;
    let mut at = 0;
    while at < bytes.len() {
        let len = u32::from_be_bytes(bytes[at..at + 4].try_into()?) as usize;
        at += 4;
        let envelope = decode_wire_envelope(&bytes[at..at + len])?;
        at += len;
        if envelope.message_type() == MessageType::SnapshotChunk {
            for domain in decode_snapshot_chunk(envelope.payload())?.1 {
                if domain.domain_id == 48 {
                    assert!(payload.is_none());
                    assert_eq!((domain.revision, domain.snapshot_type), (1, 1));
                    payload = Some(domain.payload.to_vec());
                }
            }
        }
    }
    Ok(payload)
}

#[test]
fn premchan_entry_uses_premium_owner_and_releases_failed_attempts() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "premchan", true, false).await?;
        eprintln!("POSTGRES_STARTED PREMCHAN");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let character = h
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
        let mut owner = deps.owner(&h, &character);
        let now = 1_790_812_800_000_000;
        let clock = Arc::new(Clock(Mutex::new(TrustedNow::new(now, 1_000_000))));
        let consumer = Arc::new(PremiumConsumer::default());
        let coordinator = SpellPremiumCoordinator::new(
            consumer.clone(),
            Some(clock.clone()),
            Some(PremiumActivation {
                activation_id: "premchan-test".to_owned(),
                switch_over_us: now - 60_000_000,
            }),
            Some(h.root.clone()),
        );
        owner.premium_coordinator = Some(&coordinator);
        owner.premium =
            crate::premium::refresh::PremiumRefresher::new(consumer.clone(), h.root.clone(), None);
        let account = id(40);
        let pending = GameSessionId::decode(&id(51)).map_err(debug)?;
        let before = counts(&h).await?;
        // An ordinary Channel never requires Premium or creates a temporary registration.
        assert!(
            owner
                .begin_premium_channel_entry(account, pending)
                .await
                .map_err(debug)?
                .is_none()
        );
        owner.premium_only = true;
        assert!(matches!(
            owner.begin_premium_channel_entry(account, pending).await,
            Err(AdmissionRefusal::PremiumRequired)
        ));
        assert_eq!(
            owner
                .states
                .premium_sessions
                .lock()
                .map_err(debug)?
                .account_of(pending),
            None
        );
        assert_eq!(counts(&h).await?, before);
        // A pending entry cancelled by the outer connection deadline releases its pull too.
        assert!(
            tokio::time::timeout(
                Duration::from_millis(5),
                owner.begin_premium_channel_entry(account, pending)
            )
            .await
            .is_err()
        );
        assert_eq!(
            owner
                .states
                .premium_sessions
                .lock()
                .map_err(debug)?
                .account_of(pending),
            None
        );
        consumer.prove_for_test(evidence(now, EntitlementState::Active));
        let admitted_entry = owner
            .begin_premium_channel_entry(account, pending)
            .await
            .map_err(debug)?
            .ok_or("entry")?;
        assert_eq!(
            owner
                .states
                .premium_sessions
                .lock()
                .map_err(debug)?
                .account_of(pending),
            Some(account)
        );
        // The current session holds its own registration; dropping a second entry cannot
        // cancel the account's pulls or erase its proven evidence.
        owner.admit_premium(account, deps.admitted.game_session_id);
        drop(admitted_entry);
        assert!(owner.premium_gate(account).await.premium_current());
        let current_check = owner.channel_entry_check(account, Arc::new(|_, _, _| true));
        let scope = crate::foundation::RuntimeScopeRefV1::channel(owner.world_id, owner.channel_id);
        assert!(current_check(scope, 1, "policy-1"));
        assert!(!owner
            .channel_entry_check(account, Arc::new(|_, _, _| false))(
            scope, 1, "policy-1"
        ));
        let mut expired = evidence(now, EntitlementState::Active);
        expired.effective_until_us = now - 2_000_000;
        consumer.prove_for_test(expired);
        assert!(!current_check(scope, 1, "policy-1"));
        assert!(matches!(
            owner.begin_premium_channel_entry(account, pending).await,
            Err(AdmissionRefusal::PremiumRequired)
        ));
        consumer.prove_for_test(evidence(now, EntitlementState::Revoked));
        assert!(!current_check(scope, 1, "policy-1"));
        assert!(matches!(
            owner.begin_premium_channel_entry(account, pending).await,
            Err(AdmissionRefusal::PremiumRequired)
        ));
        consumer.prove_for_test(evidence(now, EntitlementState::Active));
        *clock.0.lock().map_err(debug)? = None;
        assert!(!current_check(scope, 1, "policy-1"));
        assert!(matches!(
            owner.begin_premium_channel_entry(account, pending).await,
            Err(AdmissionRefusal::PremiumRequired)
        ));
        assert_eq!(counts(&h).await?, before);
        // Expiry/revocation does not evict an incumbent: real serve_admitted still emits its
        // snapshot, including the premium flag when selected, with no fresh entry check.
        for premium_only in [false, true] {
            owner.premium_only = premium_only;
            let payload = channel_snapshot(&owner, &deps, true)
                .await?
                .ok_or("channel domain")?;
            let mut expected = vec![0x0a, 38, 0x0a, 16];
            expected.extend_from_slice(deps.admitted.world_id.as_bytes());
            expected.extend_from_slice(&[0x12, 16]);
            expected.extend_from_slice(deps.admitted.channel_id.as_bytes());
            expected.extend_from_slice(&[0x18, u8::from(premium_only)]);
            assert_eq!(payload, expected);
            assert!(channel_snapshot(&owner, &deps, false).await?.is_none());
        }
        assert_eq!(counts(&h).await?, before);
        owner.release_premium(account, deps.admitted.game_session_id);
        drop(owner);
        drop(character);
        drop(seal);
        h.cleanup().await
    })
}
