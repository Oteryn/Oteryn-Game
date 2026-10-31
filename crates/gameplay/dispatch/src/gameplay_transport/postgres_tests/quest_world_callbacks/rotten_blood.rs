//! Source altar and paid detour use the existing literal USE, NPC, item, Bank,
//! fee and generic Quest owners. No resources or progress are seeded by SQL.
use super::*;
use crate::durability::character_progression::{CharacterProgressionError, CurrentCharacterGameplayFence};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::quest_state::rotten_blood_access::RottenBloodAccessPayment;
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::rotten_blood as rules;

#[test]
fn rotten_blood_postgres_literal_altar_consumption_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    super::super::kill_reward_e2e::scenario_thread::run("rotten-blood-altar",||
        super::scenario("rotten-blood/sacrifice",Position { x:100,y:100,floor:-7 },false))
}

#[test]
fn rotten_blood_postgres_npc_detour_coin_bank_fee_atomicity_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    super::super::kill_reward_e2e::scenario_thread::run("rotten-blood-detour",||
        super::scenario("rotten-blood-access-purchase",Position { x:100,y:100,floor:-7 },false))
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn exercise(
    owner:&mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted:&mut AdmittedSession,h:&Harness,
    authority:&crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    fence:CurrentCharacterGameplayFence,callback:&Callback,trigger:Position,purchase:bool,
) -> TestResult {
    // The shared fixture replaces its runtime after admitting the session.
    // Bind that exact current Character lease before any native NPC/USE path.
    {
        let actor=admitted.runtime_actor.ok_or("Rotten fixture actor")?;
        owner.runtime.lock().await.bind_attacker_lease(actor,admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation)
                .map_err(debug)?).map_err(debug)?;
    }
    let catalogue=owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone();
    // A generic Quest command cannot bypass either resource source.
    for (number,key) in [(901,rules::SACRIFICE),(902,rules::PURCHASE_ACCESS)] {
        let mut slot=owner.revision_sequencer.acquire(fence.character_id).await;
        let result=slot.commit_quest_transition(owner.root,owner.character,owner.holder,fence,
            QuestTransitionRequest { transition_key:key.into(),cause:QuestCause::Command(
                CommandRef::new(admitted.game_session_id,CommandId::new(number).map_err(debug)?)) },
            catalogue.clone()).await;
        assert!(matches!(result,Err(CharacterProgressionError::InvalidInput)),"{key}: {result:?}");
    }
    if purchase { return paid(owner,admitted,h,authority,trigger).await; }
    owner.states.chat.admit(admitted.game_session_id,"Sacrifice Hero",owner.owner_now());
    let key=oteryn_world_bundle::bundle::placement_key(trigger.floor,trigger.x,trigger.y,1).ok_or("altar placement")?;
    assert_eq!(use_altar(owner,admitted,key,1).await?,UseDisposition::Rejected);
    assert_eq!(evidence(h).await?,(0,0,0));
    let required=callback.item_on_target.as_ref().ok_or("bloody tear")?;
    let offering=oteryn_gameplay_quests::quest::world_callbacks::Offering {
        position:trigger,item:required.item.clone(),count:1,
    };
    input_item(owner,fence.item_fence(),&offering,60,true).await
        .map_err(|error|format!("Rotten altar stage0 materialize bloody tear32594: {error}"))?;
    assert_eq!(use_altar(owner,admitted,key,2).await?,UseDisposition::Committed);
    assert_eq!(evidence(h).await?,(1,1,0));
    let retained=evidence(h).await?;
    // A repeated command cannot consume a newly carried item or add a phase.
    let _=use_altar_revision(owner,admitted,key,2,Some(0)).await?;
    assert_eq!(evidence(h).await?,retained);
    assert_eq!(use_altar(owner,admitted,key,3).await?,UseDisposition::Rejected);
    input_item(owner,fence.item_fence(),&offering,61,true).await
        .map_err(|error|format!("Rotten altar stage1 materialize bloody tear32594: {error}"))?;
    assert_eq!(use_altar(owner,admitted,key,4).await?,UseDisposition::Committed);
    assert_eq!(evidence(h).await?,(2,1,1),"phase 1 checks possession without burning");
    assert_eq!(use_altar(owner,admitted,key,5).await?,UseDisposition::Committed);
    assert_eq!(evidence(h).await?,(3,2,0));
    input_item(owner,fence.item_fence(),&offering,62,true).await
        .map_err(|error|format!("Rotten altar stage3 materialize bloody tear32594: {error}"))?;
    assert_eq!(use_altar(owner,admitted,key,6).await?,UseDisposition::Committed);
    assert_eq!(evidence(h).await?,(4,3,0));
    assert_eq!(use_altar(owner,admitted,key,7).await?,UseDisposition::Rejected);
    assert_eq!(evidence(h).await?,(4,3,0));
    let copy=owner.root.read_character_quest_state(authority,fence.character_id).await.map_err(debug)?;
    assert_eq!(copy.tracks().get(rules::ACCESS_TRACK),Some(&4));
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1")
        .bind(rules::SACRIFICE).fetch_one(&h.pool).await?,4);
    eprintln!("ROTTEN_BLOOD POSTGRES_CASE_COMPLETED literal_altar possession_only_phase atomic_item_burn replay");
    Ok(())
}

fn advance(owner:&mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(2)).ok_or("clock")?;
    Ok(())
}

async fn use_altar(
    owner:&mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted:&mut AdmittedSession,key:u64,number:u64,
) -> TestResult<UseDisposition> {
    use_altar_revision(owner,admitted,key,number,None).await
}

async fn use_altar_revision(
    owner:&mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted:&mut AdmittedSession,key:u64,number:u64,revision:Option<u64>,
) -> TestResult<UseDisposition> {
    use crate::map::view::MapFacts;
    advance(owner)?;
    let map=owner.bundle_map().ok_or("map")?;
    let mut placement=map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target=WorldObjectTarget { placement,expected_revision:revision.unwrap_or_else(||map.facts.object_revision(key)) };
    let payload=oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
    let reply=wire(owner,admitted,number,2,payload).await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}

async fn evidence(h:&Harness) -> TestResult<(i64,i64,i64)> {
    Ok(sqlx::query_as("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0), (SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key=$3), (SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i32594' AND lifecycle=1)")
        .bind(id(41).as_slice()).bind(rules::ACCESS_TRACK).bind(rules::SACRIFICE).fetch_one(&h.pool).await?)
}

async fn paid(
    owner:&mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted:&mut AdmittedSession,h:&Harness,
    authority:&crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    trigger:Position,
) -> TestResult {
    let actor=admitted.runtime_actor.ok_or("actor")?;
    let session=admitted.game_session_id;
    let npc=super::super::rotten_blood_progression::place_npc(owner,actor,session,"oteryn:npc.bloodshade_sacrifice").await?;
    for (number,text) in [(40,"hi"),(41,"quest"),(42,"detour"),(43,"yes"),(44,"yes")] {
        advance(owner)?;
        super::super::rotten_blood_progression::talk(owner,actor,session,npc,number,text).await?;
    }
    assert_eq!(evidence(h).await?,(0,0,0));
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=13")
        .fetch_one(&h.pool).await?,0);
    let fence=owner.current_quest_fence(session).await.map_err(|_|"fence")?.ok_or("fence")?;
    let missing=payment(owner,fence,44).await?;
    assert_eq!(owner.root.reconcile_rotten_blood_access_purchase(authority,fence,missing).await.map_err(debug)?,None);
    // Five real 100-crystal stacks supply the source price. Deposit three
    // stacks through Bank so the production payment proves mixed coin+bank use.
    for n in 80..85 {
        input_item(owner,fence.item_fence(),&oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position:trigger,item:"oteryn:item.tibia.i3043".into(),count:100,
        },n,true).await.map_err(|error|format!("Rotten paid access materialize crystal stack {n}: {error}"))?;
    }
    let funds=payment(owner,fence,64).await?;
    let request=crate::gameplay_transport::npc_command::bank_request(
        &crate::gameplay_transport::bank_npc::BankNpcOperation::Deposit(3_000_000),
        Some(crate::durability::bank::BankCoinFacts {
            gold:funds.coins.gold.clone(),platinum:funds.coins.platinum.clone(),
            crystal:funds.coins.crystal.clone(),backpack:funds.coins.backpack.clone(),
        }),session,50).ok_or("bank request")?;
    let crate::durability::bank::BankRequest::Deposit(request)=request else { return Err("deposit".into()); };
    owner.root.bank_deposit(authority,owner.holder,fence.item_fence(),request).await.map_err(debug)?;
    sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())").execute(&h.pool).await?;
    let before=owner.current_quest_fence(session).await.map_err(|_|"fence")?.ok_or("fence")?;
    let retained=payment(owner,before,64).await?;
    let mut stale=before;
    stale.character_lease_generation+=1;
    assert!(owner.root.commit_rotten_blood_access_purchase(authority,owner.holder,stale,retained.clone(),
        owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone()).await.is_err());
    for (number,text) in [(60,"hi"),(61,"quest"),(62,"detour"),(63,"yes"),(64,"yes")] {
        advance(owner)?;
        super::super::rotten_blood_progression::talk(owner,actor,session,npc,number,text).await?;
    }
    let receipt=owner.root.reconcile_rotten_blood_access_purchase(authority,before,retained.clone()).await.map_err(debug)?
        .ok_or("paid NPC receipt")?;
    assert!(matches!(receipt,QuestTransitionOutcome::AlreadyCommitted(_)));
    assert_eq!(evidence(h).await?,(4,0,0));
    let fee:(i64,i64)=sqlx::query_as("SELECT fee_gold_units,bank_debit_gold_units FROM game_item_fee_burns WHERE cause_kind=13")
        .fetch_one(&h.pool).await?;
    assert_eq!(fee,(5_000_000,3_000_000));
    let mut slot=owner.revision_sequencer.acquire(before.character_id).await;
    let replay=owner.root.commit_rotten_blood_access_purchase(authority,owner.holder,before,retained.clone(),
        owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone()).await.map_err(debug)?;
    assert!(matches!(replay,QuestTransitionOutcome::AlreadyCommitted(_)));
    slot.observe_endgame_progression();
    drop(slot);
    let mut conflict=retained;
    conflict.occurrence=crate::gameplay_transport::npc_trade_command::derived(b"conflicting-quest-fee",session,64);
    assert!(matches!(owner.root.reconcile_rotten_blood_access_purchase(authority,before,conflict).await,
        Err(CharacterProgressionError::ConflictingOccurrence)));
    for (number,text) in [(71,"quest"),(72,"detour"),(73,"yes"),(74,"yes")] {
        advance(owner)?;
        super::super::rotten_blood_progression::talk(owner,actor,session,npc,number,text).await?;
    }
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=13")
        .fetch_one(&h.pool).await?,1);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1")
        .bind(rules::PURCHASE_ACCESS).fetch_one(&h.pool).await?,1);
    assert!(sqlx::query("UPDATE game_rotten_blood_access_fees SET cause_ordinal=cause_ordinal+1")
        .execute(&h.pool).await.is_err());
    eprintln!("ROTTEN_BLOOD POSTGRES_CASE_COMPLETED npc_detour missing_funds exact_coin_bank_fee generic_bypass stale_fence immutable_replay");
    Ok(())
}

async fn payment(
    owner:&crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    fence:CurrentCharacterGameplayFence,number:u64,
) -> TestResult<RottenBloodAccessPayment> {
    let (gold,platinum,crystal)={
        let runtime=owner.runtime.lock().await;
        crate::gameplay_transport::npc_command::bank_coin_definitions(owner,&runtime).ok_or("coin definitions")?
    };
    let bag=owner.root.read_character_backpack(owner.character,fence.character_id).await.map_err(debug)?.ok_or("backpack")?;
    let native=owner.active_generation.and_then(|active|active.native_gameplay()).ok_or("native")?;
    let backpack=crate::gameplay_transport::item_ref_admission::generation_item_facts(native,&bag.backpack.definition).map_err(debug)?;
    let derive=|slot:u8|crate::gameplay_transport::npc_trade_command::derived(
        format!("oteryn:rotten-blood-access-purchase:v1:{slot}").as_bytes(),fence.game_session_id,number);
    Ok(RottenBloodAccessPayment {
        command:CommandRef::new(fence.game_session_id,CommandId::new(number).map_err(debug)?),
        occurrence:derive(0),transaction_id:derive(1),event_id:derive(2),
        coins:crate::durability::item_fee_burn::FeeChangeFacts {
            item_instance_ids:[derive(3),derive(4)],gold,platinum,crystal,backpack,
        },expected_backpack:Some(bag.backpack.item_instance_id),
        occurred_at_unix_ms:crate::gameplay_transport::npc_trade_command::unix_ms_now(),
    })
}
