// Assertions inspect native Inbox custody and durable state, not an in-memory substitute.
#![allow(clippy::expect_used)] // Test fixture invariants use the neighbouring Store harness style.
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use super::store_fixtures::{platform_balance, platform_answer, platform_intent, platform_bytes};

use super::fixture::{Harness,debug,id};

use super::TestResult;

use crate::foundation::{CommandStatus,MessageType,decode_wire_envelope};

use crate::gameplay_transport::connection::{AdmittedSession,ConnectionEnd,IDLE_LIVENESS,serve_admitted};

use crate::gameplay_transport::ComposedFreshAdmission;

use crate::native_admission_source::descriptor::Operation;

use crate::native_admission_source::store::{StoreClient,StoreTransport,TransportFailure};

use oteryn_protocol_oteryn::store as wire;

use std::collections::BTreeMap;

use std::sync::{Arc,Mutex as SyncMutex};



pub(super) const LAMP:&str="oteryn:item.tibia.i29432";

#[derive(Default)]
pub(super) struct Ledger {
    pub(super) balance: u64,
    pub(super) world: String,
    pub(super) debits: usize,
    pub(super) calls: usize,
    pub(super) intents:BTreeMap<String,serde_json::Value>,
    pub(super) answers:BTreeMap<String,serde_json::Value>,
    pub(super) lose_next_answer:bool,
    pub(super) pending_claim: Option<String>,
    pub(super) claim_receipt: Option<serde_json::Value>,
    pub(super) claim_ready: Arc<tokio::sync::Notify>,
}

pub(super) struct FakePlatform(pub(super) Arc<SyncMutex<Ledger>>);

impl StoreTransport for FakePlatform {
    fn exchange<'a>(&'a mut self,operation:Operation,body:&'a str)->std::pin::Pin<Box<dyn std::future::Future<Output=Result<Vec<u8>,TransportFailure>>+Send+'a>> {
        let mut ledger=self.0.lock().expect("ledger");
        let request:serde_json::Value=serde_json::from_str(body).expect("request");
        if operation == Operation::PurchaseStoreOfferV1 { assert_eq!(request["world_id"], ledger.world); }
        let answer=match operation {
            Operation::ReadStoreBalanceV1=>platform_balance(&request, ledger.balance),
            Operation::PurchaseStoreOfferV1=>{
                ledger.calls+=1;
                let key=request["purchase_request_id"].as_str().expect("request id").to_owned();
                if let Some(answer)=ledger.answers.get(&key) {answer.clone()} else {
                    let price=request["unit_price"].as_str().expect("price string").parse::<u64>().expect("price") * request["quantity"].as_u64().expect("quantity");
                    let answer=if price>ledger.balance {platform_answer(&request, None, ledger.balance, body)} else {
                        ledger.balance-=price;ledger.debits+=1;
                        let n=u8::try_from(ledger.debits).expect("bounded orders");
                        let operation=uuid(&id(100+n));
                        let order=uuid(&id(130+n));
                        let intent=platform_intent(&request,&operation,&order);
                        ledger.intents.insert(operation.clone(),intent);
                        platform_answer(&request,Some((&order,&operation)),ledger.balance,body)
                    };
                    ledger.answers.insert(key,answer.clone());
                    if std::mem::take(&mut ledger.lose_next_answer) {return Box::pin(async{Err(TransportFailure::Unavailable)})}
                    answer
                }
            }
            Operation::ReadStoreOperationIntentV1=>{
                let operation=request["operation_id"].as_str().expect("operation");
                ledger.intents[operation].clone()
            }
            Operation::ListPendingStoreOperationsV1 => {
                let entries: Vec<_> = ledger.pending_claim.iter()
                    .filter(|_| ledger.claim_receipt.is_none())
                    .map(|id| serde_json::json!({
                        "operation_kind": "CLAIM", "operation_id": id, "source_revision": "2",
                    })).collect();
                serde_json::json!({"contract_version":1,"pending":entries})
            }
            Operation::PublishStoreDeliveryReceiptV1 => {
                if request["operation_kind"] == "CLAIM" {
                    if let Some(receipt) = &ledger.claim_receipt {
                        assert_eq!(receipt, &request);
                    } else {
                        ledger.claim_receipt = Some(request);
                        ledger.claim_ready.notify_one();
                    }
                }
                serde_json::json!({"contract_version":1,"result":"accepted"})
            }
            _=>return Box::pin(async{Err(TransportFailure::NotFound)}),
        };
        Box::pin(async move{Ok(platform_bytes(&answer))})
    }
}

pub(super) fn uuid(bytes:&[u8;16])->String {
    let hex:String=bytes.iter().map(|b|format!("{b:02x}")).collect();
    format!("{}-{}-{}-{}-{}",&hex[..8],&hex[8..12],&hex[12..16],&hex[16..20],&hex[20..])
}

pub(super) async fn serve(owner:&ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,command:(u32,Vec<u8>))->TestResult<(CommandStatus,Vec<u8>)> {
    let bytes=oteryn_protocol_oteryn::encode_client_command(1,&oteryn_protocol_oteryn::ClientCommandValue{command_id:admitted.continuity.next_command_id,command_type:command.0,payload:&command.1}).map_err(debug)?;
    let (mut server,mut client)=tokio::io::duplex(1<<20);
    client.write_all(&u32::try_from(bytes.len())?.to_be_bytes()).await?;client.write_all(&bytes).await?;client.shutdown().await?;
    let end=serve_admitted(&mut server,*admitted,owner,IDLE_LIVENESS).await;drop(server);
    if let ConnectionEnd::AdmittedThenDisconnected(current)|ConnectionEnd::AdmittedThenClosed(current,_)=end {*admitted=current;}
    let mut output=Vec::new();client.read_to_end(&mut output).await?;
    let mut cursor=0;
    while cursor<output.len() {
        let length=u32::from_be_bytes(output[cursor..cursor+4].try_into()?)as usize;
        let envelope=decode_wire_envelope(&output[cursor+4..cursor+4+length]).map_err(debug)?;cursor+=length+4;
        if envelope.message_type()==MessageType::CommandResult {
            let value=oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
            return Ok((value.status,value.payload.to_vec()));
        }
    }
    Err("missing command result".into())
}

pub(super) fn command(seed:u8,offer:u16,price:u64,custom:Option<wire::StoreHirelingCustomization>)->TestResult<(u32,Vec<u8>)> {
    let request=wire::StorePurchaseRequest{purchase_request_id:id(seed),offer_key:format!("store:hireling:{offer}"),catalog_revision:crate::gameplay_transport::store::active_catalogue_revision().into(),price,currency:"TIBIA_COIN".into()};
    let bytes=if let Some(custom)=custom {wire::encode_hireling_purchase_request(&request,&custom)}else{wire::encode_purchase_request(&request)}.map_err(debug)?;
    Ok((wire::COMMAND_TYPE_STORE_PURCHASE,bytes))
}

pub(super) fn new_hire(name:&str)->wire::StoreHirelingCustomization {wire::StoreHirelingCustomization{hireling_id:None,name:Some(name.into()),sex:Some(1)}}

pub(super) async fn purchase(owner:&ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,command:(u32,Vec<u8>))->TestResult<wire::StorePurchaseResult> {
    let result=serve(owner,admitted,command).await?;assert_eq!(result.0,CommandStatus::Accepted,"{result:?}");
    wire::decode_purchase_result(&result.1).map_err(debug)
}

pub(super) async fn count(harness:&Harness,sql:&'static str)->TestResult<i64> {Ok(sqlx::query_scalar(sql).fetch_one(&harness.pool).await?)}

pub(super) struct HouseStoreProducer(Arc<SyncMutex<Ledger>>);

/// Compose the same test producer before the native capability negotiation and handoff.
pub(super) fn install_house_store(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    world: &[u8; 16],
) -> HouseStoreProducer {
    let ledger = Arc::new(SyncMutex::new(Ledger {
        balance: 10000, world: uuid(world), ..Ledger::default()
    }));
    owner.store = Some(Arc::new(tokio::sync::Mutex::new(StoreClient::new(
        Box::new(FakePlatform(ledger.clone())), "oteryn-game/store-consumer".into(),
    ))));
    HouseStoreProducer(ledger)
}

/// The lifecycle fixture supplies an actual committed House handoff and STEP.
/// Reuse the same Platform contract and real Store wire as the Channel cases;
/// no identity, entitlement or Inbox item is seeded for this scenario.
pub(super) async fn house_scenario(
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    producer: HouseStoreProducer,
    previous_generation: Option<u64>,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("House Store fence")?;
    let actor = admitted.runtime_actor.ok_or("House Store actor")?;
    assert!(admitted.continuity.selected_capabilities.contains(154));
    assert!(matches!(fence.runtime_scope, crate::foundation::RuntimeScopeRefV1::Instance { .. }));
    let house = {
        let runtime = owner.runtime.lock().await;
        let held = runtime.house_actor_state(actor, admitted.game_session_id).map_err(debug)?;
        assert_eq!(held.scope, fence.runtime_scope);
        assert_eq!(held.generation, fence.scope_ownership_generation);
        assert_eq!(held.lease.character_id().as_bytes(), fence.character_id.as_bytes());
        assert_eq!(held.lease.generation(), fence.character_lease_generation);
        held.house
    };
    let account: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&harness.pool).await?;
    assert_eq!(admitted.controller.ok_or("House Store controller")?.account_id.as_slice(), account.as_slice());
    assert_eq!(count(harness, "SELECT count(*) FROM game_house_hirelings").await?, 0);
    let ledger = producer.0;

    let absent = purchase(owner, &mut admitted, command(50, 1004, 250, None)?).await?;
    assert_eq!(absent.code, wire::StorePurchaseCode::OfferUnavailable);
    assert_eq!(ledger.lock().expect("ledger").debits, 0);
    let mint_command = admitted.continuity.next_command_id;
    let hired = purchase(owner, &mut admitted,
        command(51, 25440, 150, Some(new_hire("Alex Moon")))?).await?;
    assert_eq!(hired.code, wire::StorePurchaseCode::Accepted, "{hired:?}");
    let operation = hired.operation_id.ok_or("House Store hire operation")?;
    let receipt: Vec<u8> = sqlx::query_scalar(
        "SELECT receipt FROM store_delivery_receipts WHERE operation_id=encode($1,'hex')::uuid",
    ).bind(operation.as_slice()).fetch_one(&harness.pool).await?;
    let scope: (String, String, i16, Vec<u8>) = sqlx::query_as(
        "SELECT s.house_key,s.scope_ownership_generation::text,s.authority_kind,a.envelope \
         FROM store_delivery_receipts d \
         JOIN game_reward_claim_inbox_mint_receipts r ON r.transaction_id=d.native_transaction_id \
         JOIN game_hireling_inbox_scope_receipts s USING(transaction_id) \
         JOIN game_item_audit_outbox a ON a.event_id=r.event_id AND a.transaction_id=r.transaction_id \
         WHERE d.operation_id=encode($1,'hex')::uuid",
    ).bind(operation.as_slice()).fetch_one(&harness.pool).await?;
    assert_eq!((&scope.0, &scope.1, scope.2), (&house, &fence.scope_ownership_generation.get().to_string(), 1));
    let (_, minted) = crate::durability::reward_claim_mint_audit::decode_reward_claim_mint_envelope(&scope.3)
        .map_err(debug)?;
    let cause = minted.source.ok_or("House Store native mint source")?;
    let claim = cause.reward_claim.ok_or("House Store offer binding")?;
    assert_eq!(claim.family, "StoreOffer");
    assert_eq!(claim.production_key, "store:hireling:25440");
    let source = cause.command_ref.ok_or("House Store native command")?;
    assert_eq!(source.game_session_id.as_slice(), admitted.game_session_id.as_bytes().as_slice());
    assert_eq!(source.command_id, mint_command);
    let inbox = minted.inbox.ok_or("House Store native Inbox audit")?;
    assert_eq!(inbox.house_key.as_deref(), Some(house.as_str()));
    assert_eq!(inbox.house_scope_ownership_generation, Some(fence.scope_ownership_generation.get()));
    assert_eq!(inbox.house_authority_kind, Some(1));
    assert_eq!(inbox.expected_session_generation, fence.connection_generation.get());
    assert_eq!(inbox.expected_character_lease_generation, fence.character_lease_generation);
    assert_eq!(inbox.character_id.as_slice(), fence.character_id.as_bytes().as_slice());
    assert_eq!(inbox.world_id.as_slice(), admitted.world_id.as_bytes().as_slice());
    assert_eq!(inbox.outputs.len(), 1);
    let output = inbox.outputs[0].after.as_ref().ok_or("House Store lamp output")?;
    assert_eq!(output.quantity, 1);
    assert_eq!(output.definition.as_ref().ok_or("House Store lamp definition")?.production_key, LAMP);
    let origin: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(r.channel_id) FROM game_reward_claim_inbox_mint_receipts r \
         JOIN store_delivery_receipts d ON d.native_transaction_id=r.transaction_id \
         WHERE d.operation_id=encode($1,'hex')::uuid",
    ).bind(operation.as_slice()).fetch_one(&harness.pool).await?;
    assert_eq!(origin.as_slice(), admitted.channel_id.as_bytes().as_slice(),
        "the origin Channel remains routing provenance beside the explicit current House");

    assert_eq!(purchase(owner, &mut admitted,
        command(51, 25440, 150, Some(new_hire("Alex Moon")))?).await?, hired);
    let replayed: Vec<u8> = sqlx::query_scalar(
        "SELECT receipt FROM store_delivery_receipts WHERE operation_id=encode($1,'hex')::uuid",
    ).bind(operation.as_slice()).fetch_one(&harness.pool).await?;
    assert_eq!(replayed, receipt, "the original Store delivery receipt is immutable");
    let delivered: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_house_hirelings h \
         JOIN game_item_instances i ON i.item_instance_id=h.lamp_item_instance_id \
         JOIN game_item_character_inbox_locations l ON l.item_instance_id=i.item_instance_id AND l.world_id=i.world_id \
         WHERE h.hireling_id=encode($1,'hex')::uuid AND NOT h.active AND h.owner_character_id=l.character_id \
           AND i.definition_production_key=$2 AND i.lifecycle=1",
    ).bind(operation.as_slice()).bind(LAMP).fetch_one(&harness.pool).await?;
    assert_eq!(delivered, 1);
    assert_eq!(count(harness, "SELECT count(*) FROM game_reward_claim_inbox_mint_receipts").await?, 1);
    assert_eq!(count(harness, "SELECT count(*) FROM game_hireling_inbox_scope_receipts").await?, 1);

    let skill = purchase(owner, &mut admitted, command(52, 1004, 250, None)?).await?;
    assert_eq!(skill.code, wire::StorePurchaseCode::Accepted, "{skill:?}");
    assert_eq!(purchase(owner, &mut admitted, command(52, 1004, 250, None)?).await?, skill);
    let duplicate = purchase(owner, &mut admitted, command(53, 1004, 250, None)?).await?;
    assert_eq!(duplicate.code, wire::StorePurchaseCode::OfferUnavailable);
    let unlocked: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_hireling_entitlements WHERE character_id=encode($1,'hex')::uuid AND entitlement_id=1004",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&harness.pool).await?;
    assert_eq!(unlocked, 1);
    assert_eq!(count(harness, "SELECT count(*) FROM store_delivery_receipts").await?, 2);
    assert_eq!(count(harness, "SELECT count(*) FROM game_store_hireling_effects").await?, 2);
    assert_eq!(count(harness, "SELECT count(*) FROM game_hireling_store_reservations WHERE state=1").await?, 0);
    let placement_generation = owner.root.store_publication_generation()
        .await.map_err(debug)?.ok_or("reconciled placement generation")?;
    if let Some(previous) = previous_generation {
        // Model the retained delivery row from the previous generation while
        // leaving the actual lamp placement and claim paths fully fenced.
        assert_ne!(placement_generation, previous.to_string());
        let mut tx = harness.pool.begin().await?;
        sqlx::query("ALTER TABLE store_delivery_receipts DISABLE TRIGGER store_delivery_receipts_immutable")
            .execute(&mut *tx).await?;
        sqlx::query("UPDATE store_delivery_receipts SET recovery_generation=$2::text::numeric,
                     receipt=convert_to(jsonb_set(convert_from(receipt,'UTF8')::jsonb, '{recovery_generation}',
                                                  to_jsonb($2::text))::text,'UTF8')
                     WHERE operation_id=encode($1,'hex')::uuid")
            .bind(operation.as_slice()).bind(previous.to_string()).execute(&mut *tx).await?;
        sqlx::query("ALTER TABLE store_delivery_receipts ENABLE TRIGGER store_delivery_receipts_immutable")
            .execute(&mut *tx).await?;
        tx.commit().await?;
    }
    let claim_id = uuid(&id(141));
    let ready = {
        let mut held = ledger.lock().expect("ledger");
        assert_eq!((held.debits, held.balance), (2, 9600));
        let delivery = uuid(&operation);
        let mut claim = held.intents[&delivery].clone();
        assert_eq!(claim["unit_kind"], "ITEM");
        claim["operation_id"] = claim_id.clone().into();
        claim["operation_kind"] = "CLAIM".into();
        claim["source_revision"] = "2".into();
        claim["delivery_operation_id"] = delivery.into();
        held.intents.insert(claim_id.clone(), claim);
        held.pending_claim = Some(claim_id.clone());
        held.claim_ready.clone()
    };
    use oteryn_protocol_oteryn::world_object as use_wire;
    let lamp_index = {
        let runtime = owner.runtime.lock().await;
        let index = crate::gameplay_transport::item_ref_admission::pinned_index(
            &runtime, owner.active_generation,
        ).ok_or("lamp definition index")?;
        let revision = index.revision(LAMP).ok_or("lamp revision")?;
        index.definition_ref("Item", LAMP, revision).ok_or("lamp wire definition")?
    };
    let query = (wire::COMMAND_TYPE_STORE_INBOX_CLAIM, vec![16, 2]);
    let (status, inbox) = serve(owner, &mut admitted, query.clone()).await?;
    assert_eq!(status, CommandStatus::Accepted, "opening the House Inbox must disclose its claim restrictions");
    assert!(inbox.ends_with(&[32, 1]), "ordinary Store claims must tell the player to leave the House");
    let lamp_ref = inbox.get(inbox.len().saturating_sub(8)..inbox.len().saturating_sub(2))
        .filter(|field| field.starts_with(&[26, 4]))
        .ok_or("the purchased lamp must have a native House USE action")?;
    let projected_index = std::num::NonZeroU32::new(u32::from_be_bytes(lamp_ref[2..].try_into()?))
        .ok_or("projected lamp definition")?;
    assert_eq!(projected_index, lamp_index);
    let use_lamp = use_wire::encode_item_use_intent(&use_wire::UseIntent {
        target: use_wire::UseTarget::ItemByDefinition(projected_index),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    }).map_err(debug)?;
    let placed = serve(owner, &mut admitted, (use_wire::COMMAND_TYPE_USE_INTENT, use_lamp)).await?;
    assert_eq!(placed.0, CommandStatus::Accepted);
    assert_eq!(use_wire::decode_use_result(&placed.1).map_err(debug)?, use_wire::UseDisposition::Committed);
    assert_eq!(count(harness, "SELECT count(*) FROM game_hireling_lamp_transitions WHERE kind=1").await?, 1);

    let client = StoreClient::new(
        Box::new(FakePlatform(ledger.clone())), "oteryn-game/store-consumer".into(),
    );
    let worker = crate::gameplay_transport::run_store_receipt_outbox(owner.root, owner.character, client);
    tokio::select! {
        () = worker => return Err("Store worker exited".into()),
        published = tokio::time::timeout(std::time::Duration::from_secs(3), ready.notified()) =>
            published.map_err(|_| "placed Store lamp never published its Platform claim")?,
    }
    {
        let held = ledger.lock().expect("ledger");
        let claim = held.claim_receipt.as_ref().ok_or("lamp claim receipt")?;
        assert_eq!(claim["operation_id"], claim_id);
        assert_eq!(claim["outcome"], "COMMITTED");
        assert_eq!(claim["minted_quantity"], 1);
        assert_eq!(claim["recovery_generation"], placement_generation, "first lamp CLAIM must use the placement generation");
        assert_eq!(held.debits, 2);
    }
    if previous_generation.is_some() {
        let intent = wire::encode_inbox_claim_request(&wire::StoreInboxClaimRequest {
            fulfilment_unit_id: uuid(&operation),
        }).map_err(debug)?;
        let replay = serve(owner, &mut admitted, (wire::COMMAND_TYPE_STORE_INBOX_CLAIM, intent)).await?;
        assert_eq!(replay.0, CommandStatus::Accepted);
        let receipt = wire::decode_claim_receipt(&replay.1).map_err(debug)?;
        assert_eq!(receipt.result, wire::StoreClaimCode::Committed);
        assert_eq!(receipt.recovery_generation.to_string(), placement_generation);
    }
    Ok(())
}

pub(super) async fn house_hunting_checkout(h: &Harness, owner: &ComposedFreshAdmission<'_, '_, '_>, mut admitted: AdmittedSession) -> TestResult {
    super::daily_reward_fixtures::epoch(h, 1).await?;
    for (seed, offer, price) in [
        (54, "store:prey-wildcards:1", 10),
        (55, "store:permanent-prey-slot", 900),
        (56, "store:xp-boost:1", 30),
    ] {
        let request = super::store_fixtures::purchase_request(seed, offer, price)?;
        let result = super::store_fixtures::purchase(owner, &mut admitted, request).await?;
        assert_eq!(result.code, wire::StorePurchaseCode::Accepted, "House checkout for {offer}");
    }
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM store_delivery_receipts")
        .fetch_one(&h.pool).await?;
    assert_eq!(receipts, 3);
    Ok(())
}
