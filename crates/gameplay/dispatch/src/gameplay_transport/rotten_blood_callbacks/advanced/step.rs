//! Crystal teleport_step_event.lua:26–227. Only a committed Player STEP can
//! consume this source-created action50001 Item. SQL work remains queued until
//! its immutable receipt is reconciled; cancellation never loses the ingress.
use super::*;
use crate::durability::{
    creature_source_items::RottenPortalCallback, spell_owner_commit::SpellLanePermit,
};
use crate::foundation::MovementPositionSnapshot;

#[derive(Debug)]
pub(super) struct PendingStep {
    callback: Arc<RottenPortalCallback>,
    centre: RoomPosition,
    elders: Vec<ExactActorRef>,
    actor: ExactActorRef,
    session: GameSessionId,
}
pub(super) fn pending_for(
    steps: &[PendingStep],
    actor: ExactActorRef,
    session: GameSessionId,
) -> bool {
    steps
        .iter()
        .any(|step| step.actor == actor && step.session == session)
}
pub(super) fn ready(steps: &[PendingStep], now: u64) -> Option<Arc<RottenPortalCallback>> {
    steps
        .iter()
        .find(|step| step.callback.due_us() <= now)
        .map(|step| Arc::clone(&step.callback))
}

pub(in crate::gameplay_transport) async fn step_in(
    owner: &super::super::super::ComposedFreshAdmission<'_, '_, '_>,
    permit: &SpellLanePermit,
    actor: ExactActorRef,
    session: GameSessionId,
    committed: MovementPositionSnapshot,
    now: u64,
) {
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let advanced = &mut states.rotten_blood_callbacks.advanced;
        #[cfg(test)]
        eprintln!(
            "SRX3_ROTTEN_PORTAL_STEP_INGRESS at={:?} active={:?} pending={}",
            committed.position(),
            advanced
                .active_portals
                .iter()
                .map(|p| p.position())
                .collect::<Vec<_>>(),
            advanced.steps.len()
        );
        let Some(source) = advanced
            .active_portals
            .iter()
            .find(|p| {
                p.position() == committed.position()
                    && !advanced
                        .steps
                        .iter()
                        .any(|s| s.callback.position() == p.position())
            })
            .cloned()
        else {
            return;
        };
        if advanced.portals.len() >= 32
            || advanced.steps.len() >= 8
            || advanced.massive.len() + 2 * advanced.steps.len() + 2 > massive::MAX_PENDING
        {
            return;
        }
        let centre = pos(committed.position());
        let Ok(census) = runtime.positioned_actor_census() else {
            return;
        };
        let elders: Vec<_> = census
            .iter()
            .filter_map(|(candidate, at, player)| {
                (player.is_none()
                    && near(centre, pos(at.position()), 15)
                    && runtime.companion_snapshot(*candidate).is_ok_and(|s| {
                        s.health > 0 && s.state.policy.definition_key == vem::ELDER_BLOODJAW_KEY
                    }))
                .then_some((*candidate, pos(at.position())))
            })
            .collect();
        if !elders.iter().any(|(_, at)| near(centre, *at, 2)) {
            #[cfg(test)]
            eprintln!("SRX3_ROTTEN_PORTAL_STEP_NO_NEAR_ELDER centre={centre:?} elders={elders:?}");
            return;
        }
        let Ok(callback) =
            source.from_committed_player_step(&runtime, actor, session, committed, now)
        else {
            #[cfg(test)]
            eprintln!("SRX3_ROTTEN_PORTAL_STEP_SOURCE_REFUSED");
            return;
        };
        let callback = Arc::new(callback);
        advanced.steps.push(PendingStep {
            callback: Arc::clone(&callback),
            centre,
            elders: elders.into_iter().map(|(a, _)| a).collect(),
            actor,
            session,
        });
        advanced.portals.push(callback);
        states.owner_wake.notify_one();
    }
    owner.resolve_rotten_portal_callbacks(permit).await;
}

pub(super) fn committed(
    owner: &super::super::super::ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    callback: &Arc<RottenPortalCallback>,
) -> Result<(), ()> {
    let Some(index) = states
        .rotten_blood_callbacks
        .advanced
        .steps
        .iter()
        .position(|s| Arc::ptr_eq(&s.callback, callback))
    else {
        return Ok(());
    };
    let step = &states.rotten_blood_callbacks.advanced.steps[index];
    let source = Arc::clone(&step.callback);
    let centre = step.centre;
    let elders = step.elders.clone();
    if states.rotten_blood_callbacks.advanced.massive.len() + 2 > massive::MAX_PENDING {
        return Err(());
    }
    massive::deadlines(source.due_us())?;
    for elder in &elders {
        if runtime.contains_live_creature(*elder) {
            runtime.remove_encounter_actor(*elder).map_err(|_| ())?;
        }
    }
    // These are source-global flags scoped to this exact registered Instance.
    // Nil boss/target leaves both flags paused, exactly as the early Lua returns.
    massive::set_paused(&mut states.rotten_blood_callbacks.advanced, centre, true);
    massive::schedule(owner, runtime, states, source, centre)?;
    states.rotten_blood_callbacks.advanced.steps.remove(index);
    Ok(())
}
