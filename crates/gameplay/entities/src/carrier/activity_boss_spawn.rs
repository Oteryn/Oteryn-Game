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
            || !self.visible_corpses().is_empty()
            || template.spite_source_setup().is_some_and(|setup| {
                active
                    .native_gameplay()
                    .is_none_or(|native| setup.source_digest() != native.source_digest())
            })
            || !template
                .issue_runtime_pin(active)
                .is_ok_and(|pin| &pin == self.content_pin())
        {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        let position = template.boss_position();
        let at = MovementLocalPosition {
            x: position.x,
            y: position.y,
            floor: i16::try_from(position.z).map_err(|_| CarrierError::InvalidSpawnDefinition)?,
        };
        self.spawn_qualified_activity_boss(template.creature_key(), at)
    }

    fn spawn_qualified_activity_boss(
        &mut self,
        creature: &str,
        at: MovementLocalPosition,
    ) -> Result<ExactActorRef, CarrierError> {
        let policy = self.companion_policy(creature)?;
        // The qualified template owns encounter admission. The creature's
        // explicit reward policy selects its death settlement.
        if policy.reward_boss.is_none() {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        let health = policy.maximum_health;
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

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_gameplay_content::content::{
        native_gameplay::callback_native_policy_fixture_at, native_gameplay_test_world_id,
    };

    #[test]
    fn qualified_ordinary_boss_spawns_with_authored_policy() -> Result<(), String> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(check_ordinary_boss_spawn)
            .map_err(debug)?
            .join()
            .map_err(|_| "boss spawn test panicked".to_owned())?
    }

    fn check_ordinary_boss_spawn() -> Result<(), String> {
        let key = "oteryn:creature.the_shatterer";
        let world = native_gameplay_test_world_id().map_err(debug)?;
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/spells.manifest.json");
        let native = callback_native_policy_fixture_at(&manifest).map_err(debug)?;
        let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
            RuntimeScopeRefV1::instance(world, id(71)).map_err(debug)?,
            NodeId::decode(&id(72)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            ChannelContentPin::from_activation(
                world,
                1,
                native.source_digest(),
                [2; 32],
                [3; 32],
                [4; 32],
                (33398, 32414, 14),
            ),
        )
        .map_err(debug)?;
        runtime
            .install_companion_policies(native.policies().clone())
            .map_err(debug)?;
        let policy = runtime.companion_policy(key).map_err(debug)?;
        assert_eq!(policy.reward_boss, Some(false));
        let boss = runtime
            .spawn_qualified_activity_boss(
                key,
                MovementLocalPosition {
                    x: 33406,
                    y: 32418,
                    floor: 14,
                },
            )
            .map_err(debug)?;
        let position = runtime.read_actor_position(boss).map_err(debug)?.position();
        assert_eq!((position.x, position.y, position.floor), (33406, 32418, 14));
        assert_eq!(
            runtime
                .companion_snapshot(boss)
                .map_err(debug)?
                .state
                .policy,
            policy
        );
        Ok(())
    }

    fn debug(value: impl std::fmt::Debug) -> String {
        format!("{value:?}")
    }

    fn id(tag: u8) -> [u8; 16] {
        let mut bytes = [tag; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes
    }
}
