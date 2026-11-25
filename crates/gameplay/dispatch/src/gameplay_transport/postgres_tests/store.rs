//! STORE_V1 (E10): admitted connection bytes read a catalogue page with the Platform balance, a
//! stale quote never reaches Platform, an accepted purchase mints one exercise weapon into the
//! Inbox once, a retry of the same request id replays without a second debit or mint, and a
//! conflicting binding or an insufficient balance writes nothing. Platform is a test double of
//! the existing Store port (`StoreTransport`).
#![allow(clippy::expect_used)]
use super::store_fixtures::{FakePlatform, Ledger, purchase, purchase_request, serve, uuid};

use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::CommandStatus;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    ControllerBinding, FirstEntryOutcome, FreshAdmissionAuthority, SessionContinuity,
};

use crate::native_admission_source::store::StoreClient;
use oteryn_protocol_oteryn::store as wire;

use std::sync::{Arc, Mutex as SyncMutex};

const SWORD: &str = "store:exercise-sword";
const SWORD_ITEM: &str = "oteryn:item.tibia.i28552";
const CHARM_EXPANSION: &str = "store:charm-expansion";

#[test]
fn store_real_dispatch_pages_purchases_mints_once_and_replays() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("store-e2e", scenario);
    eprintln!("STORE-154 POSTGRES_CASE_COMPLETED store-e2e elapsed={:?}", started.elapsed());
    result
}

fn page_request() -> TestResult<(u32, Vec<u8>)> {
    let request = wire::StoreCatalogueRequest {
        category: "consumables.exercise-weapons".into(),
        page_cursor: String::new(),
    };
    Ok((
        wire::COMMAND_TYPE_STORE_CATALOGUE_PAGE,
        wire::encode_catalogue_request(&request).map_err(debug)?,
    ))
}

async fn count(harness: &Harness, sql: &'static str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(sql).fetch_one(&harness.pool).await?)
}

async fn release_store_character(
    store: &crate::durability::fresh_admission::FreshAdmissionStore,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    let controller = admitted.controller.ok_or("Store controller")?;
    let account = uuid(&controller.account_id);
    let released = store
        .release_logout_session(admitted.game_session_id, &account, controller.transport)
        .await?;
    assert!(
        matches!(
            released,
            crate::durability::fresh_admission::ExpiredLossReleaseV1::Released { .. }
        ),
        "Store Character logout: {released:?}"
    );
    Ok(())
}

async fn swords(harness: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1",
    )
    .bind(SWORD_ITEM)
    .fetch_one(&harness.pool)
    .await?)
}

fn scenario() -> TestResult {
    scenario_with_answer(Case::Normal)
}

#[derive(Clone, Copy)]
enum Case {
    Normal,
    Malformed,
    Restored,
    Blessing,
    CrystalCharges,
    CrystalCapacity,
    OldQuote,
    RemovedOffer,
    RemovedOfferAccepted,
    LegacyLostAnswer,
    LegacyAnsweredWithoutHistory,
    LegacyCrossCharacter,
    LegacyUnpaidFullInbox,
    RetryLockOrder,
    RemovedOfferBelowCapacity,
    ReceiptRecovery,
    RestoreReceiptGeneration,
    RestoreMissingDelivery,
    RestoreMissingClaim,
    SlowClaimDiscovery,
    ClaimReceipt,
    LastSlot,
    InterruptedMint,
    LostAnswer,
    CrossCharacter,
}

#[test]
fn store_unanswered_purchase_reserves_the_last_inbox_slot() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-last-slot", || {
        scenario_with_answer(Case::LastSlot)
    })
}

#[test]
fn store_lost_answer_recovers_through_a_fresh_confirmation() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-lost-answer", || {
        scenario_with_answer(Case::LostAnswer)
    })
}

#[test]
fn store_interrupted_mint_retries_with_a_new_wire_command() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-interrupted-mint", || {
        scenario_with_answer(Case::InterruptedMint)
    })
}

#[test]
fn store_paid_item_retry_keeps_the_purchasing_character() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-retry-character", || {
        scenario_with_answer(Case::CrossCharacter)
    })
}

#[test]
fn store_crystal_charged_offers_dispatch_and_mint_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-crystal-charges", || {
        scenario_with_answer(Case::CrystalCharges)
    })
}

#[test]
fn store_crystal_keg_checks_capacity_before_platform_debit() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-crystal-capacity", || {
        scenario_with_answer(Case::CrystalCapacity)
    })
}

#[test]
fn store_accepted_legacy_sword_without_history_recovers_on_new_confirmation() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-legacy-journal", || {
        scenario_with_answer(Case::LegacyAnsweredWithoutHistory)
    })
}

#[test]
fn store_legacy_paid_item_without_history_keeps_the_purchasing_character() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("legacy-retry-character", || {
        scenario_with_answer(Case::LegacyCrossCharacter)
    })
}

#[test]
fn store_browse_cannot_debit_a_legacy_sent_item_with_a_full_inbox() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("legacy-unpaid-full-inbox", || {
        scenario_with_answer(Case::LegacyUnpaidFullInbox)
    })
}

#[test]
fn store_item_retry_leaves_character_unlocked_while_answer_writer_holds_purchase() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-item-lock-order", || {
        scenario_with_answer(Case::RetryLockOrder)
    })
}

#[test]
fn store_paid_legacy_keg_replays_after_capacity_drops() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-legacy-keg-capacity", || {
        scenario_with_answer(Case::RemovedOfferBelowCapacity)
    })
}

#[test]
fn store_retained_sword_completes_after_catalogue_update() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-old-quote", || {
        scenario_with_answer(Case::OldQuote)
    })
}

#[test]
fn store_opening_recovers_an_unanswered_purchase_of_a_removed_offer() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-removed-offer", || {
        scenario_with_answer(Case::RemovedOffer)
    })
}

#[test]
fn store_opening_recovers_an_accepted_purchase_of_a_removed_offer() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-removed-accepted", || {
        scenario_with_answer(Case::RemovedOfferAccepted)
    })
}

#[test]
fn store_fresh_confirmation_reuses_a_legacy_debit_of_an_active_offer() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-legacy-active", || {
        scenario_with_answer(Case::LegacyLostAnswer)
    })
}

#[test]
fn store_receipt_is_republished_without_repeating_the_purchase() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-receipt-recovery", || {
        scenario_with_answer(Case::ReceiptRecovery)
    })
}

#[test]
fn store_worker_leaves_platform_claim_pending_while_item_is_in_inbox() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-claim", || {
        scenario_with_answer(Case::ClaimReceipt)
    })
}

#[test]
fn store_worker_republishes_a_retained_receipt_after_restore() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-recovery-generation", || {
        scenario_with_answer(Case::RestoreReceiptGeneration)
    })
}

#[test]
fn store_restore_resolves_a_missing_delivery_or_claim() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-restore-missing", || {
        let delivery = scenario_with_answer(Case::RestoreMissingDelivery);
        let claim = scenario_with_answer(Case::RestoreMissingClaim);
        delivery.and(claim)
    })
}

#[test]
fn store_claim_discovery_yields_the_client_to_a_waiting_balance_read() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-claim-discovery", || {
        scenario_with_answer(Case::SlowClaimDiscovery)
    })
}

#[test]
fn storefix_protocol_violation_keeps_debit_uncertain_and_never_mints() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("storefix-malformed", || scenario_with_answer(Case::Malformed));
    eprintln!("STORE-154 POSTGRES_CASE_COMPLETED storefix-malformed elapsed={:?}", started.elapsed());
    result
}

#[test]
fn store_restored_backup_refuses_new_and_reconciles_sent() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("store-restored", || scenario_with_answer(Case::Restored));
    eprintln!("STORE-154-D6 POSTGRES_CASE_COMPLETED elapsed={:?}", started.elapsed());
    result
}

#[allow(clippy::too_many_lines)]
fn scenario_with_answer(case: Case) -> TestResult {
    run(async |admin| {
        let malformed = matches!(case, Case::Malformed);
        let harness = Harness::create_with_admission_seed(admin, "store_e2e", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let (controller, pin) = crate::content::native_gameplay::test_support::activated_with_loot_tables(
            deps.admitted.world_id,
            None,
        );
        let active = controller.active().ok_or("active")?;
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let account: String =
            sqlx::query_scalar("SELECT replace(account_id::text,'-','') FROM game_character_roots")
                .fetch_one(&harness.pool)
                .await?;
        let mut account_id = [0; 16];
        for (index, byte) in account_id.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&account[index * 2..index * 2 + 2], 16)?;
        }
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence =
            Some(crate::gameplay_transport::item_fence_of(current).ok_or("fence")?);
        let mut controller_binding = admitted.controller.ok_or("controller")?;
        controller_binding.account_id = account_id;
        admitted.controller = Some(controller_binding);

        assert!(!owner.offered_capabilities().iter().any(|offer| offer.id == wire::CAPABILITY_STORE_V1));
        let ledger = Arc::new(SyncMutex::new(Ledger {
            balance: 60,
            malformed,
            world: uuid(admitted.world_id.as_bytes()),
            ..Ledger::default()
        }));
        owner.store = Some(Arc::new(tokio::sync::Mutex::new(StoreClient::new(Box::new(
            FakePlatform(ledger.clone()),
        ), "oteryn-game/store-consumer".into()))));

        if matches!(case, Case::Blessing) {
            super::journal_inventory::runtime_role(owner.root).await?;
            store_blessing_holdings(&owner, &mut admitted, &ledger).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

        if matches!(case, Case::Restored) {
            owner.store_purchases = crate::gameplay_transport::StorePurchaseMode::DisabledRestoredBackup;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[wire::CAPABILITY_STORE_V1],
            ).ok_or("Store selection")?;
            for offer in [SWORD, "store:hireling:25440", "store:prey-list-expansion"] {
                let result = purchase(&owner, &mut admitted, purchase_request(60, offer, 25)?).await?;
                assert_eq!(result.code, wire::StorePurchaseCode::PurchaseDisabled);
            }
            assert_eq!(count(&harness, "SELECT count(*) FROM store_purchase_requests").await?, 0);
            assert_eq!(count(&harness, "SELECT count(*) FROM game_hireling_store_reservations").await?, 0);
            assert_eq!(count(&harness, "SELECT count(*) FROM game_prey_expansion_purchases").await?, 0);
            assert_eq!(ledger.lock().expect("ledger").purchases, 0);
            assert!(ledger.lock().expect("ledger").debits.is_empty());
            // Browsing still uses Platform balance in restore mode.
            let page = serve(&owner, &mut admitted, &[page_request()?]).await?;
            assert_eq!(wire::decode_catalogue_page(&page[0].1).map_err(debug)?.total_coins, 60);
            // Simulate a debit whose answer was lost before the Game retained it.
            owner.store_purchases = crate::gameplay_transport::StorePurchaseMode::Enabled;
            ledger.lock().expect("ledger").malformed = true;
            let uncertain = serve(&owner, &mut admitted, &[purchase_request(62, SWORD, 25)?]).await?;
            assert_eq!(uncertain, [(CommandStatus::Rejected, Vec::new())]);
            assert_eq!(count(&harness, "SELECT count(*) FROM store_purchase_requests WHERE state=1 AND answer IS NULL").await?, 1);
            assert_eq!(ledger.lock().expect("ledger").debits, [25]);
            assert_eq!(swords(&harness).await?, 0);
            owner.store_purchases = crate::gameplay_transport::StorePurchaseMode::DisabledRestoredBackup;
            ledger.lock().expect("ledger").malformed = false;
            let reconciled = purchase(&owner, &mut admitted, purchase_request(62, SWORD, 25)?).await?;
            assert_eq!(reconciled.code, wire::StorePurchaseCode::Accepted);
            assert_eq!(ledger.lock().expect("ledger").purchases, 2);
            assert_eq!(ledger.lock().expect("ledger").debits, [25]);
            assert_eq!(ledger.lock().expect("ledger").balance, 35);
            assert_eq!(swords(&harness).await?, 1);
            assert_eq!(count(&harness, "SELECT count(*) FROM store_purchase_requests WHERE state=2").await?, 1);
            let replay = purchase(&owner, &mut admitted, purchase_request(62, SWORD, 25)?).await?;
            assert_eq!(replay, reconciled);
            assert_eq!(ledger.lock().expect("ledger").purchases, 2);
            drop(owner); drop(authority); drop(seal);
            return harness.cleanup().await;
        }

        // Without the selected capability the command is no Store command.
        let values = serve(&owner, &mut admitted, &[page_request()?]).await?;
        assert_eq!(values, [(CommandStatus::Rejected, Vec::new())]);
        // A composed Platform client makes STORE_V1 available through the real offer.
        assert!(
            owner
                .offered_capabilities()
                .iter()
                .any(|offer| offer.id == wire::CAPABILITY_STORE_V1)
        );
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[wire::CAPABILITY_STORE_V1],
        )
        .ok_or("selection")?;

        if matches!(case, Case::LastSlot) {
            ledger.lock().expect("ledger").balance = 2_000;
            for seed in 60..79 {
                assert_eq!(
                    purchase(&owner, &mut admitted, purchase_request(seed, SWORD, 25)?).await?.code,
                    wire::StorePurchaseCode::Accepted,
                );
            }
            assert_eq!(count(&harness, "SELECT count(*) FROM game_item_character_inbox_locations").await?, 19);
            let request = purchase_request(80, SWORD, 25)?;
            ledger.lock().expect("ledger").fail_before_debit = true;
            assert_eq!(serve(&owner, &mut admitted, std::slice::from_ref(&request)).await?, [(CommandStatus::Rejected, Vec::new())]);
            ledger.lock().expect("ledger").fail_before_debit = false;
            let other = purchase(&owner, &mut admitted, purchase_request(81, "store:exercise-axe", 25)?).await?;
            let retry = purchase(&owner, &mut admitted, request).await?;
            assert_eq!((other.code, retry.code), (wire::StorePurchaseCode::OfferUnavailable, wire::StorePurchaseCode::Accepted));
            assert_eq!(count(&harness, "SELECT count(*) FROM game_item_character_inbox_locations").await?, 20);
            assert_eq!(ledger.lock().expect("ledger").debits.len(), 20);
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

        if matches!(case, Case::RetryLockOrder) {
            ledger.lock().expect("ledger").malformed = true;
            let command = purchase_request(80, SWORD, 25)?;
            assert_eq!(serve(&owner, &mut admitted, std::slice::from_ref(&command)).await?,
                [(CommandStatus::Rejected, Vec::new())]);
            ledger.lock().expect("ledger").malformed = false;

            // The answer writer locks the purchase before its history row takes the
            // Character foreign-key lock. Hold it while the real retry queues.
            let mut writer = harness.pool.begin().await?;
            let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()").fetch_one(&mut *writer).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:store-purchase:'||encode($1,'hex'),0))")
                .bind(id(80).as_slice()).execute(&mut *writer).await?;
            let (character_available, accepted) = {
            let retry = purchase(&owner, &mut admitted, command);
            tokio::pin!(retry);
            tokio::select! {
                result = &mut retry => return Err(format!("retry skipped purchase serialization: {result:?}").into()),
                result = super::store_fixtures::wait_for_advisory_waiter(&harness.pool, pid) => result?,
            }
            let character_available = sqlx::query("SELECT character_id FROM game_character_roots FOR KEY SHARE NOWAIT")
                .fetch_one(&mut *writer).await.is_ok();
            writer.rollback().await?;
            let accepted = tokio::time::timeout(std::time::Duration::from_secs(3), &mut retry).await??;
            (character_available, accepted)
            };
            assert!(character_available, "ordinary retry locked the character ahead of the purchase");
            assert_eq!(accepted.code, wire::StorePurchaseCode::Accepted);
            assert_eq!(ledger.lock().expect("ledger").debits, [25]);
            assert_eq!(swords(&harness).await?, 1);
            drop(owner); drop(authority); drop(seal);
            return harness.cleanup().await;
        }

        if matches!(case, Case::LostAnswer) {
            ledger.lock().expect("ledger").malformed = true;
            assert_eq!(
                serve(&owner, &mut admitted, &[purchase_request(80, SWORD, 25)?]).await?,
                [(CommandStatus::Rejected, Vec::new())],
            );
            assert_eq!(count(&harness, "SELECT count(*) FROM game_store_purchase_history").await?, 0);
            ledger.lock().expect("ledger").malformed = false;
            let retry = purchase(&owner, &mut admitted, purchase_request(81, SWORD, 25)?).await?;
            assert_eq!(retry.code, wire::StorePurchaseCode::Accepted);
            assert_eq!(ledger.lock().expect("ledger").debits, [25]);
            assert_eq!(swords(&harness).await?, 1);
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

        if matches!(case, Case::InterruptedMint | Case::CrossCharacter | Case::LegacyCrossCharacter) {
            let price = if matches!(case, Case::LegacyCrossCharacter) {
                let old = retained_purchase(&owner, admitted, include_bytes!(
                    "../../../../../../rulesets/economy/store/predecessors/catalogue.json"
                ), SWORD, RetainedAnswer::Journal).await?;
                assert_eq!(count(&harness, "SELECT count(*) FROM game_store_purchase_history").await?, 0);
                old.price
            } else {
                sqlx::raw_sql(
                    "CREATE FUNCTION fail_store_commit() RETURNS trigger LANGUAGE plpgsql AS
                    $$ BEGIN RAISE EXCEPTION 'fixture interruption after preparation'; END; $$;
                    CREATE TRIGGER fail_store_commit BEFORE INSERT ON store_delivery_receipts
                    FOR EACH ROW EXECUTE FUNCTION fail_store_commit();"
                ).execute(&harness.pool).await?;
                let request = purchase_request(80, SWORD, 25)?;
                assert_eq!(serve(&owner, &mut admitted, std::slice::from_ref(&request)).await?, [(CommandStatus::Rejected, Vec::new())]);
                assert_eq!(count(&harness, "SELECT count(*) FROM game_reward_claim_inbox_mint_reservations").await?, 1);
                assert_eq!(count(&harness, "SELECT count(*) FROM store_delivery_receipts").await?, 0);
                sqlx::query("DROP TRIGGER fail_store_commit ON store_delivery_receipts").execute(&harness.pool).await?;
                25
            };
            if matches!(case, Case::CrossCharacter | Case::LegacyCrossCharacter) {
                let store = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
                    harness.root.clone(),
                );
                release_store_character(&store, admitted).await?;
                let (second_node, second) =
                    super::chat_world_fixture::admit_second_node_for_account(&harness, account_id)
                        .await?;
                let mut second_owner = second.owner(&harness, &authority);
                second_owner.holder = &second_node;
                second_owner.active_generation = Some(active);
                second_owner.store = owner.store.clone();
                let mut second_admitted = second.admitted;
                second_admitted.continuity.selected_capabilities =
                    admitted.continuity.selected_capabilities;
                let command = purchase_request(83, SWORD, price)?;
                let retry = if matches!(case, Case::LegacyCrossCharacter) {
                    // The chat fixture admits a bootstrap-only recipient. Give this buyer
                    // the same initialized progression the Store capacity gate expects.
                    sqlx::query(
                        "INSERT INTO game_character_progression_state VALUES
                         (encode($1,'hex')::uuid,1,50,1000,'profile-1','ruleset-1','content-1',
                          'simulation-1','evidence-1','declaration-1','policy-1','reward-1')",
                    ).bind(id(super::chat_world_fixture::RECIPIENT).as_slice())
                        .execute(&harness.pool).await?;
                    let replies = serve(&second_owner, &mut second_admitted, &[command]).await?;
                    assert_eq!(ledger.lock().expect("ledger").debits, [price], "the second Character debited an unfinished legacy offer");
                    assert_eq!(replies[0].0, CommandStatus::Accepted);
                    wire::decode_purchase_result(&replies[0].1).map_err(debug)?
                } else {
                    purchase(&second_owner, &mut second_admitted, command).await?
                };
                assert_eq!(retry.code, wire::StorePurchaseCode::OfferUnavailable);
                assert_eq!(swords(&harness).await?, 0);
                release_store_character(&store, second_admitted).await?;

                let session =
                    super::full_world_support::relogin_as(&harness, id(41), account_id, id(111))
                        .await?;
                let current = store.current_session(session).await?;
                let actor = {
                    let mut runtime = owner.runtime.lock().await;
                    runtime
                        .remove_terminal_session(admitted.game_session_id, deps.actor)
                        .map_err(debug)?;
                    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
                    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                    runtime
                        .initialize_first_entry_position(actor)
                        .map_err(debug)?;
                    runtime
                        .bind_attacker_lease(actor, session, current.current_character_lease())
                        .map_err(debug)?;
                    actor
                };
                deps.initialize_player(&owner, actor, session).await;
                admitted = crate::gameplay_transport::connection::AdmittedSession {
                    game_session_id: session,
                    runtime_actor: Some(actor),
                    controller: Some(ControllerBinding {
                        account_id,
                        transport: current.current_transport().ok_or("purchaser transport")?,
                    }),
                    item_fence: Some(
                        crate::gameplay_transport::item_fence_of(current)
                            .ok_or("purchaser fence")?,
                    ),
                    continuity: SessionContinuity {
                        selected_capabilities: admitted.continuity.selected_capabilities,
                        ..SessionContinuity::FRESH
                    },
                    ..admitted
                };
            }
            let resume_id = if matches!(case, Case::LegacyCrossCharacter) { 82 } else { 81 };
            let resumed_request = purchase_request(resume_id, SWORD, price)?;
            let retry = purchase(&owner, &mut admitted, resumed_request.clone()).await?;
            assert_eq!(retry.code, wire::StorePurchaseCode::Accepted);
            assert_eq!(purchase(&owner, &mut admitted, resumed_request).await?, retry);
            assert_eq!(swords(&harness).await?, 1);
            let recipient: sqlx::types::Uuid = sqlx::query_scalar(
                "SELECT l.character_id FROM game_item_character_inbox_locations l
                 JOIN game_item_instances i USING(item_instance_id)
                 WHERE i.definition_production_key = $1",
            )
            .bind(SWORD_ITEM)
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(recipient.as_bytes(), &id(41));
            assert_eq!(ledger.lock().expect("ledger").debits, [price]);
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

        if matches!(case, Case::SlowClaimDiscovery) {
            ledger.lock().expect("ledger").advertise_claims = true;
            purchase(&owner, &mut admitted, purchase_request(80, SWORD, 25)?).await?;
            let second = purchase(&owner, &mut admitted, purchase_request(81, SWORD, 25)?).await?;
            verify_claim_discovery_yields(&owner, admitted, &ledger, second.operation_id.ok_or("delivery")?).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

        if matches!(case, Case::RestoreMissingDelivery | Case::RestoreMissingClaim) {
            let claim = matches!(case, Case::RestoreMissingClaim);
            if claim {
                ledger.lock().expect("ledger").advertise_claims = true;
                purchase(&owner, &mut admitted, purchase_request(80, SWORD, 25)?).await?;
            } else {
                retained_purchase(&owner, admitted, include_bytes!(
                    "../../../../../../rulesets/economy/store/predecessors/catalogue.json"
                ), SWORD, RetainedAnswer::Lost).await?;
            }
            drop(owner);
            drop(authority);
            let predecessor = seal.record().recovery_generation;
            let issued_at = seal.record().issued_at.checked_add(1).ok_or("issued at")?;
            drop(seal);
            let transition = harness.recovery.begin_recovery(predecessor, id(181), issued_at).map_err(debug)?;
            harness.root.reconcile_character_recovery(&transition).await.map_err(debug)?;
            let ready = {
                let mut platform = ledger.lock().expect("ledger");
                platform.recovery_generation = Some(transition.record().recovery_generation.to_string());
                platform.advertise_deliveries = !claim;
                platform.reopened = true;
                platform.receipt_ready = Arc::default();
                platform.receipt_ready.clone()
            };
            let generation = transition.record().recovery_generation;
            drop(transition);
            let client = StoreClient::new(Box::new(FakePlatform(ledger.clone())), "oteryn-game/store-consumer".into());
            let worker = crate::gameplay_transport::run_retained_store_reconciliation(&harness.root, client, generation);
            let published = tokio::select! {
                () = worker => false,
                observed = tokio::time::timeout(std::time::Duration::from_secs(2), ready.notified()) => observed.is_ok(),
            };
            assert!(published, "restore must resolve the reopened operation");
            assert_eq!(ledger.lock().expect("ledger").restore_conflicts, [if claim { "CLAIM" } else { "DELIVER" }]);
            assert_eq!(swords(&harness).await?, i64::from(claim));
            assert_eq!(count(&harness, "SELECT count(*) FROM game_store_restore_conflicts").await?, 1);
            return harness.cleanup().await;
        }

        if matches!(case, Case::RestoreReceiptGeneration | Case::ClaimReceipt) {
            let claim = matches!(case, Case::ClaimReceipt);
            {
                let mut platform = ledger.lock().expect("ledger");
                platform.advertise_claims = claim;
            }
            let result = purchase(&owner, &mut admitted, purchase_request(80, SWORD, 25)?).await?;
            assert_eq!(result.code, wire::StorePurchaseCode::Accepted);
            let pending = owner.root.list_unacknowledged_store_receipts(1).await.map_err(debug)?;
            assert!(pending.is_empty(), "delivery was already acknowledged");
            let ready = {
                let mut platform = ledger.lock().expect("ledger");
                platform.fail_receipt = false;
                platform.receipt_ready = Arc::default();
                platform.receipt_ready.clone()
            };
            let client = StoreClient::new(
                Box::new(FakePlatform(ledger.clone())),
                "oteryn-game/store-consumer".into(),
            );
            if claim {
                let worker = crate::gameplay_transport::run_store_receipt_outbox(owner.root, owner.character, client);
                let _ = tokio::time::timeout(std::time::Duration::from_secs(2), worker).await;
                assert_eq!(ledger.lock().expect("ledger").claims, 0, "item remains in the Inbox");
                super::item_look_fixtures::seed_main_backpack(&harness, &[]).await?;
                sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
                    .bind(admitted.world_id.as_bytes().as_slice())
                    .bind(admitted.channel_id.as_bytes().as_slice())
                    .execute(&harness.pool).await?;
                sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
                    .bind(admitted.world_id.as_bytes().as_slice())
                    .bind(admitted.channel_id.as_bytes().as_slice())
                    .bind(pin.identity().server_artifact_digest().as_slice())
                    .bind(pin.identity().client_artifact_digest().as_slice())
                    .bind(pin.frame_binding().digest().as_slice())
                    .execute(&harness.pool).await?;
                let request = wire::StoreInboxClaimRequest {
                    fulfilment_unit_id: uuid(&result.operation_id.ok_or("delivery ID")?),
                };
                let reply = serve(&owner, &mut admitted, &[(
                    wire::COMMAND_TYPE_STORE_INBOX_CLAIM,
                    wire::encode_inbox_claim_request(&request).map_err(debug)?,
                )]).await?;
                assert_eq!(reply[0].0, CommandStatus::Accepted);
                let receipt = wire::decode_claim_receipt(&reply[0].1).map_err(debug)?;
                assert_eq!(receipt.result, wire::StoreClaimCode::Committed);
                assert_eq!(ledger.lock().expect("ledger").claims, 1);
                assert_eq!(count(&harness, "SELECT count(*) FROM game_item_character_inbox_locations").await?, 0);
                drop(owner);
                drop(authority);
                drop(seal);
                return harness.cleanup().await;
            }
            drop(owner);
            drop(authority);
            let predecessor = seal.record().recovery_generation;
            let issued_at = seal.record().issued_at.checked_add(1).ok_or("issued at")?;
            drop(seal);
            let transition = harness.recovery.begin_recovery(predecessor, id(181), issued_at).map_err(debug)?;
            harness.root.reconcile_character_recovery(&transition).await.map_err(debug)?;
            {
                let mut platform = ledger.lock().expect("ledger");
                platform.recovery_generation = Some(transition.record().recovery_generation.to_string());
                platform.advertise_deliveries = true;
                platform.reopened = true;
                platform.receipts = 0;
            }
            let generation = transition.record().recovery_generation;
            drop(transition);
            let worker = crate::gameplay_transport::run_retained_store_reconciliation(&harness.root, client, generation);
            let published = tokio::select! {
                () = worker => false,
                observed = tokio::time::timeout(std::time::Duration::from_secs(2), ready.notified()) => observed.is_ok(),
            };
            assert!(published, "worker must publish the pending receipt");
            assert_eq!(ledger.lock().expect("ledger").claims, 0);
            assert_eq!(swords(&harness).await?, 1);
            assert_eq!(ledger.lock().expect("ledger").debits, [25]);
            return harness.cleanup().await;
        }

        if matches!(case, Case::ReceiptRecovery) {
            ledger.lock().expect("ledger").fail_receipt = true;
            let result = purchase(&owner, &mut admitted, purchase_request(80, "store:exercise-wraps", 25)?).await?;
            assert_eq!(result.code, wire::StorePurchaseCode::Accepted);
            let pending = owner.root.list_unacknowledged_store_receipts(1).await.map_err(debug)?;
            assert_eq!(pending.len(), 1);
            ledger.lock().expect("ledger").fail_receipt = false;
            let mut restarted_client = StoreClient::new(
                Box::new(FakePlatform(ledger.clone())),
                "oteryn-game/store-consumer".into(),
            );
            crate::gameplay_transport::store::receipt_outbox::publish_pending(
                owner.root, &mut restarted_client,
            ).await;
            assert_eq!(ledger.lock().expect("ledger").receipts, 1);
            assert!(owner.root.list_unacknowledged_store_receipts(1).await.map_err(debug)?.is_empty());
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

        if matches!(case, Case::LegacyLostAnswer | Case::LegacyAnsweredWithoutHistory) {
            let saved = if matches!(case, Case::LegacyAnsweredWithoutHistory) {
                RetainedAnswer::Journal
            } else {
                RetainedAnswer::Lost
            };
            let old = retained_purchase(&owner, admitted, include_bytes!(
                "../../../../../../rulesets/economy/store/predecessors/catalogue.json"
            ), SWORD, saved).await?;
            assert_eq!(count(&harness, "SELECT count(*) FROM game_store_purchase_history").await?, 0);
            assert_eq!(swords(&harness).await?, 0);
            let result = purchase(&owner, &mut admitted, purchase_request(82, SWORD, old.price)?).await?;
            assert_eq!(result.code, wire::StorePurchaseCode::Accepted);
            assert_eq!(ledger.lock().expect("ledger").debits, [old.price]);
            assert_eq!(swords(&harness).await?, 1);
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

        if matches!(case, Case::LegacyUnpaidFullInbox) {
            ledger.lock().expect("ledger").balance = 2_000;
            retained_purchase(&owner, admitted, include_bytes!(
                "../../../../../../rulesets/economy/store/predecessors/charged-catalogue.json"
            ), "store:ultimate-health-keg", RetainedAnswer::Unsent).await?;
            assert!(ledger.lock().expect("ledger").debits.is_empty());
            for seed in 60..80 {
                assert_eq!(
                    purchase(&owner, &mut admitted, purchase_request(seed, SWORD, 25)?).await?.code,
                    wire::StorePurchaseCode::Accepted,
                );
            }
            assert_eq!(count(&harness, "SELECT count(*) FROM game_item_character_inbox_locations").await?, 20);
            let open = wire::browse::Query {
                category: "featured".into(),
                search: String::new(),
                sort: 0,
                cursor: String::new(),
            };
            let replies = serve(&owner, &mut admitted, &[(wire::browse::COMMAND_TYPE, open.encode().map_err(debug)?)]).await?;
            assert_eq!(replies[0].0, CommandStatus::Accepted);
            assert_eq!(ledger.lock().expect("ledger").debits, vec![25; 20], "Store browse debited an unreserved legacy request");
            assert_eq!(count(&harness, "SELECT count(*) FROM store_purchase_requests WHERE state=1").await?, 1);
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

        if matches!(case, Case::RemovedOffer | Case::RemovedOfferAccepted | Case::RemovedOfferBelowCapacity) {
            let history = matches!(case, Case::RemovedOfferAccepted);
            let below_capacity = matches!(case, Case::RemovedOfferBelowCapacity);
            ledger.lock().expect("ledger").balance = 2_000;
            let request = retained_purchase(&owner, admitted, include_bytes!(
                "../../../../../../rulesets/economy/store/predecessors/charged-catalogue.json"
            ), "store:ultimate-health-keg", if history {
                RetainedAnswer::History
            } else if below_capacity {
                // Legacy Sent first-debit finding: bypass capacity only with a retained Platform answer.
                RetainedAnswer::Journal
            } else {
                RetainedAnswer::Lost
            }).await?;
            if below_capacity {
                super::item_look_fixtures::seed_main_backpack(
                    &harness,
                    &[("oteryn:item.tibia.i25905", 1), ("oteryn:item.tibia.i25910", 1)],
                ).await?;
            }
            assert_eq!(count(&harness, "SELECT count(*) FROM store_purchase_requests WHERE state=1").await?, i64::from(!history && !below_capacity));
            assert_eq!(count(&harness, "SELECT count(*) FROM game_store_purchase_history").await?, i64::from(history));
            assert_eq!(count(&harness, "SELECT count(*) FROM store_delivery_receipts").await?, 0);
            let query = wire::browse::Query {
                category: "featured".into(),
                search: String::new(),
                sort: 0,
                cursor: String::new(),
            };
            // The legacy path isolates journal selection from browser routing.
            let open = if history { page_request()? } else { (wire::browse::COMMAND_TYPE, query.encode().map_err(debug)?) };
            let page = serve(&owner, &mut admitted, &[open]).await?;
            assert_eq!(page[0].0, CommandStatus::Accepted);
            assert_eq!(count(&harness, "SELECT count(*) FROM store_delivery_receipts").await?, 1);
            assert_eq!(ledger.lock().expect("ledger").debits, [request.price]);
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

        if matches!(case, Case::OldQuote) {
            let request = retained_purchase(&owner, admitted, include_bytes!(
                "../../../../../../rulesets/economy/store/predecessors/catalogue.json"
            ), SWORD, RetainedAnswer::Lost).await?;
            assert_eq!(swords(&harness).await?, 0);
            let command = |request: &wire::StorePurchaseRequest| -> TestResult<_> {
                Ok((wire::COMMAND_TYPE_STORE_PURCHASE, wire::encode_purchase_request(request).map_err(debug)?))
            };
            let mut fresh = request.clone();
            fresh.purchase_request_id = id(82);
            let fresh_result = purchase(&owner, &mut admitted, command(&fresh)?).await?;
            assert_eq!(fresh_result.code, wire::StorePurchaseCode::StaleQuote);
            for _ in 0..2 {
                let result = purchase(&owner, &mut admitted, command(&request)?).await?;
                assert_eq!(result.code, wire::StorePurchaseCode::Accepted);
            }
            assert_eq!(ledger.lock().expect("ledger").debits, [25]);
            assert_eq!(swords(&harness).await?, 1);
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

        if matches!(case, Case::CrystalCapacity) {
            super::item_look_fixtures::seed_main_backpack(
                &harness,
                &[("oteryn:item.tibia.i25905", 1), ("oteryn:item.tibia.i25910", 1)],
            ).await?;
            ledger.lock().expect("ledger").balance = 2_000;
            let result = purchase(&owner, &mut admitted, purchase_request(80, "store:great-health-keg", 103)?).await?;
            assert_eq!(result.code, wire::StorePurchaseCode::OfferUnavailable);
            assert!(ledger.lock().expect("ledger").debits.is_empty());
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

        if matches!(case, Case::CrystalCharges) {
            verify_crystal_charge_purchases(&harness, &owner, &mut admitted, &ledger).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

        // The catalogue page: 24 admitted exercise weapons and the Platform balance.
        let values = serve(&owner, &mut admitted, &[page_request()?]).await?;
        assert_eq!(values.len(), 1, "{values:?}");
        assert_eq!(values[0].0, CommandStatus::Accepted, "{values:?}");
        let page = wire::decode_catalogue_page(&values[0].1).map_err(debug)?;
        assert_eq!(page.catalog_revision, crate::gameplay_transport::store::active_catalogue_revision());
        assert_eq!((page.offers.len(), page.total_coins, page.transferable_coins), (24, 60, 60));
        let sword = page
            .offers
            .iter()
            .find(|offer| offer.offer_key == SWORD)
            .ok_or("sword offer")?;
        assert_eq!((sword.price, sword.title.as_str()), (25, "Exercise Sword"));

        if malformed {
            let values = serve(&owner, &mut admitted, &[purchase_request(62, SWORD, 25)?]).await?;
            assert_eq!(values, [(CommandStatus::Rejected, Vec::new())]);
            assert_eq!(swords(&harness).await?, 0);
            assert_eq!(count(&harness, "SELECT count(*) FROM store_purchase_requests WHERE state=1 AND answer IS NULL").await?, 1);
            assert_eq!(count(&harness, "SELECT count(*) FROM store_delivery_receipts").await?, 0);
            assert_eq!(ledger.lock().expect("ledger").debits, [25]);
            assert_eq!(ledger.lock().expect("ledger").receipts, 0);
            drop(owner); drop(authority); drop(seal);
            return harness.cleanup().await;
        }

        // A stale price never reaches Platform and writes nothing.
        let stale = purchase(&owner, &mut admitted, purchase_request(60, SWORD, 24)?).await?;
        assert_eq!(stale.code, wire::StorePurchaseCode::StaleQuote);
        let unknown = purchase(&owner, &mut admitted, purchase_request(61, "store:x", 25)?).await?;
        assert_eq!(unknown.code, wire::StorePurchaseCode::OfferUnavailable);
        assert_eq!(ledger.lock().expect("ledger").purchases, 0);
        assert_eq!(count(&harness, "SELECT count(*) FROM store_purchase_requests").await?, 0);

        // Accepted: one debit, one sword in the Inbox, one delivery receipt published.
        let accepted = purchase(&owner, &mut admitted, purchase_request(62, SWORD, 25)?).await?;
        assert_eq!(accepted.code, wire::StorePurchaseCode::Accepted);
        assert_eq!(accepted.order_id, Some(id(121)));
        assert_eq!(accepted.operation_id, Some(id(101)));
        assert_eq!(swords(&harness).await?, 1);
        assert_eq!(count(&harness, "SELECT count(*) FROM store_delivery_receipts").await?, 1);
        {
            let ledger = ledger.lock().expect("ledger");
            assert_eq!((ledger.purchases, ledger.balance, ledger.receipts), (1, 35, 1));
        }

        // T130: full browse crosses the real route and ranks the durable accepted purchase.
        let browse = wire::browse::Query {
            category: "featured".into(),
            search: "".into(),
            sort: 0,
            cursor: String::new(),
        };
        let rows = serve(
            &owner,
            &mut admitted,
            &[(wire::browse::COMMAND_TYPE, browse.encode().map_err(debug)?)],
        )
        .await?;
        assert_eq!(rows[0].0, CommandStatus::Accepted, "{rows:?}");
        let browser = wire::browse::Page::decode(&rows[0].1).map_err(debug)?;
        assert_eq!(browser.total, 35);
        assert!(!browser.transferable_known);
        assert!(
            browser
                .categories
                .iter()
                .any(|c| c.key == "consumables.exercise-weapons")
        );
        assert_eq!(browser.products[0].offer.offer_key, SWORD);
        assert_eq!(browser.products[0].offer.price, 25);
        assert_eq!(browser.products[0].revision, page.catalog_revision);
        assert_eq!(browser.products[0].artwork.kind, 1);
        assert!(
            browser.products[0]
                .artwork
                .looks
                .iter()
                .all(|look| *look > 0)
        );
        assert_eq!(swords(&harness).await?, 1);
        assert_eq!(ledger.lock().expect("ledger").purchases, 1);
        // A retry of the same request id replays the saved answer: no second debit or mint.
        let replay = purchase(&owner, &mut admitted, purchase_request(62, SWORD, 25)?).await?;
        assert_eq!(replay, accepted);
        assert_eq!(swords(&harness).await?, 1);
        assert_eq!(count(&harness, "SELECT count(*) FROM store_delivery_receipts").await?, 1);
        assert_eq!(ledger.lock().expect("ledger").purchases, 1);

        let ranks = owner
            .root
            .store_popularity(vec![SWORD.into()])
            .await
            .map_err(debug)?;
        assert_eq!(
            ranks.get(SWORD),
            Some(&1),
            "replay must not inflate popularity"
        );
        let permissions:(bool,bool)=sqlx::query_as("SELECT has_table_privilege('oteryn_game_runtime','game_store_offer_popularity','SELECT'),has_table_privilege('oteryn_game_runtime','game_store_offer_popularity','UPDATE,INSERT,DELETE')").fetch_one(&harness.pool).await?;
        assert_eq!(permissions, (true, false));

        // The same request id bound to another offer is refused before Platform.
        let values = serve(
            &owner,
            &mut admitted,
            &[purchase_request(62, "store:exercise-axe", 25)?],
        )
        .await?;
        assert_eq!(values.len(), 1, "{values:?}");
        assert_ne!(values[0].0, CommandStatus::Accepted, "{values:?}");
        assert_eq!(ledger.lock().expect("ledger").purchases, 1);

        // Insufficient balance: Platform refuses, Game mints nothing, the answer is retained.
        ledger.lock().expect("ledger").balance = 10;
        let poor = purchase(&owner, &mut admitted, purchase_request(65, SWORD, 25)?).await?;
        assert_eq!(poor.code, wire::StorePurchaseCode::InsufficientBalance);
        let again = purchase(&owner, &mut admitted, purchase_request(65, SWORD, 25)?).await?;
        assert_eq!(again.code, wire::StorePurchaseCode::InsufficientBalance);
        {
            let ledger = ledger.lock().expect("ledger");
            assert_eq!((ledger.purchases, ledger.debits.len()), (2, 1));
        }
        assert_eq!(swords(&harness).await?, 1);

        // Crystal data/modules/scripts/gamestore/gamestore.lua:6554-6558 prices the
        // one-time Character product at 450; init.lua:767-770 refuses an owner.
        ledger.lock().expect("ledger").balance = 500;
        let charm = purchase(
            &owner,
            &mut admitted,
            purchase_request(70, CHARM_EXPANSION, 450)?,
        )
        .await?;
        assert_eq!(charm.code, wire::StorePurchaseCode::Accepted);
        assert_eq!(count(&harness, "SELECT count(*) FROM game_character_charm_expansions").await?, 1);
        let mut forged = harness.pool.begin().await?;
        sqlx::query("INSERT INTO store_delivery_receipts(operation_id,account_id,character_id,world_id,fulfilment_unit_id,result,recovery_generation,decided_at,receipt,item_instance_id,entitlement_key,purchase_request_id) SELECT game_character_uuid_v7(),account_id,character_id,world_id,fulfilment_unit_id||'-forged',result,recovery_generation,decided_at,receipt,NULL,'charm-expansion',purchase_request_id FROM store_delivery_receipts WHERE entitlement_key='charm-expansion' LIMIT 1")
            .execute(&mut *forged).await?;
        assert!(forged.commit().await.is_err(), "receipt without purchase proof must fail");
        let debits = ledger.lock().expect("ledger").debits.len();
        let second = purchase(
            &owner,
            &mut admitted,
            purchase_request(71, CHARM_EXPANSION, 450)?,
        )
        .await?;
        assert_eq!(second.code, wire::StorePurchaseCode::OfferUnavailable);
        assert_eq!(ledger.lock().expect("ledger").debits.len(), debits);

        // The base offer omits dynamically admitted mechanics. Compose the Charm
        // offer just as the production handshake does before selecting its command.
        let charm_offered = crate::gameplay_transport::charm_progression::offered(
            owner.offered_capabilities(), owner.charm_progression_available(),
        );
        let charm_capability = oteryn_protocol_oteryn::charm_progression::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &charm_offered,
            &[wire::CAPABILITY_STORE_V1, charm_capability],
        ).ok_or("Store and charm selection")?;
        assert!(admitted.continuity.selected_capabilities.contains(charm_capability));
        let charm_query = oteryn_protocol_oteryn::charm_progression::encode_charm_progression_request(
            &oteryn_protocol_oteryn::charm_progression::CharmProgressionAction::Query,
        ).map_err(debug)?;
        let query = serve(&owner, &mut admitted, &[(
            oteryn_protocol_oteryn::charm_progression::COMMAND_TYPE_CHARM_PROGRESSION,
            charm_query,
        )]).await?;
        assert_eq!(query[0].0, CommandStatus::Accepted, "Charm query must reach its native owner");
        let reply = oteryn_protocol_oteryn::charm_progression::decode_charm_progression_reply(&query[0].1).map_err(debug)?;
        // Crystal src/io/iobestiary.cpp:603-607 raises Premium from six slots to all
        // 25 donor runes; Oteryn represents the unbounded catalogue limit as None.
        assert_eq!(reply.charms.assignment_slot_limit, None);
        eprintln!("STORE PostgreSQL dispatch page, purchase, mint, replay and refusals verified");
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn fulfilled_store_blessing_copy_is_visible_through_holdings() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-blessing", || {
        scenario_with_answer(Case::Blessing)
    })
}

async fn store_blessing_holdings(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    ledger: &SyncMutex<Ledger>,
) -> TestResult {
    use crate::gameplay_transport::capabilities::OfferedCapability;
    use oteryn_protocol_oteryn::cyclopedia_character::blessings_dialog::{
        COMMAND_TYPE_BLESSINGS_HOLDINGS, decode_holdings,
    };
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(), &[wire::CAPABILITY_STORE_V1],
    ).ok_or("Store selection")?;
    let page = wire::encode_catalogue_request(&wire::StoreCatalogueRequest {
        category: "consumables.blessings".into(),
        page_cursor: String::new(),
    }).map_err(debug)?;
    let reply = serve(owner, admitted, &[(wire::COMMAND_TYPE_STORE_CATALOGUE_PAGE, page)]).await?;
    assert_eq!(reply[0].0, CommandStatus::Accepted);
    let page = wire::decode_catalogue_page(&reply[0].1).map_err(debug)?;
    let offer = page.offers.iter().find(|offer| offer.offer_key == "store:blessing:twist_of_fate")
        .ok_or("Twist of Fate Store offer")?;
    assert_eq!(offer.price, 8);
    let purchase_wire = wire::encode_purchase_request(&wire::StorePurchaseRequest {
        purchase_request_id: id(79),
        offer_key: offer.offer_key.clone(),
        catalog_revision: page.catalog_revision,
        price: offer.price,
        currency: offer.currency.clone(),
    }).map_err(debug)?;
    for _ in 0..2 {
        let result = purchase(owner, admitted, (wire::COMMAND_TYPE_STORE_PURCHASE, purchase_wire.clone())).await?;
        assert_eq!(result.code, wire::StorePurchaseCode::Accepted);
    }
    {
        let ledger = ledger.lock().expect("ledger");
        assert_eq!(ledger.purchases, 1);
        assert_eq!(ledger.receipts, 2, "the immutable delivery can be republished");
    }
    let offers = [
        OfferedCapability { id: 199, requires: &[] },
        OfferedCapability { id: 382, requires: &[] },
        OfferedCapability { id: 503, requires: &[199] },
        OfferedCapability { id: 584, requires: &[382, 503] },
    ];
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(&offers, &[199, 382, 503, 584]).ok_or("holdings selection")?;
    let reply = serve(owner, admitted, &[(COMMAND_TYPE_BLESSINGS_HOLDINGS, Vec::new())]).await?;
    assert_eq!(reply[0].0, CommandStatus::Accepted);
    let holdings = decode_holdings(&reply[0].1).ok_or("holdings reply")?;
    let copy = holdings.iter().find(|row| row.key == "oteryn:blessing.twist_of_fate")
        .ok_or("Twist of Fate holding")?;
    assert_eq!((copy.held, copy.store_copies), (false, 1));
    Ok(())
}

async fn verify_claim_discovery_yields(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    ledger: &Arc<SyncMutex<Ledger>>,
    delivery: [u8; 16],
) -> TestResult {
    let slow = Arc::new(super::store_fixtures::SlowClaimReads::default());
    ledger.lock().expect("ledger").slow_claim_reads = Some(slow.clone());
    let request = [(wire::COMMAND_TYPE_STORE_INBOX_CLAIM, wire::encode_inbox_claim_request(
        &wire::StoreInboxClaimRequest { fulfilment_unit_id: uuid(&delivery) },
    ).map_err(debug)?)];
    let account = uuid(&admitted.controller.ok_or("controller")?.account_id);
    let claim = serve(owner, &mut admitted, &request);
    tokio::pin!(claim);
    tokio::select! {
        result = &mut claim => return Err(format!("claim finished before discovery: {result:?}").into()),
        ready = tokio::time::timeout(std::time::Duration::from_secs(2), slow.first_started.notified()) => ready?,
    }
    let platform = owner.store.as_ref().ok_or("Platform")?;
    let balance = async { platform.lock().await.balance(&account).await };
    tokio::pin!(balance);
    tokio::select! {
        biased;
        _ = &mut balance => return Err("balance did not wait behind the first exchange".into()),
        () = std::future::ready(()) => {}
    }
    slow.release_first.notify_one();
    tokio::select! {
        biased;
        result = &mut claim => Err(format!("claim finished before balance: {result:?}").into()),
        result = &mut balance => { result.map_err(debug)?; Ok(()) }
        () = slow.second_started.notified() => Err("second intent read preempted a queued Store balance read".into()),
    }
}

#[derive(Clone, Copy)]
enum RetainedAnswer {
    Unsent,
    Lost,
    Journal,
    History,
}

async fn retained_purchase(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    catalogue: &[u8],
    offer: &str,
    saved: RetainedAnswer,
) -> TestResult<wire::StorePurchaseRequest> {
    use crate::durability::store::{PurchaseRequest, PurchaseRequestState};
    use crate::native_admission_source::store::{PurchaseResult, PurchaseStoreOffer};
    use oteryn_gameplay_economy::store as rules;

    let account = admitted.controller.ok_or("controller")?.account_id;
    let character = admitted.item_fence.ok_or("fence")?.character_id;
    let mut snapshot: serde_json::Value = serde_json::from_slice(catalogue)?;
    let price = snapshot["offers"].as_array().ok_or("predecessor offers")?.iter()
        .find(|row| row["key"] == offer).and_then(|row| row["price"].as_u64())
        .ok_or("predecessor price")?;
    snapshot.sort_all_objects();
    let revision = super::store_fixtures::sha256(&serde_json::to_vec(&snapshot)?);
    let purchase_request_id = id(81);
    let binding = rules::PurchaseStoreOffer {
        purchase_request_id: rules::PurchaseRequestId::from_bytes(purchase_request_id).map_err(debug)?,
        account_id: crate::domain::AccountId::from_bytes(account).map_err(debug)?,
        character_id: character,
        offer_key: offer.into(),
        catalog_revision: revision.clone(),
        price,
        currency: rules::CurrencyKind::TibiaCoin,
    };
    let journal = PurchaseRequest {
        purchase_request_id,
        account_id: account,
        character_id: *character.as_bytes(),
        binding: binding.binding_digest().to_vec(),
    };
    let state = owner.root.record_purchase_request(journal.clone()).await.map_err(debug)?;
    assert_eq!(state, PurchaseRequestState::Sent);
    let request = wire::StorePurchaseRequest {
        purchase_request_id,
        offer_key: offer.into(),
        catalog_revision: revision.clone(),
        price,
        currency: "TIBIA_COIN".into(),
    };
    if matches!(saved, RetainedAnswer::Unsent) {
        return Ok(request);
    }
    let answer = owner.store.as_ref().ok_or("Platform")?.lock().await.purchase(&PurchaseStoreOffer {
        purchase_request_id: uuid(&purchase_request_id),
        account_id: uuid(&account),
        character_id: uuid(character.as_bytes()),
        world_id: uuid(admitted.world_id.as_bytes()),
        offer_key: offer.into(),
        catalog_revision: revision.clone(),
        currency: "COINS".into(),
        unit_price: price.to_string(),
        quantity: 1,
    }, "ITEM").await.map_err(debug)?;
    let PurchaseResult::Accepted { order_id, operation_ids } = answer else {
        return Err("fixture purchase refused".into());
    };
    if !matches!(saved, RetainedAnswer::Lost) {
        let order_id = *sqlx::types::Uuid::parse_str(&order_id)?.as_bytes();
        owner.root.record_store_answer_with_history(
            journal,
            serde_json::to_vec(&serde_json::json!({
                "result": "ACCEPTED", "order_id": uuid(&order_id), "operation_ids": operation_ids,
            }))?,
            matches!(saved, RetainedAnswer::History).then(|| crate::durability::store::StorePurchaseHistoryEntry {
                order_id,
                offer_key: offer.into(),
                catalog_revision: revision.clone(),
                price,
                currency: "TIBIA_COIN".into(),
            }),
        ).await.map_err(debug)?;
    }
    Ok(request)
}

async fn verify_crystal_charge_purchases(
    harness: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    ledger: &SyncMutex<Ledger>,
) -> TestResult {
    use crate::content::ReferenceItemField::Known;

    let inbox_count = "SELECT count(*) FROM game_item_character_inbox_locations";
    let initial = count(harness, inbox_count).await?;
    ledger.lock().expect("ledger").balance = 2_000;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native Items")?;
    let mut seed = 80;
    for (category, offers) in [
        (
            "consumables.exercise-weapons",
            &[("exercise-wraps", 50293, 25)][..],
        ),
        (
            "consumables.kegs",
            &[
                ("great-health-keg", 25905, 103),
                ("great-mana-keg", 25910, 66),
                ("great-spirit-keg", 25913, 105),
                ("health-keg", 25903, 26),
                ("mana-keg", 25908, 26),
                ("strong-health-keg", 25904, 53),
                ("strong-mana-keg", 25909, 43),
                ("supreme-health-keg", 25907, 288),
                ("ultimate-mana-keg", 25911, 202),
                ("ultimate-spirit-keg", 25914, 202),
            ][..],
        ),
        (
            "useful-things",
            &[
                ("gold-converter", 23722, 5),
                ("magic-gold-converter", 28525, 15),
            ][..],
        ),
    ] {
        let request = wire::StoreCatalogueRequest {
            category: category.into(),
            page_cursor: String::new(),
        };
        let rows = serve(
            owner,
            admitted,
            &[(
                wire::COMMAND_TYPE_STORE_CATALOGUE_PAGE,
                wire::encode_catalogue_request(&request).map_err(debug)?,
            )],
        )
        .await?;
        assert_eq!(rows[0].0, CommandStatus::Accepted);
        let page = wire::decode_catalogue_page(&rows[0].1).map_err(debug)?;
        for &(slug, item_id, price) in offers {
            let key = format!("store:{slug}");
            let card = page
                .offers
                .iter()
                .find(|card| card.offer_key == key)
                .ok_or_else(|| format!("missing card {key}"))?;
            assert_eq!((card.price, card.currency.as_str()), (price, "TIBIA_COIN"));
            let request = wire::StorePurchaseRequest {
                purchase_request_id: id(seed),
                offer_key: key.clone(),
                catalog_revision: page.catalog_revision.clone(),
                price,
                currency: card.currency.clone(),
            };
            seed += 1;
            let result = purchase(
                owner,
                admitted,
                (
                    wire::COMMAND_TYPE_STORE_PURCHASE,
                    wire::encode_purchase_request(&request).map_err(debug)?,
                ),
            )
            .await?;
            assert_eq!(result.code, wire::StorePurchaseCode::Accepted, "{key}");
            let operation = result.operation_id.ok_or("Store operation")?;
            let minted: (String, String, String) = sqlx::query_as(
                "SELECT i.definition_production_key,i.definition_revision_ref,i.quantity::text
                 FROM store_delivery_receipts r
                 JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id
                 JOIN game_item_character_inbox_locations l
                   ON l.item_instance_id=i.item_instance_id AND l.world_id=i.world_id
                 WHERE r.operation_id=encode($1,'hex')::uuid AND i.lifecycle=1",
            )
            .bind(operation.as_slice())
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(
                minted,
                (
                    format!("oteryn:item.tibia.i{item_id}"),
                    "definition-r1".into(),
                    "1".into()
                )
            );
            let owned: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM store_delivery_receipts r
                 JOIN game_item_bindings b USING(item_instance_id,world_id,character_id)
                 WHERE r.operation_id=encode($1,'hex')::uuid AND b.cause='STORE_PURCHASE')",
            ).bind(operation.as_slice()).fetch_one(&harness.pool).await?;
            assert!(owned, "{key}: Store mint must retain purchaser custody");
            let policy = native
                .item_policy(&minted.0, &minted.1)
                .ok_or("minted definition")?;
            assert!(
                matches!(&policy.record().semantics.charges, Known(charges) if charges.count == Known(500)),
                "{key}"
            );
        }
    }
    assert_eq!(count(harness, inbox_count).await? - initial, 13);
    let unavailable = purchase(owner, admitted, purchase_request(seed, "store:ultimate-health-keg", 175)?).await?;
    assert_eq!(unavailable.code, wire::StorePurchaseCode::OfferUnavailable);
    assert_eq!(ledger.lock().expect("ledger").purchases, 13);
    Ok(())
}
