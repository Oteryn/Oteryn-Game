//! ARENA1 A4 lines110-116 known mechanics; this creates no native creature profile.
use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DromeCreatureRole {
    Domestikion,
    Hoodinion,
    Mearidion,
    Scissorion,
    Murmillion,
}
impl DromeCreatureRole {
    pub const fn base_health(self) -> u64 {
        match self {
            Self::Domestikion => 750,
            Self::Hoodinion => 800,
            Self::Mearidion => 850,
            Self::Scissorion => 950,
            Self::Murmillion => 1100,
        }
    }
    pub const fn base_maximum_hit(self) -> u32 {
        match self {
            Self::Domestikion => 150,
            Self::Hoodinion => 300,
            Self::Mearidion => 250,
            Self::Scissorion => 220,
            Self::Murmillion => 80,
        }
    }
    pub const fn paralyze_duration_ms(self) -> Option<u32> {
        if matches!(self, Self::Murmillion) {
            Some(5000)
        } else {
            None
        }
    }
    pub(in crate::carrier) fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "oteryn:creature.domestikion" => Self::Domestikion,
            "oteryn:creature.hoodinion" => Self::Hoodinion,
            "oteryn:creature.mearidion" => Self::Mearidion,
            "oteryn:creature.scissorion" => Self::Scissorion,
            "oteryn:creature.murmillion" => Self::Murmillion,
            _ => return None,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DromeRoleState {
    kind: DromeCreatureRole,
    key: Box<str>,
    revision: Box<str>,
    next_paralyze: Option<u64>,
}
impl DromeRoleState {
    pub(super) fn native(
        snapshot: &crate::carrier::runtime_actor_companion::CompanionSnapshot,
    ) -> Option<Self> {
        if snapshot.state.master.is_some() || snapshot.state.policy.is_familiar {
            return None;
        }
        Some(Self::bound(
            DromeCreatureRole::from_key(&snapshot.state.policy.definition_key)?,
            snapshot,
        ))
    }
    fn bound(
        kind: DromeCreatureRole,
        snapshot: &crate::carrier::runtime_actor_companion::CompanionSnapshot,
    ) -> Self {
        Self {
            kind,
            key: snapshot
                .state
                .policy
                .definition_key
                .clone()
                .into_boxed_str(),
            revision: snapshot
                .state
                .policy
                .definition_revision
                .clone()
                .into_boxed_str(),
            next_paralyze: None,
        }
    }
    fn current(&self, state: &crate::carrier::runtime_actor_companion::CompanionState) -> bool {
        state.master.is_none()
            && !state.policy.is_familiar
            && self.key.as_ref() == state.policy.definition_key
            && self.revision.as_ref() == state.policy.definition_revision
    }
}
impl ChannelRuntimeV1 {
    fn drome_role_state(&self, actor: ExactActorRef) -> Option<&DromeRoleState> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0).ok()?;
        let Slot::CreatureOccupied {
            health: 1..,
            spell_combat,
            companion: Some(state),
            ..
        } = &self.carrier.slots[index]
        else {
            return None;
        };
        if !spell_combat.tibiadrome {
            return None;
        }
        let role = spell_combat.tibiadrome_modifiers.as_ref()?.role.as_ref()?;
        role.current(state).then_some(role)
    }
    fn drome_role_state_mut(
        &mut self,
        actor: ExactActorRef,
    ) -> Result<&mut DromeRoleState, CarrierError> {
        self.owner_fence()?;
        self.drome_role_state(actor)
            .ok_or(CarrierError::InvalidDamage)?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied { spell_combat, .. } = &mut self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        spell_combat
            .tibiadrome_modifiers
            .as_mut()
            .and_then(|policy| policy.role.as_mut())
            .ok_or(CarrierError::InvalidDamage)
    }
    pub fn begin_drome_native_wave(
        &mut self,
        actor: ExactActorRef,
        now: u64,
    ) -> Result<bool, CarrierError> {
        self.owner_fence()?;
        if self.drome_role_state(actor).is_none() {
            return Ok(false);
        }
        let role = self.drome_role_state_mut(actor)?;
        role.next_paralyze =
            (role.kind == DromeCreatureRole::Murmillion).then(|| now.saturating_add(10_000_000));
        Ok(true)
    }
    /// Explicit mechanics grant preserves the fixture's real immutable creature policy.
    #[cfg(any(test, feature = "test-support"))]
    pub fn test_bind_drome_creature_role(
        &mut self,
        actor: ExactActorRef,
        kind: DromeCreatureRole,
        started: u64,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        if !self.is_tibiadrome(actor) {
            return Err(CarrierError::InvalidDamage);
        }
        let snapshot = self.companion_snapshot(actor)?;
        if snapshot.state.master.is_some() || snapshot.state.policy.is_familiar {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied { spell_combat, .. } = &mut self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        spell_combat
            .tibiadrome_modifiers
            .as_mut()
            .ok_or(CarrierError::InvalidDamage)?
            .role = Some(DromeRoleState::bound(kind, &snapshot));
        self.begin_drome_native_wave(actor, started)?;
        Ok(())
    }
    pub fn drome_role_paralyze_due(&self, actor: ExactActorRef, now: u64) -> bool {
        self.drome_role_state(actor).is_some_and(|role| {
            role.kind == DromeCreatureRole::Murmillion
                && role.next_paralyze.is_some_and(|due| now >= due)
        })
    }
    pub fn drome_murmillion_target_due(
        &self,
        source: ExactActorRef,
        target: ExactActorRef,
        now: u64,
    ) -> Option<bool> {
        self.owner_fence().ok()?;
        if self.drome_role_state(source)?.kind != DromeCreatureRole::Murmillion
            || !self.is_tibiadrome(target)
            || self.drome_actor_modifiers(source)?.run != self.drome_actor_modifiers(target)?.run
        {
            return None;
        }
        Some(self.drome_role_paralyze_due(source, now))
    }
    pub fn drome_role_paralyze_applied(
        &mut self,
        actor: ExactActorRef,
        now: u64,
    ) -> Result<(), CarrierError> {
        if !self.drome_role_paralyze_due(actor, now) {
            return Err(CarrierError::PlanConflict);
        }
        self.drome_role_state_mut(actor)?.next_paralyze = Some(now.saturating_add(10_000_000));
        Ok(())
    }
    pub(in crate::carrier) fn drome_aura_percent(&self, actor: ExactActorRef) -> u32 {
        let Some(run) = self.drome_actor_modifiers(actor).map(|policy| policy.run) else {
            return 0;
        };
        let count = self
            .carrier
            .slots
            .iter()
            .filter(|slot| {
                let Slot::CreatureOccupied {
                    health: 1..,
                    spell_combat,
                    companion: Some(state),
                    ..
                } = slot
                else {
                    return false;
                };
                spell_combat.tibiadrome
                    && spell_combat
                        .tibiadrome_modifiers
                        .as_ref()
                        .is_some_and(|policy| {
                            policy.run == run
                                && policy.role.as_ref().is_some_and(|role| {
                                    role.kind == DromeCreatureRole::Domestikion
                                        && role.current(state)
                                })
                        })
            })
            .take(5)
            .count();
        u32::try_from(count).unwrap_or(5) * 10
    }
    pub fn drome_scissorion_invisible(
        &mut self,
        actor: ExactActorRef,
        now: u64,
    ) -> Result<bool, CarrierError> {
        self.owner_fence()?;
        if !self
            .drome_role_state(actor)
            .is_some_and(|role| role.kind == DromeCreatureRole::Scissorion)
        {
            return Err(CarrierError::InvalidDamage);
        }
        use crate::carrier::runtime_actor_conditions::{
            ActorConditionTransition, ApplicationFacts, ConditionDefinition, ConditionSource,
            ConditionSourceKind, ConditionValues,
        };
        let snapshot = self.companion_snapshot(actor)?;
        let definition = ConditionDefinition::new(
            "oteryn:condition.drome.scissorion.invisible",
            1,
            ConditionValues::Invisible { duration_ms: 3000 },
        )
        .ok_or(CarrierError::InvalidDamage)?;
        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
            self.content_pin().server_artifact_digest(),
        );
        let time = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now);
        let facts = ApplicationFacts {
            now,
            base_speed: snapshot
                .state
                .policy
                .base_speed
                .try_into()
                .map_err(|_| CarrierError::InvalidDamage)?,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: false,
            decision_root: &root,
            occurrence: role_occurrence(actor, now, "invisible"),
        };
        let plan = self
            .prepare_actor_condition(
                actor,
                None,
                ActorConditionTransition::Apply {
                    definition: &definition,
                    source: ConditionSource {
                        actor,
                        session: None,
                        kind: ConditionSourceKind::Creature,
                    },
                    immunities: &[],
                    facts,
                },
                time,
            )
            .map_err(|_| CarrierError::InvalidDamage)?;
        self.commit_actor_condition(&plan, time)
            .map_err(|_| CarrierError::InvalidDamage)?;
        Ok(true)
    }
    pub fn drome_role_attack_started(
        &mut self,
        actor: ExactActorRef,
        now: u64,
    ) -> Result<bool, CarrierError> {
        self.owner_fence()?;
        if !self
            .drome_role_state(actor)
            .is_some_and(|role| role.kind == DromeCreatureRole::Scissorion)
        {
            return Ok(false);
        }
        use crate::carrier::runtime_actor_conditions::{ConditionType, StatusKind};
        let store = self
            .actor_conditions(actor, None)
            .map_err(|_| CarrierError::InvalidDamage)?;
        if !store.invisible_at(now) && !store.has_status(StatusKind::Invisible, now) {
            return Ok(false);
        }
        let time = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now);
        let plan = self
            .prepare_actor_condition_removal(
                actor,
                None,
                ConditionType::Invisible,
                role_occurrence(actor, now, "attack"),
                time,
            )
            .map_err(|_| CarrierError::InvalidDamage)?;
        self.commit_actor_condition(&plan, time)
            .map_err(|_| CarrierError::InvalidDamage)?;
        Ok(true)
    }
}

fn role_occurrence(
    actor: ExactActorRef,
    now: u64,
    operation: &str,
) -> oteryn_simulation_determinism::DecisionOccurrenceId {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(format!("drome-role:{operation}:{actor:?}:{now}"));
    let mut id = [0; 16];
    id.copy_from_slice(&digest[..16]);
    oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(id)
}
