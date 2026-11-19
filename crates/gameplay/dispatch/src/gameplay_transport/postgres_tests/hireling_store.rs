//! E12 Store: real client bytes, existing Platform port and committed Hireling receipts.
use super::hireling_store_fixtures::{FakePlatform, LAMP, Ledger, command, count, new_hire, purchase, serve, uuid};

use super::fixture::{Harness,debug,id,run};
use super::{Dependencies,TestResult,prepare_current_session};
use crate::foundation::{CommandStatus,MessageType,decode_wire_envelope};
use crate::gameplay_transport::capabilities::{OfferedCapability,SelectedCapabilities};
use crate::gameplay_transport::connection::{AdmittedSession,FirstEntryOutcome,IDLE_LIVENESS,serve_admitted};
use crate::gameplay_transport::ComposedFreshAdmission;

use crate::native_admission_source::store::StoreClient;
use oteryn_protocol_oteryn::store as wire;

use std::sync::{Arc,Mutex as SyncMutex};

fn exercise_sword(seed:u8)->TestResult<(u32,Vec<u8>)> {
    let request=wire::StorePurchaseRequest{purchase_request_id:id(seed),offer_key:"store:exercise-sword".into(),catalog_revision:"451c944aa7b6ab9d27cfce868fcf4949c8996a7b6857cc2641e2c296fd415c8c".into(),price:25,currency:"TIBIA_COIN".into()};
    Ok((wire::COMMAND_TYPE_STORE_PURCHASE,wire::encode_purchase_request(&request).map_err(debug)?))
}

#[derive(Debug,Clone,Copy)]
enum Case{Hire,Skill,Outfit,Rename,Sex,Uncertain,Refusals,FullInbox,DeliveryRetry,Limit,LocalEncode,Modal}
#[path="hireling_store/modal.rs"]
mod modal;
fn enabled()->bool {std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_some()}
#[test]
fn hireling_store_purchase_lamp_once_and_replay()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e_fixtures::scenario_thread::run("hireling-store-hire",||scenario(Case::Hire))}
#[test]
fn hireling_store_skill_once_and_duplicate_refusal()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e_fixtures::scenario_thread::run("hireling-store-skill",||scenario(Case::Skill))}
#[test]
fn hireling_store_outfit_once_and_duplicate_refusal()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e_fixtures::scenario_thread::run("hireling-store-outfit",||scenario(Case::Outfit))}
#[test]
fn hireling_store_rename_once_and_binding_refusal()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e_fixtures::scenario_thread::run("hireling-store-rename",||scenario(Case::Rename))}
#[test]
fn hireling_store_sex_change_toggles_only_once()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e_fixtures::scenario_thread::run("hireling-store-sex",||scenario(Case::Sex))}
#[test]
fn hireling_store_uncertain_debit_reserves_entitlement_until_retry()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e_fixtures::scenario_thread::run("hireling-store-uncertain",||scenario(Case::Uncertain))}
#[test]
fn hireling_store_refusals_do_not_debit_and_release_reservation()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e_fixtures::scenario_thread::run("hireling-store-refusals",||scenario(Case::Refusals))}
#[test]
fn hireling_store_full_inbox_refuses_before_debit_and_keeps_delivered_replay()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e_fixtures::scenario_thread::run("hireling-store-inbox",||scenario(Case::FullInbox))}
#[test]
fn hireling_store_paid_lamp_retries_after_prepared_delivery_refusal()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e_fixtures::scenario_thread::run("hireling-store-delivery-retry",||scenario(Case::DeliveryRetry))}
#[test]
fn hireling_store_uncertain_tenth_hire_reserves_character_limit()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e_fixtures::scenario_thread::run("hireling-store-limit",||scenario(Case::Limit))}

#[test]
fn storefix_local_encode_refusal_releases_reservation_without_debit()->TestResult {if !enabled(){return Ok(())}super::kill_reward_e2e_fixtures::scenario_thread::run("storefix-local-encode",||scenario(Case::LocalEncode))}

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
        if matches!(case,Case::Modal) {
            admitted.continuity.selected_capabilities=SelectedCapabilities::select(&[
                OfferedCapability{id:3,requires:&[]},OfferedCapability{id:51,requires:&[]},OfferedCapability{id:92,requires:&[]},
                OfferedCapability{id:154,requires:&[]},OfferedCapability{id:165,requires:&[3,51]},OfferedCapability{id:250,requires:&[165]},
                OfferedCapability{id:450,requires:&[92]},
            ],&[3,51,92,154,165,250,450]).ok_or("modal selection")?;
        }
        let ledger=Arc::new(SyncMutex::new(Ledger{balance:10000,world:uuid(admitted.world_id.as_bytes()),..Ledger::default()}));
        owner.store=Some(Arc::new(tokio::sync::Mutex::new(StoreClient::new(Box::new(FakePlatform(ledger.clone())), "oteryn-game/store-consumer".into()))));

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
            Case::Modal=>modal::scenario(&harness,&owner,&mut admitted,&ledger).await?,
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
            Case::LocalEncode=>{
                // An invalid local authority identity fails encoding before transport.
                owner.store=Some(Arc::new(tokio::sync::Mutex::new(StoreClient::new(Box::new(FakePlatform(ledger.clone())), "".into()))));
                let refused=purchase(&owner,&mut admitted,command(60,1002,900,None)?).await?;
                assert_eq!(refused.code,wire::StorePurchaseCode::PurchaseDisabled);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_hireling_store_reservations WHERE state=1").await?,0);
                assert_eq!(count(&harness,"SELECT count(*) FROM game_hireling_store_reservations WHERE state=3").await?,1);
                assert_eq!(ledger.lock().expect("ledger").debits,1,"only initial fixture hire debited");
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
                let operation={let ledger=ledger.lock().expect("ledger");ledger.answers[&uuid(&id(90))]["units"][0]["delivery_operation_id"].as_str().ok_or("paid operation")?.to_owned()};
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
