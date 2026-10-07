//! Real companion entities in the Channel's existing fixed slots. Content policies are installed
//! by the active artifact loader; client names only resolve those already qualified records.
use super::*;
use crate::foundation::condition::{ConditionStore, ConditionType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CreatureExactRatio {
    pub(crate) numerator: i64,
    pub(crate) denominator: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreatureResistance {
    pub(crate) damage_type: String,
    pub(crate) percent: CreatureExactRatio,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CreatureFlags {
    pub(crate) attackable: bool,
    pub(crate) illusionable: bool,
    pub(crate) health_hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledCreaturePolicy {
    pub(crate) definition_key: String,
    pub(crate) definition_revision: String,
    pub(crate) display_name: String,
    pub(crate) maximum_health: i64,
    pub(crate) base_speed: i32,
    pub(crate) outfit_look_type: u32,
    /// Source lookTypeEx is an Object appearance, never an Outfit or an invisible actor.
    pub(crate) object_look_type: Option<u32>,
    pub(crate) summonable: bool,
    pub(crate) convinceable: bool,
    pub(crate) mana_cost: Option<u32>,
    pub(crate) is_familiar: bool,
    pub(crate) condition_immunities: Vec<ConditionType>,
    pub(crate) armor: Option<u32>,
    pub(crate) mitigation: Option<CreatureExactRatio>,
    pub(crate) resistances: Vec<CreatureResistance>,
    pub(crate) damage_immunities: Vec<String>,
    /// Separate source healingMap response, not an elemental resistance.
    pub(crate) healing_from_damage: Vec<CreatureResistance>,
    pub(crate) flags: CreatureFlags,
    pub(crate) preferred_distance: Option<u32>,
    pub(crate) reward_boss: Option<bool>,
}
impl CompiledCreaturePolicy {
    /// Source game.cpp healingMap is evaluated before blockHit/immunity, from
    /// the incoming pre-buff magnitude. No attacker means no damage-derived heal.
    pub(crate) fn damage_healing(
        &self,
        damage_type: &str,
        magnitude: i64,
    ) -> Result<i64, CarrierError> {
        if magnitude < 0 {
            return Err(CarrierError::InvalidDamage);
        }
        let Some(response) = self
            .healing_from_damage
            .iter()
            .find(|v| v.damage_type == damage_type)
        else {
            return Ok(0);
        };
        if response.percent.denominator == 0 || response.percent.numerator < 0 {
            return Err(CarrierError::DamageOverflow);
        }
        let numerator = i128::from(magnitude)
            .checked_mul(i128::from(response.percent.numerator))
            .ok_or(CarrierError::DamageOverflow)?;
        let denominator = i128::from(response.percent.denominator)
            .checked_mul(100)
            .ok_or(CarrierError::DamageOverflow)?;
        let rounded = numerator
            .checked_add(denominator - 1)
            .ok_or(CarrierError::DamageOverflow)?
            / denominator;
        i64::try_from(rounded).map_err(|_| CarrierError::DamageOverflow)
    }
    pub(crate) fn outfit_appearance(&self) -> Option<u32> {
        (self.object_look_type.is_none() && self.outfit_look_type != 0)
            .then_some(self.outfit_look_type)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledCreaturePolicies {
    source_digest: [u8; 32],
    policies: Vec<Arc<CompiledCreaturePolicy>>,
}
impl CompiledCreaturePolicies {
    /// Content's already-qualified outer artifact contains these exact decoded records.
    /// Rebinding data does not issue runtime scope or current mutation authority.
    pub(crate) fn bind_qualified_outer_artifact(
        &mut self,
        digest: [u8; 32],
    ) -> Result<(), CarrierError> {
        if digest == [0; 32] {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        self.source_digest = digest;
        Ok(())
    }
    /// The activation loader supplies only decoded, artifact-qualified CreatureAuthoring data.
    /// Missing health/speed/outfit/details must be rejected there, never filled with prototypes.
    pub(crate) fn from_active_artifact(
        source_digest: [u8; 32],
        records: Vec<CompiledCreaturePolicy>,
    ) -> Result<Self, CarrierError> {
        if source_digest == [0; 32] {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let mut policies = Vec::new();
        for record in records {
            if record.definition_key.is_empty()
                || record.definition_revision.is_empty()
                || record.display_name.trim().is_empty()
                || record.maximum_health <= 0
                || record.base_speed < 0
                || record.object_look_type.is_some_and(|look| {
                    look == 0 || record.outfit_look_type != 0 || record.is_familiar
                })
                || record.mitigation.is_some_and(|v| v.denominator == 0)
                || record
                    .resistances
                    .iter()
                    .any(|v| v.damage_type.is_empty() || v.percent.denominator == 0)
                || record.resistances.iter().enumerate().any(|(i, v)| {
                    record.resistances[..i]
                        .iter()
                        .any(|old| old.damage_type == v.damage_type)
                })
                || record.healing_from_damage.iter().enumerate().any(|(i, v)| {
                    v.damage_type.is_empty()
                        || v.percent.denominator == 0
                        || v.percent.numerator < 0
                        || record.healing_from_damage[..i]
                            .iter()
                            .any(|old| old.damage_type == v.damage_type)
                })
                || record.damage_immunities.iter().any(String::is_empty)
                || (record.summonable || record.convinceable) != record.mana_cost.is_some()
                || policies.iter().any(|p: &Arc<CompiledCreaturePolicy>| {
                    p.definition_key == record.definition_key
                })
            {
                return Err(CarrierError::InvalidCreatureTarget);
            }
            policies.push(Arc::new(record));
        }
        Ok(Self {
            source_digest,
            policies,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CompanionMaster {
    pub(crate) actor: ExactActorRef,
    pub(crate) session: GameSessionId,
}
/// Qualified active-Content data for a real familiar self-defense. The consumer
/// independently proves actual actor/master/incarnation and the matching Content pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FamiliarSelfHealDefense {
    pub(crate) source_digest: [u8; 32],
    pub(crate) creature_key: String,
    pub(crate) creature_revision: String,
    pub(crate) profile_digest: [u8; 32],
    pub(crate) interval_ms: u32,
    pub(crate) chance_percent: u8,
    pub(crate) heal_min: i64,
    pub(crate) heal_max: i64,
    pub(crate) effect: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompanionState {
    pub(crate) policy: Arc<CompiledCreaturePolicy>,
    pub(crate) master: Option<CompanionMaster>,
    pub(crate) creation_speed_delta: i32,
    pub(crate) conditions: ConditionStore<ExactActorRef>,
    pub(crate) outfit_look_type: u32,
    pub(crate) object_look_type: Option<u32>,
    pub(crate) expires_at_unix: Option<i64>,
    pub(crate) lifecycle_epoch: u64,
    pub(crate) party_protection: bool,
    pub(crate) created_at_us: Option<u64>,
    pub(crate) familiar_defense: Option<FamiliarDefenseClock>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FamiliarDefenseClock {
    pub(crate) profile_digest: [u8; 32],
    pub(crate) next_due_us: u64,
    pub(crate) ordinal: u64,
}
impl CompanionState {
    pub(crate) fn current_speed(&self, now_micros: u64) -> Result<i32, CarrierError> {
        let total = i64::from(self.policy.base_speed)
            + i64::from(self.creation_speed_delta)
            + self.conditions.speed_delta_at(now_micros);
        i32::try_from(total)
            .ok()
            .filter(|speed| *speed >= 0)
            .ok_or(CarrierError::PlanConflict)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompanionSnapshot {
    pub(crate) actor: ExactActorRef,
    pub(crate) state: CompanionState,
    pub(crate) health: i64,
    pub(crate) maximum_health: i64,
    pub(crate) position: MovementLocalPosition,
    pub(crate) position_revision: u64,
}

/// A physical profile replacement prepared from the actual actor slot. It retains
/// the actor's incarnation and combat occurrence history; its definition identity changes.
/// The source encounter owns its callback replay and timer; this is not appearance.
#[derive(Debug)]
pub(crate) struct PreparedCreatureProfileTransformation {
    expected: CompanionSnapshot,
    content: (WorldId, u64, [[u8; 32]; 4], LocalPosition),
    resulting: CompanionSnapshot,
    replacement: Box<CompanionState>,
    target_identity: Arc<[u8]>,
}
impl ChannelRuntimeV1 {
    fn creature_transform_content_binding(&self) -> (WorldId, u64, [[u8; 32]; 4], LocalPosition) {
        let p = self.content_pin();
        (
            p.world_id,
            p.activation_sequence,
            [
                p.server_artifact_digest,
                p.client_artifact_digest,
                p.frame_binding_digest,
                p.map_revision_digest,
            ],
            p.entry_start,
        )
    }
    pub(crate) fn prepare_creature_profile_transformation(
        &self,
        expected: &CompanionSnapshot,
        new_definition_key: &str,
    ) -> Result<PreparedCreatureProfileTransformation, CarrierError> {
        self.assert_actor_spell_unreserved(expected.actor)?;
        self.validate_companion_snapshot(expected)?;
        let old_policy = self.companion_policy(&expected.state.policy.definition_key)?;
        let policy = self.companion_policy(new_definition_key)?;
        if old_policy.as_ref() != expected.state.policy.as_ref()
            || !self.matches_live_creature_identity(
                expected.actor,
                old_policy.definition_key.as_bytes(),
            )
            || policy.definition_key != new_definition_key
            || expected.state.policy.is_familiar
            || policy.is_familiar
            || expected.health <= 0
            || expected.health > policy.maximum_health
        {
            return Err(CarrierError::PlanConflict);
        }
        let target_identity = Arc::from(copy_bounded_binding(policy.definition_key.as_bytes())?);
        let mut resulting = expected.clone();
        resulting.maximum_health = policy.maximum_health;
        resulting.state.outfit_look_type = policy.outfit_look_type;
        resulting.state.object_look_type = policy.object_look_type;
        resulting.state.lifecycle_epoch = expected
            .state
            .lifecycle_epoch
            .checked_add(1)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        resulting.state.policy = policy;
        Ok(PreparedCreatureProfileTransformation {
            expected: expected.clone(),
            content: self.creature_transform_content_binding(),
            replacement: Box::new(resulting.state.clone()),
            resulting,
            target_identity,
        })
    }
    pub(crate) fn commit_creature_profile_transformation(
        &mut self,
        prepared: PreparedCreatureProfileTransformation,
    ) -> Result<CompanionSnapshot, CarrierError> {
        self.assert_actor_spell_unreserved(prepared.expected.actor)?;
        if self.creature_transform_content_binding() != prepared.content {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        self.validate_companion_snapshot(&prepared.expected)?;
        let policy = self.companion_policy(&prepared.resulting.state.policy.definition_key)?;
        if policy.as_ref() != prepared.resulting.state.policy.as_ref() {
            return Err(CarrierError::PlanConflict);
        }
        let index = self
            .carrier
            .validate_ref(&self.continuity, prepared.expected.actor.0)?;
        // All fallible checks and replacement allocation precede the one physical write.
        let replacement = prepared.replacement;
        let Slot::CreatureOccupied {
            companion,
            spell_combat,
            target_identity,
            ..
        } = &mut self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        *target_identity = prepared.target_identity;
        *companion = Some(replacement);
        spell_combat.maximum_health = prepared.resulting.maximum_health;
        Ok(prepared.resulting)
    }
}

/// Immutable intent prepared from the actual current Creature slot. The existing
/// combat batch reserves, revalidates and installs this master change together
/// with the caster payment; it is never a separate companion state owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedCompanionAssignment {
    owner: ExactActorRef,
    session: GameSessionId,
    expected: CompanionSnapshot,
}
impl PreparedCompanionAssignment {
    pub(crate) fn actor(&self) -> ExactActorRef {
        self.expected.actor
    }
    pub(crate) fn position(&self) -> MovementLocalPosition {
        self.expected.position
    }
    pub(crate) fn policy(&self) -> &CompiledCreaturePolicy {
        &self.expected.state.policy
    }
    pub(crate) fn prior_master(&self) -> Option<CompanionMaster> {
        self.expected.state.master
    }
    pub(crate) fn snapshot(&self) -> &CompanionSnapshot {
        &self.expected
    }
    pub(crate) fn owner(&self) -> ExactActorRef {
        self.owner
    }
    pub(crate) fn session(&self) -> GameSessionId {
        self.session
    }
}
impl ChannelRuntimeV1 {
    pub(crate) fn prepare_companion_assignment(
        &self,
        owner: ExactActorRef,
        session: GameSessionId,
        expected: &CompanionSnapshot,
    ) -> Result<PreparedCompanionAssignment, CarrierError> {
        self.assert_actor_spell_unreserved(expected.actor)?;
        self.carrier
            .player_slot_index(&self.continuity, owner.0, session)?;
        self.validate_companion_snapshot(expected)?;
        if expected.health <= 0
            || self
                .companion_policy(&expected.state.policy.definition_key)?
                .as_ref()
                != expected.state.policy.as_ref()
        {
            return Err(CarrierError::PlanConflict);
        }
        Ok(PreparedCompanionAssignment {
            owner,
            session,
            expected: expected.clone(),
        })
    }
    pub(crate) fn validate_companion_assignment(
        &self,
        prepared: &PreparedCompanionAssignment,
    ) -> Result<(), CarrierError> {
        self.carrier
            .player_slot_index(&self.continuity, prepared.owner.0, prepared.session)?;
        self.validate_companion_snapshot(&prepared.expected)?;
        if prepared.expected.health <= 0
            || self
                .companion_policy(&prepared.expected.state.policy.definition_key)?
                .as_ref()
                != prepared.expected.state.policy.as_ref()
        {
            return Err(CarrierError::PlanConflict);
        }
        Ok(())
    }
}

/// A prediction of the next actual fixed-slot incarnation. Private construction binds the
/// free-list head and its generation; holding the Channel owner lock makes installation
/// deterministic across a durable transaction. This is intent, never external authority.
#[derive(Debug, Clone)]
pub(crate) struct PreparedCompanionSpawn {
    actor: ExactActorRef,
    owner: ExactActorRef,
    session: GameSessionId,
    position: MovementLocalPosition,
    state: CompanionState,
    planned: Option<Box<Slot>>,
}
impl PreparedCompanionSpawn {
    /// Bind the real creator's frozen semantic owner time before physical reservation.
    /// Unknown source time is retained as None, never inferred from expiry/observation.
    pub(crate) fn bind_semantic_creation(&mut self, at_us: u64) -> Result<(), CarrierError> {
        if self.state.created_at_us.is_some_and(|v| v != at_us) {
            return Err(CarrierError::PlanConflict);
        }
        let Some(planned) = self.planned.as_mut() else {
            return Err(CarrierError::PlanConflict);
        };
        let Slot::CreatureOccupied {
            companion: Some(state),
            ..
        } = planned.as_mut()
        else {
            return Err(CarrierError::PlanConflict);
        };
        self.state.created_at_us = Some(at_us);
        state.created_at_us = Some(at_us);
        Ok(())
    }
    pub(crate) fn actor(&self) -> ExactActorRef {
        self.actor
    }
    pub(crate) fn position(&self) -> MovementLocalPosition {
        self.position
    }
    pub(crate) fn policy(&self) -> &CompiledCreaturePolicy {
        &self.state.policy
    }
}

impl ChannelRuntimeV1 {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_companion_spawn(
        &self,
        owner: ExactActorRef,
        session: GameSessionId,
        definition: &str,
        position: MovementLocalPosition,
        look: Option<u32>,
        expiry: Option<i64>,
        epoch: u64,
        creation_speed_delta: i32,
        party_protection: bool,
    ) -> Result<PreparedCompanionSpawn, CarrierError> {
        self.carrier
            .player_slot_index(&self.continuity, owner.0, session)?;
        let policy = self.companion_policy(definition)?;
        if !self.companion_creation_available(position)?
            || expiry.is_some_and(|v| v < 0)
            || (policy.object_look_type.is_some() && look.is_some())
            || (policy.is_familiar && epoch == 0)
            || i64::from(policy.base_speed) + i64::from(creation_speed_delta) < 0
        {
            return Err(CarrierError::PlanConflict);
        }
        let head = self
            .carrier
            .free_head
            .ok_or(CarrierError::CapacityExceeded)?;
        let index = usize::try_from(head).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        let Slot::VacantReusable { generation, .. } = self.carrier.slots[index] else {
            return Err(CarrierError::PlanConflict);
        };
        let generation = generation
            .checked_add(1)
            .ok_or(CarrierError::ActorGenerationExhausted)?;
        let local = head
            .checked_add(1)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        let actor = ExactActorRef(ActorRef {
            world_id: self.binding.world_id,
            channel_id: self.binding.channel_id,
            scope_generation: self.binding.scope_generation,
            actor_local_id: ActorLocalId(local),
            actor_local_generation: ActorLocalGeneration(generation),
        });
        let state = CompanionState {
            outfit_look_type: look.unwrap_or(policy.outfit_look_type),
            object_look_type: policy.object_look_type,
            policy: policy.clone(),
            master: Some(CompanionMaster {
                actor: owner,
                session,
            }),
            creation_speed_delta,
            conditions: ConditionStore::default(),
            expires_at_unix: expiry,
            lifecycle_epoch: epoch,
            party_protection,
            created_at_us: None,
            familiar_defense: None,
        };
        if policy.definition_key.len() > MAX_OWNER_COMMIT_BINDING_BYTES
            || !policy
                .definition_key
                .bytes()
                .all(|v| v.is_ascii_alphanumeric() || matches!(v, b':' | b'.' | b'_' | b'-' | b'/'))
        {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        let target_identity = Arc::from(copy_bounded_binding(policy.definition_key.as_bytes())?);
        let planned = Box::new(Slot::CreatureOccupied {
            generation,
            actor: ActorState(0),
            position: Some(VersionedPosition {
                actor_local_id: actor.0.actor_local_id,
                actor_local_generation: actor.0.actor_local_generation,
                context: self.pinned_position_context(),
                position: LocalPosition {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                },
                revision: 1,
                facing: None,
            }),
            target_identity,
            health: policy.maximum_health,
            committed: Box::default(),
            damage_contributors: Box::default(),
            companion: Some(Box::new(state.clone())),
            spell_combat: Box::new(runtime_actor_spell::ActorCombatState::creature(
                policy.maximum_health,
            )),
        });
        Ok(PreparedCompanionSpawn {
            actor,
            owner,
            session,
            position,
            state,
            planned: Some(planned),
        })
    }
    pub(crate) fn validate_companion_spawn(
        &self,
        prepared: &PreparedCompanionSpawn,
    ) -> Result<(), CarrierError> {
        self.carrier
            .player_slot_index(&self.continuity, prepared.owner.0, prepared.session)?;
        let index = self
            .carrier
            .validate_ref(&self.continuity, prepared.actor.0)?;
        let policy = self.companion_policy(&prepared.state.policy.definition_key)?;
        if policy.as_ref() != prepared.state.policy.as_ref() {
            return Err(CarrierError::PlanConflict);
        }
        match &self.carrier.slots[index] {
            Slot::VacantReusable { generation, .. }
                if generation.checked_add(1) == Some(prepared.actor.0.actor_local_generation.0)
                    && self.carrier.free_head.and_then(|v| usize::try_from(v).ok())
                        == Some(index) =>
            {
                let planned = prepared
                    .planned
                    .as_deref()
                    .ok_or(CarrierError::PlanConflict)?;
                self.validate_planned_companion_payload(prepared, planned)?;
            }
            Slot::CreatureReserved {
                generation,
                owner,
                session,
                planned,
            } if *generation == prepared.actor.0.actor_local_generation.0
                && *owner == prepared.owner
                && *session == prepared.session =>
            {
                self.validate_planned_companion_payload(prepared, planned)?;
            }
            _ => return Err(CarrierError::PlanConflict),
        }
        if !self.companion_position_clear_except(prepared.position, Some(prepared.actor)) {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        Ok(())
    }
    fn validate_planned_companion_payload(
        &self,
        prepared: &PreparedCompanionSpawn,
        planned: &Slot,
    ) -> Result<(), CarrierError> {
        let Slot::CreatureOccupied {
            generation,
            position: Some(position),
            health,
            spell_combat,
            companion: Some(state),
            target_identity,
            ..
        } = planned
        else {
            return Err(CarrierError::PlanConflict);
        };
        if *generation != prepared.actor.0.actor_local_generation.0
            || position.actor_local_id != prepared.actor.0.actor_local_id
            || position.actor_local_generation != prepared.actor.0.actor_local_generation
            || position.context != self.pinned_position_context()
            || position.position.x != prepared.position.x
            || position.position.y != prepared.position.y
            || position.position.floor != prepared.position.floor
            || position.revision != 1
            || position.facing.is_some()
            || *health != prepared.state.policy.maximum_health
            || spell_combat.maximum_health != prepared.state.policy.maximum_health
            || state.as_ref() != &prepared.state
            || target_identity.as_ref() != prepared.state.policy.definition_key.as_bytes()
        {
            return Err(CarrierError::PlanConflict);
        }
        Ok(())
    }
    fn companion_position_clear_except(
        &self,
        position: MovementLocalPosition,
        except: Option<ExactActorRef>,
    ) -> bool {
        let context = self.pinned_position_context();
        !self.carrier.slots.iter().enumerate().any(|(index, slot)| {
            if except.is_some_and(|actor| {
                usize::try_from(actor.0.actor_local_id.0).ok() == index.checked_add(1)
            }) {
                return false;
            }
            let position_in_slot = match slot {
                Slot::Occupied {
                    committed: true,
                    position: Some(p),
                    ..
                }
                | Slot::CreatureOccupied {
                    health: 1..,
                    position: Some(p),
                    ..
                } => Some(p),
                Slot::CreatureReserved { planned, .. } => match planned.as_ref() {
                    Slot::CreatureOccupied {
                        position: Some(p), ..
                    } => Some(p),
                    _ => None,
                },
                _ => None,
            };
            position_in_slot.is_some_and(|p| {
                p.context == context
                    && p.position.x == position.x
                    && p.position.y == position.y
                    && p.position.floor == position.floor
            })
        })
    }
    /// Claims the existing free-head slot before SQL. The generation is consumed even if
    /// a definite rollback later releases capacity. A lost COMMIT response retains this slot.
    pub(crate) fn reserve_companion_spawn(
        &mut self,
        prepared: &mut PreparedCompanionSpawn,
    ) -> Result<(), CarrierError> {
        self.validate_companion_spawn(prepared)?;
        let index = self
            .carrier
            .validate_ref(&self.continuity, prepared.actor.0)?;
        if matches!(&self.carrier.slots[index], Slot::CreatureReserved { .. }) {
            return Ok(());
        }
        let Slot::VacantReusable { next_free, .. } = self.carrier.slots[index] else {
            return Err(CarrierError::PlanConflict);
        };
        let entry = u32::try_from(index).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        let planned = prepared.planned.take().ok_or(CarrierError::PlanConflict)?;
        self.carrier.slots[index] = Slot::CreatureReserved {
            generation: prepared.actor.0.actor_local_generation.0,
            owner: prepared.owner,
            session: prepared.session,
            planned,
        };
        self.carrier.free_head = next_free;
        // VIS-3 (Codex 4178855592): within the capacity reserved at bootstrap; the install keeps
        // the entry, so the installed companion is in the census.
        self.carrier.occupied.push(entry);
        Ok(())
    }
    /// Only a definite durable rollback may call this on the same sealed preparation.
    /// No Drop implementation speculates that an ambiguous COMMIT rolled back.
    pub(crate) fn rollback_companion_spawn(
        &mut self,
        prepared: &PreparedCompanionSpawn,
    ) -> Result<(), CarrierError> {
        self.validate_companion_spawn(prepared)?;
        let index = self
            .carrier
            .validate_ref(&self.continuity, prepared.actor.0)?;
        if !matches!(&self.carrier.slots[index], Slot::CreatureReserved { .. }) {
            return Err(CarrierError::PlanConflict);
        }
        let free = u32::try_from(index).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        self.carrier.slots[index] = Slot::VacantReusable {
            generation: prepared.actor.0.actor_local_generation.0,
            next_free: self.carrier.free_head,
        };
        self.carrier.free_head = Some(free);
        self.carrier.unindex_occupied(free);
        Ok(())
    }
    pub(crate) fn install_companion_spawn(
        &mut self,
        mut prepared: PreparedCompanionSpawn,
    ) -> Result<ExactActorRef, CarrierError> {
        self.reserve_companion_spawn(&mut prepared)?;
        let index = self
            .carrier
            .validate_ref(&self.continuity, prepared.actor.0)?;
        let reserved = std::mem::replace(
            &mut self.carrier.slots[index],
            Slot::Exhausted {
                generation: prepared.actor.0.actor_local_generation.0,
            },
        );
        let Slot::CreatureReserved { planned, .. } = reserved else {
            unreachable!("validated exact companion reservation must remain reserved");
        };
        self.carrier.slots[index] = *planned;
        Ok(prepared.actor)
    }
    pub(crate) fn install_companion_policies(
        &mut self,
        table: CompiledCreaturePolicies,
    ) -> Result<(), CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        if table.source_digest != self.content.server_artifact_digest {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        if let Some(existing) = &self.companion_policies {
            return if existing.as_ref() == &table {
                Ok(())
            } else {
                Err(CarrierError::PlanConflict)
            };
        }
        self.companion_policies = Some(Arc::new(table));
        Ok(())
    }
    pub(crate) fn companion_policy(
        &self,
        name_or_key: &str,
    ) -> Result<Arc<CompiledCreaturePolicy>, CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        let table = self
            .companion_policies
            .as_ref()
            .ok_or(CarrierError::InvalidCreatureTarget)?;
        if table.source_digest != self.content.server_artifact_digest {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        // Qualified exact identity always wins over the display-name fallback.
        // Source variants may share a real display name; never select one by export order.
        if let Some(exact) = table
            .policies
            .iter()
            .find(|p| p.definition_key == name_or_key)
        {
            return Ok(Arc::clone(exact));
        }
        let mut matches = table
            .policies
            .iter()
            .filter(|p| p.display_name.eq_ignore_ascii_case(name_or_key));
        let first = matches.next().ok_or(CarrierError::InvalidCreatureTarget)?;
        if matches.next().is_some() {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        Ok(Arc::clone(first))
    }
    /// Active spawn realization binds its wild creature to its actual decoded policy. Convince
    /// subsequently reads this binding, never guesses the definition from a spawn instance id.
    pub(crate) fn install_creature_policy(
        &mut self,
        actor: ExactActorRef,
        definition_key: &str,
    ) -> Result<(), CarrierError> {
        self.assert_actor_spell_unreserved(actor)?;
        let policy = self.companion_policy(definition_key)?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied {
            health,
            spell_combat,
            companion,
            ..
        } = &mut self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if *health <= 0 || spell_combat.maximum_health != policy.maximum_health {
            return Err(CarrierError::InvalidCreatureHealth);
        }
        let state = CompanionState {
            creation_speed_delta: 0,
            conditions: ConditionStore::default(),
            outfit_look_type: policy.outfit_look_type,
            object_look_type: policy.object_look_type,
            policy,
            master: None,
            expires_at_unix: None,
            lifecycle_epoch: 0,
            party_protection: false,
            created_at_us: None,
            familiar_defense: None,
        };
        if let Some(existing) = companion {
            return if existing.as_ref() == &state {
                Ok(())
            } else {
                Err(CarrierError::PlanConflict)
            };
        }
        *companion = Some(Box::new(state));
        Ok(())
    }
    pub(crate) fn companion_snapshot(
        &self,
        actor: ExactActorRef,
    ) -> Result<CompanionSnapshot, CarrierError> {
        let snapshot = self.companion_snapshot_including_dead(actor)?;
        if snapshot.health <= 0 {
            return Err(CarrierError::CreatureNotActionable);
        }
        Ok(snapshot)
    }
    /// Compare the retained actual-slot snapshot without allocating a replacement snapshot.
    /// The immutable snapshot identifies expected data; current carrier continuity and exact
    /// incarnation are independently checked at this consuming boundary.
    pub(crate) fn validate_companion_snapshot(
        &self,
        expected: &CompanionSnapshot,
    ) -> Result<(), CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        let index = self
            .carrier
            .validate_ref(&self.continuity, expected.actor.0)?;
        let Slot::CreatureOccupied {
            generation,
            health,
            spell_combat,
            companion: Some(state),
            position: Some(position),
            ..
        } = &self.carrier.slots[index]
        else {
            return Err(CarrierError::InvalidCreatureTarget);
        };
        if *generation != expected.actor.0.actor_local_generation.0
            || position.context != self.pinned_position_context()
            || position.actor_local_id != expected.actor.0.actor_local_id
            || position.actor_local_generation != expected.actor.0.actor_local_generation
            || position.position.x != expected.position.x
            || position.position.y != expected.position.y
            || position.position.floor != expected.position.floor
            || position.revision != expected.position_revision
            || *health != expected.health
            || spell_combat.maximum_health != expected.maximum_health
            || state.as_ref() != &expected.state
        {
            return Err(CarrierError::PlanConflict);
        }
        Ok(())
    }
    /// Used only for a real lethal physical receipt hook: health remains the actual slot value.
    pub(crate) fn companion_snapshot_including_dead(
        &self,
        actor: ExactActorRef,
    ) -> Result<CompanionSnapshot, CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        let index = usize::try_from(
            actor
                .0
                .actor_local_id
                .0
                .checked_sub(1)
                .ok_or(CarrierError::InvalidActorIdentity)?,
        )
        .map_err(|_| CarrierError::InvalidActorIdentity)?;
        let Slot::CreatureOccupied {
            generation,
            health,
            spell_combat,
            position: Some(position),
            companion: Some(state),
            ..
        } = self
            .carrier
            .slots
            .get(index)
            .ok_or(CarrierError::InvalidActorIdentity)?
        else {
            return Err(CarrierError::NotCreature);
        };
        if actor.0.world_id != self.binding.world_id
            || actor.0.channel_id != self.binding.channel_id
            || actor.0.scope_generation != self.binding.scope_generation
        {
            return Err(CarrierError::WrongScope);
        }
        if *generation != actor.0.actor_local_generation.0 {
            return Err(CarrierError::StaleActorGeneration);
        }
        if position.context != self.pinned_position_context() {
            return Err(CarrierError::PositionContextMismatch);
        }
        Ok(CompanionSnapshot {
            actor,
            state: state.as_ref().clone(),
            health: *health,
            maximum_health: spell_combat.maximum_health,
            position: MovementLocalPosition {
                x: position.position.x,
                y: position.position.y,
                floor: position.position.floor,
            },
            position_revision: position.revision,
        })
    }
    pub(crate) fn owned_companions(
        &self,
        owner: ExactActorRef,
        session: GameSessionId,
    ) -> Result<Vec<CompanionSnapshot>, CarrierError> {
        self.carrier
            .player_slot_index(&self.continuity, owner.0, session)?;
        let master = CompanionMaster {
            actor: owner,
            session,
        };
        let mut out = Vec::new();
        for (index, slot) in self.carrier.slots.iter().enumerate() {
            if let Slot::CreatureOccupied {
                generation,
                companion: Some(state),
                health,
                ..
            } = slot
                && state.master == Some(master)
                && *health > 0
            {
                let local = u32::try_from(index + 1)
                    .map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
                let actor = ExactActorRef(ActorRef {
                    world_id: self.binding.world_id,
                    channel_id: self.binding.channel_id,
                    scope_generation: self.binding.scope_generation,
                    actor_local_id: ActorLocalId(local),
                    actor_local_generation: ActorLocalGeneration(*generation),
                });
                out.push(self.companion_snapshot(actor)?);
            }
        }
        Ok(out)
    }
    /// Exact current familiar deaths, read from the same physical slots. No client
    /// death notification or prototype/name match can manufacture this source census.
    pub(crate) fn dead_owned_familiars(
        &self,
        owner: ExactActorRef,
        session: GameSessionId,
    ) -> Result<Vec<ExactActorRef>, CarrierError> {
        self.carrier
            .player_slot_index(&self.continuity, owner.0, session)?;
        let master = CompanionMaster {
            actor: owner,
            session,
        };
        let mut out = Vec::new();
        for (index, slot) in self.carrier.slots.iter().enumerate() {
            if let Slot::CreatureOccupied {
                generation,
                companion: Some(state),
                health,
                ..
            } = slot
                && state.master == Some(master)
                && state.policy.is_familiar
                && *health == 0
            {
                let actor = ExactActorRef(ActorRef {
                    world_id: self.binding.world_id,
                    channel_id: self.binding.channel_id,
                    scope_generation: self.binding.scope_generation,
                    actor_local_id: ActorLocalId(
                        u32::try_from(index + 1)
                            .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                    ),
                    actor_local_generation: ActorLocalGeneration(*generation),
                });
                // A current foreign SQL reservation keeps its exact original death pending.
                self.assert_actor_spell_unreserved(actor)?;
                out.push(actor);
            }
        }
        Ok(out)
    }
    /// Complete owner census comparison without allocating or cloning live state.
    pub(crate) fn validate_owned_companions(
        &self,
        owner: ExactActorRef,
        session: GameSessionId,
        expected: &[CompanionSnapshot],
    ) -> Result<(), CarrierError> {
        self.carrier
            .player_slot_index(&self.continuity, owner.0, session)?;
        let mut cursor = 0;
        for (index, slot) in self.carrier.slots.iter().enumerate() {
            let Slot::CreatureOccupied {
                generation,
                health: 1..,
                companion: Some(state),
                ..
            } = slot
            else {
                continue;
            };
            if state.master
                != Some(CompanionMaster {
                    actor: owner,
                    session,
                })
            {
                continue;
            }
            let snapshot = expected.get(cursor).ok_or(CarrierError::PlanConflict)?;
            if usize::try_from(snapshot.actor.0.actor_local_id.0).ok() != index.checked_add(1)
                || snapshot.actor.0.actor_local_generation.0 != *generation
            {
                return Err(CarrierError::PlanConflict);
            }
            self.validate_companion_snapshot(snapshot)?;
            cursor += 1;
        }
        if cursor != expected.len() {
            return Err(CarrierError::PlanConflict);
        }
        Ok(())
    }
    pub(crate) fn companion_creation_available(
        &self,
        position: MovementLocalPosition,
    ) -> Result<bool, CarrierError> {
        self.carrier.validate_current_continuity(&self.continuity)?;
        let Some(head) = self.carrier.free_head else {
            return Ok(false);
        };
        let index = usize::try_from(head).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        if !matches!(self.carrier.slots.get(index),Some(Slot::VacantReusable {generation,..}) if generation.checked_add(1).is_some())
        {
            return Ok(false);
        }
        Ok(self.companion_position_clear_except(position, None))
    }
    /// Placement candidates are supplied by the actual map owner and rechecked for actor
    /// occupancy under this exclusive Channel borrow. All refusals precede admission.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_companion(
        &mut self,
        owner: ExactActorRef,
        session: GameSessionId,
        name: &str,
        position: MovementLocalPosition,
        look: Option<u32>,
        expiry: Option<i64>,
        epoch: u64,
    ) -> Result<ExactActorRef, CarrierError> {
        self.carrier
            .player_slot_index(&self.continuity, owner.0, session)?;
        let policy = self.companion_policy(name)?;
        let context = self.pinned_position_context();
        self.carrier.validate_position_context(context)?;
        if expiry.is_some_and(|value| value < 0)
            || (policy.is_familiar && epoch == 0)
            || (policy.object_look_type.is_some() && look.is_some())
        {
            return Err(CarrierError::InvalidCommitBinding);
        }
        if !self.companion_position_clear_except(position, None) {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        let state = Box::new(CompanionState {
            master: Some(CompanionMaster {
                actor: owner,
                session,
            }),
            creation_speed_delta: 0,
            conditions: ConditionStore::default(),
            outfit_look_type: look.unwrap_or(policy.outfit_look_type),
            object_look_type: policy.object_look_type,
            expires_at_unix: expiry,
            lifecycle_epoch: epoch,
            party_protection: false,
            created_at_us: None,
            familiar_defense: None,
            policy: policy.clone(),
        });
        let actor = self.carrier.admit_creature(
            &self.continuity,
            ActorState(0),
            &policy.definition_key,
            policy.maximum_health,
        )?;
        // Admission has established an unpositioned live slot. Direct initialization uses the
        // exact context already validated above, so no fallible operation follows allocation.
        let index = usize::try_from(actor.actor_local_id.0 - 1)
            .map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        if let Slot::CreatureOccupied {
            position: slot_position,
            companion,
            ..
        } = &mut self.carrier.slots[index]
        {
            *slot_position = Some(VersionedPosition {
                actor_local_id: actor.actor_local_id,
                actor_local_generation: actor.actor_local_generation,
                context,
                position: LocalPosition {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                },
                revision: 1,
                facing: None,
            });
            *companion = Some(state);
        }
        Ok(ExactActorRef(actor))
    }
    pub(crate) fn compare_assign_companion(
        &mut self,
        owner: ExactActorRef,
        session: GameSessionId,
        expected: &CompanionSnapshot,
    ) -> Result<CompanionSnapshot, CarrierError> {
        self.assert_actor_spell_unreserved(expected.actor)?;
        self.carrier
            .player_slot_index(&self.continuity, owner.0, session)?;
        let current = self.companion_snapshot(expected.actor)?;
        if &current != expected {
            return Err(CarrierError::PlanConflict);
        }
        let index = self
            .carrier
            .validate_ref(&self.continuity, expected.actor.0)?;
        if let Slot::CreatureOccupied {
            companion: Some(state),
            ..
        } = &mut self.carrier.slots[index]
        {
            state.master = Some(CompanionMaster {
                actor: owner,
                session,
            });
        }
        self.companion_snapshot(expected.actor)
    }
    pub(crate) fn compare_companion_state(
        &mut self,
        expected: &CompanionSnapshot,
        next: CompanionState,
    ) -> Result<(), CarrierError> {
        self.assert_actor_spell_unreserved(expected.actor)?;
        let current = self.companion_snapshot(expected.actor)?;
        if current != *expected
            || next.policy != current.state.policy
            || next.master != current.state.master
            || next.object_look_type != current.state.object_look_type
            || (next.object_look_type.is_some() && next.outfit_look_type != 0)
            || next.created_at_us != current.state.created_at_us
            || next.familiar_defense != current.state.familiar_defense
            || next
                .policy
                .base_speed
                .checked_add(next.creation_speed_delta)
                .is_none_or(|speed| speed < 0)
            || next.expires_at_unix.is_some_and(|time| time < 0)
        {
            return Err(CarrierError::PlanConflict);
        }
        if let Some(master) = next.master {
            self.carrier
                .player_slot_index(&self.continuity, master.actor.0, master.session)?;
        }
        let index = self
            .carrier
            .validate_ref(&self.continuity, expected.actor.0)?;
        if let Slot::CreatureOccupied { companion, .. } = &mut self.carrier.slots[index] {
            *companion = Some(Box::new(next));
        }
        Ok(())
    }
    /// Source party protection changes one existing owned field, without allocating
    /// a new condition/policy snapshot during durable successor installation.
    pub(crate) fn mark_companion_party_protection(
        &mut self,
        expected: &CompanionSnapshot,
    ) -> Result<(), CarrierError> {
        self.assert_actor_spell_unreserved(expected.actor)?;
        self.validate_companion_snapshot(expected)?;
        let index = self
            .carrier
            .validate_ref(&self.continuity, expected.actor.0)?;
        let Slot::CreatureOccupied {
            companion: Some(state),
            ..
        } = &mut self.carrier.slots[index]
        else {
            return Err(CarrierError::InvalidCreatureTarget);
        };
        state.party_protection = true;
        Ok(())
    }
    pub(crate) fn despawn_companion(
        &mut self,
        owner: ExactActorRef,
        session: GameSessionId,
        expected: &CompanionSnapshot,
    ) -> Result<(), CarrierError> {
        self.assert_actor_spell_unreserved(expected.actor)?;
        self.carrier
            .player_slot_index(&self.continuity, owner.0, session)?;
        if expected.state.master
            != Some(CompanionMaster {
                actor: owner,
                session,
            })
        {
            return Err(CarrierError::PlanConflict);
        }
        self.validate_companion_snapshot(expected)?;
        self.carrier
            .remove(&self.continuity, expected.actor.0)
            .map(|_| ())
    }
    /// Owner removal drops its summons without a familiar-death callback or expiry overwrite.
    pub(crate) fn remove_owned_companions(
        &mut self,
        owner: ExactActorRef,
        session: GameSessionId,
    ) -> Result<usize, CarrierError> {
        let owned = self.owned_companions(owner, session)?;
        for creature in &owned {
            self.assert_actor_spell_unreserved(creature.actor)?;
        }
        for creature in &owned {
            self.despawn_companion(owner, session, creature)?;
        }
        Ok(owned.len())
    }
    pub(crate) fn follow_companion(
        &mut self,
        owner: ExactActorRef,
        session: GameSessionId,
        expected: &CompanionSnapshot,
        destination: MovementLocalPosition,
    ) -> Result<(), CarrierError> {
        self.assert_actor_spell_unreserved(expected.actor)?;
        self.carrier
            .player_slot_index(&self.continuity, owner.0, session)?;
        if expected.state.master
            != Some(CompanionMaster {
                actor: owner,
                session,
            })
        {
            return Err(CarrierError::PlanConflict);
        }
        self.validate_companion_snapshot(expected)?;
        let current = self
            .carrier
            .read_position(&self.continuity, expected.actor.0)?;
        self.carrier.compare_commit_position(
            &self.continuity,
            current,
            self.pinned_position_context(),
            LocalPosition {
                x: destination.x,
                y: destination.y,
                floor: destination.floor,
            },
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;
    fn id(n: u8) -> [u8; 16] {
        [n, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, n]
    }
    fn policy(name: &str) -> CompiledCreaturePolicy {
        CompiledCreaturePolicy {
            definition_key: format!("creature:{name}"),
            definition_revision: "test-1".into(),
            display_name: name.into(),
            maximum_health: 812,
            base_speed: 220,
            outfit_look_type: 991,
            object_look_type: None,
            summonable: true,
            convinceable: true,
            mana_cost: Some(250),
            is_familiar: false,
            condition_immunities: Vec::new(),
            preferred_distance: Some(1),
            reward_boss: Some(false),
            armor: Some(10),
            mitigation: None,
            resistances: vec![],
            damage_immunities: vec![],
            healing_from_damage: vec![],
            flags: CreatureFlags {
                attackable: true,
                illusionable: false,
                health_hidden: false,
            },
        }
    }
    fn owner() -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
        owner_with_policies(vec![policy("rat")])
    }
    fn owner_with_policies(
        records: Vec<CompiledCreaturePolicy>,
    ) -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
        let world = WorldId::decode(&id(1)).unwrap();
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).unwrap(),
            NodeId::decode(&id(3)).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            ChannelContentPin::test(world),
        )
        .unwrap();
        let session = GameSessionId::decode(&id(4)).unwrap();
        let reserved = runtime.reserve_fresh_session(session).unwrap();
        let actor = runtime.commit_fresh_session(reserved).unwrap();
        runtime.initialize_first_entry_position(actor).unwrap();
        runtime
            .install_companion_policies(
                CompiledCreaturePolicies::from_active_artifact([1; 32], records).unwrap(),
            )
            .unwrap();
        (runtime, actor, session)
    }
    #[test]
    fn physical_profile_transform_preserves_hp_actor_history_and_returns_with_fresh_epoch() {
        let mut first = policy("guardian");
        first.maximum_health = 20;
        let mut alternate = policy("blazing");
        alternate.maximum_health = 20;
        alternate.outfit_look_type = 1001;
        alternate.base_speed = 250;
        alternate.resistances = vec![CreatureResistance {
            damage_type: "ice".into(),
            percent: CreatureExactRatio {
                numerator: -30,
                denominator: 1,
            },
        }];
        let (mut runtime, _, _) = owner_with_policies(vec![first, alternate]);
        let actor = runtime
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
                "creature:guardian",
                20,
            )
            .unwrap();
        runtime
            .install_creature_policy(actor, "creature:guardian")
            .unwrap();
        let before = runtime.companion_snapshot(actor).unwrap();
        let index = runtime
            .carrier
            .validate_ref(&runtime.continuity, actor.0)
            .unwrap();
        let previous_slot = runtime.carrier.slots[index].clone();
        let prepared = runtime
            .prepare_creature_profile_transformation(&before, "creature:blazing")
            .unwrap();
        let after = runtime
            .commit_creature_profile_transformation(prepared)
            .unwrap();
        assert_eq!(after.actor, before.actor);
        assert_eq!(after.health, 20);
        assert_eq!(after.state.policy.definition_key, "creature:blazing");
        assert_eq!(after.state.outfit_look_type, 1001);
        assert_eq!(after.state.current_speed(0).unwrap(), 250);
        assert_eq!(
            after.state.lifecycle_epoch,
            before.state.lifecycle_epoch + 1
        );
        if let (
            Slot::CreatureOccupied {
                target_identity: a,
                committed: ca,
                damage_contributors: da,
                ..
            },
            Slot::CreatureOccupied {
                target_identity: b,
                committed: cb,
                damage_contributors: db,
                ..
            },
        ) = (&previous_slot, &runtime.carrier.slots[index])
        {
            assert_eq!(a.as_ref(), b"creature:guardian");
            assert_eq!(b.as_ref(), b"creature:blazing");
            assert_eq!(ca, cb);
            assert_eq!(da, db);
        } else {
            panic!("physical Creature slots required");
        }
        assert!(runtime.validate_companion_snapshot(&before).is_err());
        let prepared = runtime
            .prepare_creature_profile_transformation(&after, "creature:guardian")
            .unwrap();
        let returned = runtime
            .commit_creature_profile_transformation(prepared)
            .unwrap();
        assert_eq!(returned.state.policy.definition_key, "creature:guardian");
        assert_eq!(returned.health, 20);
        assert_eq!(returned.state.lifecycle_epoch, 2);
    }
    #[test]
    fn physical_profile_transform_stale_actor_and_foreign_profile_never_mutate() {
        let mut first = policy("guardian");
        first.maximum_health = 20;
        let mut alternate = policy("blazing");
        alternate.maximum_health = 20;
        let mut too_small = policy("small");
        too_small.maximum_health = 19;
        let (mut runtime, _, _) = owner_with_policies(vec![first, alternate, too_small]);
        let actor = runtime
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
                "creature:guardian",
                20,
            )
            .unwrap();
        runtime
            .install_creature_policy(actor, "creature:guardian")
            .unwrap();
        let before = runtime.companion_snapshot(actor).unwrap();
        assert!(
            runtime
                .prepare_creature_profile_transformation(&before, "blazing")
                .is_err()
        );
        assert!(
            runtime
                .prepare_creature_profile_transformation(&before, "creature:unknown")
                .is_err()
        );
        assert!(
            runtime
                .prepare_creature_profile_transformation(&before, "creature:small")
                .is_err()
        );
        assert_eq!(runtime.companion_snapshot(actor).unwrap(), before);
        let stale = runtime
            .prepare_creature_profile_transformation(&before, "creature:blazing")
            .unwrap();
        let current = runtime
            .prepare_creature_profile_transformation(&before, "creature:blazing")
            .unwrap();
        runtime
            .commit_creature_profile_transformation(current)
            .unwrap();
        let after = runtime.companion_snapshot(actor).unwrap();
        assert!(
            runtime
                .commit_creature_profile_transformation(stale)
                .is_err()
        );
        assert_eq!(runtime.companion_snapshot(actor).unwrap(), after);
        let removed = runtime
            .prepare_creature_profile_transformation(&after, "creature:guardian")
            .unwrap();
        runtime.remove_test_actor(actor).unwrap();
        assert!(
            runtime
                .commit_creature_profile_transformation(removed)
                .is_err()
        );
    }
    #[test]
    fn qualified_variant_exact_keys_resolve_and_ambiguous_names_never_choose_export_order() {
        let mut second = policy("RAT");
        second.definition_key = "creature:rat-alternate".into();
        second.maximum_health = 25;
        let first = policy("rat");
        let unique = policy("wolf");
        for records in [
            vec![first.clone(), second.clone(), unique.clone()],
            vec![second.clone(), unique.clone(), first.clone()],
        ] {
            let (runtime, _, _) = owner_with_policies(records);
            assert_eq!(
                runtime
                    .companion_policy("creature:rat")
                    .unwrap()
                    .maximum_health,
                812
            );
            assert_eq!(
                runtime
                    .companion_policy("creature:rat-alternate")
                    .unwrap()
                    .maximum_health,
                25
            );
            assert!(matches!(
                runtime.companion_policy("rat"),
                Err(CarrierError::InvalidCreatureTarget)
            ));
            assert!(matches!(
                runtime.companion_policy("RaT"),
                Err(CarrierError::InvalidCreatureTarget)
            ));
            assert_eq!(
                runtime.companion_policy("WOLF").unwrap().definition_key,
                "creature:wolf"
            );
            assert!(matches!(
                runtime.companion_policy("unknown"),
                Err(CarrierError::InvalidCreatureTarget)
            ));
        }
        let mut duplicate = first.clone();
        duplicate.display_name = "different-name".into();
        assert!(matches!(
            CompiledCreaturePolicies::from_active_artifact([1; 32], vec![first, duplicate]),
            Err(CarrierError::InvalidCreatureTarget)
        ));
    }
    #[test]
    fn qualified_exact_key_wins_over_another_policy_display_name() {
        let first = policy("rat");
        let mut second = policy("other");
        second.display_name = "creature:rat".into();
        let (runtime, _, _) = owner_with_policies(vec![second, first]);
        assert_eq!(
            runtime
                .companion_policy("creature:rat")
                .unwrap()
                .definition_key,
            "creature:rat"
        );
    }

    #[test]
    fn qualified_object_appearance_survives_reserved_spawn_and_actual_snapshot() {
        let mut object = policy("object-creature");
        object.maximum_health = 20;
        object.outfit_look_type = 0;
        object.object_look_type = Some(2122);
        assert_eq!(object.outfit_appearance(), None);
        let (mut runtime, owner, session) = owner_with_policies(vec![object]);
        let position = MovementLocalPosition {
            x: 1,
            y: 0,
            floor: 0,
        };
        for override_look in [0, 2122, 991] {
            assert!(
                runtime
                    .prepare_companion_spawn(
                        owner,
                        session,
                        "object-creature",
                        position,
                        Some(override_look),
                        None,
                        0,
                        0,
                        false
                    )
                    .is_err()
            );
            assert!(
                runtime
                    .create_companion(
                        owner,
                        session,
                        "object-creature",
                        position,
                        Some(override_look),
                        None,
                        0
                    )
                    .is_err()
            );
            assert!(runtime.companion_creation_available(position).unwrap());
        }
        let mut prepared = runtime
            .prepare_companion_spawn(
                owner,
                session,
                "object-creature",
                position,
                None,
                None,
                0,
                0,
                false,
            )
            .unwrap();
        let expected = prepared.clone();
        runtime.reserve_companion_spawn(&mut prepared).unwrap();
        let actor = runtime.install_companion_spawn(expected).unwrap();
        let snapshot = runtime.companion_snapshot(actor).unwrap();
        assert_eq!(snapshot.state.outfit_look_type, 0);
        assert_eq!(snapshot.state.object_look_type, Some(2122));
        assert_eq!(snapshot.state.policy.object_look_type, Some(2122));
        assert_eq!(snapshot.state.policy.outfit_appearance(), None);
        assert!(!snapshot.state.conditions.invisible_at(0));
        runtime.validate_companion_snapshot(&snapshot).unwrap();
        for change in 0..3 {
            let mut altered = snapshot.clone();
            match change {
                0 => altered.state.object_look_type = None,
                1 => altered.state.object_look_type = Some(2123),
                _ => altered.state.outfit_look_type = 2122,
            }
            assert!(runtime.validate_companion_snapshot(&altered).is_err());
            assert!(
                runtime
                    .compare_companion_state(&snapshot, altered.state)
                    .is_err()
            );
            assert_eq!(runtime.companion_snapshot(actor).unwrap(), snapshot);
        }
        let direct = runtime
            .create_companion(
                owner,
                session,
                "object-creature",
                MovementLocalPosition {
                    x: 2,
                    y: 0,
                    floor: 0,
                },
                None,
                None,
                0,
            )
            .unwrap();
        let direct = runtime.companion_snapshot(direct).unwrap();
        assert_eq!(
            (direct.state.outfit_look_type, direct.state.object_look_type),
            (0, Some(2122))
        );
        let wild = runtime
            .admit_pinned_test_creature(MovementLocalPosition {
                x: 3,
                y: 0,
                floor: 0,
            })
            .unwrap();
        runtime
            .install_creature_policy(wild, "object-creature")
            .unwrap();
        let wild = runtime.companion_snapshot(wild).unwrap();
        assert_eq!(
            (wild.state.outfit_look_type, wild.state.object_look_type),
            (0, Some(2122))
        );
        assert!(wild.state.master.is_none());
    }
    #[test]
    fn qualified_object_policy_refuses_contradictory_outfit_flags_or_familiar_override() {
        let mut object = policy("object-creature");
        object.outfit_look_type = 0;
        object.object_look_type = Some(2122);
        CompiledCreaturePolicies::from_active_artifact([1; 32], vec![object.clone()]).unwrap();
        for change in 0..4 {
            let mut altered = object.clone();
            match change {
                0 => altered.object_look_type = Some(0),
                1 => altered.outfit_look_type = 2122,
                2 => altered.flags.illusionable = true,
                _ => altered.is_familiar = true,
            }
            if change == 2 {
                assert!(altered.flags.illusionable);
                assert_eq!(altered.outfit_appearance(), None);
                CompiledCreaturePolicies::from_active_artifact([1; 32], vec![altered]).unwrap();
            } else {
                assert!(
                    CompiledCreaturePolicies::from_active_artifact([1; 32], vec![altered]).is_err()
                );
            }
        }
    }
    #[test]
    fn reserved_spawn_uses_real_free_head_and_rechecks_occupied_and_recycled_slots() {
        let (mut runtime, owner, session) = owner();
        let cell = MovementLocalPosition {
            x: 1,
            y: 0,
            floor: 0,
        };
        let prepared = runtime
            .prepare_companion_spawn(
                owner,
                session,
                "rat",
                cell,
                Some(992),
                Some(1900),
                7,
                50,
                true,
            )
            .unwrap();
        let predicted = prepared.actor();
        assert!(!runtime.contains_live_creature(predicted));
        let duplicate = prepared.clone();
        let creature = runtime.install_companion_spawn(prepared).unwrap();
        assert_eq!(creature, predicted);
        let actual = runtime.companion_snapshot(creature).unwrap();
        assert_eq!(actual.health, 812);
        assert_eq!(actual.maximum_health, 812);
        assert_eq!(actual.state.current_speed(0).unwrap(), 270);
        assert_eq!(actual.state.outfit_look_type, 992);
        assert_eq!(actual.state.expires_at_unix, Some(1900));
        assert_eq!(actual.state.lifecycle_epoch, 7);
        assert!(actual.state.party_protection);
        assert!(runtime.install_companion_spawn(duplicate.clone()).is_err());
        assert_eq!(runtime.companion_snapshot(creature).unwrap(), actual);
        runtime.despawn_companion(owner, session, &actual).unwrap();
        assert!(runtime.install_companion_spawn(duplicate).is_err());
        assert!(runtime.owned_companions(owner, session).unwrap().is_empty());
        let fresh = runtime
            .prepare_companion_spawn(owner, session, "rat", cell, None, None, 0, 0, false)
            .unwrap();
        assert_eq!(fresh.actor().actor_local_id(), predicted.actor_local_id());
        assert!(fresh.actor().actor_local_generation() > predicted.actor_local_generation());
    }
    #[test]
    fn unresolved_actual_reservation_survives_other_admission_and_consumes_its_real_cell() {
        let (mut runtime, owner, session) = owner();
        let cell = MovementLocalPosition {
            x: 1,
            y: 0,
            floor: 0,
        };
        let mut prepared = runtime
            .prepare_companion_spawn(owner, session, "rat", cell, None, None, 0, 0, false)
            .unwrap();
        let retained = prepared.clone();
        let predicted = prepared.actor();
        runtime.reserve_companion_spawn(&mut prepared).unwrap();
        for field in 0..3 {
            let mut changed = retained.clone();
            match field {
                0 => changed.position.x += 1,
                1 => changed.state.party_protection = !changed.state.party_protection,
                _ => changed.state.lifecycle_epoch += 1,
            }
            assert!(runtime.install_companion_spawn(changed).is_err());
            runtime.validate_companion_spawn(&retained).unwrap();
        }
        assert!(!runtime.contains_live_creature(predicted));
        assert!(runtime.owned_companions(owner, session).unwrap().is_empty());
        assert!(!runtime.companion_creation_available(cell).unwrap());
        assert!(
            runtime
                .create_companion(owner, session, "rat", cell, None, None, 0)
                .is_err()
        );
        let other = runtime
            .create_companion(
                owner,
                session,
                "rat",
                MovementLocalPosition {
                    x: 2,
                    y: 0,
                    floor: 0,
                },
                None,
                None,
                0,
            )
            .unwrap();
        assert_ne!(other.actor_local_id(), predicted.actor_local_id());
        runtime.validate_companion_spawn(&retained).unwrap();
        runtime.reserve_companion_spawn(&mut prepared).unwrap();
        assert_eq!(
            runtime.install_companion_spawn(retained.clone()).unwrap(),
            predicted
        );
        assert!(runtime.install_companion_spawn(retained).is_err());
        assert_eq!(
            runtime
                .companion_snapshot(predicted)
                .unwrap()
                .maximum_health,
            812
        );
        assert_eq!(runtime.owned_companions(owner, session).unwrap().len(), 2);
    }
    #[test]
    fn definite_reservation_rollback_burns_incarnation_and_stale_proof_cannot_touch_reuse() {
        let (mut runtime, owner, session) = owner();
        let cell = MovementLocalPosition {
            x: 1,
            y: 0,
            floor: 0,
        };
        let mut prepared = runtime
            .prepare_companion_spawn(owner, session, "rat", cell, None, None, 0, 0, false)
            .unwrap();
        let predicted = prepared.actor();
        assert!(runtime.rollback_companion_spawn(&prepared).is_err());
        runtime.reserve_companion_spawn(&mut prepared).unwrap();
        runtime.rollback_companion_spawn(&prepared).unwrap();
        assert!(runtime.rollback_companion_spawn(&prepared).is_err());
        let actual = runtime
            .create_companion(owner, session, "rat", cell, None, None, 0)
            .unwrap();
        assert_eq!(actual.actor_local_id(), predicted.actor_local_id());
        assert!(actual.actor_local_generation() > predicted.actor_local_generation());
        let snapshot = runtime.companion_snapshot(actual).unwrap();
        assert!(runtime.rollback_companion_spawn(&prepared).is_err());
        assert!(runtime.install_companion_spawn(prepared).is_err());
        runtime.validate_companion_snapshot(&snapshot).unwrap();
    }
    /// VIS-3 (Codex 4178855592): the companion reserve, install and rollback paths keep the
    /// occupied-slot index equal to the slots off the free list, so installed companions are in
    /// the census.
    #[test]
    fn companion_reserve_install_and_rollback_keep_the_occupied_index_exact() {
        let by_scan = |runtime: &ChannelRuntimeV1| -> Vec<u32> {
            (0_u32..)
                .zip(runtime.carrier.slots.iter())
                .filter(|(_, slot)| {
                    matches!(
                        slot,
                        Slot::Occupied { .. }
                            | Slot::CreatureOccupied { .. }
                            | Slot::CreatureReserved { .. }
                    )
                })
                .map(|(index, _)| index)
                .collect()
        };
        let indexed = |runtime: &ChannelRuntimeV1| -> Vec<u32> {
            let mut occupied = runtime.carrier.occupied.clone();
            occupied.sort_unstable();
            occupied
        };
        let (mut runtime, owner, session) = owner();
        let cell = MovementLocalPosition {
            x: 1,
            y: 0,
            floor: 0,
        };
        let before = indexed(&runtime);
        assert_eq!(before, by_scan(&runtime));
        let mut rolled_back = runtime
            .prepare_companion_spawn(owner, session, "rat", cell, None, None, 0, 0, false)
            .unwrap();
        runtime.reserve_companion_spawn(&mut rolled_back).unwrap();
        runtime.reserve_companion_spawn(&mut rolled_back).unwrap();
        assert_eq!(indexed(&runtime), by_scan(&runtime));
        assert_eq!(indexed(&runtime).len(), before.len() + 1);
        runtime.rollback_companion_spawn(&rolled_back).unwrap();
        assert_eq!(indexed(&runtime), before);
        let mut reserved = runtime
            .prepare_companion_spawn(owner, session, "rat", cell, None, None, 0, 0, false)
            .unwrap();
        runtime.reserve_companion_spawn(&mut reserved).unwrap();
        let installed = runtime.install_companion_spawn(reserved).unwrap();
        assert_eq!(indexed(&runtime), by_scan(&runtime));
        let next_cell = MovementLocalPosition { x: 2, ..cell };
        let direct = runtime
            .create_companion(owner, session, "rat", next_cell, None, None, 0)
            .unwrap();
        assert_eq!(indexed(&runtime), by_scan(&runtime));
        assert_eq!(indexed(&runtime).len(), before.len() + 2);
        let visible = runtime.visible_entities();
        for companion in [installed, direct] {
            assert!(
                visible.creatures.iter().any(|seen| seen.actor == companion),
                "an installed companion is in the census: {companion:?}"
            );
        }
        let snapshot = runtime.companion_snapshot(installed).unwrap();
        runtime
            .despawn_companion(owner, session, &snapshot)
            .unwrap();
        assert_eq!(indexed(&runtime), by_scan(&runtime));
        assert_eq!(indexed(&runtime).len(), before.len() + 1);
    }
    #[test]
    fn pending_actual_damage_batch_blocks_companion_mutations_and_all_owner_removal_atomically() {
        use super::super::runtime_actor_spell_types::{
            OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
        };
        let (mut runtime, owner, session) = owner();
        let first = runtime
            .create_companion(
                owner,
                session,
                "rat",
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
                None,
                None,
                0,
            )
            .unwrap();
        let second = runtime
            .create_companion(
                owner,
                session,
                "rat",
                MovementLocalPosition {
                    x: 2,
                    y: 0,
                    floor: 0,
                },
                None,
                None,
                0,
            )
            .unwrap();
        let before = runtime.companion_snapshot(second).unwrap();
        let batch = OwnerCombatBatch {
            caster: owner,
            attacker: CharacterId::decode(&id(5)).unwrap(),
            current_lease_generation: 1,
            command: CommandRef::new(session, crate::foundation::CommandId::new(7).unwrap()),
            occurrence: SpellOccurrenceBinding {
                id: "test:actual-companion-pending".into(),
                revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"]
                    .map(str::to_owned),
            },
            binding: b"test:qualified-companion-damage".to_vec(),
            anchor: None,
            now_ms: 100,
            effects: vec![OwnerCombatEffect {
                target: second,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: "creature:rat".into(),
                    magnitude: 3,
                },
            }],
            deferred: None,
        };
        let mut staged = runtime.stage_spell_batch(&batch).unwrap();
        runtime.reserve_spell_batch(&mut staged).unwrap();
        assert!(runtime.actor_spell_reserved(second));
        assert_eq!(
            runtime.despawn_companion(owner, session, &before),
            Err(CarrierError::PlanConflict)
        );
        assert_eq!(
            runtime.follow_companion(
                owner,
                session,
                &before,
                MovementLocalPosition {
                    x: 3,
                    y: 0,
                    floor: 0
                }
            ),
            Err(CarrierError::PlanConflict)
        );
        assert_eq!(
            runtime.compare_companion_state(&before, before.state.clone()),
            Err(CarrierError::PlanConflict)
        );
        assert_eq!(
            runtime.mark_companion_party_protection(&before),
            Err(CarrierError::PlanConflict)
        );
        assert_eq!(
            runtime.remove_owned_companions(owner, session),
            Err(CarrierError::PlanConflict)
        );
        assert!(runtime.contains_live_creature(first));
        runtime.validate_companion_snapshot(&before).unwrap();
        assert!(runtime.commit_spell_batch(staged).unwrap().applied);
        assert_eq!(runtime.companion_snapshot(second).unwrap().health, 809);
        assert_eq!(runtime.remove_owned_companions(owner, session).unwrap(), 2);
    }
    #[test]
    fn intervening_physical_admission_invalidates_predicted_spawn_without_partial_install() {
        let (mut runtime, owner, session) = owner();
        let prepared = runtime
            .prepare_companion_spawn(
                owner,
                session,
                "rat",
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
                None,
                None,
                0,
                0,
                false,
            )
            .unwrap();
        let actual = runtime
            .create_companion(
                owner,
                session,
                "rat",
                MovementLocalPosition {
                    x: 2,
                    y: 0,
                    floor: 0,
                },
                None,
                None,
                0,
            )
            .unwrap();
        assert_eq!(actual, prepared.actor());
        let before = runtime.companion_snapshot(actual).unwrap();
        assert!(runtime.install_companion_spawn(prepared).is_err());
        assert_eq!(runtime.companion_snapshot(actual).unwrap(), before);
        assert_eq!(runtime.owned_companions(owner, session).unwrap().len(), 1);
    }
    #[test]
    fn installed_active_policy_creates_real_slot_and_recycled_generation_is_stale() {
        let (mut runtime, owner, session) = owner();
        let pos = MovementLocalPosition {
            x: 1,
            y: 0,
            floor: 0,
        };
        let creature = runtime
            .create_companion(owner, session, "rat", pos, None, None, 0)
            .unwrap();
        let before = runtime.companion_snapshot(creature).unwrap();
        assert_eq!(before.health, 812);
        assert_eq!(before.maximum_health, 812);
        assert_eq!(before.position, pos);
        assert_eq!(before.state.policy.mana_cost, Some(250));
        assert_eq!(runtime.owned_companions(owner, session).unwrap().len(), 1);
        assert!(runtime.contains_live_creature(creature));
        runtime.despawn_companion(owner, session, &before).unwrap();
        assert!(!runtime.contains_live_creature(creature));
        let replacement = runtime
            .create_companion(owner, session, "rat", pos, None, None, 0)
            .unwrap();
        assert_ne!(creature, replacement);
        assert!(matches!(
            runtime.despawn_companion(owner, session, &before),
            Err(CarrierError::StaleActorGeneration)
        ));
        assert!(runtime.contains_live_creature(replacement));
        assert_eq!(runtime.owned_companions(owner, session).unwrap().len(), 1);
    }
    #[test]
    fn artifact_policy_unknown_prototypes_and_occupancy_refuse_before_allocation() {
        let (mut runtime, owner, session) = owner();
        let wrong =
            CompiledCreaturePolicies::from_active_artifact([2; 32], vec![policy("dog")]).unwrap();
        assert_eq!(
            runtime.install_companion_policies(wrong),
            Err(CarrierError::ContentPinWorldMismatch)
        );
        let pos = MovementLocalPosition {
            x: 1,
            y: 0,
            floor: 0,
        };
        assert!(
            runtime
                .create_companion(owner, session, "unknown", pos, None, None, 0)
                .is_err()
        );
        assert!(
            runtime
                .create_companion(
                    owner,
                    session,
                    "rat",
                    MovementLocalPosition {
                        x: 0,
                        y: 0,
                        floor: 0
                    },
                    None,
                    None,
                    0
                )
                .is_err()
        );
        assert_eq!(runtime.owned_companions(owner, session).unwrap().len(), 0);
        assert!(runtime.companion_creation_available(pos).unwrap());
        assert!(
            CompiledCreaturePolicies::from_active_artifact(
                [1; 32],
                vec![policy("rat"), {
                    let mut duplicate = policy("RAT");
                    duplicate.definition_key = policy("rat").definition_key;
                    duplicate
                }]
            )
            .is_err()
        );
        let mut undefined = policy("unknown");
        undefined.mana_cost = None;
        assert!(CompiledCreaturePolicies::from_active_artifact([1; 32], vec![undefined]).is_err());
    }
    #[test]
    fn owner_removal_drops_physical_summons_and_snapshot_update_is_compared() {
        let (mut runtime, owner, session) = owner();
        let creature = runtime
            .create_companion(
                owner,
                session,
                "rat",
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
                None,
                None,
                0,
            )
            .unwrap();
        let before = runtime.companion_snapshot(creature).unwrap();
        runtime.validate_companion_snapshot(&before).unwrap();
        let mut changed = before.state.clone();
        changed.creation_speed_delta = 50;
        runtime.compare_companion_state(&before, changed).unwrap();
        let current = runtime.companion_snapshot(creature).unwrap();
        assert_eq!(
            runtime.validate_companion_snapshot(&before),
            Err(CarrierError::PlanConflict)
        );
        runtime.validate_companion_snapshot(&current).unwrap();
        for field in 0..4 {
            let mut substituted = current.clone();
            match field {
                0 => substituted.health -= 1,
                1 => substituted.maximum_health += 1,
                2 => substituted.position.x += 1,
                _ => substituted.position_revision += 1,
            }
            assert_eq!(
                runtime.validate_companion_snapshot(&substituted),
                Err(CarrierError::PlanConflict)
            );
        }
        assert_eq!(current.state.current_speed(0).unwrap(), 270);
        assert_eq!(
            runtime.compare_companion_state(&before, before.state.clone()),
            Err(CarrierError::PlanConflict)
        );
        assert_eq!(runtime.remove_owned_companions(owner, session).unwrap(), 1);
        assert!(!runtime.contains_live_creature(creature));
        assert_eq!(runtime.owned_companions(owner, session).unwrap().len(), 0);
    }
    #[test]
    fn applied_creature_damage_dispels_actual_invisibility_but_historical_replay_keeps_new_condition()
     {
        use super::super::runtime_actor_spell_types::{
            OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
        };
        use crate::foundation::condition::{
            ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionValues,
        };
        let (mut runtime, owner, session) = owner();
        let actor = runtime
            .create_companion(
                owner,
                session,
                "rat",
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
                None,
                None,
                0,
            )
            .unwrap();
        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([8; 32]);
        let facts = ApplicationFacts {
            now: 1000,
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: false,
            decision_root: &root,
            occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes([9; 16]),
        };
        let invisible = ConditionDefinition::new(
            "source.monster.invisible",
            1,
            ConditionValues::Invisible {
                duration_ms: 200_000,
            },
        )
        .unwrap();
        let install = |runtime: &mut ChannelRuntimeV1| {
            let before = runtime.companion_snapshot(actor).unwrap();
            let mut next = before.state.clone();
            next.conditions
                .apply(
                    &invisible,
                    Some(owner),
                    ConditionSourceKind::SelfUse,
                    &[],
                    &facts,
                )
                .unwrap();
            runtime.compare_companion_state(&before, next).unwrap();
        };
        install(&mut runtime);
        let batch = OwnerCombatBatch {
            caster: owner,
            attacker: CharacterId::decode(&id(5)).unwrap(),
            current_lease_generation: 1,
            command: CommandRef::new(session, crate::foundation::CommandId::new(1).unwrap()),
            occurrence: SpellOccurrenceBinding {
                id: "source-monster-drain-health".into(),
                revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"]
                    .map(str::to_owned),
            },
            binding: b"source.monster.drainHealth.invisible".to_vec(),
            anchor: None,
            now_ms: 100,
            effects: vec![OwnerCombatEffect {
                target: actor,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: "creature:rat".into(),
                    magnitude: 3,
                },
            }],
            deferred: None,
        };
        let prepared = runtime.stage_spell_batch(&batch).unwrap();
        assert!(
            runtime
                .companion_snapshot(actor)
                .unwrap()
                .state
                .conditions
                .invisible_at(1000)
        );
        assert!(runtime.commit_spell_batch(prepared).unwrap().applied);
        assert!(
            !runtime
                .companion_snapshot(actor)
                .unwrap()
                .state
                .conditions
                .invisible_at(1000)
        );
        install(&mut runtime);
        let original = runtime.companion_snapshot(actor).unwrap();
        let replay = runtime.stage_spell_batch(&batch).unwrap();
        assert!(!runtime.commit_spell_batch(replay).unwrap().applied);
        assert_eq!(runtime.companion_snapshot(actor).unwrap(), original);
    }
}
