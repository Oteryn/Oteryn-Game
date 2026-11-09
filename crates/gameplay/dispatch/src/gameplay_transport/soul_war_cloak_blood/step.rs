//! Native blood entry uses the same retained Item/HP delivery as Primal pods.
use super::super::{ComposedFreshAdmission, actor_spell::ChannelSpellStates};
use crate::durability::creature_source_items::{CloakBloodPlayerStep, cloak_blood_percentage};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementPositionSnapshot};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Debug)]
pub(in crate::gameplay_transport) struct PendingBloodStep {
    source: CloakBloodPlayerStep,
    actor: ExactActorRef,
    session: GameSessionId,
    maximum_health: u32,
    cell: (i32, i32, i16),
    applied: AtomicUsize,
}

#[allow(
    clippy::too_many_arguments,
    reason = "the actual movement owner supplies its sealed destination read and committed position"
)]
pub(in crate::gameplay_transport) fn stage(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &crate::content::QualifiedNativeEntryRoom,
    native: &crate::content::native_gameplay::NativeGameplayState,
    read: &crate::durability::spell_item_transaction::StandingTileRead,
    actor: ExactActorRef,
    session: GameSessionId,
    committed: MovementPositionSnapshot,
    now: u64,
) -> Result<(), crate::durability::DurabilityError> {
    if !read
        .items()
        .iter()
        .any(|item| cloak_blood_percentage(&item.definition.production_key).is_some())
    {
        return Ok(());
    }
    let source = CloakBloodPlayerStep::from_current_player_step(
        runtime, room, native, actor, session, committed, now, read,
    )
    .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
    let maximum_health = states
        .get(runtime, actor, session)
        .ok_or(crate::durability::DurabilityError::Unavailable)?
        .vitals()
        .max_health;
    states.cloak_blood_steps.push(Arc::new(PendingBloodStep {
        source,
        actor,
        session,
        maximum_health,
        cell: (
            committed.position().x,
            committed.position().y,
            committed.position().floor,
        ),
        applied: AtomicUsize::new(0),
    }));
    states.owner_wake.notify_one();
    Ok(())
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn resolve_cloak_blood_steps(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> bool {
        for _ in 0..32 {
            let pending = self
                .states
                .spell_states
                .lock()
                .await
                .cloak_blood_steps
                .first()
                .map(Arc::clone);
            let Some(pending) = pending else { return true };
            let receipt = match self
                .root
                .commit_cloak_blood_step(permit, self.character, self.holder, &pending.source)
                .await
            {
                Ok(receipt) => receipt,
                Err(error) => {
                    #[cfg(test)]
                    eprintln!("Cloak blood STEP commit: {error:?}");
                    #[cfg(not(test))]
                    let _ = error;
                    return false;
                }
            };
            self.states.loop_ground.invalidate();
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let Some(index) = states
                .cloak_blood_steps
                .iter()
                .position(|step| Arc::ptr_eq(step, &pending))
            else {
                continue;
            };
            if let Some(receipt) = receipt {
                let Some(native) = self
                    .active_generation
                    .and_then(|active| active.native_gameplay())
                else {
                    return false;
                };
                for (event, percent) in receipt
                    .percentages()
                    .iter()
                    .enumerate()
                    .skip(pending.applied.load(Ordering::Relaxed))
                {
                    let damage = u32::try_from(
                        u64::from(pending.maximum_health) * u64::from(*percent) / 100,
                    );
                    let Ok(damage) = damage else { return false };
                    let hit = states.apply_soul_war_blood_loss(
                        &runtime,
                        native,
                        pending.actor,
                        pending.session,
                        damage,
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                            self.owner_now().get(),
                        ),
                    );
                    if hit.is_none() && !states.is_dead(pending.actor) {
                        return false;
                    }
                    pending.applied.store(event + 1, Ordering::Relaxed);
                }
            }
            states.cloak_blood_steps.remove(index);
            states.cloak_blood_cells.remove(&pending.cell);
        }
        self.states
            .spell_states
            .lock()
            .await
            .cloak_blood_steps
            .is_empty()
    }
}
