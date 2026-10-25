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
            if runtime.binding().world_id() != self.world_id
                || runtime.binding().channel_id() != self.channel_id
            {
                return;
            }
            let Ok(owner) = runtime.owner_fence() else {
                return;
            };
            CorpseDecayFence {
                world_id: self.world_id,
                channel_id: self.channel_id,
                scope_ownership_generation: owner.generation(),
            }
        };
        let schedule = match self
            .root
            .read_corpse_decay_schedule(self.character, self.world_id, self.channel_id)
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
        for corpse in schedule.corpses {
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
        {
            let _ = runtime.retain_live_corpse_items(&live);
        }
    }
}
