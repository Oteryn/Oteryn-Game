//! Soulpit's source boss XP and weapon points reuse the normal fenced recipients.
//! Crystal 201189f src/creatures/monsters/monster.cpp:747-750,2341-2348 (stack-40 XP),
//! src/creatures/players/player.cpp:6584-6607,12932-12936 (1500 weapon points);
//! ordinary corpse/loot, Bestiary, Task Board and Forge callbacks remain excluded.
use super::runtime::{SoulpitActivityBinding, SoulpitBossReward, SoulpitRuntimeError, occurrence};
use crate::durability::character_progression::CharacterProgressionError;
use crate::gameplay_transport::ComposedFreshAdmission;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Native combat and pre-death flushes consume the same lethal snapshots as
    /// the encounter driver. No wave or deadline transition is run here.
    pub(in crate::gameplay_transport) async fn drain_pending_soulpit_rewards(
        &self,
        session: crate::foundation::GameSessionId,
    ) -> Result<(), SoulpitRuntimeError> {
        let pending = self
            .states
            .attack
            .lock()
            .await
            .kills()
            .pending_soulpit_rewards(session);
        for reward in pending {
            self.settle_soulpit_boss_reward(&reward).await?;
        }
        Ok(())
    }
    /// The lethal producer froze the event under the actual carrier/player owners.
    /// SQL independently proves current scopes, leases and each recipient's revision.
    pub(in crate::gameplay_transport) async fn settle_soulpit_boss_reward(
        &self,
        reward: &SoulpitBossReward,
    ) -> Result<(), SoulpitRuntimeError> {
        let tag = reward.death.tag;
        let expected = occurrence(
            SoulpitActivityBinding {
                handoff_id: tag.handoff_id,
                scope: reward.death.actor.runtime_scope(),
                generation: reward.death.actor.scope_generation(),
            },
            b"boss-kill",
            tag.monster_id,
        );
        if tag.stack != crate::domain::soulpit::SoulpitStack::Boss
            || tag.is_summon
            || expected != reward.occurrence
        {
            return Err(SoulpitRuntimeError::InvalidState);
        }
        let permit = self
            .spell_lane_permit()
            .await
            .ok_or(SoulpitRuntimeError::AdapterUnavailable)?;
        let queue = self.states.attack.lock().await.kills().clone();
        let (entry, settled, principal_gone) = queue
            .soulpit_reward(reward.death.actor, reward.occurrence)
            .ok_or(SoulpitRuntimeError::InvalidState)?;
        if settled {
            return Ok(());
        }
        if entry.creature != reward.species
            || entry.experience_amount() != i64::try_from(reward.experience).ok()
            || entry.facts.death_at_ms != reward.death_at_ms
        {
            return Err(SoulpitRuntimeError::InvalidState);
        }
        if !principal_gone {
            match self.settle_party_experience(&permit, &entry).await {
                Ok(result) => {
                    if let Err(crate::combat::CombatDeathRewardXpError::Progression(
                        CharacterProgressionError::Unavailable(_),
                    )) = result.xp
                    {
                        return Err(SoulpitRuntimeError::AdapterUnavailable);
                    }
                    self.states.level_refresh.mark(result.leveled);
                }
                Err(
                    CharacterProgressionError::AuthorityRejected
                    | CharacterProgressionError::MissingProgressionState
                    | CharacterProgressionError::ProgressionContextMismatch
                    | CharacterProgressionError::RespawnPending,
                ) => {
                    // The exact reward principal has departed or died. Never grant its
                    // historical event to a successor; the remaining contributor points
                    // still use the existing exact-session equipment window below.
                    super::super::operator_event("soulpit_xp_principal_no_longer_current");
                }
                Err(_) => return Err(SoulpitRuntimeError::AdapterUnavailable),
            }
        }
        // Freeze the points into the session while this same lane still excludes
        // return/logout. Their existing checkpoint may await only after releasing it.
        let checkpoints = self.credit_soulpit_weapon_reward(reward).await;
        queue.acknowledge_soulpit_reward(reward.death.actor, reward.occurrence);
        self.runtime
            .lock()
            .await
            .unpin_corpse_projection(reward.death.actor);
        drop(permit);
        for (actor, session) in checkpoints {
            let _ = self.save_weapon_proficiency(actor, session, false).await;
        }
        Ok(())
    }
}
