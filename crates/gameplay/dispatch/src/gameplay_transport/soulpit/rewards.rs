//! Crystal 201189f player.cpp:6584-6607,12932-12936: stack-40 XP and weapon
//! points use the existing kill owner; all other Soulpit stacks grant neither.
use super::runtime::SoulpitBossReward;
use crate::domain::soulpit::SoulpitStack;
use crate::gameplay_transport::{ComposedFreshAdmission, kill_reward::DrainOutcome};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn settle_soulpit_boss_rewards(
        &self,
        reward: &SoulpitBossReward,
    ) -> Result<(), ()> {
        if reward.death.tag.stack != SoulpitStack::Boss || reward.death.tag.is_summon {
            return Err(());
        }
        {
            let runtime = self.runtime.lock().await;
            runtime.owner_fence().map_err(|_| ())?;
            if runtime.soulpit_variant(reward.death.actor) != Some(reward.death.tag) {
                return Err(());
            }
        }
        // The lethal callback froze equipment and party facts in this same native queue.
        // Drain before the lifecycle can retire the actor or return its participants.
        let Some(session) = reward
            .death
            .damage_shares
            .iter()
            .find_map(|share| share.session)
        else {
            return Ok(());
        };
        match self.drain_kill_rewards(session).await {
            DrainOutcome::Drained => Ok(()),
            DrainOutcome::Deferred => Err(()),
        }
    }
}
