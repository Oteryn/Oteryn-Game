//! D113/D135 and NIGHT-W1 decision 11: corpse deadlines are independent of ordinary Ground.
use super::ComposedFreshAdmission;
use crate::durability::item_decay_retire::CorpseDecayFence;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Called by the server's one-second item clock, including with no connected players.
    /// The complete bounded read also recovers missed/partial drains after a restart. Each
    /// retirement re-proves the deadline and current scope authority inside the existing writer.
    pub(super) async fn drain_corpse_deadlines(&self) {
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
        for corpse in &schedule.corpses {
            if corpse.decay_at_unix_ms > schedule.database_now_unix_ms {
                continue;
            }
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
                Ok(_) => live.retain(|item| *item != corpse.corpse_item_instance_id),
                Err(error) => {
                    super::operator_event(&format!("corpse_decay_retire_deferred reason={error}"));
                }
            }
        }
        // A prior ambiguous commit may already have removed a corpse from the schedule. Only
        // a successful complete read may hide it, and no runtime guard crosses a durable call.
        let mut runtime = self.runtime.lock().await;
        if runtime
            .owner_fence()
            .is_ok_and(|owner| owner.generation() == fence.scope_ownership_generation)
            && runtime.binding().runtime_scope() == fence.runtime_scope
        {
            let _ = runtime.retain_live_corpse_items(&live);
            for corpse in &schedule.corpses {
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
