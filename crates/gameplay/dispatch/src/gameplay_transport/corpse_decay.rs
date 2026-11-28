//! D113/D135 and NIGHT-W1 decision 11: corpse deadlines are independent of ordinary Ground.
use super::ComposedFreshAdmission;
use crate::durability::item_decay_retire::CorpseDecayFence;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Called by the server's one-second item clock, including with no connected players.
    /// The complete bounded read also recovers missed/partial drains after a restart. Each
    /// retirement re-proves the deadline and current scope authority inside the existing writer.
    pub(super) async fn drain_corpse_deadlines(&self) {
        self.root
            .with_background_operation(self.drain_corpse_deadline_unit())
            .await;
    }

    async fn drain_corpse_deadline_unit(&self) {
        let Some(permit) = self.spell_lane_permit().await else {
            return;
        };
        let fence = {
            let runtime = self.runtime.lock().await;
            if runtime.binding().world_id() != self.world_id {
                return;
            }
            let Ok(owner) = runtime.owner_fence() else {
                return;
            };
            CorpseDecayFence {
                runtime_scope: runtime.binding().runtime_scope(),
                scope_ownership_generation: owner.generation(),
            }
        };
        let schedule = match self
            .root
            .read_scoped_corpse_decay_schedule(self.character, fence.runtime_scope)
            .await
        {
            Ok(schedule) => schedule,
            Err(error) => {
                super::operator_event(&format!("corpse_decay_read_deferred reason={error}"));
                return;
            }
        };
        let mut live: Vec<_> = schedule
            .corpses
            .iter()
            .map(|corpse| corpse.corpse_item_instance_id)
            .collect();
        let due: Vec<_> = schedule
            .corpses
            .iter()
            .filter(|corpse| {
                corpse
                    .decay_at_unix_ms
                    .is_some_and(|deadline| deadline <= schedule.database_now_unix_ms)
            })
            .collect();
        let start = self.background_start("corpse", due.len());
        if let Some(corpse) = due.get(start) {
            match self
                .root
                .retire_decayed_corpse(
                    &permit,
                    self.character,
                    self.holder,
                    fence,
                    corpse.corpse_item_instance_id,
                )
                .await
            {
                Ok(_) => {
                    live.retain(|item| *item != corpse.corpse_item_instance_id);
                    if let Ok(mut cursors) = self.states.background_cursors.lock() {
                        cursors.insert("corpse", start + 1);
                    }
                }
                Err(error) => {
                    super::operator_event(&format!("corpse_decay_retire_deferred reason={error}"));
                }
            }
        }
        // An incomplete page cannot prove absence. Rotate through visible bindings
        // outside it, checking one exact Ground row per pass under the same lane.
        let mut confirmed_absent = None;
        if !schedule.complete {
            let candidates: Vec<_> = self
                .runtime
                .lock()
                .await
                .visible_corpses()
                .into_iter()
                .map(|corpse| corpse.item.item_instance_id)
                .filter(|item| !live.contains(item))
                .collect();
            let cursor = self.background_start("corpse_projection", candidates.len());
            if let Some(&item) = candidates.get(cursor)
                && let Ok(exact) = self
                    .root
                    .read_corpse_schedule_page(self.character, fence.runtime_scope, Some(item))
                    .await
            {
                if exact.corpses.is_empty() {
                    confirmed_absent = Some(item);
                }
                if let Ok(mut cursors) = self.states.background_cursors.lock() {
                    cursors.insert("corpse_projection", cursor + 1);
                }
            }
        }
        let mut runtime = self.runtime.lock().await;
        if runtime
            .owner_fence()
            .is_ok_and(|owner| owner.generation() == fence.scope_ownership_generation)
            && runtime.binding().runtime_scope() == fence.runtime_scope
        {
            // A cache containing off-page items must be rebuilt from Ground custody.
            self.states
                .loop_ground
                .reconcile_ground_corpses(fence.runtime_scope, &live);
            if let Some(item) = confirmed_absent {
                let _ = runtime.set_corpse_item_ground_position(item, None);
            }
            if schedule.complete {
                let _ = runtime.retain_live_corpse_items(&live);
            }
            for corpse in &schedule.corpses {
                if !live.contains(&corpse.corpse_item_instance_id) {
                    if !schedule.complete {
                        self.states.loop_ground.invalidate();
                        let _ = runtime
                            .set_corpse_item_ground_position(corpse.corpse_item_instance_id, None);
                    }
                    continue;
                }
                if let Some(bytes) = &corpse.loop_ground_spatial_position
                    && let Ok(cell) = crate::durability::spell_items_abi::decode_ground_cell(bytes)
                    && let Some(floor) = i16::try_from(cell.z).ok().and_then(i16::checked_neg)
                {
                    let _ = runtime.relocate_corpse_item(
                        corpse.corpse_item_instance_id,
                        crate::foundation::MovementLocalPosition {
                            x: cell.x,
                            y: cell.y,
                            floor,
                        },
                    );
                }
                let _ = runtime.refresh_corpse_item(
                    corpse.corpse_item_instance_id,
                    &corpse.definition.family,
                    &corpse.definition.production_key,
                    &corpse.definition.revision_ref,
                );
            }
        }
    }
}
