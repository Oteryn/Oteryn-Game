//! Exact Spite source HP/defense changes; one canonical actor snapshot owns both.
use super::*;
pub const SPITE: &str = "oteryn:creature.goshnar_s_spite";
#[derive(Debug, Clone, Copy)]
pub enum SpiteChange {
    FireExpired,
    WeepingHeal,
}
impl ChannelRuntimeV1 {
    pub fn change_soul_war_spite(
        &mut self,
        expected: &runtime_actor_companion::CompanionSnapshot,
        expected_defense: u32,
        change: SpiteChange,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        self.validate_companion_snapshot(expected)?;
        self.assert_actor_spell_unreserved(expected.actor)?;
        let source_target = match change {
            SpiteChange::FireExpired => self.source_creature_is_registered(expected.actor)?,
            SpiteChange::WeepingHeal => {
                matches!(
                    self.binding().runtime_scope(),
                    RuntimeScopeRefV1::Instance { .. }
                ) && expected.health > 0
            }
        };
        if !source_target
            || expected.state.policy.definition_key != SPITE
            || expected.state.master.is_some()
            || expected.health < 0
            || expected.maximum_health <= 0
            || expected.health > expected.maximum_health
            || self.effective_creature_defense(expected)? != expected_defense
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let index = self.encounter_actor_index(expected.actor)?;
        let Slot::CreatureOccupied { spell_combat, .. } = self.encounter_slot(index) else {
            return Err(CarrierError::NotCreature);
        };
        let next_delta = match change {
            SpiteChange::FireExpired => spell_combat
                .soul_war_defense_delta
                .checked_add(10)
                .ok_or(CarrierError::CapacityArithmeticOverflow)?,
            SpiteChange::WeepingHeal => spell_combat.soul_war_defense_delta,
        };
        let next_health = match change {
            SpiteChange::WeepingHeal => expected
                .health
                .saturating_add(expected.maximum_health / 10)
                .min(expected.maximum_health),
            SpiteChange::FireExpired => expected.health,
        };
        let (carrier, _) = self.encounter_parts_mut();
        let Slot::CreatureOccupied {
            spell_combat,
            health,
            ..
        } = &mut carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        spell_combat.soul_war_defense_delta = next_delta;
        *health = next_health;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    fn id(n: u8) -> [u8; 16] {
        let mut id = [n; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id
    }

    fn runtime() -> TestResult<(ChannelRuntimeV1, ExactActorRef)> {
        let world = WorldId::decode(&id(1)).map_err(|error| format!("{error:?}"))?;
        let scope =
            RuntimeScopeRefV1::instance(world, id(2)).map_err(|error| format!("{error:?}"))?;
        let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
            scope,
            NodeId::decode(&id(3)).map_err(|error| format!("{error:?}"))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            16,
            ChannelContentPin::test(world),
        )?;
        let mut policies = super::super::soul_war_mirror_runtime::tests::policies();
        // The cached decoded native bodies retain their real callbacks. Bind
        // this clone to the actual runtime outer pin, outside the shared cache.
        policies.bind_qualified_outer_artifact(runtime.content_pin().server_artifact_digest())?;
        runtime.install_companion_policies(policies)?;
        let maximum = runtime.companion_policy(SPITE)?.maximum_health;
        let actor = ExactActorRef(runtime.admit_encounter_fixture_actor(
            ActorState(1),
            SPITE,
            maximum,
            Some(LocalPosition {
                x: 33746,
                y: 31666,
                floor: 14,
            }),
        )?);
        runtime.install_creature_policy(actor, SPITE)?;
        Ok((runtime, actor))
    }

    #[test]
    fn srx3_spite_current_actor_defense_addition_and_maximum_health_heal() -> TestResult {
        let (mut runtime, boss) = runtime()?;
        let before = runtime.companion_snapshot(boss)?;
        runtime
            .change_soul_war_spite(&before, 160, SpiteChange::FireExpired)
            .map_err(|error| format!("REACHED exact Spite mutation: {error:?}"))?;
        let after = runtime.companion_snapshot(boss)?;
        assert_eq!(
            after.state.policy.defense,
            Some(160),
            "immutable source policy remains unchanged"
        );
        assert_eq!(runtime.effective_creature_defense(&after)?, 170);
        assert_eq!(after.health, before.health);
        assert!(
            runtime
                .change_soul_war_spite(&before, 160, SpiteChange::FireExpired)
                .is_err()
        );
        // Source percentage is maximumHP, not currentHP; owner clamps at maximum.
        let index = runtime.encounter_actor_index(boss)?;
        let (carrier, _) = runtime.encounter_parts_mut();
        let Slot::CreatureOccupied { health, .. } = &mut carrier.slots[index] else {
            return Err("boss slot missing".into());
        };
        *health = 100;
        let wounded = runtime.companion_snapshot(boss)?;
        runtime.change_soul_war_spite(&wounded, 170, SpiteChange::WeepingHeal)?;
        let healed = runtime.companion_snapshot(boss)?;
        assert_eq!(
            before.maximum_health, 550_000,
            "actual source Spite maximum"
        );
        assert_eq!(healed.health, 100 + before.maximum_health / 10);
        assert_eq!(healed.maximum_health, before.maximum_health);
        assert_eq!(healed.state.policy.defense, Some(160));
        assert_eq!(runtime.effective_creature_defense(&healed)?, 170);
        Ok(())
    }
}
