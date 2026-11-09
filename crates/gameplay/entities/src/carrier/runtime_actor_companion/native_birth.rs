//! Internal native full-birth payload; source-specific callers retain all rule authority.
use super::*;

#[derive(Debug, Clone)]
pub(in crate::carrier) struct PreparedNativeCreatureBirth {
    actor: ExactActorRef,
    owner: ExactActorRef,
    session: GameSessionId,
    position: MovementLocalPosition,
    policy: Arc<CompiledCreaturePolicy>,
    planned: Option<Box<Slot>>,
}
impl PreparedNativeCreatureBirth {
    pub(in crate::carrier) fn actor(&self) -> ExactActorRef {
        self.actor
    }
    #[allow(clippy::too_many_arguments)]
    pub(in crate::carrier) fn on_data(
        carrier: &ChannelActorCarrier,
        continuity: &NamespaceContinuityGuard,
        context: PreProductionPositionContext,
        owner: ExactActorRef,
        session: GameSessionId,
        policy: Arc<CompiledCreaturePolicy>,
        health_callback: Option<Arc<oteryn_gameplay_content::content::native_soul_war_callbacks::QualifiedSoulWarHealthPolicy>>,
        mirror_births: Option<Arc<super::super::soul_war_health::MirrorBirthCatalogue>>,
        position: MovementLocalPosition,
    ) -> Result<Self, CarrierError> {
        carrier.player_slot_index(continuity, owner.0, session)?;
        carrier.validate_position_context(context)?;
        let head = carrier.free_head.ok_or(CarrierError::CapacityExceeded)?;
        let index = usize::try_from(head).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        let Slot::VacantReusable { generation, .. } = carrier.slots[index] else {
            return Err(CarrierError::PlanConflict);
        };
        let generation = generation
            .checked_add(1)
            .ok_or(CarrierError::ActorGenerationExhausted)?;
        let local = head
            .checked_add(1)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        let actor = ExactActorRef(ActorRef::from_scoped_owner_parts(
            carrier.runtime_scope,
            carrier.scope_generation,
            ActorLocalId(local),
            ActorLocalGeneration(generation),
        ));
        let state = CompanionState {
            policy: policy.clone(),
            master: None,
            creation_speed_delta: 0,
            conditions: ConditionStore::default(),
            outfit_look_type: policy.outfit_look_type,
            object_look_type: policy.object_look_type,
            expires_at_unix: None,
            lifecycle_epoch: 0,
            party_protection: false,
            created_at_us: None,
            familiar_defense: None,
        };
        let mut combat = runtime_actor_spell::ActorCombatState::creature(policy.maximum_health);
        combat.soul_war_health = health_callback;
        combat.mirror_births = mirror_births;
        if oteryn_gameplay_content::content::native_soul_war_callbacks::requires_health_policy(
            &policy.definition_key,
        ) && combat.soul_war_health.is_none()
        {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        let planned = Slot::CreatureOccupied {
            generation,
            actor: ActorState(0),
            position: Some(VersionedPosition {
                actor_local_id: actor.0.actor_local_id,
                actor_local_generation: actor.0.actor_local_generation,
                context,
                position: LocalPosition {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                },
                revision: 1,
                last_translation_revision: 0,
                facing: None,
            }),
            target_identity: Arc::from(copy_bounded_binding(policy.definition_key.as_bytes())?),
            health: policy.maximum_health,
            committed: Box::default(),
            damage_contributors: Box::default(),
            companion: Some(Box::new(state)),
            spell_combat: Box::new(combat),
        };
        Ok(PreparedNativeCreatureBirth {
            actor,
            owner,
            session,
            position,
            policy,
            planned: Some(Box::new(planned)),
        })
    }
    pub(in crate::carrier) fn reserve_on_data(
        &mut self,
        carrier: &mut ChannelActorCarrier,
        continuity: &NamespaceContinuityGuard,
    ) -> Result<(), CarrierError> {
        carrier.reserve_planned_creature_slot(
            continuity,
            self.actor,
            self.owner,
            self.session,
            &mut self.planned,
        )
    }
    pub(in crate::carrier) fn install_on_data(
        mut self,
        carrier: &mut ChannelActorCarrier,
        continuity: &NamespaceContinuityGuard,
    ) -> Result<ExactActorRef, CarrierError> {
        let order = carrier.next_tile_entry_order()?;
        self.reserve_on_data(carrier, continuity)?;
        let index = carrier.validate_ref(continuity, self.actor.0)?;
        self.publish_reserved(carrier, index, order);
        Ok(self.actor)
    }
    /// Only the held canonical owner invokes this after exact reservation,
    /// payload and tile-entry-capacity comparison. No allocation/fallible work.
    pub(in crate::carrier) fn publish_reserved(
        &self,
        carrier: &mut ChannelActorCarrier,
        index: usize,
        order: u64,
    ) {
        let reserved = std::mem::replace(
            &mut carrier.slots[index],
            Slot::Exhausted {
                generation: self.actor.0.actor_local_generation.0,
            },
        );
        let Slot::CreatureReserved { planned, .. } = reserved else {
            unreachable!("validated native birth reservation");
        };
        carrier.slots[index] = *planned;
        carrier.install_tile_entry_order(index, order);
    }
}

impl ChannelRuntimeV1 {
    /// Private physical primitive, not a source grant. The caller must qualify the
    /// closed source callback and forced/extended placement before calling this.
    pub(in crate::carrier) fn prepare_native_creature_birth(
        &self,
        owner: ExactActorRef,
        session: GameSessionId,
        key: &str,
        position: MovementLocalPosition,
    ) -> Result<PreparedNativeCreatureBirth, CarrierError> {
        self.prepare_native_creature_birth_on_data(&self.carrier, owner, session, key, position)
    }

    /// The ordinary spell successor is a data clone; continuity and current
    /// source policies are still borrowed independently from the actual owner.
    pub(in crate::carrier) fn prepare_native_creature_birth_on_data(
        &self,
        carrier: &ChannelActorCarrier,
        owner: ExactActorRef,
        session: GameSessionId,
        key: &str,
        position: MovementLocalPosition,
    ) -> Result<PreparedNativeCreatureBirth, CarrierError> {
        carrier.validate_current_continuity(&self.continuity)?;
        self.carrier
            .player_slot_index(&self.continuity, owner.0, session)?;
        let policy = self.companion_policy(key)?;
        let table = self
            .companion_policies
            .as_deref()
            .ok_or(CarrierError::InvalidCreatureTarget)?;
        let health_callback = table
            .soul_war_health_policy(&policy.definition_key, &policy.definition_revision)
            .cloned()
            .map(Arc::new);
        let mirror_births = health_callback.as_deref().filter(|p| p.kind() == oteryn_gameplay_content::content::native_soul_war_callbacks::SoulWarHealthCallbackKind::MirrorImage)
            .map(|_| super::super::soul_war_health::MirrorBirthCatalogue::from_installed(table, self.content.server_artifact_digest()).map(Arc::new)).transpose()?;
        PreparedNativeCreatureBirth::on_data(
            carrier,
            &self.continuity,
            self.pinned_position_context(),
            owner,
            session,
            policy,
            health_callback,
            mirror_births,
            position,
        )
    }

    pub(in crate::carrier) fn validate_native_creature_birth(
        &self,
        birth: &PreparedNativeCreatureBirth,
    ) -> Result<(), CarrierError> {
        self.carrier
            .player_slot_index(&self.continuity, birth.owner.0, birth.session)?;
        if self
            .companion_policy(&birth.policy.definition_key)?
            .as_ref()
            != birth.policy.as_ref()
        {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let index = self.carrier.validate_ref(&self.continuity, birth.actor.0)?;
        let planned = match &self.carrier.slots[index] {
            Slot::VacantReusable { generation, .. }
                if generation.checked_add(1) == Some(birth.actor.0.actor_local_generation.0)
                    && self.carrier.free_head.and_then(|v| usize::try_from(v).ok())
                        == Some(index) =>
            {
                birth.planned.as_deref()
            }
            Slot::CreatureReserved {
                generation,
                owner,
                session,
                planned,
            } if *generation == birth.actor.0.actor_local_generation.0
                && *owner == birth.owner
                && *session == birth.session =>
            {
                Some(planned.as_ref())
            }
            _ => None,
        }
        .ok_or(CarrierError::PlanConflict)?;
        let Slot::CreatureOccupied {
            generation,
            position: Some(at),
            health,
            committed,
            damage_contributors,
            companion: Some(companion),
            spell_combat,
            ..
        } = planned
        else {
            return Err(CarrierError::PlanConflict);
        };
        if *generation != birth.actor.0.actor_local_generation.0
            || at.actor_local_id != birth.actor.0.actor_local_id
            || at.actor_local_generation != birth.actor.0.actor_local_generation
            || at.context != self.pinned_position_context()
            || at.position.x != birth.position.x
            || at.position.y != birth.position.y
            || at.position.floor != birth.position.floor
            || at.revision != 1
            || at.last_translation_revision != 0
            || at.facing.is_some()
            || *health != birth.policy.maximum_health
            || spell_combat.maximum_health != birth.policy.maximum_health
            || companion.master.is_some()
            || companion.policy != birth.policy
            || !committed.entries.is_empty()
            || !damage_contributors.entries.is_empty()
        {
            return Err(CarrierError::PlanConflict);
        }
        Ok(())
    }

    pub(in crate::carrier) fn reserve_native_creature_birth(
        &mut self,
        birth: &mut PreparedNativeCreatureBirth,
    ) -> Result<(), CarrierError> {
        self.validate_native_creature_birth(birth)?;
        self.reserve_planned_creature_slot(
            birth.actor,
            birth.owner,
            birth.session,
            &mut birth.planned,
        )
    }
    pub(in crate::carrier) fn rollback_native_creature_birth(
        &mut self,
        birth: &PreparedNativeCreatureBirth,
    ) -> Result<(), CarrierError> {
        self.validate_native_creature_birth(birth)?;
        self.release_planned_creature_slot(birth.actor, birth.owner, birth.session)
    }
    pub(in crate::carrier) fn install_native_creature_birth(
        &mut self,
        birth: PreparedNativeCreatureBirth,
    ) -> Result<ExactActorRef, CarrierError> {
        self.validate_native_creature_birth(&birth)?;
        birth.install_on_data(&mut self.carrier, &self.continuity)
    }
}
