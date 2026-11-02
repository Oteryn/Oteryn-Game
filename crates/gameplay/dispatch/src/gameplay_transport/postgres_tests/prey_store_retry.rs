//! A charged Prey operation must finish after the offer is disabled or its quote changes.
//! The fixture begins at the durable crash boundary: its originally enabled quote passed,
//! the reservation exists, and Platform has one debit whose reply may or may not be retained.
use super::*;
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
use crate::native_admission_source::descriptor::Operation;
use crate::native_admission_source::store::{StoreClient, StoreTransport, TransportFailure};
use oteryn_gameplay_economy::store as rules;
use oteryn_protocol_oteryn::{CommandStatus, store as wire};
use std::sync::{Arc, Mutex as SyncMutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const OFFER: &str = "store:prey-list-expansion";
const OLD_REVISION: &str = "prey-retry-test-original-r0";

#[test]
fn prey_diff_store_accepted_purchase_resumes_after_catalogue_change_without_another_debit() -> TestResult {
    run_case(true)
}

#[test]
fn prey_diff_store_lost_platform_answer_reconciles_original_id_after_catalogue_change() -> TestResult {
    run_case(false)
}

fn run_case(answered: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("PREY_DIFF POSTGRES_CASE_COMPLETED route_less");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20)
        .spawn(move || scenario(answered).map_err(|error| error.to_string()))?
        .join().map_err(|_| "Prey Store retry case panicked")??;
    eprintln!("PREY_DIFF POSTGRES_CASE_COMPLETED");
    Ok(())
}

struct PlatformLedger {
    original: serde_json::Value,
    intent: serde_json::Value,
    answer: serde_json::Value,
    purchases: usize,
    debits: usize,
    receipts: Vec<serde_json::Value>,
}

struct ExistingDebit(Arc<SyncMutex<PlatformLedger>>);
impl StoreTransport for ExistingDebit {
    fn exchange<'a>(
        &'a mut self, operation: Operation, body: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, TransportFailure>> + Send + 'a>> {
        let mut ledger = self.0.lock().expect("Platform ledger");
        let request: serde_json::Value = serde_json::from_str(body).expect("Store request JSON");
        let answer = match operation {
            Operation::PurchaseStoreOfferV1 => {
                ledger.purchases += 1;
                // Platform's existing idempotency contract: the exact old id/quote replays
                // the accepted order. A request that could start a second debit fails here.
                for key in ["purchase_request_id", "account_id", "character_id", "offer_key", "catalog_revision", "price", "currency"] {
                    assert_eq!(request[key], ledger.original[key], "retry changed {key}");
                }
                assert_eq!(ledger.debits, 1);
                ledger.answer.clone()
            }
            Operation::ReadStoreDeliveryIntentV1 => {
                assert_eq!(request["operation_id"], ledger.intent["operation_id"]);
                serde_json::json!({"contract_version":1,"intent":ledger.intent})
            }
            Operation::PublishStoreReceiptV1 => {
                ledger.receipts.push(request);
                serde_json::json!({"contract_version":1,"result":"accepted"})
            }
            _ => return Box::pin(async { Err(TransportFailure::NotFound) }),
        };
        Box::pin(async move { Ok(answer.to_string().into_bytes()) })
    }
}

fn scenario(answered: bool) -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "prey_store_retry", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (generation, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let deps = Dependencies::with_content(channel_pin, room, crate::spell::cast::v1_spell_book().map_err(debug)?)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = generation.active();
        deps.initialize(&owner).await;
        owner.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
        let current = crate::gameplay_transport::character_gameplay_fence::current(
            &owner, deps.admitted.game_session_id, false,
        ).await.ok_or("current fence")?;
        let account: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(current.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        let account: [u8; 16] = account.try_into().map_err(|_| "account UUID length")?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(current.item_fence());
        admitted.controller.as_mut().ok_or("controller binding")?.account_id = account;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &[OfferedCapability { id: wire::CAPABILITY_STORE_V1, requires: &[] }],
            &[wire::CAPABILITY_STORE_V1],
        ).ok_or("Store capability")?;
        let request = wire::StorePurchaseRequest {
            purchase_request_id: id(230), offer_key: OFFER.into(), catalog_revision: OLD_REVISION.into(),
            price: 37, currency: "TIBIA_COIN".into(), // fixture quote, never a proposed production price
        };
        let old_binding = rules::PurchaseStoreOffer {
            purchase_request_id: rules::PurchaseRequestId::from_bytes(request.purchase_request_id).map_err(debug)?,
            account_id: crate::domain::AccountId::from_bytes(account).map_err(debug)?,
            character_id: current.character_id,
            offer_key: request.offer_key.clone(), catalog_revision: request.catalog_revision.clone(),
            price: request.price, currency: rules::CurrencyKind::TibiaCoin,
        };
        let journal = crate::durability::store::PurchaseRequest {
            purchase_request_id: request.purchase_request_id, account_id: account,
            character_id: *current.character_id.as_bytes(), binding: old_binding.binding_digest().to_vec(),
        };
        owner.root.record_purchase_request(journal.clone()).await.map_err(debug)?;
        assert!(owner.root.reserve_prey_expansion_purchase(
            owner.character, owner.holder, current, journal.clone(),
        ).await.map_err(debug)?);
        let operation = uuid(&id(231));
        let order = uuid(&id(232));
        if answered {
            let retained = serde_json::to_vec(&serde_json::json!({
                "result":"ACCEPTED", "order_id":order, "operation_ids":[operation],
            }))?;
            owner.root.record_store_answer(journal, retained).await.map_err(debug)?;
        }
        let ledger = Arc::new(SyncMutex::new(PlatformLedger {
            original: serde_json::json!({
                "purchase_request_id":uuid(&request.purchase_request_id), "account_id":uuid(&account),
                "character_id":uuid(current.character_id.as_bytes()), "offer_key":OFFER,
                "catalog_revision":OLD_REVISION, "price":request.price, "currency":"TIBIA_COIN",
            }),
            intent: serde_json::json!({
                "operation_id":operation, "account_id":uuid(&account),
                "character_id":uuid(current.character_id.as_bytes()), "world_id":uuid(admitted.world_id.as_bytes()),
                "fulfilment_unit_id":"prey-retry-original-unit", "catalog_revision":OLD_REVISION, "offer_key":OFFER,
            }),
            answer: serde_json::json!({"contract_version":1,"result":"ACCEPTED","order_id":order,"operation_ids":[operation]}),
            purchases: 0, debits: 1, receipts: Vec::new(),
        }));
        owner.store = Some(Arc::new(tokio::sync::Mutex::new(StoreClient::new(Box::new(ExistingDebit(ledger.clone()))))));

        // A changed binding of the original request id cannot exploit the resume path.
        for field in 0..3 {
            let mut forged = request.clone();
            match field {
                0 => forged.price += 1,
                1 => forged.catalog_revision.push('x'),
                _ => forged.currency = "TRANSFERABLE_TIBIA_COIN".into(),
            }
            assert_eq!(send(&owner, &mut admitted, &forged).await?.0, CommandStatus::Rejected);
        }
        assert!(!owner.root.prey_list_expansion_owned(owner.character, current.character_id).await.map_err(debug)?);
        assert_eq!(ledger.lock().expect("ledger").purchases, 0);

        // A new request still requires the current enabled quote and writes no journal.
        let mut fresh = request.clone();
        fresh.purchase_request_id = id(233);
        let (status, bytes) = send(&owner, &mut admitted, &fresh).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let fresh_result = wire::decode_purchase_result(&bytes).map_err(debug)?;
        assert!(matches!(fresh_result.code, wire::StorePurchaseCode::OfferUnavailable | wire::StorePurchaseCode::StaleQuote));
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM store_purchase_requests").fetch_one(&h.pool).await?, 1);
        assert_eq!(ledger.lock().expect("ledger").purchases, 0);

        let (status, bytes) = send(&owner, &mut admitted, &request).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let result = wire::decode_purchase_result(&bytes).map_err(debug)?;
        assert_eq!(result.code, wire::StorePurchaseCode::Accepted);
        assert_eq!(result.order_id, Some(id(232)));
        assert_eq!(result.operation_id, Some(id(231)));
        assert!(owner.root.prey_list_expansion_owned(owner.character, current.character_id).await.map_err(debug)?);
        let replay = send(&owner, &mut admitted, &request).await?;
        assert_eq!(replay, (CommandStatus::Accepted, bytes));
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_prey_expansions").fetch_one(&h.pool).await?, 1);
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM store_delivery_receipts WHERE entitlement_key='prey-list-expansion' AND item_instance_id IS NULL").fetch_one(&h.pool).await?, 1);
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM store_receipt_outbox").fetch_one(&h.pool).await?, 1);
        {
            let ledger = ledger.lock().expect("ledger");
            assert_eq!(ledger.purchases, usize::from(!answered));
            assert_eq!(ledger.debits, 1);
            assert_eq!(ledger.receipts.len(), 2);
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn send(
    owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    request: &wire::StorePurchaseRequest,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let payload = wire::encode_purchase_request(request).map_err(debug)?;
    let envelope = oteryn_protocol_oteryn::encode_client_command(1, &oteryn_protocol_oteryn::ClientCommandValue {
        command_id: admitted.continuity.next_command_id,
        command_type: wire::COMMAND_TYPE_STORE_PURCHASE, payload: &payload,
    }).map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client.write_all(&u32::try_from(envelope.len())?.to_be_bytes()).await?;
    client.write_all(&envelope).await?;
    client.shutdown().await?;
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current) | ConnectionEnd::AdmittedThenClosed(current, _) = end {
        *admitted = current;
    }
    let mut cursor = 0;
    let mut results = Vec::new();
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let envelope = decode_wire_envelope(&output[cursor + 4..cursor + 4 + length]).map_err(debug)?;
        cursor += length + 4;
        if envelope.message_type() == MessageType::CommandResult {
            let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
            results.push((result.status, result.payload.to_vec()));
        }
    }
    assert_eq!(results.len(), 1);
    Ok(results.remove(0))
}

fn uuid(bytes: &[u8; 16]) -> String {
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}
