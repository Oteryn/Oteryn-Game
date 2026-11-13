//! Retained addEvent work uses the same scoped physical Item owner as Pods.
use super::*;
#[derive(Debug, Clone)]
pub(super) struct Pending {
    pub(super) original_due: u64,
    pub(super) callback: Arc<crate::durability::creature_source_items::RottenPortalCallback>,
}
impl Pending {
    pub(super) fn from_callback(
        callback: Arc<crate::durability::creature_source_items::RottenPortalCallback>,
    ) -> Self {
        Self {
            original_due: callback.due_us(),
            callback,
        }
    }
}
pub(super) fn plan(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    now: u64,
) -> Result<(), ()> {
    plan_through(runtime, states, room, native, now, now, false)
}
#[allow(clippy::too_many_arguments)]
pub(super) fn plan_through(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    through: u64,
    now: u64,
    one: bool,
) -> Result<(), ()> {
    let census = runtime
        .source_registered_creature_census()
        .map_err(|_| ())?;
    let advanced = &mut states.rotten_blood_callbacks.advanced;
    let selected = one
        .then(|| {
            advanced
                .bosses
                .iter()
                .filter_map(|boss| {
                    boss.teleport
                        .next_at_ms()
                        .and_then(|due| due.checked_mul(1000))
                        .filter(|due| *due <= through && *due <= now)
                        .map(|due| (due, boss.actor))
                })
                .min_by_key(|(due, _)| *due)
                .map(|(_, actor)| actor)
        })
        .flatten();
    for boss in &mut advanced.bosses {
        let Some(original_due) = boss
            .teleport
            .next_at_ms()
            .and_then(|due| due.checked_mul(1000))
        else {
            continue;
        };
        if original_due > now || original_due > through || (one && selected != Some(boss.actor)) {
            continue;
        }
        let centre = center(&boss.key);
        let current = census.iter().find_map(|(actor, at)| {
            let at = at.position();
            if at.floor != centre.floor
                || at.x.abs_diff(centre.x) > 15
                || at.y.abs_diff(centre.y) > 15
            {
                return None;
            }
            let snapshot = runtime.companion_snapshot_including_dead(*actor).ok()?;
            (snapshot.state.policy.definition_key == boss.key).then_some(*actor)
        });
        let jaw = census.iter().any(|(actor, at)| {
            let at = at.position();
            at.floor == centre.floor
                && at.x.abs_diff(centre.x) <= 15
                && at.y.abs_diff(centre.y) <= 15
                && runtime
                    .companion_snapshot_including_dead(*actor)
                    .is_ok_and(|s| {
                        s.state.policy.definition_key == "oteryn:creature.elder_bloodjaw"
                    })
        });
        let mut preview = boss.teleport;
        if preview
            .due(now / 1000, current.is_some(), boss.paused, jaw)
            .is_some()
        {
            if advanced.portals.len() >= 32 {
                return Err(());
            }
            let ordinal = boss.portal_ordinal.checked_add(1).ok_or(())?;
            let callback = boss
                .portal_source
                .from_current_named_boss(runtime, room, native, current.ok_or(())?, ordinal, now)
                .map_err(|_| ())?;
            advanced.portals.push(Pending {
                original_due,
                callback: Arc::new(callback),
            });
            boss.portal_ordinal = ordinal;
        }
        boss.teleport = preview;
    }
    Ok(())
}
impl super::super::super::ComposedFreshAdmission<'_, '_, '_> {
    #[cfg(test)]
    pub(crate) async fn drain_rotten_portal_callbacks(&self) {
        let Some(permit) = self.spell_lane_permit().await else {
            return;
        };
        self.resolve_rotten_portal_callbacks(&permit).await;
    }
    /// One scheduled callback, keeping its full read/write unit together.
    pub(crate) async fn drain_rotten_portal_callback_unit(&self) {
        let Some(permit) = self.spell_lane_permit().await else {
            return;
        };
        self.resolve_rotten_portal_callbacks_limited(&permit, 1)
            .await;
    }
    pub(in crate::gameplay_transport) async fn resolve_rotten_portal_callbacks(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) {
        self.resolve_rotten_portal_callbacks_limited(permit, 8)
            .await;
    }
    async fn resolve_rotten_portal_callbacks_limited(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        limit: usize,
    ) {
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return;
        };
        for _ in 0..limit {
            let now = self.owner_now().get();
            let (pending, prefix, binding) = {
                let runtime = self.runtime.lock().await;
                let states = self.states.spell_states.lock().await;
                let prefix = match super::super::source_prefix(
                    &runtime,
                    &states,
                    native,
                    DueKind::PortalItem,
                    now,
                ) {
                    Ok(prefix) => prefix,
                    Err(()) => return,
                };
                let advanced = &states.rotten_blood_callbacks.advanced;
                // A synchronous STEP's unknown result precedes later expiry or
                // creation callbacks; they must not consume its current Item.
                let pending = if let Some(prefix) = &prefix {
                    advanced
                        .portals
                        .iter()
                        .filter(|p| p.original_due <= prefix.through && p.callback.due_us() <= now)
                        .min_by_key(|p| p.original_due)
                        .map(|p| Arc::clone(&p.callback))
                } else {
                    step::ready(&advanced.steps, now).or_else(|| {
                        if !advanced.steps.is_empty() {
                            return None;
                        }
                        advanced
                            .portals
                            .iter()
                            .find(|p| p.callback.due_us() <= now)
                            .map(|p| Arc::clone(&p.callback))
                    })
                };
                (pending, prefix, super::super::context(&runtime))
            };
            let Some(pending) = pending else {
                return;
            };
            if let Err(error) = self
                .root
                .commit_rotten_portal_callback(permit, self.character, self.holder, &pending, now)
                .await
            {
                #[cfg(test)]
                eprintln!(
                    "SRX3_ROTTEN_PORTAL_CALLBACK_REFUSED create={} at={:?} error={error:?}",
                    pending.is_create(),
                    pending.position()
                );
                #[cfg(not(test))]
                let _ = error;
                return;
            }
            self.states.loop_ground.invalidate();
            let expiry = if pending.is_create() {
                match pending.expiry() {
                    Ok(expiry) => Some(Arc::new(expiry)),
                    Err(_) => return,
                }
            } else {
                None
            };
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if runtime.owner_fence().is_err()
                || binding != super::super::context(&runtime)
                || prefix
                    .as_ref()
                    .is_some_and(|prefix| !prefix.current(&runtime, &states))
                || !states
                    .rotten_blood_callbacks
                    .advanced
                    .portals
                    .iter()
                    .any(|p| Arc::ptr_eq(&p.callback, &pending))
            {
                return;
            }
            if step::committed(self, &mut runtime, &mut states, &pending).is_err() {
                return;
            }
            let advanced = &mut states.rotten_blood_callbacks.advanced;
            if pending.is_create() {
                advanced
                    .active_portals
                    .retain(|old| old.position() != pending.position());
                advanced.active_portals.push(Arc::clone(&pending));
            } else {
                advanced
                    .active_portals
                    .retain(|old| old.position() != pending.position());
            }
            let queue = &mut states.rotten_blood_callbacks.advanced.portals;
            queue.retain(|p| !Arc::ptr_eq(&p.callback, &pending));
            if let Some(expiry) = expiry {
                queue.push(Pending::from_callback(expiry));
            }
            states.owner_wake.notify_one();
            if prefix.is_some() {
                return;
            }
        }
    }
}
