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
            || !self.permits(fence, request, &self.key, carried)
        {
            return Ok(false);
        }
        // The exact actor's placement identity includes World, runtime scope,
        // ownership generation, local slot and slot generation. A fresh command,
        // another character or another callback cannot capture it a second time.
        let result = sqlx::query("INSERT INTO game_quest_creature_captures(target_identity,character_id,cause_id,cause_ordinal,transition_key,creature_key) VALUES($1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6) ON CONFLICT(target_identity) DO NOTHING")
            .bind(target.placement_identity().as_slice())
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(self.command.game_session_id().as_bytes().as_slice())
            .bind(self.command.command_id().get().to_string())
            .bind(&callback.key).bind(&self.key).execute(&mut **tx).await?;
        Ok(result.rows_affected() == 1)
    }
}
