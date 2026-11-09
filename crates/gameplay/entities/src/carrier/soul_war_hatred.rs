//! Hatred's exact-incarnation source counter, not an immutable policy mutation.
use super::*;
const HATRED: &str = "oteryn:creature.goshnar_s_hatred";
fn empowered_boss(key: &str) -> bool {
    matches!(
        key,
        HATRED
            | "oteryn:creature.goshnar_s_megalomania_purple"
            | "oteryn:creature.goshnar_s_megalomania_green"
            | "oteryn:creature.goshnar_s_megalomania_blue"
    )
}
fn burning(key: &str) -> bool {
    matches!(
        key,
        "oteryn:creature.ashes_of_burning_hatred"
            | "oteryn:creature.spark_of_burning_hatred"
            | "oteryn:creature.flame_of_burning_hatred"
            | "oteryn:creature.blaze_of_burning_hatred"
    )
}
#[derive(Debug, Clone, Copy)]
pub enum HatredEmpowerment {
    Add(u32),
    Reset,
}
impl ChannelRuntimeV1 {
    pub fn change_soul_war_hatred_percent(
        &mut self,
        expected: &runtime_actor_companion::CompanionSnapshot,
        expected_percent: u32,
        change: HatredEmpowerment,
    ) -> Result<(), CarrierError> {
        self.validate_companion_snapshot(expected)?;
        self.assert_actor_spell_unreserved(expected.actor)?;
        let current = self.soul_war_hatred_percent(expected.actor)?;
        if current != expected_percent || expected.health <= 0 {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let next = match change {
            HatredEmpowerment::Add(value) => current
                .checked_add(value)
                .ok_or(CarrierError::CapacityArithmeticOverflow)?,
            HatredEmpowerment::Reset => 0,
        };
        let index = self.encounter_actor_index(expected.actor)?;
        let (carrier, _) = self.encounter_parts_mut();
        let Slot::CreatureOccupied { spell_combat, .. } = &mut carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        spell_combat.soul_war_hatred_percent = next;
        Ok(())
    }
    pub fn change_soul_war_burning_form(
        &mut self,
        expected: &runtime_actor_companion::CompanionSnapshot,
        next_key: &str,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        if !matches!(
            self.binding().runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        ) || expected.state.master.is_some()
            || !burning(&expected.state.policy.definition_key)
            || !burning(next_key)
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let prepared =
            self.prepare_restoring_soul_war_burning_transformation(expected, next_key)?;
        self.commit_creature_profile_transformation(prepared)?;
        Ok(())
    }
    /// Only current native boss snapshots can expose the source instance counter.
    pub fn soul_war_hatred_percent(&self, actor: ExactActorRef) -> Result<u32, CarrierError> {
        self.owner_fence()?;
        if !matches!(
            self.binding().runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        ) {
            return Err(CarrierError::WrongScope);
        }
        let snapshot = self.companion_snapshot(actor)?;
        if snapshot.state.master.is_some() || !empowered_boss(&snapshot.state.policy.definition_key)
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let index = self.encounter_actor_index(actor)?;
        match self.encounter_slot(index) {
            Slot::CreatureOccupied { spell_combat, .. } => Ok(spell_combat.soul_war_hatred_percent),
            _ => Err(CarrierError::NotCreature),
        }
    }
}

impl super::runtime_actor_spell::ActorCombatState {
    /// The actual combined health callback runs once on the staged target Slot.
    /// Reward attribution never substitutes for the actual Player source role.
    pub(in crate::carrier) fn stage_hatred_incoming_health(
        &mut self,
        target: &runtime_actor_companion::CompiledCreaturePolicy,
        source: &super::soul_war_health::HealthSourceContext,
    ) -> Result<(), CarrierError> {
        if !matches!(
            source.kind,
            super::soul_war_health::HealthSourceKind::Player(_)
        ) {
            return Ok(());
        }
        let Some(policy) = self
            .soul_war_health
            .as_deref()
            .filter(|policy| policy.is_empowered_boss())
        else {
            return Ok(());
        };
        if !policy.matches(
            source.content,
            &target.definition_key,
            &target.definition_revision,
        ) {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let next = self
            .soul_war_defense_delta
            .checked_add(self.soul_war_hatred_percent)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        target
            .defense
            .ok_or(CarrierError::InvalidCreatureTarget)?
            .checked_add(next)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        self.soul_war_defense_delta = next;
        Ok(())
    }
}

impl ChannelRuntimeV1 {
    /// Creature(name) sees registered actors until the death dispatcher removes
    /// them (Crystal game.cpp1150-1175), including a queued zero-HP successor.
    pub fn soul_war_megalomania_name_present(&self) -> Result<bool, CarrierError> {
        self.owner_fence()?;
        Ok(self
            .carrier
            .occupied
            .iter()
            .any(|&index| match &self.carrier.slots[index as usize] {
                Slot::CreatureOccupied {
                    companion: Some(companion),
                    spell_combat,
                    ..
                } => {
                    matches!(
                        companion.policy.definition_key.as_str(),
                        "oteryn:creature.goshnar_s_megalomania_purple"
                            | "oteryn:creature.goshnar_s_megalomania_green"
                            | "oteryn:creature.goshnar_s_megalomania_blue"
                    ) && spell_combat
                        .soul_war_health
                        .as_deref()
                        .is_some_and(|policy| {
                            policy.is_empowered_boss()
                                && policy.matches(
                                    self.content_pin().server_artifact_digest(),
                                    &companion.policy.definition_key,
                                    &companion.policy.definition_revision,
                                )
                        })
                }
                _ => false,
            }))
    }
}

impl ChannelRuntimeV1 {
    /// Literal Player-target branch, after block and before origin reductions.
    /// Lua binary64 multiplication is narrowed exactly like the donor's
    /// getNumber<int32_t> (creatureevent.cpp473; lua_functions_loader.hpp74-90).
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "finite nonnegative result is checked against the donor signed32 boundary before truncation"
    )]
    pub fn soul_war_hatred_player_primary(
        &self,
        source: ExactActorRef,
        original: u32,
    ) -> Result<u32, CarrierError> {
        self.owner_fence()?;
        let index = self.encounter_actor_index(source)?;
        let Slot::CreatureOccupied {
            companion: Some(companion),
            spell_combat,
            ..
        } = self.encounter_slot(index)
        else {
            return Ok(original);
        };
        // The literal alternate condition tests the Player's name against
        // Goshnar's Megalomania. CharacterName forbids apostrophes; no admitted
        // Player can satisfy it. Do not repair it into a Mego attacker test.
        if companion.policy.definition_key != HATRED {
            return Ok(original);
        }
        let policy = spell_combat
            .soul_war_health
            .as_deref()
            .ok_or(CarrierError::ContentPinWorldMismatch)?;
        if !policy.is_empowered_boss()
            || !policy.matches(
                self.content_pin().server_artifact_digest(),
                HATRED,
                &companion.policy.definition_revision,
            )
        {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        if spell_combat.soul_war_hatred_percent == 0 {
            return Ok(original);
        }
        let scaled =
            f64::from(original) * (1.0 + f64::from(spell_combat.soul_war_hatred_percent) / 100.0);
        if !scaled.is_finite() || scaled < 0.0 || scaled > f64::from(i32::MAX) {
            return Err(CarrierError::DamageOverflow);
        }
        Ok(scaled.trunc() as u32)
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
        runtime
            .install_companion_policies(super::super::soul_war_mirror_runtime::tests::policies())?;
        let maximum = runtime
            .companion_policy(HATRED)
            .map_err(|error| format!("actual native Hatred policy: {error:?}"))?
            .maximum_health;
        let actor = ExactActorRef(runtime.admit_encounter_fixture_actor(
            ActorState(1),
            HATRED,
            maximum,
            Some(LocalPosition {
                x: 33746,
                y: 31666,
                floor: 14,
            }),
        )?);
        runtime.install_creature_policy(actor, HATRED)?;
        Ok((runtime, actor))
    }

    #[test]
    fn srx3_hatred_actual_boss_percent_is_snapshot_fenced_and_reset_only_explicitly() -> TestResult
    {
        let (mut runtime, boss) = runtime()?;
        assert_eq!(runtime.soul_war_hatred_percent(boss)?, 0);
        let original = runtime.companion_snapshot(boss)?;
        runtime
            .change_soul_war_hatred_percent(&original, 0, HatredEmpowerment::Add(10))
            .map_err(|error| format!("REACHED exact Hatred mutation: {error:?}"))?;
        assert_eq!(runtime.soul_war_hatred_percent(boss)?, 10);
        let current = runtime.companion_snapshot(boss)?;
        runtime.change_soul_war_hatred_percent(&current, 10, HatredEmpowerment::Reset)?;
        assert_eq!(runtime.soul_war_hatred_percent(boss)?, 0);
        Ok(())
    }
    #[test]
    fn srx3_hatred_burning_settype_preserves_actor_and_restores_source_health() -> TestResult {
        let (mut runtime, _) = runtime()?;
        let ashes = "oteryn:creature.ashes_of_burning_hatred";
        let actor = ExactActorRef(runtime.admit_encounter_fixture_actor(
            ActorState(2),
            ashes,
            15000,
            Some(LocalPosition {
                x: 33743,
                y: 31599,
                floor: 14,
            }),
        )?);
        runtime.install_creature_policy(actor, ashes)?;
        runtime.set_test_creature_health(actor, 100)?;
        let before = runtime.companion_snapshot(actor)?;
        runtime
            .change_soul_war_burning_form(&before, "oteryn:creature.spark_of_burning_hatred")
            .map_err(|error| format!("REACHED exact Burning setType: {error:?}"))?;
        let after = runtime.companion_snapshot(actor)?;
        assert_eq!(after.actor, before.actor);
        assert_eq!((after.health, after.maximum_health), (15000, 15000));
        assert_eq!(after.state.policy.object_look_type, Some(34010));
        assert_eq!(
            runtime.read_actor_position(actor)?.position(),
            runtime.read_actor_position(before.actor)?.position()
        );
        assert!(
            runtime
                .change_soul_war_burning_form(&after, HATRED)
                .is_err()
        );
        assert!(
            runtime
                .change_soul_war_burning_form(&before, ashes)
                .is_err(),
            "stale pre-setType source cannot overwrite current form"
        );
        Ok(())
    }
    #[test]
    fn srx3_hatred_literal_ashes_spawn_forces_occupied_center_only_for_source_child() -> TestResult
    {
        let (mut runtime, boss) = runtime()?;
        let at = runtime.read_actor_position(boss)?.position();
        let ashes = "oteryn:creature.ashes_of_burning_hatred";
        let actor = runtime
            .spawn_soul_war_child(boss, ashes, at, |_, _| Some(true))
            .map_err(|error| format!("REACHED exact Hatred literal child spawn: {error:?}"))?;
        assert_eq!(runtime.read_actor_position(actor)?.position(), at);
        assert_eq!(runtime.companion_snapshot(actor)?.health, 15000);
        assert!(runtime.native_summon_master(actor).is_none());
        assert!(
            runtime
                .spawn_soul_war_child(
                    boss,
                    "oteryn:creature.spark_of_burning_hatred",
                    at,
                    |_, _| Some(true)
                )
                .is_err()
        );
        Ok(())
    }
}
