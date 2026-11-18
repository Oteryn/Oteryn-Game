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

fn scenario() -> TestResult { scenario_with_answer(false, false) }

#[test]
fn storefix_protocol_violation_keeps_debit_uncertain_and_never_mints() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("storefix-malformed", || scenario_with_answer(true, false));
    eprintln!("STORE-154 POSTGRES_CASE_COMPLETED storefix-malformed elapsed={:?}", started.elapsed());
    result
}

#[test]
fn store_restored_backup_refuses_new_and_reconciles_sent() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("store-restored", || scenario_with_answer(false, true));
    eprintln!("STORE-154-D6 POSTGRES_CASE_COMPLETED elapsed={:?}", started.elapsed());
    result
}

#[allow(clippy::too_many_lines)]
fn scenario_with_answer(malformed: bool, restored: bool) -> TestResult {
    run(async |admin| {
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

        if restored {
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

        // The catalogue page: 23 admitted exercise weapons and the Platform balance.
        let values = serve(&owner, &mut admitted, &[page_request()?]).await?;
        assert_eq!(values.len(), 1, "{values:?}");
        assert_eq!(values[0].0, CommandStatus::Accepted, "{values:?}");
        let page = wire::decode_catalogue_page(&values[0].1).map_err(debug)?;
        assert_eq!(page.catalog_revision, "451c944aa7b6ab9d27cfce868fcf4949c8996a7b6857cc2641e2c296fd415c8c");
        assert_eq!((page.offers.len(), page.total_coins, page.transferable_coins), (23, 60, 60));
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
