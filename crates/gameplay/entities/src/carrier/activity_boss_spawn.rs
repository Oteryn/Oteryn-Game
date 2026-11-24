//! One source-qualified boss in a freshly allocated activity runtime.
//! BossLever creates one boss per attempt; it is never an ordinary respawn point.
use super::*;
use oteryn_gameplay_content::content::{
    ActiveGeneration, bosses::activity_template::QualifiedActivityRoomTemplate,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeriodicDamageAttribution {
    pub character: CharacterId,
    pub lease_generation: u64,
    pub session: GameSessionId,
    pub actor: ExactActorRef,
}

/// Exact scoped kill settlement input for a periodic death with a player contributor.
pub struct HeraldPeriodicKill {
    pub target: ExactActorRef,
    pub death: CreatureDeathOccurrenceKey,
    pub principal: PeriodicDamageAttribution,
    pub death_at_ms: u64,
}

impl ChannelRuntimeV1 {
    fn activity_herald_slot(&self) -> Option<usize> {
        self.carrier.occupied.iter().find_map(|&index| {
            let index = index as usize;
            matches!(&self.carrier.slots[index], Slot::CreatureOccupied { spell_combat, .. }
                 if spell_combat.herald_fire_field_map.is_some())
            .then_some(index)
        })
    }

    /// Called while the physical owner still holds the exact World party revision.
    pub fn freeze_herald_periodic_parties(
        &mut self,
        views: Vec<super::runtime_actor_party::PartySnapshot>,
    ) {
        if let Some(index) = self.activity_herald_slot()
            && let Slot::CreatureOccupied {
                health: 0,
                committed,
                spell_combat,
                ..
            } = &mut self.carrier.slots[index]
            && committed
                .lethal()
                .is_some_and(|hit| hit.origin.is_none() && hit.reflection.is_none())
        {
            spell_combat
                .herald_periodic_party_at_death
                .get_or_insert(views);
        }
    }

    /// A periodic receipt without current World evidence admits no shared peers.
    pub fn herald_periodic_parties(
        &self,
        actor: ExactActorRef,
    ) -> Option<&[super::runtime_actor_party::PartySnapshot]> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0).ok()?;
        let Slot::CreatureOccupied {
            health: 0,
            committed,
            spell_combat,
            ..
        } = &self.carrier.slots[index]
        else {
            return None;
        };
        if spell_combat.herald_fire_field_map.is_none()
            || committed
                .lethal()
                .is_none_or(|hit| hit.origin.is_some() || hit.reflection.is_some())
        {
            return None;
        }
        Some(
            spell_combat
                .herald_periodic_party_at_death
                .as_deref()
                .unwrap_or_default(),
        )
    }

    pub fn activity_herald_is_alive(&self) -> bool {
        self.activity_herald_slot().is_some_and(|index| {
            matches!(&self.carrier.slots[index], Slot::CreatureOccupied { health, .. } if *health > 0)
        })
    }

    pub fn herald_periodic_kill(&self) -> Result<Option<HeraldPeriodicKill>, CarrierError> {
        let Some(index) = self.activity_herald_slot() else {
            return Ok(None);
        };
        let Slot::CreatureOccupied {
            generation,
            health: 0,
            committed,
            spell_combat,
            ..
        } = &self.carrier.slots[index]
        else {
            return Ok(None);
        };
        if spell_combat.herald_periodic_kill_queued {
            return Ok(None);
        }
        if committed
            .lethal()
            .is_none_or(|hit| hit.origin.is_some() || hit.reflection.is_some())
        {
            return Ok(None);
        }
        self.owner_fence()?;
        let target = ExactActorRef(ActorRef::from_scoped_owner_parts(
            self.binding.runtime_scope,
            self.binding.scope_generation,
            ActorLocalId(
                u32::try_from(index + 1).map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
            ),
            ActorLocalGeneration(*generation),
        ));
        let top = self
            .carrier
            .top_damage_contributor_inner(&self.continuity, target.0)?;
        let principal = top
            .present()
            .map(|winner| PeriodicDamageAttribution {
                character: winner.character,
                lease_generation: winner.lease_generation,
                session: winner.session,
                actor: winner.actor,
            })
            .or(committed.lethal_periodic_player);
        let Some(principal) = principal else {
            return Ok(None);
        };
        let receipt = self
            .carrier
            .committed_lethal_receipt_inner(&self.continuity, target.0)?;
        let death_at_ms = receipt
            .projection
            .death_at_ms
            .ok_or(CarrierError::CommittedLethalUnavailable)?;
        Ok(Some(HeraldPeriodicKill {
            target,
            death: receipt.projection.occurrence().death_key()?,
            principal,
            death_at_ms,
        }))
    }

    pub fn acknowledge_herald_periodic_kill(
        &mut self,
        target: ExactActorRef,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        if !self
            .herald_periodic_kill()?
            .is_some_and(|kill| kill.target == target)
        {
            return Err(CarrierError::CommittedLethalUnavailable);
        }
        let index = self.carrier.validate_ref(&self.continuity, target.0)?;
        let Slot::CreatureOccupied {
            health: 0,
            spell_combat,
            ..
        } = &mut self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        spell_combat.herald_periodic_kill_queued = true;
        Ok(())
    }

    /// Retain the lethal receipt and its original transition time across a retry.
    pub fn prepare_uncredited_herald_defeat(
        &mut self,
        owner_now_ms: u64,
        unix_now_ms: i64,
    ) -> Result<Option<(CreatureDeathOccurrenceKey, i64)>, CarrierError> {
        self.owner_fence()?;
        let Some(index) = self.activity_herald_slot() else {
            return Ok(None);
        };
        let Slot::CreatureOccupied {
            generation,
            health: 0,
            spell_combat,
            committed,
            ..
        } = &self.carrier.slots[index]
        else {
            return Ok(None);
        };
        // The periodic queue owns this death while a current principal exists.
        if self.herald_periodic_kill()?.is_some() {
            return Ok(None);
        }
        if spell_combat.herald_uncredited_defeat_settled
            || spell_combat.herald_periodic_kill_queued
            || committed
                .lethal()
                .is_none_or(|hit| hit.origin.is_some() || hit.reflection.is_some())
        {
            return Ok(None);
        }
        let actor = ActorRef::from_scoped_owner_parts(
            self.binding.runtime_scope,
            self.binding.scope_generation,
            ActorLocalId(
                u32::try_from(index + 1).map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
            ),
            ActorLocalGeneration(*generation),
        );
        let receipt = self
            .carrier
            .committed_lethal_receipt_inner(&self.continuity, actor)?;
        let death = receipt.projection.occurrence().death_key()?;
        let elapsed = receipt
            .projection
            .death_at_ms
            .map_or(0, |then| owner_now_ms.saturating_sub(then));
        let estimate = unix_now_ms.saturating_sub(i64::try_from(elapsed).unwrap_or(i64::MAX));
        let Slot::CreatureOccupied { spell_combat, .. } = &mut self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        let at = *spell_combat
            .herald_uncredited_death_at
            .get_or_insert(estimate);
        Ok(Some((death, at)))
    }

    pub fn acknowledge_uncredited_herald_defeat(
        &mut self,
        death: CreatureDeathOccurrenceKey,
        at: i64,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        let index = self
            .activity_herald_slot()
            .ok_or(CarrierError::NotCreature)?;
        let Slot::CreatureOccupied {
            generation,
            health: 0,
            spell_combat,
            ..
        } = &mut self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if death.runtime_scope() != self.binding.runtime_scope
            || death.scope_ownership_generation() != self.binding.scope_generation
            || usize::try_from(death.actor_local_id()).ok() != Some(index + 1)
            || death.actor_local_generation() != *generation
            || spell_combat.herald_uncredited_death_at != Some(at)
        {
            return Err(CarrierError::CorpseProjectionConflict);
        }
        spell_combat.herald_uncredited_defeat_settled = true;
        Ok(())
    }

    /// Bind only the freshly realized Herald to the current Instance's map.
    pub fn bind_herald_fire_field_map(
        &mut self,
        actor: ExactActorRef,
        template: &QualifiedActivityRoomTemplate,
        room: &oteryn_gameplay_content::content::QualifiedNativeEntryRoom,
    ) -> Result<(), CarrierError> {
        use oteryn_gameplay_content::content::native_cell_lookup::NativeMovementCollisionIndex;
        self.owner_fence()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let NativeMovementCollisionIndex::Bundle(map) = room.movement_cells().index() else {
            return Err(CarrierError::InvalidSpawnDefinition);
        };
        let RuntimeScopeRefV1::Instance { instance_id, .. } = self.binding.runtime_scope() else {
            return Err(CarrierError::InvalidSpawnDefinition);
        };
        let position = self.read_actor_position(actor)?.position();
        if !template.has_herald_callbacks()
            || actor.runtime_scope() != self.binding.runtime_scope()
            || room.map_revision_digest() != self.map_revision_digest()
            || room.frame_binding().digest() != self.content_pin().frame_binding_digest()
            || map
                .0
                .herald_fire_field(
                    instance_id,
                    oteryn_gameplay_content::content::LogicalCell {
                        x: position.x,
                        y: position.y,
                        z: i32::from(position.floor),
                    },
                )
                .is_none()
        {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        let Slot::CreatureOccupied {
            target_identity,
            spell_combat,
            ..
        } = &mut self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if target_identity.as_ref() != template.creature_key().as_bytes()
            || spell_combat.herald_fire_field_map.is_some()
        {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        spell_combat.herald_fire_field_map = Some(map.clone());
        Ok(())
    }

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
        let creature = template.creature_key();
        let policy = self.companion_policy(creature)?;
        if policy.reward_boss != Some(true) && !template.has_herald_callbacks() {
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

pub(super) fn herald_occupies_fire_field(
    scope: RuntimeScopeRefV1,
    slot: &Slot,
) -> Result<bool, CarrierError> {
    let Slot::CreatureOccupied {
        position,
        spell_combat,
        ..
    } = slot
    else {
        return Ok(false);
    };
    let Some(map) = &spell_combat.herald_fire_field_map else {
        return Ok(false);
    };
    let RuntimeScopeRefV1::Instance { instance_id, .. } = scope else {
        return Err(CarrierError::PositionContextMismatch);
    };
    let p = position
        .as_ref()
        .ok_or(CarrierError::PositionContextMismatch)?
        .position;
    map.0
        .herald_fire_field(
            instance_id,
            oteryn_gameplay_content::content::LogicalCell {
                x: p.x,
                y: p.y,
                z: i32::from(p.floor),
            },
        )
        .ok_or(CarrierError::PositionContextMismatch)
}
