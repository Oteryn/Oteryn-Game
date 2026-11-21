//! Literal retained-position return. Current occupants and current HP replace
//! historical actor references exactly as the donor's Tile/getSpectators reads do.
use super::*;
impl NativeTimeGuardian {
    pub(super) async fn drain(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        context: &QualifiedCallbackSourceContext,
        eligible_through_us: u64,
        now: u64,
    ) -> Result<(), ()> {
        let mut preview = self.timers.as_ref().ok_or(())?.clone();
        let eligible: Vec<_> = self
            .pending
            .iter()
            .filter(|pending| {
                self.timers
                    .as_ref()
                    .and_then(|timers| timers.occurrence_due(Family, pending.key))
                    .is_some_and(|due| due.get() <= eligible_through_us)
            })
            .map(|pending| pending.key)
            .collect();
        let due = preview.drain_due_matching(
            &Now(now),
            runtime.owner_fence().map_err(|_| ())?,
            |_| true,
            |_, key| eligible.contains(&key),
        );
        for fired in due {
            let pending = self
                .pending
                .iter()
                .find(|pending| pending.key == fired.occurrence)
                .cloned()
                .ok_or(())?;
            return_current(runtime, native, map, context, &pending, now).await?;
            self.timers.as_mut().ok_or(())?.cancel(Family, pending.key);
            self.pending.retain(|entry| entry.key != pending.key);
        }
        Ok(())
    }
}
async fn return_current(
    runtime: &mut ChannelRuntimeV1,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    context: &QualifiedCallbackSourceContext,
    pending: &Pending,
    now: u64,
) -> Result<(), ()> {
    if !exists(map, runtime, pending.parking, context).await? {
        return Ok(());
    }
    let guardian = runtime
        .current_top_actor_on_cell(pending.parking)
        .map_err(|_| ())?;
    // The alternate script explicitly returns on a missing parking top.
    if guardian.is_none() && pending.alternate {
        return Ok(());
    }
    let center = if pending.alternate {
        [32813, 32664, 14]
    } else {
        [32977, 31662, 14]
    };
    let mut phases = Vec::new();
    let mut old = pending.old;
    for (actor, position, session) in runtime.positioned_actor_census().map_err(|_| ())? {
        if session.is_some()
            || (!runtime.matches_live_creature_identity(actor, FIRE)
                && !runtime.matches_live_creature_identity(actor, ICE))
        {
            continue;
        }
        let position = position.position();
        let Some(source) =
            context
                .source_frame()
                .source_position((position.x, position.y, position.floor))
        else {
            return Err(());
        };
        if source[2] == center[2]
            && (source[0] - center[0]).abs() <= 15
            && (source[1] - center[1]).abs() <= 15
        {
            old = position;
            phases.push(actor);
        }
    }
    if phases.is_empty() {
        if let Some(guardian) = guardian {
            if !map.validate_transaction(runtime).await {
                return Err(());
            }
            let snapshot = runtime.companion_snapshot(guardian).map_err(|_| ())?;
            let stamp = runtime.issue_owner_work().map_err(|_| ())?;
            runtime
                .remove_current_native_encounter_subject(
                    stamp,
                    guardian,
                    &snapshot.state.policy.definition_key,
                    native.source_digest(),
                )
                .map_err(|_| ())?;
        }
        return Ok(());
    }
    if !exists(map, runtime, old, context).await? || !map.validate_transaction(runtime).await {
        return Err(());
    }
    // Original Lua reaches its nil guardian error only AFTER teleporting the
    // first remaining phase; the alternate nil guard above has no such effect.
    let Some(guardian) = guardian else {
        map.view()
            .relocate_source_creature(runtime, phases[0], pending.parking)
            .map_err(|_| ())?;
        return Ok(());
    };
    let mut last_sample = None;
    for phase in phases {
        map.view()
            .relocate_source_creature(runtime, phase, pending.parking)
            .map_err(|_| ())?;
        last_sample = Some(
            runtime
                .capture_native_null_health_sample(native, &pending.source, phase)
                .map_err(|_| ())?,
        );
    }
    let sample = last_sample.ok_or(())?;
    let target = runtime.companion_snapshot(guardian).map_err(|_| ())?;
    let occurrence = format!("{}:time-return", pending.occurrence);
    runtime
        .commit_native_null_health_sync_at(
            native,
            &pending.source,
            &sample,
            &target,
            occurrence.as_bytes(),
            &binding(&occurrence, guardian, sample.health()),
            now,
        )
        .map_err(|_| ())?;
    map.view()
        .relocate_source_creature(runtime, guardian, old)
        .map_err(|_| ())?;
    Ok(())
}
