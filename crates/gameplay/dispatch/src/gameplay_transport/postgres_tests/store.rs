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
    FirstEntryOutcome, FreshAdmissionAuthority,
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
    CrystalCharges,
    CrystalCapacity,
    OldQuote,
    ReceiptRecovery,
    RestoreReceiptGeneration,
    ClaimReceipt,
    LastSlot,
    InterruptedMint,
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
fn store_interrupted_mint_retries_with_a_new_wire_command() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-interrupted-mint", || {
        scenario_with_answer(Case::InterruptedMint)
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
fn store_retained_sword_completes_after_catalogue_update() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-old-quote", || {
        scenario_with_answer(Case::OldQuote)
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
fn store_worker_finishes_platform_claim_for_committed_delivery() -> TestResult {
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
        let (controller, _) = crate::content::native_gameplay::test_support::activated_with_loot_tables(
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
            let other = purchase(&owner, &mut admitted, purchase_request(81, SWORD, 25)?).await?;
            let retry = purchase(&owner, &mut admitted, request).await?;
            assert_eq!((other.code, retry.code), (wire::StorePurchaseCode::OfferUnavailable, wire::StorePurchaseCode::Accepted));
            assert_eq!(count(&harness, "SELECT count(*) FROM game_item_character_inbox_locations").await?, 20);
            assert_eq!(ledger.lock().expect("ledger").debits.len(), 20);
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

        if matches!(case, Case::InterruptedMint) {
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
            let resumed_request = purchase_request(81, SWORD, 25)?;
            let retry = purchase(&owner, &mut admitted, resumed_request.clone()).await?;
            assert_eq!(retry.code, wire::StorePurchaseCode::Accepted);
            assert_eq!(purchase(&owner, &mut admitted, resumed_request).await?, retry);
            assert_eq!(swords(&harness).await?, 1);
            assert_eq!(ledger.lock().expect("ledger").debits, [25]);
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

        if matches!(case, Case::RestoreReceiptGeneration | Case::ClaimReceipt) {
            let claim = matches!(case, Case::ClaimReceipt);
            {
                let mut platform = ledger.lock().expect("ledger");
                platform.fail_receipt = true;
                platform.advertise_claims = claim;
            }
            let result = purchase(&owner, &mut admitted, purchase_request(80, SWORD, 25)?).await?;
            assert_eq!(result.code, wire::StorePurchaseCode::Accepted);
            let pending = owner.root.list_unacknowledged_store_receipts(1).await.map_err(debug)?;
            assert_eq!(pending.len(), 1);
            if !claim {
                // Simulate a reconciled successor installed after recovering this
                // backup, retaining the same committed Store delivery and outbox.
                let mut tx = harness.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
                let next: String = sqlx::query_scalar(
                    "UPDATE game_character_recovery_admissions
                     SET recovery_generation=recovery_generation+1
                     RETURNING recovery_generation::text",
                ).fetch_one(&mut *tx).await?;
                tx.commit().await?;
                ledger.lock().expect("ledger").recovery_generation = Some(next);
            }
            let ready = {
                let mut platform = ledger.lock().expect("ledger");
                platform.fail_receipt = false;
                platform.receipt_ready.clone()
            };
            let worker = crate::gameplay_transport::run_store_receipt_outbox(
                owner.root,
                owner.character,
                StoreClient::new(Box::new(FakePlatform(ledger.clone())), "oteryn-game/store-consumer".into()),
            );
            let published = tokio::select! {
                () = worker => false,
                observed = tokio::time::timeout(std::time::Duration::from_secs(2), ready.notified()) => observed.is_ok(),
            };
            assert!(published, "worker must publish the pending receipt");
            assert_eq!(ledger.lock().expect("ledger").claims, usize::from(claim));
            assert_eq!(swords(&harness).await?, 1);
            assert_eq!(ledger.lock().expect("ledger").debits, [25]);
            drop(owner);
            drop(authority);
            drop(seal);
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

        if matches!(case, Case::OldQuote) {
            let request = retained_sword(&owner, admitted).await?;
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

async fn retained_sword(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult<wire::StorePurchaseRequest> {
    use crate::durability::store::{PurchaseRequest, PurchaseRequestState};
    use crate::native_admission_source::store::{PurchaseResult, PurchaseStoreOffer};
    use oteryn_gameplay_economy::store as rules;

    let account = admitted.controller.ok_or("controller")?.account_id;
    let character = admitted.item_fence.ok_or("fence")?.character_id;
    let mut snapshot: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../../rulesets/economy/store/predecessors/catalogue.json"
    ))?;
    snapshot.sort_all_objects();
    let revision = super::store_fixtures::sha256(&serde_json::to_vec(&snapshot)?);
    let purchase_request_id = id(81);
    let binding = rules::PurchaseStoreOffer {
        purchase_request_id: rules::PurchaseRequestId::from_bytes(purchase_request_id).map_err(debug)?,
        account_id: crate::domain::AccountId::from_bytes(account).map_err(debug)?,
        character_id: character,
        offer_key: SWORD.into(),
        catalog_revision: revision.clone(),
        price: 25,
        currency: rules::CurrencyKind::TibiaCoin,
    };
    let state = owner.root.record_purchase_request(PurchaseRequest {
        purchase_request_id,
        account_id: account,
        character_id: *character.as_bytes(),
        binding: binding.binding_digest().to_vec(),
    }).await.map_err(debug)?;
    assert_eq!(state, PurchaseRequestState::Sent);
    let answer = owner.store.as_ref().ok_or("Platform")?.lock().await.purchase(&PurchaseStoreOffer {
        purchase_request_id: uuid(&purchase_request_id),
        account_id: uuid(&account),
        character_id: uuid(character.as_bytes()),
        world_id: uuid(admitted.world_id.as_bytes()),
        offer_key: SWORD.into(),
        catalog_revision: revision.clone(),
        currency: "COINS".into(),
        unit_price: "25".into(),
        quantity: 1,
    }, "ITEM").await.map_err(debug)?;
    assert!(matches!(answer, PurchaseResult::Accepted { .. }));
    Ok(wire::StorePurchaseRequest {
        purchase_request_id,
        offer_key: SWORD.into(),
        catalog_revision: revision,
        price: 25,
        currency: "TIBIA_COIN".into(),
    })
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
