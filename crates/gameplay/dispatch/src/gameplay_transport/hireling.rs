//! E12: one durable Hireling owner, native lamp USE/Inbox custody, and scoped NPC projection.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::{AdmittedSession,UseCommand}};
use crate::durability::house_hireling::{HirelingLampPlacement,HirelingLampReturn,HirelingLampInput,HirelingReturnInput,HIRELING_LAMP_KEY,command_identity};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_use::{ItemUseKind,ItemUseRequest,ItemUseFlaskPlan};
use crate::foundation::{CommandId,CommandRef,ExactActorRef,MovementFacing,NpcPlacement};
use oteryn_protocol_oteryn::hireling::{self as wire,management::{Reply,Refusal}};
use oteryn_protocol_oteryn::world_object::UseDisposition;
#[path="hireling/pending.rs"]
mod pending;
pub(in crate::gameplay_transport) use pending::{PendingLampUse,resolve as resolve_pending_lamp,
    recover_ingress as recover_pending_lamp_ingress};

pub(super) fn offered(base:&[OfferedCapability],available:bool)->Vec<OfferedCapability>{
    let mut offers:Vec<_>=base.iter().copied().filter(|c|c.id!=wire::CAPABILITY_HIRELING_V1).collect();
    if available && [3,51].iter().all(|id|offers.iter().any(|c|c.id==*id)) {
        let at=offers.partition_point(|c|c.id<wire::CAPABILITY_HIRELING_V1);
        offers.insert(at,OfferedCapability{id:wire::CAPABILITY_HIRELING_V1,requires:&[3,51]});
    }
    offers
}
macro_rules! authority_methods {()=>{
    fn hireling_available(&self)->bool{false}
    fn hireling(&self,_:$crate::gameplay_transport::connection::AdmittedSession,_:u64,_:oteryn_protocol_oteryn::hireling::Request)->impl std::future::Future<Output=Option<bool>>{async{Some(false)}}
};}
pub(super) use authority_methods;
macro_rules! composed_methods {()=>{
    fn hireling_available(&self)->bool{self.active_generation.and_then(crate::content::ActiveGeneration::native_gameplay).is_some()}
    async fn hireling(&self,admitted:connection::AdmittedSession,command:u64,request:oteryn_protocol_oteryn::hireling::Request)->Option<bool>{hireling::execute(self,admitted,command,request).await}
};}
pub(super) use composed_methods;

/// Preserve command220 while selecting a unique existing identity. It never creates a free hire.
pub(super) async fn execute(owner:&ComposedFreshAdmission<'_,'_,'_>,admitted:AdmittedSession,command:u64,request:wire::Request)->Option<bool>{
    if !admitted.continuity.selected_capabilities.contains(wire::CAPABILITY_HIRELING_V1){return Some(false)}
    let fence=admitted.item_fence?;let actor=admitted.runtime_actor?;
    if !resolve_pending_lamp(owner,actor,admitted.game_session_id).await{return None}
    let house=owner.runtime.lock().await.house_actor_state(actor,admitted.game_session_id).ok()?.house;
    let use_command=UseCommand{game_session_id:admitted.game_session_id,command_id:command,item_fence:Some(fence)};
    match request {
        wire::Request::Query=>{
            let reply=owner.root.query_legacy_hireling(owner.character,owner.holder,fence,*owner.channel_id.as_bytes(),command).await.ok()?;
            refresh(owner,actor,use_command).await?;
            return Some(matches!(reply,Reply::State(entries) if !entries.is_empty()));
        }
        wire::Request::Summon{x,y,floor}=>{
            let input=HirelingLampInput::LegacySummon{x,y,floor};
            if let Some(reply)=owner.root.read_hireling_bound_receipt(owner.character,fence,command,input.binding()).await.ok()?{refresh(owner,actor,use_command).await?;return Some(reply==Reply::Committed)}
            let state=owner.runtime.lock().await.house_actor_state(actor,admitted.game_session_id).ok()?;
            if (state.position.x,state.position.y,state.position.floor)!=(i32::from(x),i32::from(y),-i16::from(floor)){
                record_refusal(owner,use_command,input.binding(),Refusal::Conflict).await?;return Some(false)
            }
            let mut owned=owner.root.read_owned_hirelings(owner.character,owner.holder,fence,*owner.channel_id.as_bytes()).await.ok()?.into_iter().filter(|h|h.position.is_none()&&h.lamp.is_some());
            let Some(h)=owned.next() else{record_refusal(owner,use_command,input.binding(),Refusal::NotActive).await?;return Some(false)};
            if owned.next().is_some(){record_refusal(owner,use_command,input.binding(),Refusal::Conflict).await?;return Some(false)}
            if use_lamp(owner,actor,use_command,h.lamp?,input).await?!=UseDisposition::Committed{return Some(false)}
        }
        wire::Request::Dismiss=>{
            let input=HirelingReturnInput::LegacyDismiss;
            if let Some(reply)=owner.root.read_hireling_bound_receipt(owner.character,fence,command,input.binding([0;16])).await.ok()?{refresh(owner,actor,use_command).await?;return Some(!matches!(reply,Reply::Committed|Reply::Refused(Refusal::NotActive)))}
            let mut owned=owner.root.read_owned_hirelings(owner.character,owner.holder,fence,*owner.channel_id.as_bytes()).await.ok()?.into_iter().filter(|h|h.house.as_deref()==Some(house.as_str())&&h.position.is_some());
            let Some(h)=owned.next() else{record_refusal(owner,use_command,input.binding([0;16]),Refusal::NotActive).await?;return Some(false)};
            if owned.next().is_some(){record_refusal(owner,use_command,input.binding([0;16]),Refusal::Conflict).await?;return Some(true)}
            let reply=return_to_lamp_input(owner,actor,use_command,h.id,input).await?;
            return Some(reply!=Reply::Committed);
        }
    }
    let current=refresh(owner,actor,use_command).await?;
    Some(current)
}

pub(super) async fn refresh(owner:&ComposedFreshAdmission<'_,'_,'_>,actor:ExactActorRef,command:UseCommand)->Option<bool>{
    let fence=command.item_fence?;
    // Keep the actual owner lock through the bounded DB read and publication:
    // a slower old refresh must never overwrite a newer placement projection.
    let mut runtime=owner.runtime.lock().await;
    let state=runtime.house_actor_state(actor,command.game_session_id).ok()?;
    if state.scope!=fence.runtime_scope||state.generation!=fence.scope_ownership_generation{return None}
    let snapshots=owner.root.read_house_hirelings(owner.character,owner.holder,fence,*owner.channel_id.as_bytes()).await.ok()?;
    let projections=snapshots.into_iter().map(|h|{
        let (x,y,floor)=h.position?;
        Some(crate::foundation::HirelingProjection{key:h.npc_key(),name:h.name,placement:NpcPlacement{x,y,floor,direction:MovementFacing::South},look_type:h.look_type,colors:h.colors})
    }).collect::<Option<Vec<_>>>()?;
    runtime.reconcile_house_hirelings(actor,command.game_session_id,&projections).ok()?;
    Some(!projections.is_empty())
}

pub(super) async fn refresh_if_house(owner:&ComposedFreshAdmission<'_,'_,'_>,actor:ExactActorRef,command:UseCommand)->Option<bool>{
    match command.item_fence?.runtime_scope {
        crate::foundation::RuntimeScopeRefV1::Channel{..}=>Some(false),
        crate::foundation::RuntimeScopeRefV1::Instance{..}=>refresh(owner,actor,command).await,
    }
}

async fn record_refusal(owner:&ComposedFreshAdmission<'_,'_,'_>,command:UseCommand,binding:[u8;32],reason:Refusal)->Option<Reply>{
    owner.root.record_hireling_refusal(owner.character,owner.holder,command.item_fence?,*owner.channel_id.as_bytes(),command.command_id,crate::durability::house_hireling::HirelingRefusal{binding,reason}).await.ok()
}

fn reference(command:UseCommand)->Option<CommandRef>{Some(CommandRef::new(command.game_session_id,CommandId::new(command.command_id).ok()?))}

async fn lamp_definition(owner:&ComposedFreshAdmission<'_,'_,'_>)->Option<TypedDefinitionRef>{
    let runtime=owner.runtime.lock().await;
    let index=super::item_ref_admission::pinned_index(&runtime,owner.active_generation)?;
    Some(TypedDefinitionRef{family:"Item".into(),production_key:HIRELING_LAMP_KEY.into(),revision_ref:index.revision(HIRELING_LAMP_KEY)?.into()})
}

/// USE resolves the physical Inbox item inside the native item owner, never a fabricated bag.
pub(super) async fn use_lamp(owner:&ComposedFreshAdmission<'_,'_,'_>,actor:ExactActorRef,command:UseCommand,lamp:[u8;16],input:HirelingLampInput)->Option<UseDisposition>{
    if !resolve_pending_lamp(owner,actor,command.game_session_id).await{return None}
    // Serialize the real position predecessor, lamp commit and projection with
    // native House STEP. Recovery callers acquire no nested lane in refresh.
    let _lane=owner.spell_lane_permit().await?;
    let fence=command.item_fence?;
    if let Some(reply)=pending::replay(owner,actor,command,input).await.ok()? {return Some(reply)}
    if let Some(refused)=super::item_use::hireling_refusal(&*owner.states.spell_states.lock().await,actor,command.game_session_id,command.command_id){return Some(refused)}
    let Some(hireling)=owner.root.hireling_lamp_identity(owner.character,fence,lamp).await.ok()? else {
        record_refusal(owner,command,input.binding(),Refusal::NotOwner).await?;
        return Some(UseDisposition::Rejected)
    };
    if !owner.states.player_trades.physical(command.game_session_id,Some(lamp)){
        record_refusal(owner,command,input.binding(),Refusal::Conflict).await?;
        return Some(UseDisposition::Rejected)
    }
    let state=owner.runtime.lock().await.house_actor_state(actor,command.game_session_id).ok();
    let Some(state)=state else {
        record_refusal(owner,command,input.binding(),Refusal::NotInHouse).await?;
        return Some(UseDisposition::Rejected)
    };
    let (x,y,z)=(u16::try_from(state.position.x).ok()?,u16::try_from(state.position.y).ok()?,i8::try_from(state.position.floor).ok()?);
    let floor=z.checked_neg()?;
    // Crystal hireling_lamp.lua:7-24: the exact player's House tile, never a door tile.
    if let Some(map)=owner.bundle_map(){
        let tile=crate::map::overlay::TilePos{x,y,floor};
        let house_id=super::house_query::source_house_id(&state.house)?;
        if map.facts.house_id(tile)!=Some(house_id)||map.facts.house_door(tile,house_id).is_some(){record_refusal(owner,command,input.binding(),Refusal::Conflict).await?;return Some(UseDisposition::Rejected)}
    }else{
        let cell=crate::content::LogicalCell{x:state.position.x,y:state.position.y,z:i32::from(state.position.floor)};
        let placement=owner.movement_cells.house_tiles().house_at(owner.movement_cells.scope(),cell).ok()??;
        if placement.house().key!=state.house||placement.door_at(cell).is_some(){record_refusal(owner,command,input.binding(),Refusal::Conflict).await?;return Some(UseDisposition::Rejected)}
    }
    let definition=lamp_definition(owner).await?;
    let inventory=owner.root.read_hireling_use_inventory(owner.character,owner.holder,fence,*owner.channel_id.as_bytes()).await.ok()?;
    let reference=reference(command)?;
    let occurred_at_unix_ms=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_millis().try_into().ok()?;
    let request=ItemUseRequest{kind:ItemUseKind::HirelingLamp{origin_channel:*owner.channel_id.as_bytes()},command_id:command.command_id,
        content_revision:inventory.content_revision,ruleset_revision:inventory.ruleset_revision,sim_revision:crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
        transaction_id:command_identity(reference,b"lamp-place"),event_id:command_identity(reference,b"lamp-place-event"),occurred_at_unix_ms,
        server_build_id:crate::durability::character_authority::SERVER_BUILD_ID.into(),source_item_instance_id:lamp,source_definition:definition,source_quantity_before:1,flask:ItemUseFlaskPlan::None};
    let plan=HirelingLampPlacement{hireling,input,house:state.house,origin_channel:*owner.channel_id.as_bytes(),position:(x,y,floor),item_use:request};
    let pending=PendingLampUse::prepared(actor,command,plan)?;
    {let runtime=owner.runtime.lock().await;
        runtime.house_actor_state(actor,command.game_session_id).ok()?;
        if !pending.park(&mut *owner.states.spell_states.lock().await){return None}}
    pending::run_parked(owner,actor,command.game_session_id).await?.disposition()
}

pub(super) async fn replay_lamp(owner:&ComposedFreshAdmission<'_,'_,'_>,actor:ExactActorRef,
    command:UseCommand,input:HirelingLampInput)->Result<Option<UseDisposition>,()> {
    if !resolve_pending_lamp(owner,actor,command.game_session_id).await{return Err(())}
    let _lane=owner.spell_lane_permit().await.ok_or(())?;
    pending::replay(owner,actor,command,input).await
}

pub(super) async fn return_to_lamp(owner:&ComposedFreshAdmission<'_,'_,'_>,actor:ExactActorRef,command:UseCommand,hireling:[u8;16])->Option<Reply>{
    return_to_lamp_input(owner,actor,command,hireling,HirelingReturnInput::Management).await
}
async fn return_to_lamp_input(owner:&ComposedFreshAdmission<'_,'_,'_>,actor:ExactActorRef,command:UseCommand,hireling:[u8;16],input:HirelingReturnInput)->Option<Reply>{
    use crate::durability::character_inbox::{RewardClaimInboxPlan,RewardClaimInboxMintRequest,RewardClaimInboxMintOutput};
    let fence=command.item_fence?;
    if let Some(reply)=owner.root.read_hireling_bound_receipt(owner.character,fence,command.command_id,input.binding(hireling)).await.ok()?{
        refresh_if_house(owner,actor,command).await?;return Some(reply)
    }
    if owner.runtime.lock().await.house_actor_state(actor,command.game_session_id).is_err(){
        return record_refusal(owner,command,input.binding(hireling),Refusal::NotInHouse).await
    }
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    let _lane=owner.spell_lane_permit().await?;
    owner.runtime.lock().await.house_actor_state(actor,command.game_session_id).ok()?;
    let definition=lamp_definition(owner).await?;
    let native=owner.active_generation?.native_gameplay()?;
    let item=super::item_ref_admission::generation_inbox_item_facts(native,&definition).ok()?;
    let inventory=owner.root.read_hireling_use_inventory(owner.character,owner.holder,fence,*owner.channel_id.as_bytes()).await.ok()?;
    let reference=reference(command)?;
    let source=RewardClaimInboxPlan{command:reference,character_id:fence.character_id,world_id:owner.world_id,channel_id:owner.channel_id,
        transaction_id:command_identity(reference,b"lamp-return"),source_binding:input.binding(hireling),item_instance_ids:vec![command_identity(reference,b"lamp-return-item")]};
    let mint=RewardClaimInboxMintRequest{claim:definition,content_revision:inventory.content_revision,ruleset_revision:inventory.ruleset_revision,
        sim_revision:crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),outputs:vec![RewardClaimInboxMintOutput{source_line:0,item,quantity:1,key_number:None}]};
    let reply=owner.root.return_hireling_to_lamp(owner.character,owner.holder,fence,HirelingLampReturn{hireling,input,source,mint},&super::item_ref_admission::NativeInboxItemWeights(native,owner.premium_coordinator)).await.ok()?;
    refresh(owner,actor,command).await?;Some(reply)
}

/// Runs on the existing House disposition/recovery pass, including while the former owner is offline.
pub(super) async fn recall_lost_houses(owner:&ComposedFreshAdmission<'_,'_,'_>)->Option<()> {
    use crate::durability::character_inbox::{RewardClaimInboxMintRequest,RewardClaimInboxMintOutput};
    loop {
        let pending=owner.root.pending_hireling_recalls(owner.character,owner.holder,owner.world_id).await.ok()?;
        if pending.is_empty(){return Some(())}
        let definition=lamp_definition(owner).await?;
        let native=owner.active_generation?.native_gameplay()?;
        let item=super::item_ref_admission::generation_inbox_item_facts(native,&definition).ok()?;
        for pending in pending {
            let mint=RewardClaimInboxMintRequest{claim:definition.clone(),content_revision:pending.content_revision,ruleset_revision:pending.ruleset_revision,
                sim_revision:crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),outputs:vec![RewardClaimInboxMintOutput{source_line:0,item:item.clone(),quantity:1,key_number:None}]};
            owner.root.recall_hireling_to_lamp(owner.character,owner.holder,pending.authority,HirelingLampReturn{hireling:pending.hireling.id,input:HirelingReturnInput::Management,source:pending.source,mint}).await.ok()?;
        }
    }
}
