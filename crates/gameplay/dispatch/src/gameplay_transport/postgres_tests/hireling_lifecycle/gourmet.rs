//! Real client USE471, native burn/audit, shared exhaustion and persisted native
//! condition recovery. Food inventory is a fixture; consumption is production code.
use super::{Harness,TestResult,debug,id};
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{AdmittedSession,UseCommand};
use crate::durability::item_use::{ItemUseError,ItemUseFlaskPlan,ItemUseKind,ItemUseRequest};
use crate::durability::item_use::gourmet::GourmetUseRequest;
use oteryn_gameplay_domain::domain::hireling::food::{GourmetBuff,GourmetConditions};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::world_object::{self as wire,UseDisposition,UseIntent,UseTarget};

pub(super) async fn scenario(h:&Harness,owner:&ComposedFreshAdmission<'_,'_,'_>,mut admitted:AdmittedSession,
    actor:ExactActorRef,_hireling:[u8;16])->TestResult {
    super::super::item_look::seed_main_backpack(h,&[
        ("oteryn:item.tibia.i29409",1),("oteryn:item.tibia.i29409",1),("oteryn:item.tibia.i29413",1),
    ]).await?;
    let fence=admitted.item_fence.ok_or("House item fence")?;
    let (carrot,fishing)={
        let runtime=owner.runtime.lock().await;
        let index=crate::gameplay_transport::item_ref_admission::pinned_index(&runtime,owner.active_generation).ok_or("item index")?;
        let reference=|key:&str|index.revision(key).and_then(|revision|index.definition_ref("Item",key,revision));
        (reference("oteryn:item.tibia.i29409").ok_or("carrot index")?,reference("oteryn:item.tibia.i29413").ok_or("fishing food index")?)
    };
    let before_state={let states=owner.states.spell_states.lock().await;states.actors.iter()
        .find(|(a,s,_)|*a==actor&&*s==admitted.game_session_id).ok_or("native player before-state")?.2.clone()};
    let before_revision=before_state.revision();
    let body=wire::encode_item_use_intent(&UseIntent{target:UseTarget::ItemByDefinition(carrot),
        use_with:None,use_with_map_item:None,use_with_position:None}).map_err(debug)?;
    let first_command=admitted.continuity.next_command_id;
    let (status,disposition)=exchange(owner,&mut admitted,&body).await?;
    assert_eq!((status,disposition),(CommandStatus::Accepted,UseDisposition::Committed));
    assert_eq!(count(h,"SELECT count(*) FROM game_item_use_records WHERE cause_kind=471").await?,1);
    assert_eq!(count(h,"SELECT count(*) FROM game_hireling_gourmet_receipts WHERE food_id=29409 AND cooldown_until=started_at+interval '600 seconds'").await?,1);
    assert_eq!(remaining(h,29409).await?,1,"one carrot retired, one remains");
    assert_eq!(remaining(h,35173).await?,0,"Q10 source defect cannot mint a toolbox");
    {
        let states=owner.states.spell_states.lock().await;
        let (_,_,state)=states.actors.iter().find(|(a,s,_)|*a==actor&&*s==admitted.game_session_id).ok_or("native player state")?;
        assert_eq!(state.revision(),before_revision+1);
        assert_eq!(state.owned_skill_adjustments(owner.owner_now().get()).map_err(debug)?.adjust_skill(10,4),Some(17));
    }
    let audit:Vec<u8>=sqlx::query_scalar("SELECT a.envelope FROM game_item_audit_outbox a JOIN game_item_use_records u USING(transaction_id) WHERE u.cause_kind=471")
        .fetch_one(&h.pool).await?;
    let (envelope,event)=crate::durability::item_use_audit::decode_item_use_envelope(&audit).map_err(debug)?;
    assert_eq!(event.house_key.as_deref(),Some(super::HOUSE));
    assert_eq!(event.cause.as_ref().map(|cause|cause.kind),Some(471));
    assert!(event.source_entry.is_some());assert!(event.source_inbox_ordinal.is_none());
    assert_eq!(event.source_before.as_ref().map(|item|item.quantity),Some(1));
    assert_eq!(event.source_after.as_ref().map(|item|item.quantity),Some(0));

    // Bypass the connection replay cache: the production USE owner must read its
    // immutable ingress receipt before looking for the now-retired first source.
    let command=UseCommand{game_session_id:admitted.game_session_id,command_id:first_command,item_fence:Some(fence)};
    assert_eq!(owner.use_item_definition(actor,command,carrot).await.disposition,UseDisposition::Committed);
    assert_eq!(runtime_revision(owner,actor,admitted.game_session_id).await?,before_revision+1);
    assert_eq!(remaining(h,29409).await?,1);
    assert_eq!(owner.use_item_definition(actor,command,fishing).await.disposition,UseDisposition::Rejected);
    let other=UseCommand{command_id:7000,..command};
    assert_eq!(owner.use_item_definition(actor,other,fishing).await.disposition,UseDisposition::Exhausted,
        "a different gourmet food shares the same durable cooldown");
    assert_eq!(remaining(h,29413).await?,1);
    assert_eq!(count(h,"SELECT count(*) FROM game_hireling_gourmet_receipts").await?,1);

    let stored_wire:Vec<u8>=sqlx::query_scalar("SELECT wire_binding FROM game_hireling_gourmet_receipts WHERE food_id=29409")
        .fetch_one(&h.pool).await?;
    let wire:[u8;32]=stored_wire.try_into().map_err(|_|"wire binding")?;
    assert!(owner.root.read_gourmet_use_receipt(owner.character,fence,first_command,wire).await.map_err(debug)?);
    let mut changed_wire=wire;changed_wire[0]^=1;
    assert!(matches!(owner.root.read_gourmet_use_receipt(owner.character,fence,first_command,changed_wire).await,Err(ItemUseError::ConflictingCommand)));

    // Model cancellation at the owner publication boundary after the real
    // native transaction committed: retain its exact prepared successor and
    // before-state. The ordinary checkpoint must reconcile that same receipt
    // before it can persist conditions, never clear the committed food effect.
    let source=event.source_before.as_ref().ok_or("committed gourmet source")?;
    let definition=source.definition.as_ref().ok_or("committed definition")?;
    let cause=event.cause.as_ref().ok_or("committed source cause")?;
    let original=GourmetUseRequest {wire_binding:wire,
        conditions:GourmetConditions {attributes:Some(GourmetBuff {item_id:29409,remaining_ms:3_600_000}),haste:None},
        item:ItemUseRequest {kind:ItemUseKind::HirelingFood {origin_channel:*owner.channel_id.as_bytes()},
            command_id:first_command,content_revision:cause.content_revision.clone(),ruleset_revision:cause.ruleset_revision.clone(),
            sim_revision:cause.sim_revision.clone(),transaction_id:envelope.transaction_event.as_ref().ok_or("membership")?.transaction_id.as_slice().try_into().map_err(|_|"transaction id")?,
            event_id:envelope.event_id.as_slice().try_into().map_err(|_|"event id")?,occurred_at_unix_ms:envelope.occurred_at_unix_ms,
            server_build_id:envelope.server_build_id.clone(),source_item_instance_id:source.item_instance_id.as_slice().try_into().map_err(|_|"source id")?,
            source_definition:crate::durability::item_mint::TypedDefinitionRef {family:definition.family.clone(),production_key:definition.production_key.clone(),revision_ref:definition.revision_ref.clone()},
            source_quantity_before:source.quantity,flask:ItemUseFlaskPlan::None}};
    {
        let mut states=owner.states.spell_states.lock().await;
        let index=states.actors.iter().position(|(a,s,_)|*a==actor&&*s==admitted.game_session_id).ok_or("pending actor")?;
        let prepared=states.actors[index].2.clone();
        states.actors[index].2=before_state;
        states.gourmet_checkpoints.retain(|(a,s,_,_)|*a!=actor||*s!=admitted.game_session_id);
        assert!(crate::gameplay_transport::item_use::gourmet::PendingUse::new(
            actor,admitted.game_session_id,fence,original,prepared,before_revision).park(&mut states));
        assert!(states.has_pending_spell_commit(actor,admitted.game_session_id));
    }
    assert!(crate::gameplay_transport::item_use::gourmet::checkpoint(owner,actor,admitted.game_session_id,true).await);
    {
        let states=owner.states.spell_states.lock().await;
        assert!(!states.has_pending_spell_commit(actor,admitted.game_session_id));
        let state=&states.actors.iter().find(|(a,s,_)|*a==actor&&*s==admitted.game_session_id).ok_or("resolved owner")?.2;
        assert_eq!(state.revision(),before_revision+1);
        assert_eq!(state.owned_skill_adjustments(owner.owner_now().get()).map_err(debug)?.adjust_skill(10,4),Some(17));
    }
    assert_eq!(count(h,"SELECT count(*) FROM game_item_use_records WHERE cause_kind=471").await?,1);
    assert_eq!(remaining(h,29409).await?,1,"pending recovery never burns a second carrot");
    assert_eq!(owner.use_item_definition(actor,command,carrot).await.disposition,UseDisposition::Committed);
    assert_eq!(runtime_revision(owner,actor,admitted.game_session_id).await?,before_revision+1);

    let inventory=owner.root.read_hireling_use_inventory(owner.character,owner.holder,fence,*owner.channel_id.as_bytes()).await.map_err(debug)?;
    let item=inventory.backpack.as_ref().ok_or("backpack")?.entries.iter()
        .find(|e|e.item.definition.production_key=="oteryn:item.tibia.i29409").ok_or("remaining carrot")?;
    let request=GourmetUseRequest{wire_binding:[0x53;32],conditions:GourmetConditions{
        attributes:Some(GourmetBuff{item_id:29409,remaining_ms:3_600_000}),haste:None},item:ItemUseRequest{
        kind:ItemUseKind::HirelingFood{origin_channel:*owner.channel_id.as_bytes()},command_id:7001,
        content_revision:inventory.content_revision.clone(),ruleset_revision:inventory.ruleset_revision.clone(),
        sim_revision:crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),transaction_id:id(180),event_id:id(181),
        occurred_at_unix_ms:1_700_000_000_000,server_build_id:crate::durability::character_authority::SERVER_BUILD_ID.into(),
        source_item_instance_id:item.item.item_instance_id,source_definition:item.item.definition.clone(),
        source_quantity_before:1,flask:ItemUseFlaskPlan::None}};
    let mut stale=fence;stale.character_lease_generation+=1;
    assert!(matches!(owner.root.commit_gourmet_use(owner.character,owner.holder,stale,request.clone()).await,Err(ItemUseError::AuthorityRejected)));
    assert!(matches!(owner.root.commit_gourmet_use(owner.character,owner.holder,fence,request).await,Err(ItemUseError::Cooldown)));
    assert_eq!(remaining(h,29409).await?,1,"stale and cooling writes consume no item");

    let saved=GourmetConditions{attributes:Some(GourmetBuff{item_id:29409,remaining_ms:600_000}),haste:None};
    owner.root.save_gourmet_conditions(owner.character,owner.holder,fence,*owner.channel_id.as_bytes(),saved).await.map_err(debug)?;
    assert_eq!(owner.root.read_gourmet_conditions(owner.character,fence.character_id).await.map_err(debug)?,saved);
    let increased=GourmetConditions{attributes:Some(GourmetBuff{item_id:29409,remaining_ms:600_001}),haste:None};
    assert!(matches!(owner.root.save_gourmet_conditions(owner.character,owner.holder,fence,*owner.channel_id.as_bytes(),increased).await,Err(ItemUseError::StaleState)));
    let facts={let states=owner.states.spell_states.lock().await;states.actors.iter().find(|(a,s,_)|*a==actor&&*s==admitted.game_session_id).ok_or("actor facts")?.2.character_facts()};
    // A new native condition/vitals owner reads durable remaining ticks, exactly
    // the factory and restore hook used by first_entry (not the old runtime state).
    let mut restored=crate::spell::cast::PlayerSpellState::new(facts,0,0).ok_or("fresh native owner")?;
    crate::spell::actor_conditions::restore_gourmet_food(&mut restored,
        owner.root.read_gourmet_conditions(owner.character,fence.character_id).await.map_err(debug)?,9_000_000).map_err(debug)?;
    assert_eq!(restored.owned_skill_adjustments(9_000_000_000).map_err(debug)?.adjust_skill(10,4),Some(17));
    assert_eq!(restored.owned_skill_adjustments(9_600_000_000).map_err(debug)?.adjust_skill(10,4),Some(10));
    assert_eq!(count(h,"SELECT count(*) FROM game_item_use_records WHERE cause_kind=471").await?,1);
    Ok(())
}

async fn runtime_revision(owner:&ComposedFreshAdmission<'_,'_,'_>,actor:ExactActorRef,session:crate::foundation::GameSessionId)->TestResult<u64>{
    Ok(owner.states.spell_states.lock().await.actors.iter().find(|(a,s,_)|*a==actor&&*s==session).ok_or("native player state")?.2.revision())
}
async fn count(h:&Harness,sql:&'static str)->TestResult<i64>{Ok(sqlx::query_scalar(sql).fetch_one(&h.pool).await?)}
async fn remaining(h:&Harness,item:u32)->TestResult<i64>{Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
    .bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)}
async fn exchange(owner:&ComposedFreshAdmission<'_,'_,'_>,admitted:&mut AdmittedSession,body:&[u8])->TestResult<(CommandStatus,UseDisposition)>{
    let (status, payload)=super::wire::exchange(owner,admitted,wire::COMMAND_TYPE_USE_INTENT,body).await?;
    Ok((status,wire::decode_use_result(&payload).map_err(debug)?))
}
