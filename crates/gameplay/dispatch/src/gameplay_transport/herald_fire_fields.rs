//! The Herald's source attack runs on the private Instance AI clock.
use std::sync::Arc;

use super::ComposedFreshAdmission;
use crate::content::bosses::activity_template::herald::HeraldFireFieldAttack;
use crate::durability::{
    creature_source_items::HeraldFieldRefresh, spell_owner_commit::SpellLanePermit,
};

#[derive(Debug, Default)]
pub(super) struct HeraldFieldSchedule {
    next_cast_us: Option<u64>,
    pending: Option<Arc<HeraldFieldRefresh>>,
}

impl HeraldFieldSchedule {
    pub(super) fn next_due(&self) -> Option<u64> {
        self.pending
            .as_ref()
            .map(|refresh| refresh.due_us())
            .or(self.next_cast_us)
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn drain_herald_fire_field_attack(
        &self,
        permit: &SpellLanePermit,
        room: &crate::content::QualifiedNativeEntryRoom,
        now_us: u64,
    ) {
        let Some(map) = self.bundle_map() else {
            return;
        };
        let scope = map.overlay.runtime_scope();
        if !matches!(scope, crate::foundation::RuntimeScopeRefV1::Instance { .. }) {
            return;
        }
        let Some(native) = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            return;
        };
        let due = {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err() || runtime.binding().runtime_scope() != scope {
                return;
            }
            let Some(owner) = self
                .activity_runtimes
                .get(scope, runtime.binding().scope_generation().get())
            else {
                return;
            };
            let Some(key) = owner.template.as_ref() else {
                return;
            };
            let Some(template) = self.activity_runtimes.template(&key.key, &key.revision) else {
                return;
            };
            let Some(source) = template.herald_fire_field_attack() else {
                return;
            };
            let mut states = self.states.spell_states.lock().await;
            let schedule = &mut states.herald_field_schedule;
            if let Some(refresh) = &schedule.pending {
                if now_us < refresh.due_us() {
                    return;
                }
                Arc::clone(refresh)
            } else {
                if !runtime.activity_herald_is_alive() {
                    schedule.next_cast_us = None;
                    return;
                }
                let Ok(census) = runtime.positioned_melee_census() else {
                    return;
                };
                if !census.iter().any(|(_, _, session)| session.is_some()) {
                    schedule.next_cast_us = None;
                    return;
                }
                let Some(next_cast_us) = schedule.next_cast_us else {
                    schedule.next_cast_us = now_us.checked_add(HeraldFireFieldAttack::INTERVAL_US);
                    return;
                };
                if now_us < next_cast_us {
                    return;
                }
                let Some(actor) = runtime
                    .visible_entities()
                    .creatures
                    .into_iter()
                    .find(|creature| {
                        runtime.matches_live_creature_identity(
                            creature.actor,
                            HeraldFireFieldAttack::CREATURE.as_bytes(),
                        )
                    })
                    .map(|creature| creature.actor)
                else {
                    return;
                };
                let refresh = match HeraldFieldRefresh::from_live_cast(
                    &runtime, room, native, source, actor, now_us,
                ) {
                    Ok(refresh) => refresh,
                    Err(error) => {
                        super::operator_event(&format!(
                            "herald_fire_field_prepare_deferred error={error:?}"
                        ));
                        return;
                    }
                };
                // The Instance BundleMap owns a separate outbox from the origin Channel.
                for tile in source
                    .cells()
                    .filter_map(crate::content::bundle_spell_tiles::tile_pos)
                {
                    let _ = map
                        .facts
                        .emit_movement_effect(tile, HeraldFireFieldAttack::EFFECT);
                }
                schedule.pending = Some(Arc::new(refresh));
                schedule.next_cast_us = now_us.checked_add(HeraldFireFieldAttack::INTERVAL_US);
                return;
            }
        };
        match self
            .root
            .commit_herald_field_refresh(permit, self.character, self.holder, &due, now_us)
            .await
        {
            Ok(_) => {
                self.states
                    .spell_states
                    .lock()
                    .await
                    .herald_field_schedule
                    .pending = None;
                self.states.loop_ground.invalidate();
                let _ = super::loop_move::refresh_herald_rune_fields(self).await;
            }
            Err(error) => super::operator_event(&format!(
                "herald_fire_field_refresh_deferred error={error:?}"
            )),
        }
    }
}
