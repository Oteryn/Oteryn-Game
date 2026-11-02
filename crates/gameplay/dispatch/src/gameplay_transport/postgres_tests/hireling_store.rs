//! E12 Store: real client bytes, existing Platform port and committed Hireling receipts.
// Assertions inspect native Inbox custody and durable state, not an in-memory substitute.
#![allow(clippy::expect_used)] // Test fixture invariants use the neighbouring Store harness style.
use super::fixture::{Harness,debug,id,run};
use super::{Dependencies,TestResult,prepare_current_session};
use crate::foundation::{CommandStatus,MessageType,decode_wire_envelope};
use crate::gameplay_transport::capabilities::{OfferedCapability,SelectedCapabilities};
use crate::gameplay_transport::connection::{AdmittedSession,ConnectionEnd,FirstEntryOutcome,IDLE_LIVENESS,serve_admitted};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::native_admission_source::descriptor::Operation;
use crate::native_admission_source::store::{StoreClient,StoreTransport,TransportFailure};
use oteryn_protocol_oteryn::store as wire;
use std::collections::BTreeMap;
use std::sync::{Arc,Mutex as SyncMutex};
use tokio::io::{AsyncReadExt,AsyncWriteExt};

const REVISION:&str="store-hireling-crystal-201189f-v1";
const LAMP:&str="oteryn:item.tibia.i29432";

#[derive(Default)]
struct Ledger {
    balance:u64,world:String,debits:usize,calls:usize,
    intents:BTreeMap<String,serde_json::Value>,
    answers:BTreeMap<String,serde_json::Value>,
    lose_next_answer:bool,
}
struct FakePlatform(Arc<SyncMutex<Ledger>>);
impl StoreTransport for FakePlatform {
    fn exchange<'a>(&'a mut self,operation:Operation,body:&'a str)->std::pin::Pin<Box<dyn std::future::Future<Output=Result<Vec<u8>,TransportFailure>>+Send+'a>> {
        let mut ledger=self.0.lock().expect("ledger");
        let request:serde_json::Value=serde_json::from_str(body).expect("request");
        let answer=match operation {
            Operation::ReadStoreBalanceV1=>serde_json::json!({"contract_version":1,"balance":{"currency":request["currency"],"balance":if request["currency"]=="TIBIA_COIN" {ledger.balance}else{0}}}),
            Operation::PurchaseStoreOfferV1=>{
                ledger.calls+=1;
                let key=request["purchase_request_id"].as_str().expect("request id").to_owned();
                if let Some(answer)=ledger.answers.get(&key) {answer.clone()} else {
                    let price=request["price"].as_u64().expect("price");
                    let answer=if price>ledger.balance {serde_json::json!({"contract_version":1,"result":"INSUFFICIENT_BALANCE"})} else {
                        ledger.balance-=price;ledger.debits+=1;
                        let n=u8::try_from(ledger.debits).expect("bounded orders");
                        let operation=uuid(&id(100+n));
                        let intent=serde_json::json!({"operation_id":operation,"account_id":request["account_id"],"character_id":request["character_id"],"world_id":ledger.world,"fulfilment_unit_id":format!("hireling-{n}"),"catalog_revision":request["catalog_revision"],"offer_key":request["offer_key"]});
                        ledger.intents.insert(operation.clone(),intent);
                        serde_json::json!({"contract_version":1,"result":"ACCEPTED","order_id":uuid(&id(130+n)),"operation_ids":[operation]})
                    };
                    ledger.answers.insert(key,answer.clone());
                    if std::mem::take(&mut ledger.lose_next_answer) {return Box::pin(async{Err(TransportFailure::Unavailable)})}
                    answer
                }
            }
            Operation::ReadStoreDeliveryIntentV1=>{
                let operation=request["operation_id"].as_str().expect("operation");
                serde_json::json!({"contract_version":1,"intent":ledger.intents[operation]})
            }
            Operation::PublishStoreReceiptV1=>serde_json::json!({"contract_version":1,"result":"accepted"}),
            _=>return Box::pin(async{Err(TransportFailure::NotFound)}),
        };
        Box::pin(async move{Ok(answer.to_string().into_bytes())})
    }
}

fn uuid(bytes:&[u8;16])->String {
    let hex:String=bytes.iter().map(|b|format!("{b:02x}")).collect();
    format!("{}-{}-{}-{}-{}",&hex[..8],&hex[8..12],&hex[12..16],&hex[16..20],&hex[20..])
}

async fn serve(owner:&ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,command:(u32,Vec<u8>))->TestResult<(CommandStatus,Vec<u8>)> {
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

fn command(seed:u8,offer:u16,price:u64,custom:Option<wire::StoreHirelingCustomization>)->TestResult<(u32,Vec<u8>)> {
    let request=wire::StorePurchaseRequest{purchase_request_id:id(seed),offer_key:format!("store:hireling:{offer}"),catalog_revision:REVISION.into(),price,currency:"TIBIA_COIN".into()};
    let bytes=if let Some(custom)=custom {wire::encode_hireling_purchase_request(&request,&custom)}else{wire::encode_purchase_request(&request)}.map_err(debug)?;
    Ok((wire::COMMAND_TYPE_STORE_PURCHASE,bytes))
}
fn new_hire(name:&str)->wire::StoreHirelingCustomization {wire::StoreHirelingCustomization{hireling_id:None,name:Some(name.into()),sex:Some(1)}}
fn exercise_sword(seed:u8)->TestResult<(u32,Vec<u8>)> {
    let request=wire::StorePurchaseRequest{purchase_request_id:id(seed),offer_key:"store:exercise-sword".into(),catalog_revision:"store-catalogue-r1".into(),price:25,currency:"TIBIA_COIN".into()};
    Ok((wire::COMMAND_TYPE_STORE_PURCHASE,wire::encode_purchase_request(&request).map_err(debug)?))
}
async fn purchase(owner:&ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,command:(u32,Vec<u8>))->TestResult<wire::StorePurchaseResult> {
    let result=serve(owner,admitted,command).await?;assert_eq!(result.0,CommandStatus::Accepted,"{result:?}");
    wire::decode_purchase_result(&result.1).map_err(debug)
}
async fn count(harness:&Harness,sql:&'static str)->TestResult<i64> {Ok(sqlx::query_scalar(sql).fetch_one(&harness.pool).await?)}

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
        Box::new(FakePlatform(ledger.clone())),
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
    let held = ledger.lock().expect("ledger");
    assert_eq!((held.debits, held.balance), (2, 9600));
    Ok(())
}

#[derive(Debug,Clone,Copy)]
enum Case{Hire,Skill,Outfit,Rename,Sex,Uncertain,Refusals,FullInbox,DeliveryRetry,Limit}
fn enabled()->bool {std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_some()}
#[test]
fn hireling_store_purchase_lamp_once_and_replay()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e::scenario_thread::run("hireling-store-hire",||scenario(Case::Hire))}
#[test]
fn hireling_store_skill_once_and_duplicate_refusal()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e::scenario_thread::run("hireling-store-skill",||scenario(Case::Skill))}
#[test]
fn hireling_store_outfit_once_and_duplicate_refusal()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e::scenario_thread::run("hireling-store-outfit",||scenario(Case::Outfit))}
#[test]
fn hireling_store_rename_once_and_binding_refusal()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e::scenario_thread::run("hireling-store-rename",||scenario(Case::Rename))}
#[test]
fn hireling_store_sex_change_toggles_only_once()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e::scenario_thread::run("hireling-store-sex",||scenario(Case::Sex))}
#[test]
fn hireling_store_uncertain_debit_reserves_entitlement_until_retry()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e::scenario_thread::run("hireling-store-uncertain",||scenario(Case::Uncertain))}
#[test]
fn hireling_store_refusals_do_not_debit_and_release_reservation()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e::scenario_thread::run("hireling-store-refusals",||scenario(Case::Refusals))}
#[test]
fn hireling_store_full_inbox_refuses_before_debit_and_keeps_delivered_replay()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e::scenario_thread::run("hireling-store-inbox",||scenario(Case::FullInbox))}
#[test]
fn hireling_store_paid_lamp_retries_after_prepared_delivery_refusal()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e::scenario_thread::run("hireling-store-delivery-retry",||scenario(Case::DeliveryRetry))}
#[test]
fn hireling_store_uncertain_tenth_hire_reserves_character_limit()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e::scenario_thread::run("hireling-store-limit",||scenario(Case::Limit))}

fn scenario(case:Case)->TestResult {
    run(async |admin| {
        let harness=Harness::create_with_admission_seed(admin,"hireling_store",true,false).await?;
        prepare_current_session(&harness).await?;
        let seal=harness.recovery.seal_current().map_err(debug)?;
        let authority=harness.root.open_character_authority(&seal).await.map_err(debug)?;
        let deps=Dependencies::new()?;
        let (controller,_)=crate::content::native_gameplay::test_support::activated_with_loot_tables(deps.admitted.world_id,None);
        let active=controller.active().ok_or("active")?;
        let mut owner=deps.owner(&harness,&authority);owner.active_generation=Some(active);
        deps.initialize(&owner).await;
        owner.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
        let current=crate::durability::fresh_admission::FreshAdmissionStore::from_root(harness.root.clone()).current_session_at(deps.admitted.game_session_id).await?.0;
        let account:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots").fetch_one(&harness.pool).await?;
        let mut admitted=deps.admitted;admitted.first_entry=FirstEntryOutcome::Positioned;
        admitted.item_fence=Some(crate::gameplay_transport::item_fence_of(current).ok_or("fence")?);
        let mut controller_binding=admitted.controller.ok_or("controller")?;controller_binding.account_id=account.try_into().map_err(|_|"account uuid")?;admitted.controller=Some(controller_binding);
        admitted.continuity.selected_capabilities=SelectedCapabilities::select(&[
            OfferedCapability{id:3,requires:&[]},OfferedCapability{id:51,requires:&[]},
            OfferedCapability{id:154,requires:&[]},OfferedCapability{id:165,requires:&[3,51]},OfferedCapability{id:250,requires:&[165]},
        ],&[3,51,154,165,250]).ok_or("capability selection")?;
        let ledger=Arc::new(SyncMutex::new(Ledger{balance:10000,world:uuid(admitted.world_id.as_bytes()),..Ledger::default()}));
        owner.store=Some(Arc::new(tokio::sync::Mutex::new(StoreClient::new(Box::new(FakePlatform(ledger.clone()))))));

        // No Hireling means no purchasable skill; this must precede any Platform debit.
        let absent=purchase(&owner,&mut admitted,command(50,1001,250,None)?).await?;
        assert_eq!(absent.code,wire::StorePurchaseCode::OfferUnavailable);
        assert_eq!(ledger.lock().expect("ledger").debits,0);
        let hired=purchase(&owner,&mut admitted,command(51,25440,150,Some(new_hire("Alex Moon")))?).await?;
        assert_eq!(hired.code,wire::StorePurchaseCode::Accepted);
        let hireling=hired.operation_id.ok_or("hireling operation")?;
        assert_eq!(count(&harness,"SELECT count(*) FROM game_house_hirelings").await?,1);
        let lamps:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_item_character_inbox_locations l USING(item_instance_id,world_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1").bind(LAMP).fetch_one(&harness.pool).await?;
        assert_eq!(lamps,1);
        let customize=|name:Option<&str>|wire::StoreHirelingCustomization{hireling_id:Some(hireling),name:name.map(str::to_owned),sex:None};
        match case {
            Case::Hire=>{
                let replay=purchase(&owner,&mut admitted,command(51,25440,150,Some(new_hire("Alex Moon")))?).await?;assert_eq!(replay,hired);
                assert_eq!(ledger.lock().expect("ledger").debits,1);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_house_hirelings").await?,1);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_store_hireling_effects").await?,1);
                // Crystal schema.sql697-713 keys Hirelings by ID, not by display name.
                let same_name=purchase(&owner,&mut admitted,command(52,25440,150,Some(new_hire("aLEX mOON")))?).await?;
                assert_eq!(same_name.code,wire::StorePurchaseCode::Accepted);
                assert_ne!(same_name.operation_id,hired.operation_id);
                assert_eq!(purchase(&owner,&mut admitted,command(52,25440,150,Some(new_hire("Alex Moon")))?).await?,same_name);
                assert_eq!(ledger.lock().expect("ledger").debits,2);
                assert_eq!(count(&harness,"SELECT count(DISTINCT hireling_id) FROM game_house_hirelings WHERE name='Alex Moon'").await?,2);
                // An uncertain reservation of that same display name must not lock it globally.
                ledger.lock().expect("ledger").lose_next_answer=true;
                let lost=serve(&owner,&mut admitted,command(65,25440,150,Some(new_hire("Alex Moon")))?).await?;
                assert_ne!(lost.0,CommandStatus::Accepted);
                let independent=purchase(&owner,&mut admitted,command(66,25440,150,Some(new_hire("Alex Moon")))?).await?;
                assert_eq!(independent.code,wire::StorePurchaseCode::Accepted);
                let recovered=purchase(&owner,&mut admitted,command(65,25440,150,Some(new_hire("Alex Moon")))?).await?;
                assert_eq!(recovered.code,wire::StorePurchaseCode::Accepted);
                assert_ne!(independent.operation_id,recovered.operation_id);
                assert_eq!(ledger.lock().expect("ledger").debits,4);
                assert_eq!(count(&harness,"SELECT count(DISTINCT hireling_id) FROM game_house_hirelings WHERE name='Alex Moon'").await?,4);
                assert_eq!(count(&harness,"SELECT count(DISTINCT lamp_item_instance_id) FROM game_house_hirelings WHERE name='Alex Moon'").await?,4);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_hireling_store_reservations WHERE state=1").await?,0);
            }
            Case::Skill|Case::Outfit=>{
                let (offer,price)=if matches!(case,Case::Skill){(1001,250)}else{(2001,500)};
                let first=purchase(&owner,&mut admitted,command(53,offer,price,None)?).await?;assert_eq!(first.code,wire::StorePurchaseCode::Accepted);
                let replay=purchase(&owner,&mut admitted,command(53,offer,price,None)?).await?;assert_eq!(replay,first);
                let duplicate=purchase(&owner,&mut admitted,command(54,offer,price,None)?).await?;assert_eq!(duplicate.code,wire::StorePurchaseCode::OfferUnavailable);
                assert_eq!(ledger.lock().expect("ledger").debits,2);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_hireling_entitlements").await?,1);
                // A subsequently hired identity sees the same character entitlement.
                let second=purchase(&owner,&mut admitted,command(55,25440,150,Some(new_hire("Morgan")))?).await?;assert_eq!(second.code,wire::StorePurchaseCode::Accepted);
                let inherited:i64=sqlx::query_scalar("SELECT count(*) FROM game_house_hirelings h JOIN game_hireling_entitlements e ON e.character_id=h.owner_character_id WHERE e.entitlement_id=$1").bind(offer as i16).fetch_one(&harness.pool).await?;assert_eq!(inherited,2);
            }
            Case::Rename=>{
                let first=purchase(&owner,&mut admitted,command(56,25438,250,Some(customize(Some("mORGAN"))))?).await?;assert_eq!(first.code,wire::StorePurchaseCode::Accepted);
                let replay=purchase(&owner,&mut admitted,command(56,25438,250,Some(customize(Some("Morgan"))))?).await?;assert_eq!(replay,first);
                let name:String=sqlx::query_scalar("SELECT name FROM game_house_hirelings").fetch_one(&harness.pool).await?;assert_eq!(name,"Morgan");
                let conflict=serve(&owner,&mut admitted,command(56,25438,250,Some(customize(Some("River"))))?).await?;assert_ne!(conflict.0,CommandStatus::Accepted);
                assert_eq!(ledger.lock().expect("ledger").debits,2);
            }
            Case::Sex=>{
                let first=purchase(&owner,&mut admitted,command(57,25437,120,Some(customize(None)))?).await?;assert_eq!(first.code,wire::StorePurchaseCode::Accepted);
                let replay=purchase(&owner,&mut admitted,command(57,25437,120,Some(customize(None)))?).await?;assert_eq!(replay,first);
                let state:(i16,i32)=sqlx::query_as("SELECT sex,look_type FROM game_house_hirelings").fetch_one(&harness.pool).await?;assert_eq!(state,(2,1107));
                assert_eq!(ledger.lock().expect("ledger").debits,2);
            }
            Case::Uncertain=>{
                ledger.lock().expect("ledger").lose_next_answer=true;
                let lost=serve(&owner,&mut admitted,command(58,1004,250,None)?).await?;assert_ne!(lost.0,CommandStatus::Accepted);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_hireling_store_reservations WHERE state=1").await?,1);
                let blocked=purchase(&owner,&mut admitted,command(59,1004,250,None)?).await?;assert_eq!(blocked.code,wire::StorePurchaseCode::OfferUnavailable);
                let recovered=purchase(&owner,&mut admitted,command(58,1004,250,None)?).await?;assert_eq!(recovered.code,wire::StorePurchaseCode::Accepted);
                assert_eq!(ledger.lock().expect("ledger").debits,2);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_hireling_store_reservations WHERE state=1").await?,0);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_hireling_entitlements WHERE entitlement_id=1004").await?,1);
            }
            Case::Refusals=>{
                let stale=purchase(&owner,&mut admitted,command(60,1002,899,None)?).await?;assert_eq!(stale.code,wire::StorePurchaseCode::StaleQuote);
                let invalid=purchase(&owner,&mut admitted,command(61,25440,150,Some(new_hire("Game Master")))?).await?;assert_eq!(invalid.code,wire::StorePurchaseCode::OfferUnavailable);
                let foreign=wire::StoreHirelingCustomization{hireling_id:Some(id(240)),name:None,sex:None};
                let not_owned=purchase(&owner,&mut admitted,command(62,25437,120,Some(foreign))?).await?;assert_eq!(not_owned.code,wire::StorePurchaseCode::OfferUnavailable);
                ledger.lock().expect("ledger").balance=0;
                let poor=purchase(&owner,&mut admitted,command(63,1002,900,None)?).await?;assert_eq!(poor.code,wire::StorePurchaseCode::InsufficientBalance);
                let again=purchase(&owner,&mut admitted,command(63,1002,900,None)?).await?;assert_eq!(again,poor);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_hireling_store_reservations WHERE state=3").await?,1);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_hireling_store_reservations WHERE state=1").await?,0);
                assert_eq!(ledger.lock().expect("ledger").debits,1);
                ledger.lock().expect("ledger").balance=1000;
                let retry=purchase(&owner,&mut admitted,command(64,1002,900,None)?).await?;assert_eq!(retry.code,wire::StorePurchaseCode::Accepted);
            }
            Case::FullInbox=>{
                // Fill through the existing Store owner, retaining its native audit and custody.
                for seed in 70..89 {
                    let result=purchase(&owner,&mut admitted,exercise_sword(seed)?).await?;
                    assert_eq!(result.code,wire::StorePurchaseCode::Accepted);
                }
                assert_eq!(count(&harness,"SELECT count(*) FROM game_item_character_inbox_locations").await?,20);
                let blocked=purchase(&owner,&mut admitted,command(90,25440,150,Some(new_hire("River")))?).await?;
                assert_eq!(blocked.code,wire::StorePurchaseCode::OfferUnavailable);
                assert_eq!(ledger.lock().expect("ledger").debits,20);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_house_hirelings").await?,1);
                let replay=purchase(&owner,&mut admitted,command(51,25440,150,Some(new_hire("Alex Moon")))?).await?;
                assert_eq!(replay,hired);assert_eq!(ledger.lock().expect("ledger").debits,20);
            }
            Case::DeliveryRetry=>{
                // One slot remains when the real pre-debit Hireling reservation commits.
                for seed in 70..88 {
                    assert_eq!(purchase(&owner,&mut admitted,exercise_sword(seed)?).await?.code,wire::StorePurchaseCode::Accepted);
                }
                assert_eq!(count(&harness,"SELECT count(*) FROM game_item_character_inbox_locations").await?,19);
                ledger.lock().expect("ledger").lose_next_answer=true;
                let request=command(90,25440,150,Some(new_hire("River")))?;
                assert_eq!(serve(&owner,&mut admitted,request.clone()).await?.0,CommandStatus::Rejected);
                assert_eq!(ledger.lock().expect("ledger").debits,20);
                // A separate native Store item can occupy the last slot while the answer is lost.
                let last_sword=purchase(&owner,&mut admitted,exercise_sword(89)?).await?;
                assert_eq!(last_sword.code,wire::StorePurchaseCode::Accepted);
                let failed_command=admitted.continuity.next_command_id;
                assert_eq!(serve(&owner,&mut admitted,request.clone()).await?.0,CommandStatus::Rejected);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_item_character_inbox_locations").await?,20);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_house_hirelings").await?,1);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_hireling_store_reservations WHERE state=1").await?,1);
                let operation={let ledger=ledger.lock().expect("ledger");ledger.answers[&uuid(&id(90))]["operation_ids"][0].as_str().ok_or("paid operation")?.to_owned()};
                let failed:(String,String,i16)=sqlx::query_as("SELECT transaction_id::text,item_instance_ids[1]::text,work_units_used FROM game_reward_claim_inbox_mint_reservations WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                    .bind(admitted.game_session_id.as_bytes().as_slice()).bind(failed_command.to_string()).fetch_one(&harness.pool).await?;
                assert!((1..=3).contains(&failed.2));
                let no_value:bool=sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM store_delivery_receipts WHERE operation_id=$1::uuid) AND NOT EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=$2::uuid) AND NOT EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE transaction_id=$3::uuid) AND NOT EXISTS(SELECT 1 FROM game_reward_claim_inbox_mint_receipts WHERE transaction_id=$3::uuid) AND NOT EXISTS(SELECT 1 FROM game_store_hireling_effects WHERE operation_id=$1::uuid)")
                    .bind(&operation).bind(&failed.1).bind(&failed.0).fetch_one(&harness.pool).await?;
                assert!(no_value,"failed native preparation must retain no value or outbox");

                // Fixture-only retirement frees one ordinary item; all immutable Store/native
                // history and the prepared Hireling attempt remain available for replay checks.
                let mut tx=harness.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
                let ordinary:String=sqlx::query_scalar("SELECT i.item_instance_id::text FROM store_delivery_receipts r JOIN game_item_instances i USING(item_instance_id,world_id) JOIN game_item_character_inbox_locations l USING(item_instance_id,world_id) WHERE r.operation_id=encode($1,'hex')::uuid AND l.character_id=encode($2,'hex')::uuid AND l.world_id=encode($3,'hex')::uuid AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i28552' AND i.definition_revision_ref='definition-r1' AND i.quantity=1 AND i.lifecycle=1")
                    .bind(last_sword.operation_id.ok_or("ordinary operation")?.as_slice())
                    .bind(admitted.item_fence.ok_or("item fence")?.character_id.as_bytes().as_slice())
                    .bind(admitted.world_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                assert_eq!(sqlx::query("DELETE FROM game_item_character_inbox_locations WHERE item_instance_id=$1::uuid").bind(&ordinary).execute(&mut *tx).await?.rows_affected(),1);
                assert_eq!(sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,state_revision=state_revision+1,last_transaction_id=encode($2,'hex')::uuid WHERE item_instance_id=$1::uuid").bind(&ordinary).bind(id(245).as_slice()).execute(&mut *tx).await?.rows_affected(),1);
                tx.commit().await?;

                let recovered=purchase(&owner,&mut admitted,request.clone()).await?;
                assert_eq!(recovered.code,wire::StorePurchaseCode::Accepted);
                assert_eq!(recovered.operation_id.map(|id|uuid(&id)).as_deref(),Some(operation.as_str()));
                assert_eq!(purchase(&owner,&mut admitted,request).await?,recovered);
                assert_eq!(ledger.lock().expect("ledger").debits,21);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_item_character_inbox_locations").await?,20);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_house_hirelings").await?,2);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_hireling_store_reservations WHERE state=1").await?,0);
                let committed:(String,String)=sqlx::query_as("SELECT native_transaction_id::text,item_instance_id::text FROM store_delivery_receipts WHERE operation_id=$1::uuid")
                    .bind(&operation).fetch_one(&harness.pool).await?;
                assert_ne!(committed.0,failed.0);assert_ne!(committed.0,operation);assert_eq!(committed.1,failed.1);
                let lamp_transaction:String=sqlx::query_scalar("SELECT minted_transaction_id::text FROM game_item_instances WHERE item_instance_id=$1::uuid AND lifecycle=1")
                    .bind(&committed.1).fetch_one(&harness.pool).await?;
                assert_eq!(lamp_transaction,committed.0);
                let proof:(i64,i64,i64,i64,i16)=sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claim_inbox_mint_reservations WHERE item_instance_ids=ARRAY[$1::uuid]),(SELECT count(*) FROM game_reward_claim_inbox_mint_lines WHERE item_instance_id=$1::uuid),(SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=$2::uuid),(SELECT count(*) FROM game_store_hireling_effects WHERE operation_id=$3::uuid),(SELECT work_units_used FROM game_reward_claim_inbox_mint_reservations WHERE transaction_id=$4::uuid)")
                    .bind(&committed.1).bind(&committed.0).bind(&operation).bind(&failed.0).fetch_one(&harness.pool).await?;
                assert_eq!(proof,(2,1,1,1,failed.2),"a fresh wire retry must preserve the failed reservation and commit exactly one native mint/effect");
            }
            Case::Limit=>{
                for (offset,name) in ["River","Alden","Bryn","Cora","Dara","Elin","Faye","Glen"].iter().enumerate() {
                    let result=purchase(&owner,&mut admitted,command(70+u8::try_from(offset)?,25440,150,Some(new_hire(name)))?).await?;
                    assert_eq!(result.code,wire::StorePurchaseCode::Accepted);
                }
                ledger.lock().expect("ledger").lose_next_answer=true;
                let lost=serve(&owner,&mut admitted,command(80,25440,150,Some(new_hire("Hana")))?).await?;
                assert_ne!(lost.0,CommandStatus::Accepted);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_house_hirelings").await?,9);
                let blocked=purchase(&owner,&mut admitted,command(81,25440,150,Some(new_hire("Iris")))?).await?;
                assert_eq!(blocked.code,wire::StorePurchaseCode::OfferUnavailable);
                assert_eq!(ledger.lock().expect("ledger").debits,10);
                let recovered=purchase(&owner,&mut admitted,command(80,25440,150,Some(new_hire("Hana")))?).await?;
                assert_eq!(recovered.code,wire::StorePurchaseCode::Accepted);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_house_hirelings").await?,10);
                assert_eq!(ledger.lock().expect("ledger").debits,10);
            }
        }
        eprintln!("HIRELING_STORE {case:?} POSTGRES_CASE_COMPLETED");
        drop(owner);drop(authority);drop(seal);harness.cleanup().await
    })
}
