//! STORE_V1 (E10): admitted connection bytes read a catalogue page with the Platform balance, a
//! stale quote never reaches Platform, an accepted purchase mints one exercise weapon into the
//! Inbox once, a retry of the same request id replays without a second debit or mint, and a
//! conflicting binding or an insufficient balance writes nothing. Platform is a test double of
//! the existing Store port (`StoreTransport`).
#![allow(clippy::expect_used)]

use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{CommandStatus, MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::native_admission_source::descriptor::Operation;
use crate::native_admission_source::store::{StoreClient, StoreTransport, TransportFailure};
use oteryn_protocol_oteryn::store as wire;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex as SyncMutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use sha2::{Digest, Sha256};

const SWORD: &str = "store:exercise-sword";
const SWORD_ITEM: &str = "oteryn:item.tibia.i28552";
const CHARM_EXPANSION: &str = "store:charm-expansion";

#[derive(Default)]
pub(super) struct Ledger {
    pub(super) balance: u64,
    pub(super) world: String,
    pub(super) purchases: usize,
    pub(super) debits: Vec<u64>,
    pub(super) intents: BTreeMap<String, serde_json::Value>,
    pub(super) receipts: usize,
    pub(super) malformed: bool,
    pub(super) answers: BTreeMap<String, (String, serde_json::Value)>,
}

/// The Platform side of the existing Store port: balance, debit, order and delivery intent.
pub(super) struct FakePlatform(pub(super) Arc<SyncMutex<Ledger>>);

impl StoreTransport for FakePlatform {
    fn exchange<'a>(
        &'a mut self,
        operation: Operation,
        body: &'a str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<u8>, TransportFailure>> + Send + 'a>,
    > {
        let mut ledger = self.0.lock().expect("ledger");
        let request: serde_json::Value = serde_json::from_str(body).expect("json body");
        assert_eq!(request["source_authority"], "oteryn-game/store-consumer");
        if operation == Operation::PurchaseStoreOfferV1 { assert_eq!(request["world_id"], ledger.world); }
        let answer = match operation {
            Operation::ReadStoreBalanceV1 => {
                platform_balance(&request, ledger.balance)
            }
            Operation::PurchaseStoreOfferV1 => {
                ledger.purchases += 1;
                let request_id = request["purchase_request_id"].as_str().expect("purchase id").to_owned();
                if let Some((original, answer)) = ledger.answers.get(&request_id) {
                    let result = if original == body { Ok(platform_bytes(answer)) }
                        else { Err(TransportFailure::Conflict) };
                    return Box::pin(async move { result });
                }
                let price = request["unit_price"].as_str().expect("price string").parse::<u64>().expect("price") * request["quantity"].as_u64().expect("quantity");
                if price > ledger.balance {
                    platform_answer(&request, None, ledger.balance, body)
                } else {
                    ledger.balance -= price;
                    ledger.debits.push(price);
                    let n = u8::try_from(ledger.debits.len()).expect("few orders");
                    let operation = uuid(&id(100 + n));
                    let order = uuid(&id(120 + n));
                    let intent = platform_intent(&request, &operation, &order);
                    ledger.intents.insert(operation.clone(), intent);
                    let mut answer = platform_answer(&request, Some((&order, &operation)), ledger.balance, body);
                    ledger.answers.insert(request_id, (body.to_owned(), answer.clone()));
                    if ledger.malformed {
                        let unit = answer["units"][0].clone();
                        answer["units"].as_array_mut().expect("units").push(unit);
                    }
                    answer
                }
            }
            Operation::ReadStoreOperationIntentV1 => {
                let operation = request["operation_id"].as_str().expect("operation");
                ledger.intents[operation].clone()
            }
            Operation::PublishStoreDeliveryReceiptV1 => {
                assert_eq!(request["binding_digest"], sha256(&platform_bytes(&ledger.intents[request["operation_id"].as_str().expect("operation")])));
                assert_eq!(request["result_code"], "STORE_OK");
                ledger.receipts += 1;
                serde_json::json!({"contract_version":1,"result":"accepted"})
            }
            _ => return Box::pin(async { Err(TransportFailure::NotFound) }),
        };
        Box::pin(async move { Ok(platform_bytes(&answer)) })
    }
}

pub(super) fn platform(world: &[u8; 16]) -> crate::gameplay_transport::store::StorePlatform {
    Arc::new(tokio::sync::Mutex::new(StoreClient::new(
        Box::new(FakePlatform(Arc::new(SyncMutex::new(Ledger {
            balance: 60, world: uuid(world), ..Ledger::default()
        })))), "oteryn-game/store-consumer".into(),
    )))
}

pub(super) fn uuid(bytes: &[u8; 16]) -> String {
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

#[test]
fn store_real_dispatch_pages_purchases_mints_once_and_replays() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("store-e2e", scenario);
    eprintln!("STORE-154 POSTGRES_CASE_COMPLETED store-e2e elapsed={:?}", started.elapsed());
    result
}

/// One connection carrying `commands` from the session's next command id.
pub(super) async fn serve(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    commands: &[(u32, Vec<u8>)],
) -> TestResult<Vec<(CommandStatus, Vec<u8>)>> {
    let mut input = Vec::new();
    for (command_id, (command_type, payload)) in
        (admitted.continuity.next_command_id..).zip(commands)
    {
        let bytes = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: *command_type,
                payload,
            },
        )
        .map_err(debug)?;
        input.extend(u32::try_from(bytes.len())?.to_be_bytes());
        input.extend(bytes);
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client.write_all(&input).await?;
    client.shutdown().await?;
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &end
    {
        *admitted = *current;
    }
    let mut cursor = 0;
    let mut values = Vec::new();
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let envelope =
            decode_wire_envelope(&output[cursor + 4..cursor + 4 + length]).map_err(debug)?;
        cursor += length + 4;
        if envelope.message_type() == MessageType::CommandResult {
            let value =
                oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
            values.push((value.status, value.payload.to_vec()));
        }
    }
    Ok(values)
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

pub(super) fn purchase_request(seed: u8, offer: &str, price: u64) -> TestResult<(u32, Vec<u8>)> {
    let request = wire::StorePurchaseRequest {
        purchase_request_id: id(seed),
        offer_key: offer.into(),
        catalog_revision: "451c944aa7b6ab9d27cfce868fcf4949c8996a7b6857cc2641e2c296fd415c8c".into(),
        price,
        currency: "TIBIA_COIN".into(),
    };
    Ok((
        wire::COMMAND_TYPE_STORE_PURCHASE,
        wire::encode_purchase_request(&request).map_err(debug)?,
    ))
}

pub(super) async fn purchase(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: (u32, Vec<u8>),
) -> TestResult<wire::StorePurchaseResult> {
    let values = serve(owner, admitted, &[command]).await?;
    assert_eq!(values.len(), 1, "{values:?}");
    assert_eq!(values[0].0, CommandStatus::Accepted, "{values:?}");
    wire::decode_purchase_result(&values[0].1).map_err(debug)
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
    let result = super::kill_reward_e2e::scenario_thread::run("storefix-malformed", || scenario_with_answer(true, false));
    eprintln!("STORE-154 POSTGRES_CASE_COMPLETED storefix-malformed elapsed={:?}", started.elapsed());
    result
}

#[test]
fn store_restored_backup_refuses_new_and_reconciles_sent() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("store-restored", || scenario_with_answer(false, true));
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

// Shared test-only Platform wire builders. The ordered serializer mirrors the
// Platform fixture order, so these fakes exercise the strict production decoder.
pub(super) fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}
pub(super) fn platform_balance(request: &serde_json::Value, balance: u64) -> serde_json::Value {
    serde_json::json!({"contract_version":1,"result":"OK","account_id":request["account_id"],"balances":[{"currency":"COINS","available":balance.to_string(),"reserved":"0"}],"read_at":"1"})
}
pub(super) fn platform_answer(request: &serde_json::Value, accepted: Option<(&str, &str)>, balance: u64, body: &str) -> serde_json::Value {
    let mut answer = serde_json::json!({"contract_version":1,"purchase_request_id":request["purchase_request_id"],"request_digest":sha256(body.as_bytes()),"result":"INSUFFICIENT_BALANCE","decided_at":"1"});
    if let Some((order, operation)) = accepted {
        answer["result"] = "ACCEPTED".into();
        answer["order_id"] = order.into();
        answer["currency"] = request["currency"].clone();
        answer["total_price"] = (request["unit_price"].as_str().expect("unit price").parse::<u64>().expect("amount") * request["quantity"].as_u64().expect("quantity")).to_string().into();
        answer["available_after"] = balance.to_string().into();
        answer["units"] = serde_json::json!([{"unit_kind":"ITEM","entitlement_id":operation,"delivery_operation_id":operation}]);
    }
    answer
}
pub(super) fn platform_intent(request: &serde_json::Value, operation: &str, order: &str) -> serde_json::Value {
    serde_json::json!({"contract_version":1,"operation_kind":"DELIVER","issuer_authority":"OTERYN_PLATFORM_STORE_AUTHORITY","source_revision":"1","operation_id":operation,"order_id":order,"entitlement_id":operation,"account_id":request["account_id"],"product_id":"oteryn.store.item_grant","product_version":"1","unit_kind":"ITEM","catalog_revision":request["catalog_revision"],"offer_key":request["offer_key"],"grant_key":"item:test","grant_revision":"1","quantity":1,"issued_at_source":"1","audience":"OTERYN_GAME_STORE"})
}
pub(super) fn platform_bytes(v: &serde_json::Value) -> Vec<u8> {
    let fields = if v.get("operation_kind").is_some() {
        "contract_version operation_kind issuer_authority source_revision operation_id order_id entitlement_id account_id product_id product_version unit_kind catalog_revision offer_key grant_key grant_revision quantity issued_at_source audience"
    } else if v["operation"] == "PurchaseStoreOfferV1" {
        "contract_version operation source_authority purchase_request_id account_id character_id world_id catalog_revision offer_key currency unit_price quantity"
    } else if v.get("balances").is_some() { "contract_version result account_id balances read_at" }
    else if v.get("purchase_request_id").is_some() {
        if v["result"] == "ACCEPTED" { "contract_version purchase_request_id request_digest result order_id currency total_price available_after units decided_at" }
        else { "contract_version purchase_request_id request_digest result decided_at" }
    } else if v.get("available").is_some() { "currency available reserved" }
    else if v.get("delivery_operation_id").is_some() { "unit_kind entitlement_id delivery_operation_id" }
    else { "contract_version result" };
    let parts: Vec<_> = fields.split_whitespace().map(|k| {
        let value = &v[k];
        let encoded = if let Some(array) = value.as_array() {
            format!("[{}]", array.iter().map(|v| String::from_utf8(platform_bytes(v)).expect("UTF8")).collect::<Vec<_>>().join(","))
        } else { value.to_string() };
        format!("\"{k}\":{encoded}")
    }).collect();
    format!("{{{}}}", parts.join(",")).into_bytes()
}
