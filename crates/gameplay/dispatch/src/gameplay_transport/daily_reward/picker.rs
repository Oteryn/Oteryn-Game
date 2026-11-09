use super::*;
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn daily_picker(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        kind: u32,
        payload: &[u8],
    ) -> Option<Vec<u8>> {
        if kind == wire::picker::CLAIM {
            let selection = wire::picker::Claim::decode(payload).ok()?;
            return self
                .daily_command(
                    actor,
                    command,
                    Some(wire::DailyRewardClaim),
                    Some(selection),
                )
                .await;
        }
        self.daily_content()?;
        {
            let runtime = self.runtime.lock().await;
            if runtime
                .player_control_facts(actor, command.game_session_id)
                .is_err()
            {
                return None;
            }
        }
        let fence = command.item_fence?;
        self.root
            .read_daily_reward_picker(
                self.character,
                self.holder,
                fence,
                CommandRef::new(
                    command.game_session_id,
                    CommandId::new(command.command_id).ok()?,
                ),
                &super::super::spell_premium_coordinator::DailyRewardPremium(
                    self.premium_coordinator,
                ),
            )
            .await
            .ok()?
            .encode()
            .ok()
    }
}
