//! Distinct live occurrence credit participates in the existing quest transaction.
use super::*;
use crate::durability::DurabilityError;
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;

impl QuestCreatureUseProof<'_> {
    pub(in crate::durability::reward_claim_mint) async fn claim_capture(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        fence: CurrentCharacterGameplayFence,
        request: &QuestTransitionRequest,
        callback: &RewardCallback,
        carried: &super::super::QuestCarriedUse,
    ) -> Result<bool, DurabilityError> {
        let Some(target) = self.live_target() else {
            return Ok(false);
        };
        if !callback.capture_creature
            || callback.fresh_corpse
            || !self.permits(fence, request, callback, carried)
        {
            return Ok(false);
        }
        // The exact actor's placement identity includes World, runtime scope,
        // ownership generation, local slot and slot generation. A fresh command,
        // another character or another callback cannot capture it a second time.
        let Target::Live { entity, .. } = self.target else {
            return Ok(false);
        };
        let crate::foundation::RuntimeScopeRefV1::Channel { world_id, channel_id } = fence.runtime_scope else {
            return Ok(false);
        };
        let result = sqlx::query("INSERT INTO game_quest_creature_captures(character_id,cause_id,cause_ordinal,transition_key,world_id,channel_id,scope_generation,target_identity,target_generation,creature_key,source_item,ingress) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7::text::numeric,$8,$9::text::numeric,$10,encode($11,'hex')::uuid,$12) ON CONFLICT DO NOTHING")
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(self.command.game_session_id().as_bytes().as_slice())
            .bind(self.command.command_id().get().to_string()).bind(&callback.key)
            .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
            .bind(fence.scope_ownership_generation.get().to_string())
            .bind(target.placement_identity().as_slice()).bind(entity.generation.to_string())
            .bind(&self.key).bind(carried.source.as_slice()).bind(&carried.intent)
            .execute(&mut **tx).await?;
        Ok(result.rows_affected() == 1)
    }
}
