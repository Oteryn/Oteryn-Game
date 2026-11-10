//! Retained addEvent work uses the same scoped physical Item owner as Pods.
use super::*;
pub(super) fn plan(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    now: u64,
) -> Result<(), ()> {
    let census = runtime.positioned_actor_census().map_err(|_| ())?;
    let advanced = &mut states.rotten_blood_callbacks.advanced;
    for boss in &mut advanced.bosses {
        if boss
            .teleport
            .next_at_ms()
            .is_none_or(|due| due.saturating_mul(1000) > now)
        {
            continue;
        }
        let centre = center(&boss.key);
        let current = census.iter().find_map(|(actor, at, session)| {
            let at = at.position();
            if session.is_some()
                || at.floor != centre.floor
                || at.x.abs_diff(centre.x) > 15
                || at.y.abs_diff(centre.y) > 15
            {
                return None;
            }
            let snapshot = runtime.companion_snapshot(*actor).ok()?;
            (snapshot.health > 0 && snapshot.state.policy.definition_key == boss.key)
                .then_some(*actor)
        });
        let jaw = census.iter().any(|(actor, at, session)| {
            let at = at.position();
            session.is_none()
                && at.floor == centre.floor
                && at.x.abs_diff(centre.x) <= 15
                && at.y.abs_diff(centre.y) <= 15
                && runtime.companion_snapshot(*actor).is_ok_and(|s| {
                    s.health > 0
                        && s.state.policy.definition_key == "oteryn:creature.elder_bloodjaw"
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
            advanced.portals.push(Arc::new(callback));
            boss.portal_ordinal = ordinal;
        }
        boss.teleport = preview;
    }
    Ok(())
}
impl super::super::super::ComposedFreshAdmission<'_, '_, '_> {
    pub(crate) async fn drain_rotten_portal_callbacks(&self) {
        let Some(permit) = self.spell_lane_permit().await else {
            return;
        };
        self.resolve_rotten_portal_callbacks(&permit).await;
    }
    pub(in crate::gameplay_transport) async fn resolve_rotten_portal_callbacks(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) {
        for _ in 0..8 {
            let now = self.owner_now().get();
            let pending = {
                let states = self.states.spell_states.lock().await;
                let advanced = &states.rotten_blood_callbacks.advanced;
                // A synchronous STEP's unknown result precedes later expiry or
                // creation callbacks; they must not consume its current Item.
                step::ready(&advanced.steps, now).or_else(|| {
                    if !advanced.steps.is_empty() {
                        return None;
                    }
                    advanced.portals.iter().find(|p| p.due_us() <= now).cloned()
                })
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
            queue.retain(|p| !Arc::ptr_eq(p, &pending));
            if let Some(expiry) = expiry {
                queue.push(expiry);
            }
            states.owner_wake.notify_one();
        }
    }
}
