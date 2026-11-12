//! PREMIUM-ACT-1: real mTLS pulls, durable fencing and the gameplay activation gate.
use crate::bestiary_postgres_harness::{Harness, TestResult, configured_admin, runtime};
use crate::gameplay_transport::spell_premium_coordinator::{
    PremiumActivation, RegisteredPremiumClock, SpellPremiumCoordinator, clock_registration,
};
use crate::premium::test_producer::{Reply, TestPki, TestProducer, snapshot};
use production_server::premium::{
    TrustedNow,
    client::{PremiumClientConfig, PremiumSnapshotClient},
    refresh::PremiumRefresher,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const ACCOUNT: [u8; 16] = [0x61; 16];
const T0: i64 = 1_790_769_600_000_000;
const MINUTE: i64 = 60_000_000;

struct Clock(Mutex<i64>);
impl clock_registration::Registered for Clock {}
impl RegisteredPremiumClock for Clock {
    fn read_current_window(&self) -> Option<TrustedNow> {
        TrustedNow::new(*self.0.lock().expect("clock"), 0)
    }
}

#[test]
fn premium_activation_refresh_fixture_expiry_and_unreachable() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime().map_err(|e| e.to_string())?.block_on(async {
                let harness = Harness::create(admin, "premium_act_refresh", false).await?;
                let root = &harness.root;
                let clock = Arc::new(Clock(Mutex::new(T0 + MINUTE)));
                let reads = SpellPremiumCoordinator::new(
                    Arc::default(), Some(clock.clone()),
                    Some(PremiumActivation {
                        activation_id: "fixture-activation".to_owned(),
                        switch_over_us: T0,
                    }), Some(root.clone()),
                );
                let pki = TestPki::new();
                let producer = TestProducer::start(&pki, |request| {
                    Reply::json(snapshot(request, 7, &[]))
                }).await;
                let client = PremiumSnapshotClient::new(&PremiumClientConfig {
                    origin: producer.origin.clone(),
                    identity_pem: pki.client_identity_pem.into_bytes(),
                    platform_ca_pem: pki.ca_pem.into_bytes(),
                })?;
                let refresh = PremiumRefresher::new(reads.consumer(), root.clone(), Some(client));
                assert!(!reads.premium_gate(ACCOUNT).await.premium_current());
                refresh.admit(ACCOUNT);
                tokio::time::timeout(Duration::from_secs(5), async {
                    while !reads.premium_gate(ACCOUNT).await.premium_current() {
                        tokio::task::yield_now().await;
                    }
                }).await?;
                producer.script(|request| Reply::json(snapshot(request, 8, &[
                    ("authority_issued_at", "2026-09-30T12:10:00Z".into()),
                    ("authority_valid_until", "2026-09-30T13:10:00Z".into()),
                    ("refresh_after", "2026-09-30T12:50:00Z".into()),
                ])));
                // The past fixture cutoff selects the accepted 60-second refresh floor.
                // Wait for the actual scheduled request, without a reconnect wake-up.
                tokio::time::timeout(Duration::from_secs(70), async {
                    while reads.consumer().refresh_after_us(ACCOUNT) != Some(T0 + 50 * MINUTE) {
                        tokio::time::sleep(Duration::from_millis(20)).await;
                    }
                }).await?;
                assert_eq!(producer.max_in_flight(), 1);
                assert_eq!(producer.requests().len(), 2);
                *clock.0.lock().expect("clock") = T0 + 40 * MINUTE;
                assert!(reads.premium_gate(ACCOUNT).await.premium_current());
                // New proof extended the cutoff; benefit still ends at its refresh_after.
                *clock.0.lock().expect("clock") = T0 + 50 * MINUTE;
                assert!(!reads.premium_gate(ACCOUNT).await.premium_current());
                *clock.0.lock().expect("clock") = T0 + 45 * MINUTE;
                assert!(reads.premium_gate(ACCOUNT).await.premium_current());
                // Reconnect refreshes immediately; a dead Platform removes even valid proof.
                drop(producer);
                refresh.admit(ACCOUNT);
                tokio::time::timeout(Duration::from_secs(5), async {
                    while reads.premium_gate(ACCOUNT).await.premium_current() {
                        tokio::task::yield_now().await;
                    }
                }).await?;
                refresh.release(ACCOUNT);
                refresh.admit(ACCOUNT);
                assert!(!reads.premium_gate(ACCOUNT).await.premium_current());
                drop(refresh);
                eprintln!("PREMIUM-ACT-1 PostgreSQL: fixture activation, expiry, unreachable refresh executed");
                harness.cleanup().await
            }).map_err(|e: Box<dyn std::error::Error>| e.to_string())
        })?
        .join()
        .map_err(|_| "Premium test thread failed")?
        .map_err(Into::into)
}
