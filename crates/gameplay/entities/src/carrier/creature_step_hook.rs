//! Crystal step-in hooks that end a wild creature without a death: `trap.lua:39-42` removes a
//! Starving Wolf on the magic wolf trap and `thornfire_wolf.lua:8-12` replaces a Thornfire Wolf
//! on bog water with a Crystal Wolf (Crystal `201189f8`).
use super::*;

impl ChannelRuntimeV1 {
    /// Removes the wild creature `actor` of definition `definition_key` that stands at `at`, as
    /// Crystal `creature:remove()` does: no death, corpse, loot or experience. The spawn scan then
    /// finds its cell empty and schedules the ordinary respawn (`scan_spawn_points`). With
    /// `replacement`, a fresh wild creature of that definition is placed on the same cell
    /// (Crystal `Game.createMonster(name, position, false, true)`), outside every spawn schedule.
    ///
    /// Returns the end key of the removed creature, the owner-committed identity a MINT caused by
    /// this removal is keyed by, and the replacement actor. Nothing changes when any check fails;
    /// the replacement policy is resolved before the removal.
    pub fn remove_stepped_creature(
        &mut self,
        actor: ExactActorRef,
        at: MovementLocalPosition,
        definition_key: &str,
        replacement: Option<&str>,
    ) -> Result<(CreatureDeathOccurrenceKey, Option<ExactActorRef>), CarrierError> {
        self.owner_fence()?;
        self.assert_actor_spell_unreserved(actor)?;
        let snapshot = self.companion_snapshot(actor)?;
        if snapshot.state.master.is_some()
            || snapshot.state.policy.definition_key != definition_key
            || snapshot.position != at
            || self.native_summon_role(actor) != Ok(None)
        {
            return Err(CarrierError::PlanConflict);
        }
        let channel_id = actor.channel_id().ok_or(CarrierError::WrongScope)?;
        let policy = replacement
            .map(|key| self.companion_policy(key))
            .transpose()?;
        self.carrier.remove(&self.continuity, actor.0)?;
        let end = CreatureDeathOccurrenceKey {
            actor: actor.0,
            channel_id,
        };
        let Some(policy) = policy else {
            return Ok((end, None));
        };
        let replacement = ExactActorRef(self.carrier.admit_creature(
            &self.continuity,
            ActorState(0),
            &policy.definition_key,
            policy.maximum_health,
        )?);
        let context = self.pinned_position_context();
        let placed = self
            .carrier
            .initialize_position(
                &self.continuity,
                replacement.0,
                context,
                LocalPosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                },
            )
            .and_then(|_| self.install_creature_policy(replacement, &policy.definition_key));
        if let Err(error) = placed {
            let _ = self.carrier.remove(&self.continuity, replacement.0);
            return Err(error);
        }
        Ok((end, Some(replacement)))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::runtime_actor_companion::{
        CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags,
    };
    use super::*;

    fn policy(key: &str, name: &str, health: i64) -> CompiledCreaturePolicy {
        CompiledCreaturePolicy {
            definition_key: key.to_owned(),
            definition_revision: "definition-r1".to_owned(),
            display_name: name.to_owned(),
            maximum_health: health,
            forge_difficulty: None,
            base_speed: 100,
            outfit_look_type: 27,
            object_look_type: None,
            summonable: false,
            convinceable: false,
            mana_cost: None,
            is_familiar: false,
            condition_immunities: Vec::new(),
            defense: None,
            armor: None,
            mitigation: None,
            resistances: Vec::new(),
            damage_immunities: Vec::new(),
            healing_from_damage: Vec::new(),
            flags: CreatureFlags {
                attackable: true,
                illusionable: true,
                health_hidden: false,
            },
            preferred_distance: None,
            reward_boss: None,
            blocked_by_nearby_players: false,
        }
    }

    fn uuid(seed: u8) -> [u8; 16] {
        let mut bytes = [seed; 16];
        bytes[6] = 0x70;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        bytes
    }

    fn runtime() -> ChannelRuntimeV1 {
        let world = WorldId::decode(&uuid(1)).expect("world");
        let digest = [9; 32];
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&uuid(2)).expect("channel"),
            NodeId::decode(&uuid(3)).expect("node"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            ChannelContentPin::from_activation(
                world,
                1,
                digest,
                [2; 32],
                [3; 32],
                [4; 32],
                (0, 0, 0),
            ),
        )
        .expect("runtime");
        runtime
            .install_companion_policies(
                CompiledCreaturePolicies::from_active_artifact(
                    digest,
                    vec![
                        policy("oteryn:creature.starving_wolf", "Starving Wolf", 85),
                        policy("oteryn:creature.crystal_wolf", "Crystal Wolf", 750),
                    ],
                )
                .expect("policies"),
            )
            .expect("installed");
        runtime
    }

    fn wolf(runtime: &mut ChannelRuntimeV1, at: MovementLocalPosition) -> ExactActorRef {
        let context = runtime.pinned_position_context();
        let actor = ExactActorRef(
            runtime
                .carrier
                .admit_creature(
                    &runtime.continuity,
                    ActorState(0),
                    "oteryn:creature.starving_wolf",
                    85,
                )
                .expect("admitted"),
        );
        runtime
            .carrier
            .initialize_position(
                &runtime.continuity,
                actor.0,
                context,
                LocalPosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                },
            )
            .expect("placed");
        runtime
            .install_creature_policy(actor, "oteryn:creature.starving_wolf")
            .expect("policy");
        actor
    }

    const AT: MovementLocalPosition = MovementLocalPosition {
        x: 100,
        y: 100,
        floor: 7,
    };

    #[test]
    fn m10_stepped_wolf_is_removed_without_a_death_and_keyed_by_its_identity() {
        let mut runtime = runtime();
        let actor = wolf(&mut runtime, AT);
        // Another definition or another cell changes nothing.
        assert!(
            runtime
                .remove_stepped_creature(actor, AT, "oteryn:creature.thornfire_wolf", None)
                .is_err()
        );
        assert!(
            runtime
                .remove_stepped_creature(
                    actor,
                    MovementLocalPosition { x: 101, ..AT },
                    "oteryn:creature.starving_wolf",
                    None
                )
                .is_err()
        );
        assert!(runtime.contains_live_creature(actor));
        let (end, replacement) = runtime
            .remove_stepped_creature(actor, AT, "oteryn:creature.starving_wolf", None)
            .expect("removed");
        assert!(replacement.is_none());
        assert!(!runtime.contains_live_creature(actor));
        assert_eq!(end.actor_local_id(), actor.0.actor_local_id.0);
        assert_eq!(
            end.actor_local_generation(),
            actor.0.actor_local_generation.0
        );
        assert!(runtime.companion_snapshot_including_dead(actor).is_err());
    }

    #[test]
    fn m10_stepped_wolf_is_replaced_on_its_cell_by_the_named_creature() {
        let mut runtime = runtime();
        let actor = wolf(&mut runtime, AT);
        // An unknown replacement is refused before the removal.
        assert!(
            runtime
                .remove_stepped_creature(
                    actor,
                    AT,
                    "oteryn:creature.starving_wolf",
                    Some("oteryn:creature.unknown")
                )
                .is_err()
        );
        assert!(runtime.contains_live_creature(actor));
        let (_, replacement) = runtime
            .remove_stepped_creature(
                actor,
                AT,
                "oteryn:creature.starving_wolf",
                Some("oteryn:creature.crystal_wolf"),
            )
            .expect("replaced");
        let replacement = replacement.expect("replacement");
        assert!(!runtime.contains_live_creature(actor));
        let snapshot = runtime.companion_snapshot(replacement).expect("live");
        assert_eq!(
            snapshot.state.policy.definition_key,
            "oteryn:creature.crystal_wolf"
        );
        assert_eq!(snapshot.position, AT);
        assert_eq!(snapshot.health, 750);
        assert!(runtime.creature_spawn_cell(replacement).is_none());
    }
}
