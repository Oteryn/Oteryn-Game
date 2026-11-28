//! A disposable Premium grant survives terminal logout through a second authority pull.
use super::full_world_support::premium_test_producer::{
    ProducerRequest, Reply, TestPki, TestProducer, snapshot,
};
use super::full_world_support::*;
use crate::foundation::CommandStatus;
use crate::gameplay_transport::connection::{AdmittedSession, ControllerBinding};
use crate::gameplay_transport::spell_premium_coordinator::{
    PremiumActivation, RegisteredPremiumClock, SpellPremiumCoordinator, clock_registration,
};
use crate::premium::{
    IngestOutcome, PremiumConsumer, TrustedNow,
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

#[test]
fn premium_snapshot_retries_the_same_evidence_when_the_holder_is_busy() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("premium-holder-retry", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let h = Harness::create_with_admission_seed(
                    admin,
                    "premium_holder_retry",
                    false,
                    false,
                )
                .await?;
                let consumer = PremiumConsumer::default();
                let account = id(40);
                let request = ProducerRequest {
                    account_id: crate::gameplay_transport::canonical_uuid(&account),
                    nonce: "a".repeat(32),
                };
                let body = snapshot(&request, 7, &[]);
                let holder = h.root.try_acquire_ready()?;
                // Poll ingestion while the checkout is still held. The other branch then
                // returns it, without a sleep or a second Platform response.
                let (outcome, ()) = tokio::time::timeout(Duration::from_secs(3), async {
                    tokio::join!(biased;
                        consumer.ingest(&h.root, account, &request.nonce, &body),
                        async { drop(holder) }
                    )
                })
                .await?;
                assert_eq!(outcome, IngestOutcome::Accepted);
                let fence = h
                    .root
                    .load_premium_fence(account)
                    .await?
                    .fence
                    .ok_or("durable Premium fence")?;
                assert_eq!(fence.latest.authority_revision, 7);
                h.cleanup().await
            })
    })
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
        let reply = Reply::json(snapshot(request, next, &[]));
        if next == 1 {
            // Hold the first response in flight while the overview is requested.
            return reply.delayed(Duration::from_millis(500));
        }
        reply
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
            // The full-world fixture's runtime actor is a Monk. Give the durable
            // character the same vocation through the qualification owner before relog.
            let policy = owner
                .active_generation
                .and_then(|active| active.progression())
                .ok_or("fixture progression")?
                .policy_for("profile-1", "ruleset-1", "content-1")
                .map_err(debug)?;
            let current = FreshAdmissionStore::from_root(h.root.clone())
                .current_session(session)
                .await?;
            provision::prepare_character_with_policy(
                &h.root,
                &authority,
                &h.node,
                &current,
                provision::QualificationBuildSpec {
                    vocation: "monk",
                    level: 8,
                    exact_level: false,
                    context: policy.context.clone(),
                    xp_occurrence: id(183),
                    build_occurrence: id(184),
                    magic_seed: None,
                },
                policy,
            )
            .await?;
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

            let capability = oteryn_protocol_oteryn::character_info::CAPABILITY_CHARACTER_INFO_V1;
            admitted.continuity.selected_capabilities =
                SelectedCapabilities::select(owner.offered_capabilities(), &[capability])
                    .ok_or("character info capability")?;
            owner.admit_premium(account, session);
            let overview = tokio::time::timeout(
                Duration::from_secs(3),
                owner.character_info(
                    admitted,
                    oteryn_protocol_oteryn::character_info::CharacterInfoOperation::QueryPremium,
                ),
            )
            .await?;
            let crate::gameplay_transport::character_info::CharacterInfoReply::Payload(body) =
                overview
            else {
                return Err("pending Premium overview was rejected".into());
            };
            let overview = oteryn_protocol_oteryn::premium_overview::Overview::decode(&body)
                .ok_or("Premium overview codec")?;
            assert_eq!(
                overview.state,
                oteryn_protocol_oteryn::premium_overview::State::Active
            );
            assert_eq!(overview.authority_revision, 1);
            // A same-session reconnect wakes the existing schedule, below its 60 s floor.
            let first = reads
                .consumer()
                .current_premium(account, Some(now))
                .ok_or("first proof")?;
            owner.admit_premium(account, session);
            let reconnected = tokio::time::timeout(Duration::from_secs(3), async {
                loop {
                    if let Some(proof) = reads.consumer().current_premium(account, Some(now))
                        && proof.authority_revision > first.authority_revision
                    {
                        break proof;
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            })
            .await?;

            assert_eq!(
                owner.logout_session(admitted).await,
                LogoutReply::Result(LogoutResult::Accepted)
            );
            assert!(!reads.consumer().premium_current(account, Some(now)));

            // Drive the actual Premium Channel admission wait after terminal logout.
            owner.premium_only = true;
            let successor = GameSessionId::decode(&id(110)).map_err(debug)?;
            let entry = owner
                .begin_premium_channel_entry(account, successor)
                .await
                .map_err(debug)?
                .ok_or("successor Premium entry")?;
            // The real Premium admission wait is bounded at two seconds. Its proof must be
            // newer than the revision already accepted before logout.
            let relogged = reads
                .consumer()
                .current_premium(account, Some(now))
                .ok_or("relog proof")?;
            assert!(relogged.authority_revision > reconnected.authority_revision);
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
            assert_eq!(pulls.len(), 3);
            assert_ne!(pulls[0].nonce, pulls[1].nonce);
            assert_ne!(pulls[1].nonce, pulls[2].nonce);
            eprintln!("PG_EXECUTED Premium fixture pulled after logout and Quick Loot accepted");

            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        }
    )
}
