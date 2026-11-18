use super::fixture::debug;

use super::TestResult;

/// Setup drains actual retained work at one execution clock. Equal deadlines
/// alone cannot show progress: preserve source IDs and the real Profile cursor.
pub(super) async fn drain_actual_native_through(
    view: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    now: u64,
    finish_thinks: bool,
) -> TestResult {
    let now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now);
    for turn in 0..256 {
        let (due, before) = actual_native_progress(view, now.get(), finish_thinks).await?;
        if !due {
            return Ok(());
        }
        Box::pin(view.drain_monster_melee_at(now)).await;
        let (due, after) = actual_native_progress(view, now.get(), finish_thinks).await?;
        if !due {
            return Ok(());
        }
        if before == after {
            return Err(format!(
                "actual native predecessor made no progress at fixed clock {}; turn={turn}; {after}",
                now.get()
            ).into());
        }
    }
    Err("actual native predecessor drain exceeded its finite 256-turn fixture bound".into())
}

async fn actual_native_progress(
    view: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    now: u64,
    finish_thinks: bool,
) -> TestResult<(bool, String)> {
    let native = view
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("actual native generation for predecessor observation")?;
    let runtime = view.runtime.lock().await;
    let states = view.states.spell_states.lock().await;
    let original =
        crate::gameplay_transport::source_death::next_source_work(&runtime, &states, native)
            .map_err(|_| "actual original owner deadline unavailable")?;
    let profile = states.native_profile_pass.pending().map(|pending| {
        (
            format!("{:p}", std::sync::Arc::as_ptr(&pending)),
            format!("{pending:?}"),
        )
    });
    let normal = states.primal_callbacks.normal_pending().map(|pending| {
        (
            format!("{:p}", std::sync::Arc::as_ptr(&pending)),
            pending.batch().observed_at_us(),
            pending.source_known(),
            pending.source_ready(),
            pending.ai_known(),
        )
    });
    let deferred = states
        .primal_callbacks
        .deferred_original_deadlines_for_test()
        .map_err(|_| "actual retained Primal owner unavailable")?;
    let pods = states
        .primal_callbacks
        .pods
        .iter()
        .map(|pod| (pod.source_transaction_id(), pod.due_us()))
        .collect::<Vec<_>>();
    let chains = states
        .monster_chains
        .as_ref()
        .map(|owner| owner.pending_sources())
        .transpose()
        .map_err(debug)?
        .unwrap_or_default()
        .into_iter()
        .map(|pending| {
            (
                pending.source,
                pending.proposal,
                pending.session,
                pending.due,
                pending.from,
            )
        })
        .collect::<Vec<_>>();
    let due = original.is_some_and(|(_, due)| due <= now)
        || profile.is_some()
        || normal.is_some()
        || runtime.has_pending_source_creature_deaths()
        || (finish_thinks
            && (states.next_monster_ai_pass_us <= now
                || states
                    .monster_melee
                    .creature_ai
                    .earliest_due()
                    .is_some_and(|due| due <= now)));
    Ok((
        due,
        format!(
            "original={original:?}; profile={profile:?}; normal={normal:?}; deferred={deferred:?}; pods={pods:?}; executions={:?}; expiries={:?}; native={:?}; steps={:?}; windups={:?}; chains={chains:?}; ai={:?}; next_pass={}; ai_sequence={}",
            states.primal_callbacks.pod_execution_deadlines_for_test(),
            states.primal_callbacks.queued_pod_expiry_sources_for_test(),
            states.native_callbacks,
            states.spite_steps.creatures,
            states.monster_spells.pending_windups(),
            states.monster_melee.creature_ai,
            states.next_monster_ai_pass_us,
            states.monster_ai_sequence
        ),
    ))
}
