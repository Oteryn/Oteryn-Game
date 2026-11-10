//! Source-qualified Soul War creature mutations under the existing physical owner.
//! Crystal 201189f powerful_soul.lua:98-105, goshnars_greed.lua:149-171 and
//! lib/quests/soul_war.lua:979-1004. These are mutable actor facts, not edits to
//! the active Content policy table; the exact actor snapshot fences every write.
use super::*;

#[cfg(test)]
mod maw_tests;
mod megalomania;

pub const GREED: &str = "oteryn:creature.goshnar_s_greed";
pub const MALICE: &str = "oteryn:creature.goshnar_s_malice";
const HATRED: &str = "oteryn:creature.goshnar_s_hatred";
const CRUELTY: &str = "oteryn:creature.goshnar_s_cruelty";
const MEGALOMANIA_PURPLE: &str = "oteryn:creature.goshnar_s_megalomania_purple";
const MEGALOMANIA_GREEN: &str = "oteryn:creature.goshnar_s_megalomania_green";
const MEGALOMANIA_BLUE: &str = "oteryn:creature.goshnar_s_megalomania_blue";

#[derive(Debug, Clone, Copy)]
pub enum SoulWarCreatureChange {
    Immune(bool),
    Speed(u16),
    HealFull,
    Buff {
        defense: u32,
        reflection: u32,
        maximum_health: u32,
        health: u32,
    },
}

/// A source continuation retained by the Instance callback owner. Constructed
/// from a live source parent; it is not an actor reference or a placement grant.
#[derive(Debug)]
pub struct SoulWarSpawnSource {
    stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
    content: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
    family: &'static str,
}
fn soul_war_child_allowed(parent: &str, child: &str) -> bool {
    match parent {
        GREED => matches!(
            child,
            "oteryn:creature.greedbeast"
                | "oteryn:creature.soulsnatcher"
                | "oteryn:creature.weak_soul"
                | "oteryn:creature.strong_soul"
                | "oteryn:creature.powerful_soul"
                | "oteryn:creature.soul_sphere"
        ),
        MALICE => child == "oteryn:creature.soul_cage",
        HATRED => child == "oteryn:creature.ashes_of_burning_hatred",
        CRUELTY => child == "oteryn:creature.a_greedy_eye",
        // mechanics.lua930 uses default createMonster placement for this
        // delayed onDeath continuation. Lever force placement is separate.
        MEGALOMANIA_PURPLE | MEGALOMANIA_GREEN | MEGALOMANIA_BLUE => {
            child == "oteryn:creature.aspect_of_power"
        }
        _ => false,
    }
}

fn source_damage_immunities() -> Vec<String> {
    [
        "physical",
        "fire",
        "earth",
        "energy",
        "ice",
        "holy",
        "death",
        "life_drain",
        "mana_drain",
        "drown",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

impl ChannelRuntimeV1 {
    fn cruelty_slot(
        &self,
        expected: &runtime_actor_companion::CompanionSnapshot,
    ) -> Result<usize, CarrierError> {
        self.validate_companion_snapshot(expected)?;
        if expected.state.master.is_some()
            || expected.state.policy.definition_key != "oteryn:creature.goshnar_s_cruelty"
            || !self
                .soul_war_health_callback(expected.actor)?
                .is_some_and(|policy| policy.is_cruelty())
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        self.encounter_actor_index(expected.actor)
    }
    /// Source KV DATA from this exact current boss, independent of immutable type.
    pub fn soul_war_cruelty_drain(
        &self,
        expected: &runtime_actor_companion::CompanionSnapshot,
    ) -> Result<Option<u64>, CarrierError> {
        let index = self.cruelty_slot(expected)?;
        match self.encounter_slot(index) {
            Slot::CreatureOccupied { spell_combat, .. } => Ok(spell_combat.soul_war_cruelty_drain),
            _ => Err(CarrierError::NotCreature),
        }
    }
    /// The fixed source callback adds two defense and seeds absent drain at two.
    pub fn increase_soul_war_cruelty(
        &mut self,
        expected: &runtime_actor_companion::CompanionSnapshot,
        expected_drain: Option<u64>,
    ) -> Result<u64, CarrierError> {
        self.require_soul_war_scope()?;
        let index = self.cruelty_slot(expected)?;
        self.assert_actor_spell_unreserved(expected.actor)?;
        let (defense, drain) = match self.encounter_slot(index) {
            Slot::CreatureOccupied { spell_combat, .. } => (
                spell_combat.soul_war_defense_delta,
                spell_combat.soul_war_cruelty_drain,
            ),
            _ => return Err(CarrierError::NotCreature),
        };
        if drain != expected_drain {
            return Err(CarrierError::PlanConflict);
        }
        let defense = defense
            .checked_add(2)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        let drain = drain
            .unwrap_or(2)
            .checked_add(1)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        let (carrier, _) = self.encounter_parts_mut();
        let Slot::CreatureOccupied { spell_combat, .. } = &mut carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        spell_combat.soul_war_defense_delta = defense;
        spell_combat.soul_war_cruelty_drain = Some(drain);
        Ok(drain)
    }

    fn cruelty_maw_values(
        &self,
        expected: &runtime_actor_companion::CompanionSnapshot,
        expected_defense: u32,
        expected_drain: Option<u64>,
    ) -> Result<(usize, u32, Option<u64>), CarrierError> {
        self.require_soul_war_scope()?;
        let index = self.cruelty_slot(expected)?;
        if !self.source_creature_is_registered(expected.actor)?
            || self.effective_creature_defense(expected)? != expected_defense
        {
            return Err(CarrierError::PlanConflict);
        }
        let Slot::CreatureOccupied { spell_combat, .. } = self.encounter_slot(index) else {
            return Err(CarrierError::NotCreature);
        };
        if spell_combat.soul_war_cruelty_drain != expected_drain {
            return Err(CarrierError::PlanConflict);
        }
        let delta = spell_combat.soul_war_defense_delta;
        let defense = if delta == 0 {
            0
        } else {
            delta.checked_sub(2).ok_or(CarrierError::PlanConflict)?
        };
        Ok((
            index,
            defense,
            expected_drain.map(|value| value.saturating_sub(1)),
        ))
    }

    /// Preflight the source decrease while its exact physical slots remain reserved.
    pub fn validate_soul_war_cruelty_maw(
        &self,
        expected: &runtime_actor_companion::CompanionSnapshot,
        expected_defense: u32,
        expected_drain: Option<u64>,
    ) -> Result<(), CarrierError> {
        self.cruelty_maw_values(expected, expected_defense, expected_drain)
            .map(|_| ())
    }

    /// Crystal mechanics.lua:892-902: reduce only the source defense delta and
    /// the existing drain KV. An absent drain remains absent; base policy is immutable.
    pub fn decrease_soul_war_cruelty_from_maw(
        &mut self,
        expected: &runtime_actor_companion::CompanionSnapshot,
        expected_defense: u32,
        expected_drain: Option<u64>,
    ) -> Result<(), CarrierError> {
        let (index, defense, drain) =
            self.cruelty_maw_values(expected, expected_defense, expected_drain)?;
        self.assert_actor_spell_unreserved(expected.actor)?;
        let (carrier, _) = self.encounter_parts_mut();
        let Slot::CreatureOccupied { spell_combat, .. } = &mut carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        spell_combat.soul_war_defense_delta = defense;
        spell_combat.soul_war_cruelty_drain = drain;
        Ok(())
    }

    /// Complete the retained source USE only after a known fused receipt. Every
    /// fallible physical check precedes release; publication has no await or
    /// competing actor writer between the release and the canonical mutation.
    #[allow(
        clippy::expect_used,
        reason = "all retained slots and source values are preflighted before atomic publication"
    )]
    pub fn complete_soul_war_cruelty_maw(
        &mut self,
        reservation: super::runtime_actor_source_reservation::SourceActorReservation,
        proof: &impl super::runtime_actor_source_reservation::SourceActorReservationProof,
        target: Option<(
            &runtime_actor_companion::CompanionSnapshot,
            u32,
            Option<u64>,
        )>,
    ) -> Result<(), CarrierError> {
        self.validate_source_actor_reservation(&reservation, proof)?;
        if let Some((expected, defense, drain)) = target {
            self.validate_soul_war_cruelty_maw(expected, defense, drain)?;
        }
        self.release_source_actors_after_observed_outcome(reservation, proof)?;
        if let Some((expected, defense, drain)) = target {
            self.decrease_soul_war_cruelty_from_maw(expected, defense, drain)
                .expect("preflighted source Maw decrease after exact reservation release");
        }
        Ok(())
    }
}

impl ChannelRuntimeV1 {
    /// The sole permitted native-policy delta here is the exact source immunity
    /// vector installed on this current Greed/Malice incarnation. Every other
    /// immutable field must still equal the independently active native type.
    pub(super) fn accepts_source_derived_companion_policy(
        &self,
        current: &runtime_actor_companion::CompanionSnapshot,
    ) -> Result<bool, CarrierError> {
        let base = self.companion_policy(&current.state.policy.definition_key)?;
        if base.as_ref() == current.state.policy.as_ref() {
            return Ok(true);
        }
        let index = self
            .carrier
            .validate_ref(&self.continuity, current.actor.0)?;
        if !matches!(
            self.binding().runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        ) || !matches!(current.state.policy.definition_key.as_str(), GREED | MALICE)
            || current.state.master.is_some()
            || !matches!(&self.carrier.slots[index], Slot::CreatureOccupied { spell_combat, .. }
                if spell_combat.soul_war_source_immunity)
        {
            return Ok(false);
        }
        let mut expected = base.as_ref().clone();
        expected.damage_immunities = source_damage_immunities();
        Ok(&expected == current.state.policy.as_ref())
    }
    /// Borrow-only defense view of the independently current native actor.
    /// Source additive defense is installed by the same physical stat owner.
    pub fn effective_creature_defense(
        &self,
        expected: &runtime_actor_companion::CompanionSnapshot,
    ) -> Result<u32, CarrierError> {
        self.owner_fence()?;
        self.validate_companion_snapshot(expected)?;
        let index = self
            .carrier
            .validate_ref(&self.continuity, expected.actor.0)?;
        let Slot::CreatureOccupied { spell_combat, .. } = &self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        expected
            .state
            .policy
            .defense
            .ok_or(CarrierError::InvalidCreatureTarget)?
            .checked_add(spell_combat.soul_war_defense_delta)
            .ok_or(CarrierError::CapacityArithmeticOverflow)
    }
    pub fn soul_war_health_callback(&self,actor:ExactActorRef)
    ->Result<Option<&oteryn_gameplay_content::content::native_soul_war_callbacks::QualifiedSoulWarHealthPolicy>,CarrierError>{
        self.owner_fence()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied {
            companion,
            spell_combat,
            ..
        } = &self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        let Some(policy) = spell_combat.soul_war_health.as_deref() else {
            return Ok(None);
        };
        let Some(companion) = companion else {
            return Err(CarrierError::NotCreature);
        };
        if !policy.matches(
            self.content_pin().server_artifact_digest(),
            &companion.policy.definition_key,
            &companion.policy.definition_revision,
        ) {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        Ok(Some(policy))
    }
    fn require_soul_war_scope(&self) -> Result<(), CarrierError> {
        self.owner_fence()?;
        if !matches!(
            self.binding().runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        ) {
            return Err(CarrierError::WrongScope);
        }
        Ok(())
    }

    /// Exact source families only; the native loader must independently contain the
    /// child and the map owner must prove its current destination.
    pub fn spawn_soul_war_child(
        &mut self,
        parent: ExactActorRef,
        child: &str,
        at: MovementLocalPosition,
        map_admits: impl FnOnce(&ChannelRuntimeV1, MovementLocalPosition) -> Option<bool>,
    ) -> Result<ExactActorRef, EncounterSpawnError> {
        self.require_soul_war_scope()
            .map_err(EncounterSpawnError::Native)?;
        let snapshot = self
            .companion_snapshot(parent)
            .map_err(EncounterSpawnError::Native)?;
        let allowed = soul_war_child_allowed(&snapshot.state.policy.definition_key, child);
        if !allowed || self.native_summon_master(parent).is_some() {
            return Err(EncounterSpawnError::Native(
                CarrierError::CreatureTargetMismatch,
            ));
        }
        self.assert_actor_spell_unreserved(parent)
            .map_err(EncounterSpawnError::Native)?;
        let policy = self
            .companion_policy(child)
            .map_err(EncounterSpawnError::Native)?;
        let revision = policy.definition_revision.clone();
        // Crystal goshnars_greed.lua:161 and lib/quests/soul_war.lua:1000
        // force the Sphere/Cage centre; the Greed minions use force=false.
        // Hatred lever source soul_war.lua397-398 and boss_lever.lua228-229
        // likewise creates only its literal Ashes child with force=true.
        self.admit_native_masterless_with_placement(
            child,
            &revision,
            self.content_pin().server_artifact_digest(),
            at,
            matches!(
                child,
                "oteryn:creature.soul_sphere"
                    | "oteryn:creature.soul_cage"
                    | "oteryn:creature.ashes_of_burning_hatred"
                    | "oteryn:creature.a_greedy_eye"
            ),
            map_admits,
        )
    }

    /// The caller must separately qualify the bounded native source closure and
    /// exact pinned room. This captures current owner continuity, not those rules.
    pub fn qualify_soul_war_spawn_source(
        &mut self,
        parent: ExactActorRef,
    ) -> Result<SoulWarSpawnSource, CarrierError> {
        self.require_soul_war_scope()?;
        let snapshot = self.companion_snapshot(parent)?;
        let family = match snapshot.state.policy.definition_key.as_str() {
            GREED => GREED,
            MALICE => MALICE,
            HATRED => HATRED,
            CRUELTY => CRUELTY,
            MEGALOMANIA_PURPLE => MEGALOMANIA_PURPLE,
            MEGALOMANIA_GREEN => MEGALOMANIA_GREEN,
            MEGALOMANIA_BLUE => MEGALOMANIA_BLUE,
            _ => return Err(CarrierError::CreatureTargetMismatch),
        };
        if snapshot.state.master.is_some() || self.native_summon_master(parent).is_some() {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        self.assert_actor_spell_unreserved(parent)?;
        let stamp = self.issue_owner_work()?;
        let pin = self.content_pin();
        Ok(SoulWarSpawnSource {
            stamp,
            family,
            content: pin.server_artifact_digest(),
            client: pin.client_artifact_digest(),
            frame: pin.frame_binding_digest(),
            map: self.map_revision_digest(),
        })
    }

    /// Global Lua callbacks survive their original boss's death, but never their
    /// Instance owner. Current map admission and actual native policies remain mandatory.
    pub fn spawn_soul_war_child_from_source(
        &mut self,
        source: &SoulWarSpawnSource,
        child: &str,
        at: MovementLocalPosition,
        map_admits: impl FnOnce(&ChannelRuntimeV1, MovementLocalPosition) -> Option<bool>,
    ) -> Result<ExactActorRef, EncounterSpawnError> {
        self.require_soul_war_scope()
            .map_err(EncounterSpawnError::Native)?;
        let pin = self.content_pin();
        if !self
            .owner_fence()
            .map_err(EncounterSpawnError::Native)?
            .accepts_stamp(source.stamp)
            || pin.server_artifact_digest() != source.content
            || pin.client_artifact_digest() != source.client
            || pin.frame_binding_digest() != source.frame
            || self.map_revision_digest() != source.map
            || !soul_war_child_allowed(source.family, child)
        {
            return Err(EncounterSpawnError::Native(CarrierError::WrongScope));
        }
        let revision = self
            .companion_policy(child)
            .map_err(EncounterSpawnError::Native)?
            .definition_revision
            .clone();
        self.admit_native_masterless_with_placement(
            child,
            &revision,
            source.content,
            at,
            matches!(
                child,
                "oteryn:creature.soul_sphere"
                    | "oteryn:creature.soul_cage"
                    | "oteryn:creature.ashes_of_burning_hatred"
                    | "oteryn:creature.a_greedy_eye"
            ),
            map_admits,
        )
    }

    /// Fixed Malice/Megalomania shared white-floor callback authority. The original
    /// source boss may have retired; the current Instance and full pin must not.
    pub fn soul_war_white_tile_source_current(&self, source: &SoulWarSpawnSource) -> bool {
        matches!(
            source.family,
            MALICE | MEGALOMANIA_PURPLE | MEGALOMANIA_GREEN | MEGALOMANIA_BLUE
        ) && self.require_soul_war_scope().is_ok()
            && self
                .owner_fence()
                .is_ok_and(|fence| fence.accepts_stamp(source.stamp))
            && self.content_pin().server_artifact_digest() == source.content
            && self.content_pin().client_artifact_digest() == source.client
            && self.content_pin().frame_binding_digest() == source.frame
            && self.map_revision_digest() == source.map
    }

    pub fn change_soul_war_creature(
        &mut self,
        expected: &runtime_actor_companion::CompanionSnapshot,
        change: SoulWarCreatureChange,
    ) -> Result<(), CarrierError> {
        self.require_soul_war_scope()?;
        self.validate_companion_snapshot(expected)?;
        self.assert_actor_spell_unreserved(expected.actor)?;
        if !matches!(
            expected.state.policy.definition_key.as_str(),
            GREED | MALICE
        ) || expected.state.master.is_some()
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let index = self.encounter_actor_index(expected.actor)?;
        let mut next = expected.state.clone();
        let mut maximum = expected.maximum_health;
        let mut health = expected.health;
        let mut defense_delta = match self.encounter_slot(index) {
            Slot::CreatureOccupied { spell_combat, .. } => spell_combat.soul_war_defense_delta,
            _ => return Err(CarrierError::NotCreature),
        };
        let mut reflection = match self.encounter_slot(index) {
            Slot::CreatureOccupied { spell_combat, .. } => spell_combat.soul_war_reflection,
            _ => return Err(CarrierError::NotCreature),
        };
        match change {
            SoulWarCreatureChange::Immune(immune) => {
                let base = self.companion_policy(&next.policy.definition_key)?;
                let policy = Arc::make_mut(&mut next.policy);
                policy.damage_immunities = if immune {
                    source_damage_immunities()
                } else {
                    base.damage_immunities.clone()
                };
            }
            SoulWarCreatureChange::Speed(speed) => {
                next.creation_speed_delta = i32::from(speed)
                    .checked_sub(next.policy.base_speed)
                    .ok_or(CarrierError::CapacityArithmeticOverflow)?;
            }
            SoulWarCreatureChange::HealFull => health = maximum,
            SoulWarCreatureChange::Buff {
                defense,
                reflection: increase,
                maximum_health,
                health: heal,
            } => {
                next.policy
                    .defense
                    .ok_or(CarrierError::InvalidCreatureTarget)?
                    .checked_add(defense_delta)
                    .and_then(|value| value.checked_add(defense))
                    .ok_or(CarrierError::CapacityArithmeticOverflow)?;
                defense_delta = defense_delta
                    .checked_add(defense)
                    .ok_or(CarrierError::CapacityArithmeticOverflow)?;
                reflection = reflection
                    .checked_add(increase)
                    .ok_or(CarrierError::CapacityArithmeticOverflow)?;
                maximum = maximum
                    .checked_add(i64::from(maximum_health))
                    .ok_or(CarrierError::CapacityArithmeticOverflow)?;
                health = health
                    .checked_add(i64::from(heal))
                    .ok_or(CarrierError::CapacityArithmeticOverflow)?
                    .min(maximum);
            }
        }
        // All fallible checks/allocation finish before the physical slot replacement.
        let (carrier, _) = self.encounter_parts_mut();
        let Slot::CreatureOccupied {
            companion,
            health: stored_health,
            spell_combat,
            ..
        } = &mut carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        *companion = Some(Box::new(next));
        *stored_health = health;
        spell_combat.maximum_health = maximum;
        spell_combat.soul_war_reflection = reflection;
        spell_combat.soul_war_defense_delta = defense_delta;
        if let SoulWarCreatureChange::Immune(immune) = change {
            spell_combat.soul_war_source_immunity = immune;
        }
        Ok(())
    }

    pub fn soul_war_reflection(
        &self,
        actor: ExactActorRef,
        element: &str,
    ) -> Result<u32, CarrierError> {
        self.owner_fence()?;
        if !matches!(
            self.binding().runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        ) {
            return Ok(0);
        }
        let index = self.encounter_actor_index(actor)?;
        match self.encounter_slot(index) {
            Slot::CreatureOccupied { spell_combat, .. }
                if matches!(
                    element,
                    "physical" | "fire" | "earth" | "energy" | "ice" | "holy" | "death"
                ) =>
            {
                Ok(spell_combat.soul_war_reflection)
            }
            Slot::CreatureOccupied { .. } => Ok(0),
            _ => Err(CarrierError::NotCreature),
        }
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
        let maximum = runtime.companion_policy(GREED)?.maximum_health;
        let actor = ExactActorRef(runtime.admit_encounter_fixture_actor(
            ActorState(1),
            GREED,
            maximum,
            Some(LocalPosition {
                x: 33746,
                y: 31666,
                floor: 14,
            }),
        )?);
        runtime.install_creature_policy(actor, GREED)?;
        Ok((runtime, actor))
    }

    #[test]
    fn srx3_greed_forced_sphere_stacks_but_normal_minion_requires_a_free_cell() -> TestResult {
        let (mut runtime, boss) = runtime()?;
        let occupied = runtime.read_actor_position(boss)?.position();
        assert!(
            runtime
                .spawn_soul_war_child(boss, "oteryn:creature.weak_soul", occupied, |_, _| Some(
                    true
                ),)
                .is_err()
        );
        let sphere = runtime
            .spawn_soul_war_child(boss, "oteryn:creature.soul_sphere", occupied, |_, _| {
                Some(true)
            })
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(runtime.read_actor_position(sphere)?.position(), occupied);
        assert!(
            runtime
                .spawn_soul_war_child(boss, "oteryn:creature.soul_sphere", occupied, |_, _| None,)
                .is_err(),
            "force never replaces the current map proof"
        );
        let nearby = MovementLocalPosition {
            x: occupied.x - 1,
            ..occupied
        };
        let weak = runtime
            .spawn_soul_war_child(boss, "oteryn:creature.weak_soul", nearby, |_, _| Some(true))
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(runtime.read_actor_position(weak)?.position(), nearby);
        Ok(())
    }

    #[test]
    fn srx3_greed_physical_buff_and_immunity_are_actor_fenced_and_leave_content_unchanged()
    -> TestResult {
        let (mut runtime, actor) = runtime()?;
        let before = runtime.companion_snapshot(actor)?;
        runtime.change_soul_war_creature(&before, SoulWarCreatureChange::Immune(true))?;
        let immune = runtime.companion_snapshot(actor)?;
        assert!(
            immune
                .state
                .policy
                .damage_immunities
                .iter()
                .any(|key| key == "physical")
        );
        assert!(
            runtime
                .companion_policy(GREED)?
                .damage_immunities
                .is_empty()
        );
        assert_eq!(
            runtime.change_soul_war_creature(&before, SoulWarCreatureChange::HealFull),
            Err(CarrierError::PlanConflict)
        );
        runtime.change_soul_war_creature(&immune, SoulWarCreatureChange::Immune(false))?;
        let released = runtime.companion_snapshot(actor)?;
        assert!(released.state.policy.damage_immunities.is_empty());
        runtime.change_soul_war_creature(
            &released,
            SoulWarCreatureChange::Buff {
                defense: 10,
                reflection: 10,
                maximum_health: 10_000,
                health: 10_000,
            },
        )?;
        let buffed = runtime.companion_snapshot(actor)?;
        assert_eq!(
            before.maximum_health, 550_000,
            "actual source Greed maximum"
        );
        assert_eq!(
            (buffed.health, buffed.maximum_health),
            (before.health + 10_000, before.maximum_health + 10_000)
        );
        assert_eq!(buffed.state.policy.defense, Some(160));
        assert_eq!(runtime.effective_creature_defense(&buffed)?, 170);
        assert_eq!(runtime.companion_policy(GREED)?.defense, Some(160));
        assert_eq!(runtime.soul_war_reflection(actor, "death")?, 10);
        assert_eq!(runtime.soul_war_reflection(actor, "life_drain")?, 0);
        let registration = runtime.bind_creature_self_heal(
            GREED,
            "oteryn:ability.creature.goshnar_s_greed.defense-2",
            u64::try_from(before.maximum_health)?,
            1_000,
            1_000,
        )?;
        let mut heals = CreatureSelfHealLedger::default();
        let healed =
            runtime.commit_creature_self_heal(&mut heals, &registration, actor, 1, 1_000)?;
        assert_eq!(healed.health_after, buffed.maximum_health);
        runtime.retire_owner_cycle();
        assert_eq!(
            runtime.change_soul_war_creature(&buffed, SoulWarCreatureChange::HealFull),
            Err(CarrierError::WrongScope)
        );
        Ok(())
    }
    #[test]
    fn srx3_soul_war_source_continuation_survives_parent_retirement_but_not_scope_retirement()
    -> TestResult {
        let (mut runtime, boss) = runtime()?;
        let source = runtime.qualify_soul_war_spawn_source(boss)?;
        runtime.remove_encounter_actor(boss)?;
        let child = runtime
            .spawn_soul_war_child_from_source(
                &source,
                "oteryn:creature.weak_soul",
                MovementLocalPosition {
                    x: 11,
                    y: 10,
                    floor: 7,
                },
                |_, _| Some(true),
            )
            .map_err(|e| format!("{e:?}"))?;
        assert!(runtime.contains_live_creature(child));
        runtime.retire_owner_cycle();
        assert!(
            runtime
                .spawn_soul_war_child_from_source(
                    &source,
                    "oteryn:creature.weak_soul",
                    MovementLocalPosition {
                        x: 12,
                        y: 10,
                        floor: 7
                    },
                    |_, _| Some(true),
                )
                .is_err()
        );
        Ok(())
    }
    #[test]
    fn srx3_greed_source_immunity_keeps_canonical_periodic_owner_usable() -> TestResult {
        let (mut runtime, actor) = runtime()?;
        let before = runtime.companion_snapshot(actor)?;
        runtime.change_soul_war_creature(&before, SoulWarCreatureChange::Immune(true))?;
        let prepared=runtime.stage_creature_periodic_turn(actor,1000,
            oteryn_gameplay_foundation::foundation::condition::TickFacts::default())
            .map_err(|error|format!("source-derived immunity must not reject the same current native actor: {error:?}"))?;
        let receipt = runtime.commit_creature_periodic_turn(prepared)?;
        assert_eq!(receipt.health_after, before.health);
        assert_eq!(
            runtime.companion_policy(GREED)?.damage_immunities,
            before.state.policy.damage_immunities
        );
        Ok(())
    }
}
