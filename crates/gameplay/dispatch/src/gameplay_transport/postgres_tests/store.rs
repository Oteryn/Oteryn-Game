//! STORE_V1 (E10): admitted connection bytes read a catalogue page with the Platform balance, a
//! stale quote never reaches Platform, an accepted purchase mints one exercise weapon into the
//! Inbox once, a retry of the same request id replays without a second debit or mint, and a
//! conflicting binding or an insufficient balance writes nothing. Platform is a test double of
//! the existing Store port (`StoreTransport`).
#![allow(clippy::expect_used)]

use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{CommandStatus, MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
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

#[derive(Default)]
struct Ledger {
    balance: u64,
    world: String,
    purchases: usize,
    debits: Vec<u64>,
    intents: BTreeMap<String, serde_json::Value>,
    receipts: usize,
}

/// The Platform side of the existing Store port: balance, debit, order and delivery intent.
struct FakePlatform(Arc<SyncMutex<Ledger>>);

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
                    platform_answer(&request, Some((&order, &operation)), ledger.balance, body)
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

fn uuid(bytes: &[u8; 16]) -> String {
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
    super::kill_reward_e2e::scenario_thread::run("store-e2e", scenario)
}

/// One connection carrying `commands` from the session's next command id.
async fn serve(
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

fn purchase_request(seed: u8, offer: &str, price: u64) -> TestResult<(u32, Vec<u8>)> {
    let request = wire::StorePurchaseRequest {
        purchase_request_id: id(seed),
        offer_key: offer.into(),
        catalog_revision: "1f6ee35b3b46aa4e5e1a36d7eb0e0b76ce08fa78907a91e3aac5ad22bb11050d".into(),
        price,
        currency: "TIBIA_COIN".into(),
    };
    Ok((
        wire::COMMAND_TYPE_STORE_PURCHASE,
        wire::encode_purchase_request(&request).map_err(debug)?,
    ))
}

async fn purchase(
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

#[allow(clippy::too_many_lines)]
fn scenario() -> TestResult {
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

        let ledger = Arc::new(SyncMutex::new(Ledger {
            balance: 60,
            world: uuid(admitted.world_id.as_bytes()),
            ..Ledger::default()
        }));
        owner.store = Some(Arc::new(tokio::sync::Mutex::new(StoreClient::new(Box::new(
            FakePlatform(ledger.clone()),
        ), "oteryn-game/store-consumer".into()))));

        // Without the selected capability the command is no Store command.
        let values = serve(&owner, &mut admitted, &[page_request()?]).await?;
        assert_eq!(values, [(CommandStatus::Rejected, Vec::new())]);
        // STORE_V1 is registered but not offered (no production Platform Store client is
        // composed), so the test offers it explicitly.
        assert!(
            !owner
                .offered_capabilities()
                .iter()
                .any(|offer| offer.id == wire::CAPABILITY_STORE_V1)
        );
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &[OfferedCapability {
                id: wire::CAPABILITY_STORE_V1,
                requires: &[],
            }],
            &[wire::CAPABILITY_STORE_V1],
        )
        .ok_or("selection")?;

        // The catalogue page: 23 admitted exercise weapons and the Platform balance.
        let values = serve(&owner, &mut admitted, &[page_request()?]).await?;
        assert_eq!(values.len(), 1, "{values:?}");
        assert_eq!(values[0].0, CommandStatus::Accepted, "{values:?}");
        let page = wire::decode_catalogue_page(&values[0].1).map_err(debug)?;
        assert_eq!(page.catalog_revision, "1f6ee35b3b46aa4e5e1a36d7eb0e0b76ce08fa78907a91e3aac5ad22bb11050d");
        assert_eq!((page.offers.len(), page.total_coins, page.transferable_coins), (23, 60, 60));
        let sword = page
            .offers
            .iter()
            .find(|offer| offer.offer_key == SWORD)
            .ok_or("sword offer")?;
        assert_eq!((sword.price, sword.title.as_str()), (25, "Exercise Sword"));

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

        // A retry of the same request id replays the saved answer: no second debit or mint.
        let replay = purchase(&owner, &mut admitted, purchase_request(62, SWORD, 25)?).await?;
        assert_eq!(replay, accepted);
        assert_eq!(swords(&harness).await?, 1);
        assert_eq!(count(&harness, "SELECT count(*) FROM store_delivery_receipts").await?, 1);
        assert_eq!(ledger.lock().expect("ledger").purchases, 1);

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
