//! Retained canonical onMove callbacks and CURRENT-item addEvent continuations.
use super::*;
use crate::durability::creature_source_items::RottenFootprintCallback;
use oteryn_gameplay_entities::carrier::native_translation_events::CommittedRottenTranslation;
#[derive(Debug)]
pub(super) struct Pending {
    callback: Arc<RottenFootprintCallback>,
    movement: Option<CommittedRottenTranslation>,
}
impl Pending {
    pub(super) fn blocking(&self) -> bool {
        self.movement.is_some() || self.callback.execution_started()
    }
    pub(super) fn due_us(&self) -> u64 {
        self.callback.due_us()
    }
}
impl super::super::RottenBloodCallbacks {
    pub(in crate::gameplay_transport) fn movement_callbacks_pending(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> bool {
        runtime.rotten_movement_pending() || self.advanced.footprints.iter().any(Pending::blocking)
    }
}
impl super::super::super::ComposedFreshAdmission<'_, '_, '_> {
    pub(crate) async fn drain_rotten_movement_callbacks(&self) {
        let Some(permit) = self.spell_lane_permit().await else {
            return;
        };
        self.resolve_rotten_movement_callbacks(&permit).await;
    }
    /// True only after every synchronous onMove and started callback has a
    /// known physical outcome. The caller retains its ORIGINAL source cursor
    /// when false; it must not execute a later Item callback or teleport.
    pub(in crate::gameplay_transport) async fn resolve_rotten_movement_callbacks(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> bool {
        self.resolve_rotten_movement_callbacks_inner(permit).await;
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        !states
            .rotten_blood_callbacks
            .movement_callbacks_pending(&runtime)
    }
    async fn resolve_rotten_movement_callbacks_inner(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) {
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return;
        };
        for _ in 0..32 {
            let now = self.owner_now().get();
            let (callback, prefix, binding) = {
                let runtime = self.runtime.lock().await;
                let mut states = self.states.spell_states.lock().await;
                let prefix = match super::super::source_prefix(
                    &runtime,
                    &states,
                    native,
                    DueKind::Footprint,
                    now,
                ) {
                    Ok(prefix) => prefix,
                    Err(()) => return,
                };
                let advanced = &mut states.rotten_blood_callbacks.advanced;
                // Preserve the original physical movement event until a known
                // Item receipt. No reconstruction from the mover's later pose.
                let events = match runtime.pending_rotten_movement(16) {
                    Ok(events) => events,
                    Err(_) => return,
                };
                for event in events {
                    if prefix
                        .as_ref()
                        .is_some_and(|prefix| event.observed_us() > prefix.through)
                    {
                        continue;
                    }
                    if advanced
                        .footprints
                        .iter()
                        .any(|pending| pending.movement.as_ref() == Some(&event))
                    {
                        continue;
                    }
                    if advanced.footprints.len() >= 1024
                        || advanced.footprints.try_reserve(1).is_err()
                    {
                        return;
                    }
                    let Some(room) = self.effective_rooms().gate_wire_position() else {
                        return;
                    };
                    let callback = match RottenFootprintCallback::from_committed_translation(
                        &runtime, room, native, &event,
                    ) {
                        Ok(callback) => callback,
                        Err(error) => {
                            #[cfg(test)]
                            eprintln!("SRX3_ROTTEN_MOVE_SOURCE_REFUSED {error:?}");
                            #[cfg(not(test))]
                            let _ = error;
                            return;
                        }
                    };
                    advanced.footprints.push(Pending {
                        callback: Arc::new(callback),
                        movement: Some(event),
                    });
                }
                let selected = advanced
                    .footprints
                    .iter()
                    .enumerate()
                    .filter(|(_, pending)| {
                        pending.due_us() <= now
                            && prefix
                                .as_ref()
                                .is_none_or(|prefix| pending.due_us() <= prefix.through)
                    })
                    .min_by_key(|(_, pending)| pending.due_us())
                    .map(|(index, _)| index);
                let callback = if let Some(index) = selected {
                    let started = match advanced.footprints[index].callback.begin_execution(now) {
                        Ok(callback) => callback,
                        Err(_) => return,
                    };
                    advanced.footprints[index].callback = Arc::new(started);
                    Some(Arc::clone(&advanced.footprints[index].callback))
                } else {
                    None
                };
                (callback, prefix, super::super::context(&runtime))
            };
            let Some(mut callback) = callback else { return };
            if callback.needs_intrinsic_decay_clock() {
                let sample = match self
                    .root
                    .sample_rotten_footprint_clock(permit, self.character, self.holder, &callback)
                    .await
                {
                    Ok(sample) => sample,
                    Err(_) => return,
                };
                // Pair the fenced epoch sample with this SAME owner clock as
                // soon as the read returns. Retain before any physical write.
                let sampled_owner_us = self.owner_now().get();
                let started = match callback.with_intrinsic_decay_clock(sample, sampled_owner_us) {
                    Ok(started) => Arc::new(started),
                    Err(_) => return,
                };
                let mut states = self.states.spell_states.lock().await;
                let Some(pending) = states
                    .rotten_blood_callbacks
                    .advanced
                    .footprints
                    .iter_mut()
                    .find(|pending| Arc::ptr_eq(&pending.callback, &callback))
                else {
                    return;
                };
                pending.callback = Arc::clone(&started);
                callback = started;
            }
            let outcome = match self
                .root
                .commit_rotten_footprint_callback(
                    permit,
                    self.character,
                    self.holder,
                    &callback,
                    now,
                )
                .await
            {
                Ok(outcome) => outcome,
                Err(error) => {
                    #[cfg(test)]
                    eprintln!(
                        "SRX3_ROTTEN_MOVE_ITEM_REFUSED at={:?} create={} {error:?}",
                        callback.position(),
                        callback.is_create()
                    );
                    #[cfg(not(test))]
                    let _ = error;
                    return;
                }
            };
            self.states.loop_ground.invalidate();
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if runtime.owner_fence().is_err()
                || binding != super::super::context(&runtime)
                || prefix
                    .as_ref()
                    .is_some_and(|prefix| !prefix.current(&runtime, &states))
            {
                return;
            }
            let advanced = &mut states.rotten_blood_callbacks.advanced;
            let Some(index) = advanced
                .footprints
                .iter()
                .position(|pending| Arc::ptr_eq(&pending.callback, &callback))
            else {
                return;
            };
            if let Some(event) = advanced.footprints[index].movement.as_ref()
                && runtime.acknowledge_rotten_movement(event).is_err()
            {
                return;
            }
            // Existing bounded slot is reused by the next nested callback;
            // there is no fallible allocation after acknowledgement.
            if let Some(next) = outcome.next {
                advanced.footprints[index] = Pending {
                    callback: Arc::new(next),
                    movement: None,
                };
            } else {
                advanced.footprints.remove(index);
            }
            states.owner_wake.notify_one();
            if prefix.is_some() {
                return;
            }
        }
    }
}
