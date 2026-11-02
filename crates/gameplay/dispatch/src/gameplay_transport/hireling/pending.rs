//! Frozen native lamp USE work belongs to the existing actor spell owner.
//! An unknown commit or cancelled projection never becomes a cached wire refusal.
use super::*;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::house_hireling::HirelingReceiptRead;
use crate::foundation::GameSessionId;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;

#[derive(Debug, Clone)]
pub(in crate::gameplay_transport) struct PendingLampUse {
    pub(in crate::gameplay_transport) actor: ExactActorRef,
    pub(in crate::gameplay_transport) session: GameSessionId,
    fence: CurrentCharacterItemFence,
    command_id: u64,
    input: HirelingLampInput,
    work: Work,
}

#[derive(Debug,Clone)]
enum Work { ReceiptProbe, Place(Box<HirelingLampPlacement>), ProjectCommitted }

pub(super) enum Resolution { Decided(UseDisposition), NoReceipt }
impl Resolution {
    pub(super) fn disposition(self)->Option<UseDisposition>{
        match self {Self::Decided(result)=>Some(result),Self::NoReceipt=>None}
    }
}

impl PendingLampUse {
    pub(in crate::gameplay_transport) fn prepared(
        actor: ExactActorRef, command: UseCommand, plan: HirelingLampPlacement,
    ) -> Option<Self> {
        let fence=command.item_fence?;
        (fence.game_session_id==command.game_session_id
            && plan.item_use.command_id==command.command_id).then_some(Self {
                actor,session:command.game_session_id,fence,command_id:command.command_id,
                input:plan.input,work:Work::Place(Box::new(plan)),
            })
    }

    fn committed(actor: ExactActorRef, command: UseCommand, input: HirelingLampInput) -> Option<Self> {
        let fence=command.item_fence?;
        (fence.game_session_id==command.game_session_id).then_some(Self {
                actor,session:command.game_session_id,fence,command_id:command.command_id,input,work:Work::ProjectCommitted,
        })
    }

    /// The caller has the same lane as native House STEP and takes runtime before states.
    pub(in crate::gameplay_transport) fn park(self, states: &mut ChannelSpellStates) -> bool {
        if states.has_pending_spell_commit(self.actor,self.session)
            || !states.actors.iter().any(|(actor,session,_)|*actor==self.actor&&*session==self.session)
            || states.pending_hireling_lamps.try_reserve(1).is_err() {return false;}
        states.pending_hireling_lamps.push(self);
        true
    }

    fn command(&self) -> UseCommand {
        UseCommand {game_session_id:self.session,command_id:self.command_id,item_fence:Some(self.fence)}
    }
}

/// The caller holds the lane. A receipt is authoritative before current item selection,
/// including when an earlier return has retired/replaced the source lamp.
pub(super) async fn replay(
    owner:&ComposedFreshAdmission<'_,'_,'_>,actor:ExactActorRef,command:UseCommand,input:HirelingLampInput,
) -> Result<Option<UseDisposition>,()> {
    let fence=command.item_fence.ok_or(())?;
    let reply=owner.root.read_hireling_receipt_state(owner.character,fence,command.command_id,input.binding()).await;
    match reply {
        Ok(HirelingReceiptRead::Missing)=>Ok(None),
        Ok(HirelingReceiptRead::Found(Reply::Committed))|Err(_)=>{
            let committed=matches!(reply,Ok(HirelingReceiptRead::Found(Reply::Committed)));
            let mut pending=PendingLampUse::committed(actor,command,input).ok_or(())?;
            if !committed {pending.work=Work::ReceiptProbe;}
            let runtime=owner.runtime.lock().await;
            runtime.house_actor_state(actor,command.game_session_id).map_err(|_|())?;
            let mut states=owner.states.spell_states.lock().await;
            if !pending.park(&mut states){return Err(())}
            drop(states);drop(runtime);
            if committed {finish(owner,actor,command.game_session_id,Reply::Committed).await.map(Some).ok_or(())}
                else {Err(())}
        }
        Ok(HirelingReceiptRead::Conflict|HirelingReceiptRead::Found(_))=>Ok(Some(UseDisposition::Rejected)),
    }
}

/// Do not acquire the lane again: initial USE and the resolver both already hold it.
pub(super) async fn run_parked(
    owner:&ComposedFreshAdmission<'_,'_,'_>,actor:ExactActorRef,session:GameSessionId,
) -> Option<Resolution> {
    let pending=owner.states.spell_states.lock().await.pending_hireling_lamps.iter()
        .find(|p|p.actor==actor&&p.session==session).cloned()?;
    let reply=match owner.root.read_hireling_receipt_state(owner.character,pending.fence,
        pending.command_id,pending.input.binding()).await.ok()? {
        HirelingReceiptRead::Found(reply)=>reply,
        HirelingReceiptRead::Conflict=>Reply::Refused(Refusal::Conflict),
        HirelingReceiptRead::Missing=>{
            if matches!(pending.work,Work::ReceiptProbe) {
                // No value was attempted: the failed read had only prevented an unsafe ack.
                remove(owner,actor,session,pending.command_id).await?;
                return Some(Resolution::NoReceipt);
            }
            let Work::Place(plan)=&pending.work else{return None};
            // An uncommitted retry retains its real position predecessor. Committed replay
            // deliberately skips this check and publishes the present durable House state.
            let current={let runtime=owner.runtime.lock().await;
                runtime.house_actor_state(actor,session).ok().is_some_and(|state|
                    state.scope==pending.fence.runtime_scope
                    && state.generation==pending.fence.scope_ownership_generation
                    && state.lease.character_id().as_bytes()==pending.fence.character_id.as_bytes()
                    && state.lease.generation()==pending.fence.character_lease_generation
                    && state.house==plan.house
                    && state.position.x==i32::from(plan.position.0)
                    && state.position.y==i32::from(plan.position.1)
                    && state.position.floor==-i16::from(plan.position.2))};
            if !current{return None}
            match owner.root.commit_hireling_lamp(owner.character,owner.holder,pending.fence,(**plan).clone()).await {
                Ok(reply)=>reply,
                // This read serializes behind the original writer's same CommandRef lock.
                // Anything still unresolved keeps the original plan and the wire open.
                Err(_)=>match owner.root.read_hireling_receipt_state(owner.character,pending.fence,
                    pending.command_id,pending.input.binding()).await.ok()? {
                    HirelingReceiptRead::Found(reply)=>reply,
                    HirelingReceiptRead::Conflict=>Reply::Refused(Refusal::Conflict),
                    HirelingReceiptRead::Missing=>return None,
                },
            }
        }
    };
    finish(owner,actor,session,reply).await.map(Resolution::Decided)
}

async fn finish(owner:&ComposedFreshAdmission<'_,'_,'_>,actor:ExactActorRef,session:GameSessionId,
    reply:Reply)->Option<UseDisposition> {
    let (command,input)={let states=owner.states.spell_states.lock().await;
        let pending=states.pending_hireling_lamps.iter().find(|p|p.actor==actor&&p.session==session)?;
        (pending.command(),pending.input)};
    // The receipt decides the result. Current ACL can independently refuse a projection
    // read after a House transfer/revocation; it must not turn Committed into Rejected or
    // keep known work blocking exit. Cancellation during the read still retains the marker.
    if reply==Reply::Committed {let _=refresh(owner,actor,command).await;}
    let mut states=owner.states.spell_states.lock().await;
    let index=states.pending_hireling_lamps.iter().position(|p|p.actor==actor&&p.session==session
        && p.command_id==command.command_id)?;
    let disposition=if reply==Reply::Committed{UseDisposition::Committed}else{UseDisposition::Rejected};
    // Keep the frozen work until its original ingress reservation owns the proven
    // verdict. An active stream retains it for cancellation recovery and publishes
    // normally; a disconnected stream completes it now. Retention/completion and
    // marker removal have no await between them.
    let reference=reference(command)?;
    match input {
        HirelingLampInput::LegacySummon{..}=>owner.states.command_ingress
            .complete_hireling_summon(reference,reply==Reply::Committed)?,
        HirelingLampInput::ItemInstance(_)|HirelingLampInput::ItemDefinition(_)=>owner.states.command_ingress
            .complete_item_use(reference,disposition)?,
    }
    states.pending_hireling_lamps.swap_remove(index);
    super::super::item_use::remember_hireling_result(&mut states,actor,session,command.command_id,disposition);
    Some(disposition)
}

async fn remove(owner:&ComposedFreshAdmission<'_,'_,'_>,actor:ExactActorRef,session:GameSessionId,
    command:u64)->Option<()> {
    let mut states=owner.states.spell_states.lock().await;
    let index=states.pending_hireling_lamps.iter().position(|p|p.actor==actor&&p.session==session&&p.command_id==command)?;
    states.pending_hireling_lamps.swap_remove(index);Some(())
}

pub(in crate::gameplay_transport) async fn resolve(
    owner:&ComposedFreshAdmission<'_,'_,'_>,actor:ExactActorRef,session:GameSessionId,
)->bool {
    if !owner.states.spell_states.lock().await.pending_hireling_lamps.iter()
        .any(|p|p.actor==actor&&p.session==session){return true}
    let Some(_lane)=owner.spell_lane_permit().await else{return false};
    run_parked(owner,actor,session).await.is_some()
}

/// The existing ingress owner must know an outstanding command's outcome before
/// another stream opens. Only its exact parked native occurrence is eligible;
/// a saved caller counter cannot create or replace terminal/high-water truth.
pub(in crate::gameplay_transport) async fn recover_ingress(
    owner:&ComposedFreshAdmission<'_,'_,'_>,admitted:&AdmittedSession,
)->bool {
    let Some(actor)=admitted.runtime_actor else{return true};
    let pending={let states=owner.states.spell_states.lock().await;
        states.pending_hireling_lamps.iter().find(|p|p.actor==actor&&p.session==admitted.game_session_id).cloned()};
    let Some(pending)=pending else{return true};
    let Some(reference)=reference(pending.command()) else{return false};
    if !owner.states.command_ingress.pending_reconciliation(reference){return true}
    let retry=super::super::AdmissionRetryBudget::for_command(owner.root);
    if !retry.retry_until(async || resolve(owner,actor,admitted.game_session_id).await).await{return false}
    // A missing probe is not a terminal verdict. Preserve the unresolved ingress
    // rather than fabricating a refusal or executing replacement client bytes.
    !owner.states.command_ingress.pending_reconciliation(reference)
}
