use super::*;
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn daily_details(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        kind: u32,
    ) -> Option<Vec<u8>> {
        let (_, content) = self.daily_content()?;
        let fence = command.item_fence?;
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).ok()?,
        );
        // The existing durability read proves recovery, node, character, scope, session and generation.
        {
            let runtime = self.runtime.lock().await;
            if runtime
                .player_control_facts(actor, command.game_session_id)
                .is_err()
            {
                return None;
            }
        }
        let view = self
            .root
            .read_daily_reward_view(self.character, self.holder, fence, command_ref)
            .await
            .ok()?;
        let mut details = wire::details::Details::empty(view.epoch);
        if kind == wire::details::COMMAND_TYPE_WITH_RESET {
            details.next_reset_at = Some(view.next_reset_at);
        }
        for (index, day) in content.days.iter().enumerate() {
            let tier = |t: crate::content::daily_reward::DailyRewardTier| wire::details::Tier {
                cosmetic_tokens: t.cosmetic_tokens,
                prey_wildcards: t.prey_wildcards,
                temple_scrolls: t.temple_teleport_scrolls,
                xp_percent: t.xp_boost_percent,
                xp_minutes: t.xp_boost_minutes,
                bounty_tokens: t.bounty_reroll_tokens,
            };
            *details.days.get_mut(index)? = wire::details::Day {
                free: tier(content.reward_on_reset(day.day, false, view.reset_date)?),
                premium: tier(content.reward_on_reset(day.day, true, view.reset_date)?),
            };
        }
        for (slot, receipt) in details.history.iter_mut().zip(view.history) {
            *slot = Some(receipt);
        }
        details.encode().ok()
    }
}
