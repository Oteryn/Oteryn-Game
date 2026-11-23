//! One source-qualified boss in a freshly allocated activity runtime.
//! BossLever creates one boss per attempt; it is never an ordinary respawn point.
use super::*;
use oteryn_gameplay_content::content::{
    ActiveGeneration, bosses::activity_template::QualifiedActivityRoomTemplate,
};

impl ChannelRuntimeV1 {
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
