//! Source Soulpit entry, world cadence and return through the shared Instance owner.
use super::runtime::{
    self, ActivityFuture, ActivityHandoffOwnerRuntime, ActivityHandoffScene,
    SoulpitActivityBinding, SoulpitKernel, SoulpitRuntimeError,
};
use super::{BossActivityHandoff, SoulpitEntryIntent, outcome};
use crate::content::{LogicalCell, NativeEntryMovementCells};
use crate::durability::activity_handoff::{
    ActivityContentPin, ActivityEnterRequest, ActivityHandoffOwner, ActivityParticipant,
    ActivityReturnReason,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::soulpit::{
    SOULPIT_ACTIVITY_KEY, SoulpitAction, SoulpitEntryRequest, SoulpitParticipantFact,
    SoulpitStepOutcome, SoulpitStepRequest, SoulpitStoredEncounter,
};
use crate::foundation::{
    CharacterId, CommandId, CommandRef, ExactActorRef, GameSessionId, MovementLocalPosition,
    RuntimeScopeRefV1, ScopeOwnershipGeneration,
};
use crate::gameplay_transport::activity_runtime::{
    ActivityRuntimeAssets, ActivityTemplateKey, ScopeKey,
};
use crate::gameplay_transport::connection::{ConnectionIdentifiers, UseOutcome};
use crate::gameplay_transport::native_activity::NativeActivityOutcome;
use crate::gameplay_transport::{ComposedFreshAdmission, SeamRooms, SecureIdentifiers};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::{future::Future, pin::Pin};

#[derive(Clone)]
pub(in crate::gameplay_transport) struct SoulpitRun {
    pub binding: SoulpitActivityBinding,
    pub members: Vec<(CharacterId, u8)>,
}
#[derive(Clone)]
pub(in crate::gameplay_transport) struct PreparedEntry {
    pub actor: ExactActorRef,
    pub entry: ActivityEnterRequest,
    pub core: SoulpitEntryRequest,
    pub refused: bool,
}
#[derive(Default)]
pub(in crate::gameplay_transport) struct ProductionSoulpitHandoff {
    #[cfg(test)]
    test_handoff_id: Option<[u8; 16]>,
}

impl ProductionSoulpitHandoff {
    #[cfg(test)]
    pub(in crate::gameplay_transport) fn with_test_handoff_id(id: [u8; 16]) -> Result<Self, ()> {
        if id[6] >> 4 != 7 || id[8] >> 6 != 2 {
            return Err(());
        }
        Ok(Self {
            test_handoff_id: Some(id),
        })
    }
}

impl BossActivityHandoff for ProductionSoulpitHandoff {
    fn enter_soulpit<'a>(
        &'a self,
        admission: &'a ComposedFreshAdmission<'_, '_, '_>,
        request: SoulpitEntryIntent,
    ) -> Pin<Box<dyn Future<Output = Result<Option<UseOutcome>, ()>> + Send + 'a>> {
        #[cfg(test)]
        let handoff_id = self.test_handoff_id;
        #[cfg(not(test))]
        let handoff_id = None;
        Box::pin(enter(admission, request, handoff_id))
    }
    fn drive_soulpit<'a>(
        &'a self,
        admission: &'a ComposedFreshAdmission<'_, '_, '_>,
        now: u64,
        global: bool,
    ) -> Pin<Box<dyn Future<Output = Result<Option<u64>, ()>> + Send + 'a>> {
        Box::pin(async move {
            let _cycle = admission.activity_runtimes.soulpit_cycle.lock().await;
            let runs = admission
                .activity_runtimes
                .soulpit_runs
                .lock()
                .map_err(|_| ())?
                .values()
                .cloned()
                .collect::<Vec<_>>();
            let mut deadline = None;
            for run in runs {
                let key = ScopeKey::new(run.binding.scope, run.binding.generation.get());
                let Some(registered) = admission
                    .activity_runtimes
                    .get(run.binding.scope, run.binding.generation.get())
                else {
                    admission
                        .activity_runtimes
                        .soulpit_runs
                        .lock()
                        .map_err(|_| ())?
                        .remove(&key);
                    continue;
                };
                let view = admission.runtime_view(&registered);
                let mut owner = LiveRuntime {
                    admission: &view,
                    run,
                    survivors: Vec::new(),
                    now,
                };
                let result = if global {
                    runtime::drive_soulpit_activity(&mut owner, now).await
                } else {
                    runtime::drive_soulpit_deadlines(&mut owner, now).await
                };
                match result {
                    Ok(Some(due)) => deadline = Some(deadline.map_or(due, |old: u64| old.min(due))),
                    Ok(None) => {
                        admission
                            .activity_runtimes
                            .soulpit_runs
                            .lock()
                            .map_err(|_| ())?
                            .remove(&key);
                    }
                    Err(error) => {
                        // A CAS race or transient holder does not retire the unrelated origin
                        // Channel. Reconcile this same physical Instance on its next cadence.
                        crate::gameplay_transport::operator_event(&format!(
                            "soulpit_drive_deferred reason={error:?}"
                        ));
                        let retry = now.saturating_add(100_000);
                        deadline = Some(deadline.map_or(retry, |old| old.min(retry)));
                    }
                }
            }
            Ok(deadline)
        })
    }
    fn return_from_soulpit<'a, 'assets: 'a, 'authority: 'a, 'services: 'a>(
        &'a self,
        admission: &'a ComposedFreshAdmission<'assets, 'authority, 'services>,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        target: oteryn_protocol_oteryn::world_object::WorldObjectTarget,
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>> {
        Box::pin(return_one(admission, actor, session, command_id, target))
    }
}

async fn return_one<'a, 'f, 's>(
    admission: &ComposedFreshAdmission<'a, 'f, 's>,
    actor: ExactActorRef,
    session: GameSessionId,
    command_id: u64,
    target: oteryn_protocol_oteryn::world_object::WorldObjectTarget,
) -> Result<(), ()> {
    if command_id == 0 {
        return Err(());
    }
    let admitted = admission.activity_runtimes.connection(session).ok_or(())?;
    if admitted.runtime_actor != Some(actor) {
        return Err(());
    }
    let fence = admitted.item_fence.ok_or(())?;
    let key = ScopeKey::new(fence.runtime_scope, fence.scope_ownership_generation.get());
    let run = admission
        .activity_runtimes
        .soulpit_runs
        .lock()
        .map_err(|_| ())?
        .get(&key)
        .cloned()
        .ok_or(())?;
    if !run
        .members
        .iter()
        .any(|(character, _)| character.as_bytes() == fence.character_id.as_bytes())
    {
        return Err(());
    }
    admission.queue_soulpit_return(SoulpitReturnIntent {
        scope: fence.runtime_scope,
        generation: fence.scope_ownership_generation.get(),
        session,
        admitted,
        command: Some(command_id),
        exit_target: Some(target),
        reason: ActivityReturnReason::Exit,
    })
}

fn in_arena(at: MovementLocalPosition) -> bool {
    at.floor == 8 && (32362..=32390).contains(&at.x) && (31132..=31153).contains(&at.y)
}

mod entry;
mod live;
mod returning;
use entry::enter;
use live::LiveRuntime;
pub(in crate::gameplay_transport) use returning::SoulpitReturnIntent;
