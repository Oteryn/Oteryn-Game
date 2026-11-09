//! Historical committed native HP callbacks. The receipt grants no input,
//! inventory, death or actor-admission authority; the existing scope/Map/Item
//! owners independently re-prove the current owner before its fixed effect.
use super::*;
use oteryn_gameplay_foundation::foundation::RuntimeWorkStamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedCloakBloodEvent {
    pub(super) stamp: Option<RuntimeWorkStamp>,
    actor: ExactActorRef,
    position: MovementLocalPosition,
    creature: oteryn_gameplay_content::content::ProjectV2DefinitionRef,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
    source_body: [u8; 32],
    occurrence: [u8; 32],
}
impl CommittedCloakBloodEvent {
    pub fn actor(&self) -> ExactActorRef {
        self.actor
    }
    pub fn position(&self) -> MovementLocalPosition {
        self.position
    }
    pub fn runtime_scope(&self) -> RuntimeScopeRefV1 {
        self.actor.runtime_scope()
    }
    pub fn scope_generation(&self) -> u64 {
        self.actor.scope_generation().get()
    }
    pub fn creature(&self) -> &oteryn_gameplay_content::content::ProjectV2DefinitionRef {
        &self.creature
    }
    pub fn server_digest(&self) -> [u8; 32] {
        self.server
    }
    pub fn client_digest(&self) -> [u8; 32] {
        self.client
    }
    pub fn frame_digest(&self) -> [u8; 32] {
        self.frame
    }
    pub fn map_digest(&self) -> [u8; 32] {
        self.map
    }
    pub fn source_body_digest(&self) -> [u8; 32] {
        self.source_body
    }
    pub fn occurrence(&self) -> &[u8; 32] {
        &self.occurrence
    }
    /// Parent liveness is intentionally historical: the source creates blood
    /// before lethal removal, while its existing Item transaction may complete
    /// after that removal. Current scope, pins and owner still must match.
    pub fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        runtime
            .owner_fence()
            .is_ok_and(|owner| self.stamp.is_some_and(|stamp| owner.accepts_stamp(stamp)))
            && runtime.binding().runtime_scope() == self.runtime_scope()
            && runtime.binding().scope_generation().get() == self.scope_generation()
            && runtime.content_pin().server_artifact_digest() == self.server
            && runtime.content_pin().client_artifact_digest() == self.client
            && runtime.content_pin().frame_binding_digest() == self.frame
            && runtime.map_revision_digest() == self.map
    }
}
impl ChannelRuntimeV1 {
    /// Bounded pending work in this actual owner, never a detached source poll.
    pub fn pending_cloak_blood_events(
        &self,
        limit: usize,
    ) -> Result<Vec<CommittedCloakBloodEvent>, CarrierError> {
        self.owner_fence()?;
        Ok(self
            .carrier
            .cloak_blood_events
            .iter()
            .take(limit.min(MAX_PENDING_CLOAK_BLOOD))
            .cloned()
            .collect())
    }
}

// Crystal soul_war_mechanics.lua:803-829: original onHealthChange
// components qualify before prepareDeath cancellation or healing clamp.
// Backpressure bound, not a gameplay cap: pending work is never silently evicted.
pub(super) const MAX_PENDING_CLOAK_BLOOD: usize = 1024;
impl ChannelActorCarrier {
    #[allow(
        clippy::too_many_arguments,
        reason = "the actual health callback binds its source, both components and exact current owner proof together"
    )]
    pub(super) fn stage_cloak_blood_event(
        &self,
        continuity: &NamespaceContinuityGuard,
        pin: &ChannelContentPin,
        actor: ExactActorRef,
        source: &super::soul_war_health::HealthSourceContext,
        primary: i64,
        secondary: i64,
        occurrence: [u8; 32],
    ) -> Result<Option<CommittedCloakBloodEvent>, CarrierError> {
        use super::soul_war_health::HealthSourceKind;
        if !source.invoke_health_change {
            return Ok(None);
        }
        let eligible = match source.kind {
            HealthSourceKind::Null => false,
            // Accepted authoring13.7 Q6a: actual Player HIT leaves a pool.
            // Preserve raw signed components for every other source callback.
            HealthSourceKind::Player(_) => primary != 0 || secondary != 0,
            HealthSourceKind::NonPlayer => secondary > 0,
        };
        if !eligible {
            return Ok(None);
        }
        let index = self.validate_ref(continuity, actor.0)?;
        let Slot::CreatureOccupied {
            spell_combat,
            companion: Some(companion),
            position: Some(position),
            ..
        } = &self.slots[index]
        else {
            return Ok(None);
        };
        let Some(policy) = spell_combat.soul_war_health.as_deref() else {
            return Ok(None);
        };
        if companion.policy.definition_key != "oteryn:creature.cloak_of_terror"
            || !policy.matches(
                source.content,
                &companion.policy.definition_key,
                &companion.policy.definition_revision,
            )
            || source.content != pin.server_artifact_digest()
        {
            return Ok(None);
        }
        Ok(Some(CommittedCloakBloodEvent {
            stamp: None,
            actor,
            position: MovementLocalPosition {
                x: position.position.x,
                y: position.position.y,
                floor: position.position.floor,
            },
            creature: oteryn_gameplay_content::content::ProjectV2DefinitionRef {
                family: oteryn_gameplay_content::content::ProjectV2Family::Creature,
                key: companion.policy.definition_key.clone(),
                revision: companion.policy.definition_revision.clone(),
            },
            server: pin.server_artifact_digest(),
            client: pin.client_artifact_digest(),
            frame: pin.frame_binding_digest(),
            map: pin.map_revision_digest(),
            source_body: BODY_DIGEST,
            occurrence,
        }))
    }
    pub(super) fn reserve_cloak_blood_capacity(
        &mut self,
        incoming: &[CommittedCloakBloodEvent],
    ) -> Result<(), CarrierError> {
        let mut occurrences = Vec::new();
        occurrences
            .try_reserve(MAX_PENDING_CLOAK_BLOOD)
            .map_err(|_| CarrierError::AllocationFailed)?;
        for event in &self.cloak_blood_events {
            occurrences.push(event.occurrence);
        }
        for slot in self.slots.iter() {
            let state = match slot {
                Slot::Occupied { spell_combat, .. }
                | Slot::CreatureOccupied { spell_combat, .. } => spell_combat,
                _ => continue,
            };
            if let Some(reservation) = &state.pending_owner {
                for event in reservation.cloak_blood_events() {
                    if !occurrences.contains(&event.occurrence) {
                        if occurrences.len() == MAX_PENDING_CLOAK_BLOOD {
                            return Err(CarrierError::CapacityExceeded);
                        }
                        occurrences.push(event.occurrence);
                    }
                }
            }
        }
        for event in incoming {
            if !occurrences.contains(&event.occurrence) {
                if occurrences.len() == MAX_PENDING_CLOAK_BLOOD {
                    return Err(CarrierError::CapacityExceeded);
                }
                occurrences.push(event.occurrence);
            }
        }
        self.cloak_blood_events
            .try_reserve(
                occurrences
                    .len()
                    .saturating_sub(self.cloak_blood_events.len()),
            )
            .map_err(|_| CarrierError::AllocationFailed)
    }
    pub(super) fn install_cloak_blood_events(
        &mut self,
        incoming: impl IntoIterator<Item = CommittedCloakBloodEvent>,
    ) {
        for event in incoming {
            if !self
                .cloak_blood_events
                .iter()
                .any(|old| old.occurrence == event.occurrence)
            {
                self.cloak_blood_events.push(event);
            }
        }
    }
}
impl ChannelRuntimeV1 {
    pub fn acknowledge_cloak_blood_event(
        &mut self,
        event: &CommittedCloakBloodEvent,
    ) -> Result<(), CarrierError> {
        if !event.current_for(self) {
            return Err(CarrierError::WrongScope);
        }
        self.carrier
            .cloak_blood_events
            .retain(|pending| pending.occurrence != event.occurrence);
        Ok(())
    }
}

const BODY_DIGEST: [u8; 32] = [
    158, 17, 7, 220, 65, 171, 193, 107, 79, 86, 231, 149, 24, 12, 28, 23, 201, 23, 216, 231, 55,
    84, 84, 124, 68, 63, 7, 14, 211, 22, 121, 22,
];

impl CurrentOwnerExactActorCommit<'_> {
    pub(super) fn prepare_cloak_blood_direct(
        &mut self,
        actor: ExactActorRef,
        command: CommandRef,
        ordinal: u16,
        damage: &OwnerDamageCommand<'_>,
        source: Option<&super::soul_war_health::HealthSourceContext>,
    ) -> Result<Option<CommittedCloakBloodEvent>, CarrierError> {
        use sha2::{Digest, Sha256};
        let (Some(pin), Some(source)) = (self.content, source) else {
            return Ok(None);
        };
        let occurrence = Sha256::new()
            .chain_update(b"oteryn:cloak-health-event-direct:v1")
            .chain_update(command.game_session_id().as_bytes())
            .chain_update(command.command_id().get().to_be_bytes())
            .chain_update(ordinal.to_be_bytes())
            .chain_update(actor.placement_identity())
            .chain_update(damage.binding)
            .finalize()
            .into();
        let Some(mut event) = self.carrier.stage_cloak_blood_event(
            self.continuity,
            pin,
            actor,
            source,
            // Combat loss reaches Lua with its original negative sign.
            damage
                .damage
                .checked_neg()
                .ok_or(CarrierError::InvalidDamage)?,
            0,
            occurrence,
        )?
        else {
            return Ok(None);
        };
        self.carrier
            .reserve_cloak_blood_capacity(std::slice::from_ref(&event))?;
        let owner = self
            .owner_cycle
            .as_deref_mut()
            .ok_or(CarrierError::WrongScope)?;
        let ordinal = owner
            .accept_input(owner.generation())
            .map_err(|_| CarrierError::WrongScope)?;
        event.stamp = Some(owner.stamp(ordinal));
        Ok(Some(event))
    }
}
