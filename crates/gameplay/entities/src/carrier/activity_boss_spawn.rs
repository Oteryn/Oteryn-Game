//! One source-qualified boss in a freshly allocated activity runtime.
//! BossLever creates one boss per attempt; it is never an ordinary respawn point.
use super::*;
use oteryn_gameplay_content::content::{
    ActiveGeneration, bosses::activity_template::QualifiedActivityRoomTemplate,
};

impl ChannelRuntimeV1 {
    pub fn realize_activity_boss(
        &mut self,
        template: &QualifiedActivityRoomTemplate,
        active: &ActiveGeneration,
    ) -> Result<ExactActorRef, CarrierError> {
        self.owner_fence()?;
        if !matches!(
            self.binding.runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        ) || !self.positioned_actor_census()?.is_empty()
            || !template
                .issue_runtime_pin(active)
                .is_ok_and(|pin| &pin == self.content_pin())
        {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        let creature = template.creature_key();
        let policy = self.companion_policy(creature)?;
        if policy.reward_boss != Some(true) {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        let health = policy.maximum_health;
        let position = template.boss_position();
        let at = MovementLocalPosition {
            x: position.x,
            y: position.y,
            floor: i16::try_from(position.z).map_err(|_| CarrierError::InvalidSpawnDefinition)?,
        };
        if !self.native_summon_cell_free(at) {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        let context = self.pinned_position_context();
        let actor = ExactActorRef(self.carrier.admit_creature(
            &self.continuity,
            ActorState(0),
            creature,
            health,
        )?);
        let result = self
            .carrier
            .initialize_position(
                &self.continuity,
                actor.0,
                context,
                LocalPosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                },
            )
            .and_then(|_| self.install_creature_policy(actor, creature));
        if let Err(error) = result {
            let _ = self.carrier.remove(&self.continuity, actor.0);
            return Err(error);
        }
        Ok(actor)
    }
}
