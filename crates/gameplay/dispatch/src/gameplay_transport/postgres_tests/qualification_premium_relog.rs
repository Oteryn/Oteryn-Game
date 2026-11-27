//! A disposable Premium grant survives terminal logout through a second authority pull.
use super::full_world_support::premium_test_producer::{Reply, TestPki, TestProducer, snapshot};
use super::full_world_support::*;
use crate::foundation::CommandStatus;
use crate::gameplay_transport::connection::{AdmittedSession, ControllerBinding};
use crate::gameplay_transport::spell_premium_coordinator::{
    PremiumActivation, RegisteredPremiumClock, SpellPremiumCoordinator, clock_registration,
};
use crate::premium::{
    TrustedNow,
    client::{PremiumClientConfig, PremiumSnapshotClient},
    refresh::PremiumRefresher,
};
use oteryn_protocol_oteryn::quick_loot::{
    QuickLootAction, QuickLootCategory, QuickLootRecipient, QuickLootRequest,
};
use std::sync::atomic::{AtomicU64, Ordering};

struct SnapshotClock(TrustedNow);
impl clock_registration::Registered for SnapshotClock {}
impl RegisteredPremiumClock for SnapshotClock {
    fn read_current_window(&self) -> Option<TrustedNow> {
        Some(self.0)
    }
}

#[test]
fn qualification_premium_snapshot_survives_logout_and_fresh_admission() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("qualification-premium-relog", scenario)
}

fn scenario() -> TestResult {
    let authority_runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()?;
    let account = id(40);
    let account_uuid = crate::gameplay_transport::canonical_uuid(&account);
    let pki = TestPki::new();
    let revision = AtomicU64::new(0);
    let producer = authority_runtime.block_on(TestProducer::start(&pki, move |request| {
        if request.account_id != account_uuid {
            return Reply::status(404);
        }
        let next = revision.fetch_add(1, Ordering::Relaxed) + 1;
        Reply::json(snapshot(request, next, &[]))
    }));
    let client = PremiumSnapshotClient::new(&PremiumClientConfig {
        origin: producer.origin.clone(),
        identity_pem: pki.client_identity_pem.into_bytes(),
        platform_ca_pem: pki.ca_pem.into_bytes(),
    })?;

    world_scenario!(
        "qualification_premium_relog",
        |h,
         _request,
         seal,
         authority,
         world_id,
         native,
         world,
         cells,
         room,
         pin,
         deps,
         actor,
         session,
         owner,
         fence,
         admitted| {
            const ISSUED: i64 = 1_790_769_600_000_000;
            let now = TrustedNow::new(ISSUED + 60_000_000, 0).ok_or("snapshot clock")?;
            let reads = SpellPremiumCoordinator::new(
                Arc::default(),
                Some(Arc::new(SnapshotClock(now))),
                Some(PremiumActivation {
                    activation_id: "packet-fixture".into(),
                    switch_over_us: ISSUED,
                }),
                Some(h.root.clone()),
            );
            owner.premium_coordinator = Some(&reads);
            owner.premium = PremiumRefresher::new(reads.consumer(), h.root.clone(), Some(client));

            // Drive the same Premium entry wait that a fresh Premium Channel admission uses.
            owner.premium_only = true;
            owner
                .begin_premium_channel_entry(account, session)
                .await
                .map_err(debug)?
                .ok_or("initial Premium entry")?
                .retain();
            assert_eq!(producer.requests().len(), 1);
            assert_eq!(
                owner.logout_session(admitted).await,
                LogoutReply::Result(LogoutResult::Accepted)
            );
            assert!(!reads.consumer().premium_current(account, Some(now)));

            let successor = GameSessionId::decode(&id(110)).map_err(debug)?;
            let entry = owner
                .begin_premium_channel_entry(account, successor)
                .await
                .map_err(debug)?
                .ok_or("successor Premium entry")?;
            let (next_session, request) = relogin_request(&h).await?;
            assert_eq!(next_session, successor);
            entry.retain();

            let current = FreshAdmissionStore::from_root(h.root.clone())
                .current_session(next_session)
                .await?;
            let next_actor = {
                let mut runtime = deps.runtime.lock().await;
                let reservation = runtime.reserve_fresh_session(next_session).map_err(debug)?;
                runtime.commit_fresh_session(reservation).map_err(debug)?
            };
            let transport = current.current_transport().ok_or("successor transport")?;
            let retry =
                crate::gameplay_transport::AdmissionRetryBudget::for_root(&next_session, &h.root);
            let (first_entry, item_fence, _) = Box::pin(owner.enter_committed_actor(
                &retry,
                &request,
                next_session,
                transport,
                next_actor,
            ))
            .await;
            assert!(matches!(
                first_entry,
                FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
            ));
            let readmitted = AdmittedSession {
                game_session_id: next_session,
                runtime_actor: Some(next_actor),
                first_entry,
                controller: Some(ControllerBinding {
                    transport,
                    account_id: account,
                }),
                item_fence,
                ..admitted
            };
            let assignment = QuickLootRequest {
                selected_item: None,
                review_filters: false,
                action: QuickLootAction::Assign,
                category: Some(QuickLootCategory::Gold),
                recipient: Some(QuickLootRecipient::MainContainer),
                corpse: None,
                recipient_id: None,
                fallback_to_main: false,
                filter_mode: None,
                filter_definition_ref: None,
            };
            let (status, _) = owner
                .quick_loot(
                    readmitted,
                    UseCommand {
                        game_session_id: next_session,
                        command_id: 1,
                        item_fence,
                    },
                    assignment,
                )
                .await
                .ok_or("Quick Loot reply missing")?;
            assert_eq!(status, CommandStatus::Accepted);
            let pulls = producer.requests();
            assert_eq!(pulls.len(), 2);
            assert_ne!(pulls[0].nonce, pulls[1].nonce);
            eprintln!("PG_EXECUTED Premium fixture pulled after logout and Quick Loot accepted");

            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        }
    )
}
