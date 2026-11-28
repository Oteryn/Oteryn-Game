#![allow(clippy::expect_used)]

use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use super::fixture::{debug, id};

use super::TestResult;

use crate::foundation::{CommandStatus, MessageType, decode_wire_envelope};

use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, IDLE_LIVENESS,
    serve_admitted,
};

use crate::gameplay_transport::ComposedFreshAdmission;

use crate::native_admission_source::descriptor::Operation;

use crate::native_admission_source::store::{StoreClient, StoreTransport, TransportFailure};

use oteryn_protocol_oteryn::store as wire;

use std::collections::BTreeMap;

use std::sync::{Arc, Mutex as SyncMutex};

use sha2::{Digest, Sha256};

#[derive(Default)]
pub(super) struct Ledger {
    pub(super) balance: u64,
    pub(super) world: String,
    pub(super) purchases: usize,
    pub(super) debits: Vec<u64>,
    pub(super) intents: BTreeMap<String, serde_json::Value>,
    pub(super) receipts: usize,
    pub(super) fail_receipt: bool,
    pub(super) recovery_generation: Option<String>,
    pub(super) advertise_claims: bool,
    pub(super) advertise_deliveries: bool,
    pub(super) claims: usize,
    pub(super) reopened: bool,
    pub(super) restore_conflicts: Vec<String>,
    pub(super) claim_reads: usize,
    pub(super) slow_claim_reads: Option<Arc<SlowClaimReads>>,
    pub(super) receipt_ready: Arc<tokio::sync::Notify>,
    pub(super) malformed: bool,
    pub(super) entitlement_override: Option<String>,
    pub(super) unit_kind_override: Option<&'static str>,
    pub(super) enforce_active_import: bool,
    pub(super) fail_before_debit: bool,
    pub(super) answers: BTreeMap<String, (String, serde_json::Value)>,
}

#[derive(Default)]
pub(super) struct SlowClaimReads {
    pub(super) first_started: tokio::sync::Notify,
    pub(super) release_first: tokio::sync::Notify,
    pub(super) second_started: tokio::sync::Notify,
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
                if ledger.fail_before_debit {
                    return Box::pin(async { Err(TransportFailure::Unavailable) });
                }
                ledger.purchases += 1;
                let request_id = request["purchase_request_id"].as_str().expect("purchase id").to_owned();
                if let Some((original, answer)) = ledger.answers.get(&request_id) {
                    let result = if original == body { Ok(platform_bytes(answer)) }
                        else { Err(TransportFailure::Conflict) };
                    return Box::pin(async move { result });
                }
                let imported_kind = if ledger.enforce_active_import {
                    let bytes = include_bytes!(
                        "../../../../../../../rulesets/economy/store/platform-catalogue.json"
                    );
                    let imported: serde_json::Value = serde_json::from_slice(bytes).expect("Platform import");
                    let row = imported["offers"].as_array().expect("imported offers").iter()
                        .find(|row| row["offer_key"] == request["offer_key"]);
                    let kind = row.filter(|row| {
                        request["catalog_revision"] == sha256(bytes)
                            && request["currency"] == row["currency"]
                            && request["unit_price"] == row["unit_price"]
                    }).and_then(|row| row["units"][0]["unit_kind"].as_str());
                    let Some(kind) = kind else {
                        let mut answer = platform_answer(&request, None, ledger.balance, body);
                        answer["result"] = "STALE_QUOTE".into();
                        return Box::pin(async move { Ok(platform_bytes(&answer)) });
                    };
                    Some(kind.to_owned())
                } else {
                    None
                };
                let price = request["unit_price"].as_str().expect("price string").parse::<u64>().expect("price") * request["quantity"].as_u64().expect("quantity");
                if price > ledger.balance {
                    platform_answer(&request, None, ledger.balance, body)
                } else {
                    ledger.balance -= price;
                    ledger.debits.push(price);
                    let n = u8::try_from(ledger.debits.len()).expect("few orders");
                    let operation = uuid(&id(100 + n));
                    let order = uuid(&id(120 + n));
                    let mut intent = platform_intent(&request, &operation, &order);
                    let mut answer = platform_answer(&request, Some((&order, &operation)), ledger.balance, body);
                    if let Some(kind) = imported_kind.as_deref().or(ledger.unit_kind_override) {
                        intent["unit_kind"] = kind.into();
                        answer["units"][0]["unit_kind"] = kind.into();
                    }
                    if let Some(key) = &ledger.entitlement_override {
                        intent["entitlement_id"] = key.as_str().into();
                        answer["units"][0]["entitlement_id"] = key.as_str().into();
                    }
                    ledger.intents.insert(operation.clone(), intent);
                    ledger.answers.insert(request_id, (body.to_owned(), answer.clone()));
                    if ledger.malformed {
                        let unit = answer["units"][0].clone();
                        answer["units"].as_array_mut().expect("units").push(unit);
                    }
                    answer
                }
            }
            Operation::ListPendingStoreOperationsV1 | Operation::ListStoreOperationStatesV1 => {
                let entries = ledger.intents.values().filter(|intent| {
                    (intent["operation_kind"] == "CLAIM" && ledger.claims == 0
                        || intent["operation_kind"] == "DELIVER" && ledger.advertise_deliveries && ledger.receipts == 0)
                        && (intent["source_revision"].as_str().and_then(|value| value.parse::<u64>().ok()), intent["operation_id"].as_str())
                            > (request["after_source_revision"].as_str().and_then(|value| value.parse::<u64>().ok()), request["after_operation_id"].as_str())
                }).map(|intent| serde_json::json!({
                    "operation_kind":intent["operation_kind"], "operation_id":intent["operation_id"],
                    "source_revision":intent["source_revision"]
                })).collect::<Vec<_>>();
                if operation == Operation::ListStoreOperationStatesV1 {
                    let entries: Vec<_> = entries.into_iter().map(|mut entry| {
                        entry["state"] = if ledger.reopened { "REOPENED" } else { "PENDING" }.into();
                        entry["receipt_digest"] = serde_json::Value::Null;
                        entry["recovery_generation"] = serde_json::Value::Null;
                        entry
                    }).collect();
                    serde_json::json!({"contract_version":1,"operations":entries})
                } else {
                    serde_json::json!({"contract_version":1,"pending":entries})
                }
            }
            Operation::ReadStoreOperationIntentV1 => {
                let operation = request["operation_id"].as_str().expect("operation");
                let intent = ledger.intents[operation].clone();
                if intent["operation_kind"] == "CLAIM"
                    && let Some(slow) = ledger.slow_claim_reads.clone()
                {
                    ledger.claim_reads += 1;
                    let first = ledger.claim_reads == 1;
                    return Box::pin(async move {
                        if first {
                            slow.first_started.notify_one();
                            slow.release_first.notified().await;
                            Ok(platform_bytes(&intent))
                        } else {
                            slow.second_started.notify_one();
                            std::future::pending().await
                        }
                    });
                }
                intent
            }
            Operation::PublishStoreDeliveryReceiptV1 => {
                assert_eq!(request["binding_digest"], sha256(&platform_bytes(&ledger.intents[request["operation_id"].as_str().expect("operation")])));
                if ledger.fail_receipt {
                    return Box::pin(async { Err(TransportFailure::Unavailable) });
                }
                if let Some(generation) = &ledger.recovery_generation
                    && request["recovery_generation"] != generation.as_str()
                {
                    return Box::pin(async { Err(TransportFailure::Conflict) });
                }
                if request["outcome"] == "REJECTED" {
                    assert!(ledger.reopened);
                    assert_eq!(request["result_code"], "STORE_OPERATION_CONFLICT");
                    ledger.restore_conflicts.push(request["operation_kind"].as_str().expect("kind").into());
                    ledger.receipt_ready.notify_one();
                    return Box::pin(async { Ok(br#"{"contract_version":1,"result":"accepted"}"#.to_vec()) });
                }
                assert_eq!(request["result_code"], "STORE_OK");
                if request["operation_kind"] == "CLAIM" {
                    assert_eq!(request["minted_quantity"], 1);
                    ledger.claims += 1;
                    ledger.receipt_ready.notify_one();
                } else {
                    ledger.receipts += 1;
                    if ledger.advertise_claims {
                        let mut claim = ledger.intents[request["operation_id"].as_str().expect("delivery")].clone();
                        let id = uuid(&id(112 + u8::try_from(ledger.receipts).expect("few receipts")));
                        claim["operation_id"] = id.clone().into();
                        claim["operation_kind"] = "CLAIM".into();
                        claim["source_revision"] = "2".into();
                        claim["delivery_operation_id"] = request["operation_id"].clone();
                        ledger.intents.insert(id, claim);
                    } else {
                        ledger.receipt_ready.notify_one();
                    }
                }
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

/// Probe the buyer lock taken by the answer writer's history foreign key.
pub(super) async fn assert_store_retry_lock_order<T: std::fmt::Debug>(
    pool: &sqlx::PgPool,
    purchase: [u8; 16],
    character: [u8; 16],
    retry: impl std::future::Future<Output = T>,
) -> TestResult<T> {
    let mut writer = pool.begin().await?;
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()").fetch_one(&mut *writer).await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:store-purchase:'||encode($1,'hex'),0))")
        .bind(purchase.as_slice()).execute(&mut *writer).await?;
    tokio::pin!(retry);
    tokio::select! {
        result = &mut retry => return Err(format!("retry skipped purchase serialization: {result:?}").into()),
        result = wait_for_advisory_waiter(pool, pid) => result?,
    }
    let character_available = sqlx::query(
        "SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR KEY SHARE NOWAIT",
    ).bind(character.as_slice()).fetch_one(&mut *writer).await.is_ok();
    writer.rollback().await?;
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), &mut retry).await?;
    assert!(character_available, "Store reservation locked the buyer ahead of the purchase");
    Ok(result)
}

/// Wait until an actual PostgreSQL writer queues behind the transaction's advisory lock.
pub(super) async fn wait_for_advisory_waiter(pool: &sqlx::PgPool, holder: i32) -> TestResult {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let queued: bool = sqlx::query_scalar(
                "SELECT EXISTS (
                   SELECT 1 FROM pg_locks blocked
                   JOIN pg_locks held USING(locktype,database,classid,objid,objsubid)
                   WHERE held.pid=$1 AND held.locktype='advisory' AND held.granted AND NOT blocked.granted
                 )",
            )
            .bind(holder)
            .fetch_one(pool)
            .await?;
            if queued {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.map_err(|_| "Store writer did not reach the advisory barrier")?
}

pub(super) fn purchase_request(seed: u8, offer: &str, price: u64) -> TestResult<(u32, Vec<u8>)> {
    let request = wire::StorePurchaseRequest {
        purchase_request_id: id(seed),
        offer_key: offer.into(),
        catalog_revision: crate::gameplay_transport::store::active_catalogue_revision().into(),
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
        answer["units"] = serde_json::json!([{"unit_kind":platform_intent(request, operation, order)["unit_kind"],"entitlement_id":operation,"delivery_operation_id":operation}]);
    }
    answer
}

pub(super) fn platform_intent(request: &serde_json::Value, operation: &str, order: &str) -> serde_json::Value {
    let revision = request["catalog_revision"].as_str().expect("catalogue revision");
    let key = request["offer_key"].as_str().expect("offer key");
    let unit_kind = crate::gameplay_transport::store::platform_unit_kind(revision, key).unwrap_or("ITEM");
    let hunting_offer = crate::durability::store::StoreHuntingOffer::from_key(key).is_some();
    let grant_key = if hunting_offer { key } else { "item:test" };
    let product_id = if hunting_offer { "oteryn.store.character_service" } else { "oteryn.store.item_grant" };
    serde_json::json!({"contract_version":1,"operation_kind":"DELIVER","issuer_authority":"OTERYN_PLATFORM_STORE_AUTHORITY","source_revision":"1","operation_id":operation,"order_id":order,"entitlement_id":operation,"account_id":request["account_id"],"product_id":product_id,"product_version":"1","unit_kind":unit_kind,"catalog_revision":request["catalog_revision"],"offer_key":request["offer_key"],"grant_key":grant_key,"grant_revision":"1","quantity":1,"issued_at_source":"1","audience":"OTERYN_GAME_STORE"})
}

pub(super) fn platform_bytes(v: &serde_json::Value) -> Vec<u8> {
    let fields = if v.get("pending").is_some() {
        "contract_version pending"
    } else if v.get("operations").is_some() {
        "contract_version operations"
    } else if v.get("state").is_some() {
        "operation_kind operation_id source_revision state receipt_digest recovery_generation"
    } else if v["operation_kind"] == "CLAIM" && v.get("issuer_authority").is_some() {
        "contract_version operation_kind issuer_authority source_revision operation_id delivery_operation_id entitlement_id account_id product_id product_version unit_kind grant_key grant_revision quantity issued_at_source audience"
    } else if v.get("operation_kind").is_some() && v.get("issuer_authority").is_none() {
        "operation_kind operation_id source_revision"
    } else if v.get("operation_kind").is_some() {
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
