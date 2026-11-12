use super::*;
use crate::durability::daily_reward_delivery::ReturnerOutcome;
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn daily_returner(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        kind: u32,
        payload: &[u8],
    ) -> Option<Vec<u8>> {
        self.daily_content()?;
        let fail = |code| {
            if kind == wire::returner::CLAIM {
                refusal(code)
            } else {
                None
            }
        };
        let _permit = self.spell_lane_permit().await?;
        {
            let runtime = self.runtime.lock().await;
            if runtime
                .player_control_facts(actor, command.game_session_id)
                .is_err()
            {
                return fail(7);
            }
        }
        let claim = if kind == wire::returner::CLAIM {
            Some(wire::returner::decode_claim(payload).ok()?)
        } else {
            None
        };
        if claim.is_some()
            && !self
                .states
                .player_trades
                .physical(command.game_session_id, None)
        {
            return fail(7);
        }
        let fence = command.item_fence?;
        match self
            .root
            .daily_returner_reward(
                self.character,
                self.holder,
                fence,
                CommandRef::new(
                    command.game_session_id,
                    CommandId::new(command.command_id).ok()?,
                ),
                claim,
            )
            .await
        {
            Ok(ReturnerOutcome::Announcement(reply)) => reply.encode().ok(),
            Ok(ReturnerOutcome::Claim(reply)) => reply.encode().ok(),
            Err(DailyRewardClaimError::AuthorityRejected) => fail(7),
            Err(_) => fail(5),
        }
    }
}
