//! Crystal Lesser onSpawn: one actual XML incarnation becomes Greater after
//! 120 seconds. setType does not invoke the replacement's onSpawn callback.
use super::*;

const LESSER: &str = "oteryn:creature.lesser_splinter_of_madness";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PreparedSplinter {
    pub(super) actor: ExactActorRef,
    pub(super) due_us: u64,
}

pub(super) fn next(
    runtime: &ChannelRuntimeV1,
    native: &NativeGameplayState,
) -> Result<Option<PreparedSplinter>, ()> {
    if !matches!(
        runtime.binding().runtime_scope(),
        crate::foundation::RuntimeScopeRefV1::Instance { .. }
    ) || !qualifies_source(runtime, native)
    {
        return Ok(None);
    }
    let mut next: Option<PreparedSplinter> = None;
    for (actor, _) in runtime
        .source_registered_creature_census()
        .map_err(|_| ())?
    {
        if source_actor(runtime, native, actor).as_deref() != Some(LESSER) {
            continue;
        }
        if let Some(due_us) = runtime.soul_war_splinter_due_us(actor).map_err(|_| ())? {
            let plan = PreparedSplinter { actor, due_us };
            if next.is_none_or(|old| {
                (due_us, actor.placement_identity()) < (old.due_us, old.actor.placement_identity())
            }) {
                next = Some(plan);
            }
        }
    }
    Ok(next)
}

/// Scheduling DATA for the shared original-due arbiter, including registered
/// HP-zero actors. An unavailable birth never silently becomes epoch zero.
pub(in crate::gameplay_transport) fn next_due_us(
    runtime: &ChannelRuntimeV1,
    native: &NativeGameplayState,
) -> Result<Option<u64>, ()> {
    Ok(next(runtime, native)?.map(|plan| plan.due_us))
}

pub(super) fn publish(
    runtime: &mut ChannelRuntimeV1,
    native: &NativeGameplayState,
    prepared: PreparedSplinter,
    now_us: u64,
) -> Result<(), ()> {
    if prepared.due_us > now_us || next(runtime, native)? != Some(prepared) {
        return Err(());
    }
    let before = runtime
        .companion_snapshot_including_dead(prepared.actor)
        .map_err(|_| ())?;
    let change = runtime
        .prepare_soul_war_splinter_profile_change(&before, prepared.due_us)
        .map_err(|_| ())?;
    runtime
        .commit_creature_profile_transformation(change)
        .map_err(|_| ())?;
    Ok(())
}
