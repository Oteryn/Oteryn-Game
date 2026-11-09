//! Logical source registration during the existing deferred death phase.
//! A physical slot can outlive this phase while its immutable SQL drop settles.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCreatureDeathSeal {
    actor: ExactActorRef,
    death: CreatureDeathOccurrenceKey,
    profile: Arc<runtime_actor_companion::CompiledCreaturePolicy>,
    position: MovementLocalPosition,
    has_reward_damage_credit: bool,
    content: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
    client: [u8; 32],
    stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
    events: Arc<
        oteryn_gameplay_content::content::native_creature_death_events::QualifiedNativeDeathEvents,
    >,
}

/// Known completion of the copied source vector. An unfinished BEGIN cannot
/// construct this receipt; it remains usable after physical Slot retirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCreatureDeathCompletion {
    begin: SourceCreatureDeathSeal,
}
impl SourceCreatureDeathCompletion {
    pub fn begin(&self) -> &SourceCreatureDeathSeal {
        &self.begin
    }
    pub fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.begin.current_for(runtime)
    }
    pub fn actor(&self) -> ExactActorRef {
        self.begin.actor()
    }
    pub fn death_key(&self) -> CreatureDeathOccurrenceKey {
        self.begin.death_key()
    }
}
impl SourceCreatureDeathSeal {
    pub fn client_digest(&self) -> [u8; 32] {
        self.client
    }
    /// Historical task data remains usable after physical actor retirement,
    /// but never outside its actual source owner/content generation.
    pub fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        let Ok(owner) = runtime.owner_fence() else {
            return false;
        };
        let pin = runtime.content_pin();
        owner.accepts_stamp(self.stamp)
            && pin.server_artifact_digest() == self.content
            && pin.client_artifact_digest() == self.client
            && pin.frame_binding_digest() == self.frame
            && pin.map_revision_digest() == self.map
            && self.events.matches(
                self.content,
                self.definition_key(),
                self.definition_revision(),
            )
    }
    pub fn events(&self) -> &[oteryn_gameplay_content::content::native_creature_death_events::QualifiedNativeDeathEvent]{
        self.events.events()
    }
    pub fn actor(&self) -> ExactActorRef {
        self.actor
    }
    pub fn death_key(&self) -> CreatureDeathOccurrenceKey {
        self.death
    }
    pub fn definition_key(&self) -> &str {
        &self.profile.definition_key
    }
    pub fn definition_revision(&self) -> &str {
        &self.profile.definition_revision
    }
    /// Original ordinary reward credit at BEGIN, including master credit.
    /// This is deliberately separate from Lua actual-Player damage spans.
    pub fn has_reward_damage_credit(&self) -> bool {
        self.has_reward_damage_credit
    }
    pub fn position(&self) -> MovementLocalPosition {
        self.position
    }
    pub fn content_digest(&self) -> [u8; 32] {
        self.content
    }
    pub fn frame_digest(&self) -> [u8; 32] {
        self.frame
    }
    pub fn map_digest(&self) -> [u8; 32] {
        self.map
    }
}

impl ChannelRuntimeV1 {
    /// DATA only: unlike the ordinary actionable census this includes HP0
    /// until its source onDeath finishes. It grants no attack or input authority.
    pub fn source_creature_is_registered(
        &self,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        self.owner_fence()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        match &self.carrier.slots[index] {
            Slot::CreatureOccupied {
                spell_combat,
                companion: Some(_),
                position: Some(_),
                ..
            } => Ok(!spell_combat.source_death_completed),
            _ => Err(CarrierError::NotCreature),
        }
    }
    pub fn source_registered_creature_census(
        &self,
    ) -> Result<Vec<(ExactActorRef, MovementPositionSnapshot)>, CarrierError> {
        self.owner_fence()?;
        self.carrier.validate_current_continuity(&self.continuity)?;
        let mut result = Vec::new();
        result
            .try_reserve(self.carrier.occupied.len())
            .map_err(|_| CarrierError::AllocationFailed)?;
        for &slot in &self.carrier.occupied {
            let index =
                usize::try_from(slot).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
            let Some(Slot::CreatureOccupied {
                generation,
                spell_combat,
                companion: Some(_),
                position: Some(_),
                ..
            }) = self.carrier.slots.get(index)
            else {
                continue;
            };
            if spell_combat.source_death_completed {
                continue;
            }
            let actor = ExactActorRef(ActorRef::from_scoped_owner_parts(
                self.binding.runtime_scope,
                self.binding.scope_generation,
                ActorLocalId(
                    u32::try_from(index + 1)
                        .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                ),
                ActorLocalGeneration(*generation),
            ));
            let position = self.read_actor_position(actor)?;
            if position.context() != self.pinned_movement_context() {
                return Err(CarrierError::PositionContextMismatch);
            }
            result.push((actor, position));
        }
        Ok(result)
    }
    /// Scheduling hint only; no source mutation or authority follows from it.
    pub fn has_pending_source_creature_deaths(&self) -> bool {
        !self.carrier.native_deaths.is_empty()
    }
    pub fn source_creature_death_pending(
        &self,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        self.owner_fence()?;
        self.carrier.validate_ref(&self.continuity, actor.0)?;
        Ok(self
            .carrier
            .native_deaths
            .iter()
            .any(|notice| notice.actor == actor))
    }
    pub fn next_source_creature_death(
        &self,
    ) -> Result<Option<SourceCreatureDeathNotice>, CarrierError> {
        self.owner_fence()?;
        Ok(self.carrier.native_deaths.first().cloned())
    }
    /// Freeze the actual task-BEGIN type and copied callback order once. A
    /// later handler may setType, but cannot replace this task's drop facts.
    pub fn begin_source_creature_death(
        &mut self,
        notice: &SourceCreatureDeathNotice,
    ) -> Result<SourceCreatureDeathSeal, CarrierError> {
        self.owner_fence()?;
        self.assert_actor_spell_unreserved(notice.actor)?;
        if self.carrier.native_deaths.first() != Some(notice) {
            return Err(CarrierError::CorpseProjectionConflict);
        }
        let original = self
            .carrier
            .committed_lethal_receipt_inner(&self.continuity, notice.actor.0)?;
        if original.projection.occurrence.death_key()? != notice.death {
            return Err(CarrierError::CorpseProjectionConflict);
        }
        let index = self
            .carrier
            .validate_ref(&self.continuity, notice.actor.0)?;
        let Slot::CreatureOccupied {
            health,
            companion: Some(companion),
            spell_combat,
            damage_contributors,
            ..
        } = &self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if *health != 0 || spell_combat.source_death_completed {
            return Err(CarrierError::CommittedLethalUnavailable);
        }
        if let Some(begin) = &spell_combat.source_death_begin {
            return Ok(begin.as_ref().clone());
        }
        let has_reward_damage_credit = damage_contributors
            .entries
            .iter()
            .any(|entry| entry.total > 0);
        let events = spell_combat
            .native_death_events
            .clone()
            .ok_or(CarrierError::InvalidSpawnDefinition)?;
        let pin = self.content_pin();
        if !events.matches(
            pin.server_artifact_digest(),
            &companion.policy.definition_key,
            &companion.policy.definition_revision,
        ) {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let profile = self.companion_policy(&companion.policy.definition_key)?;
        if profile.as_ref() != companion.policy.as_ref() {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let position = self.read_actor_position(notice.actor)?.position();
        let (content, client, frame, map) = (
            pin.server_artifact_digest(),
            pin.client_artifact_digest(),
            pin.frame_binding_digest(),
            pin.map_revision_digest(),
        );
        let stamp = self.issue_owner_work()?;
        let seal = SourceCreatureDeathSeal {
            actor: notice.actor,
            death: notice.death,
            profile,
            position,
            has_reward_damage_credit,
            content,
            client,
            frame,
            map,
            stamp,
            events,
        };
        let retained = Arc::new(seal.clone());
        let Slot::CreatureOccupied { spell_combat, .. } = &mut self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        spell_combat.source_death_begin = Some(retained);
        Ok(seal)
    }
    /// Current copied event offset. Registration indices may have gaps; this
    /// offset always addresses the immutable BEGIN vector in its source order.
    pub fn source_creature_death_event_index(
        &self,
        begin: &SourceCreatureDeathSeal,
    ) -> Result<Option<usize>, CarrierError> {
        self.owner_fence()?;
        if !begin.current_for(self) {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let index = self.carrier.validate_ref(&self.continuity, begin.actor.0)?;
        let Slot::CreatureOccupied {
            health,
            spell_combat,
            ..
        } = &self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if *health != 0 || spell_combat.source_death_begin.as_deref() != Some(begin) {
            return Err(CarrierError::CorpseProjectionConflict);
        }
        if spell_combat.source_death_completed {
            return Ok(None);
        }
        if self
            .carrier
            .native_deaths
            .first()
            .is_none_or(|notice| notice.actor != begin.actor || notice.death != begin.death)
        {
            return Err(CarrierError::CorpseProjectionConflict);
        }
        let cursor = usize::from(spell_combat.source_death_event_cursor);
        if cursor > begin.events().len() {
            return Err(CarrierError::CorpseProjectionConflict);
        }
        Ok((cursor < begin.events().len()).then_some(cursor))
    }

    /// A known-completed copied handler is acknowledged under the same owner.
    /// Replaying its ACK cannot advance a later event or skip an earlier one.
    pub fn acknowledge_source_creature_death_event(
        &mut self,
        begin: &SourceCreatureDeathSeal,
        event: &oteryn_gameplay_content::content::native_creature_death_events::QualifiedNativeDeathEvent,
    ) -> Result<bool, CarrierError> {
        let current = self.source_creature_death_event_index(begin)?;
        let copied = begin
            .events()
            .iter()
            .position(|entry| entry == event)
            .ok_or(CarrierError::CorpseProjectionConflict)?;
        let index = self.carrier.validate_ref(&self.continuity, begin.actor.0)?;
        let Slot::CreatureOccupied { spell_combat, .. } = &self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        if copied < usize::from(spell_combat.source_death_event_cursor) {
            return Ok(false);
        }
        if current != Some(copied) {
            return Err(CarrierError::CorpseProjectionConflict);
        }
        self.assert_actor_spell_unreserved(begin.actor)?;
        let Slot::CreatureOccupied { spell_combat, .. } = &mut self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        spell_combat.source_death_event_cursor = spell_combat
            .source_death_event_cursor
            .checked_add(1)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        Ok(true)
    }

    /// The ordered source-task consumer calls this only after all copied
    /// handlers finish and before yielding to physical Item persistence.
    pub fn finish_source_creature_death(
        &mut self,
        begin: &SourceCreatureDeathSeal,
    ) -> Result<SourceCreatureDeathSeal, CarrierError> {
        self.owner_fence()?;
        if !begin.current_for(self) {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        self.assert_actor_spell_unreserved(begin.actor)?;
        let index = self.carrier.validate_ref(&self.continuity, begin.actor.0)?;
        let pin = self.content_pin();
        if pin.server_artifact_digest() != begin.content
            || pin.client_artifact_digest() != begin.client
            || pin.frame_binding_digest() != begin.frame
            || pin.map_revision_digest() != begin.map
        {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let Slot::CreatureOccupied {
            health,
            spell_combat,
            ..
        } = &self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if *health != 0 || spell_combat.source_death_begin.as_deref() != Some(begin) {
            return Err(CarrierError::CorpseProjectionConflict);
        }
        if spell_combat.source_death_completed {
            return Ok(begin.clone());
        }
        if usize::from(spell_combat.source_death_event_cursor) != begin.events().len() {
            return Err(CarrierError::CorpseProjectionConflict);
        }
        if self
            .carrier
            .native_deaths
            .first()
            .is_none_or(|notice| notice.actor != begin.actor || notice.death != begin.death)
        {
            return Err(CarrierError::CorpseProjectionConflict);
        }
        let Slot::CreatureOccupied { spell_combat, .. } = &mut self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        spell_combat.source_death_completed = true;
        self.carrier.native_deaths.remove(0);
        Ok(begin.clone())
    }

    /// Retain the completed phase immediately, before physical persistence is
    /// allowed to retire the actor. Historical replay needs no live pointer.
    pub fn finish_source_creature_death_task(
        &mut self,
        begin: &SourceCreatureDeathSeal,
    ) -> Result<SourceCreatureDeathCompletion, CarrierError> {
        let begin = self.finish_source_creature_death(begin)?;
        Ok(SourceCreatureDeathCompletion { begin })
    }
    // Fixture-only compatibility for the original logical-phase contract.
    // Production must pass through BEGIN and the ordered handler consumer.
    #[cfg(any(test, feature = "test-support"))]
    pub fn complete_source_creature_death(
        &mut self,
        actor: ExactActorRef,
        expected: CreatureDeathOccurrenceKey,
    ) -> Result<SourceCreatureDeathSeal, CarrierError> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        if let Slot::CreatureOccupied { spell_combat, .. } = &self.carrier.slots[index]
            && spell_combat.source_death_completed
        {
            return spell_combat
                .source_death_begin
                .as_deref()
                .filter(|seal| seal.death == expected)
                .cloned()
                .ok_or(CarrierError::CorpseProjectionConflict);
        }
        let notice = self
            .next_source_creature_death()?
            .ok_or(CarrierError::CommittedLethalUnavailable)?;
        if notice.actor != actor || notice.death != expected {
            return Err(CarrierError::CorpseProjectionConflict);
        }
        let begin = self.begin_source_creature_death(&notice)?;
        // Fixture compatibility explicitly completes the copied invocations;
        // production never uses this callback-free shortcut.
        for event in begin.events() {
            self.acknowledge_source_creature_death_event(&begin, event)?;
        }
        self.finish_source_creature_death(&begin)
    }
}

/// Data retained only by actual canonical HP publication. It is not a grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCreatureDeathNotice {
    actor: ExactActorRef,
    death: CreatureDeathOccurrenceKey,
    /// Original committed owner clock; never infer chronology from a wake hint.
    enqueued_at_ms: Option<u64>,
    enqueued_at_us: Option<u64>,
}
impl SourceCreatureDeathNotice {
    pub fn actor(&self) -> ExactActorRef {
        self.actor
    }
    pub fn death_key(&self) -> CreatureDeathOccurrenceKey {
        self.death
    }
    pub fn enqueued_at_ms(&self) -> Option<u64> {
        self.enqueued_at_ms
    }
    pub fn enqueued_at_us(&self) -> Option<u64> {
        self.enqueued_at_us
    }
}
// Backpressure only; pending source tasks are never evicted or silently skipped.
pub(super) const MAX_PENDING_NATIVE_DEATHS: usize = 1024;
impl ChannelActorCarrier {
    pub(super) fn native_death_notice_for_record(
        actor: ExactActorRef,
        record: &OwnerCommitRecord,
        at_ms: Option<u64>,
        at_us: Option<u64>,
    ) -> Result<SourceCreatureDeathNotice, CarrierError> {
        if !record.result.applied
            || record.result.health_before <= 0
            || record.result.health_after != 0
        {
            return Err(CarrierError::CommittedLethalUnavailable);
        }
        let occurrence = CreatureDeathOccurrenceRef {
            actor,
            commit_binding: copy_bounded_binding(&record.binding)?,
            damage: record.damage,
            health_before: record.result.health_before,
        };
        Ok(SourceCreatureDeathNotice {
            actor,
            death: occurrence.death_key()?,
            enqueued_at_ms: at_ms,
            enqueued_at_us: at_us,
        })
    }
    pub(super) fn native_death_notice_for_slot(
        &self,
        actor: ExactActorRef,
        slot: &Slot,
        at_us: Option<u64>,
    ) -> Result<Option<SourceCreatureDeathNotice>, CarrierError> {
        if self
            .native_deaths
            .iter()
            .any(|notice| notice.actor == actor)
        {
            return Ok(None);
        }
        let Slot::CreatureOccupied {
            health: 0,
            committed,
            spell_combat,
            companion: Some(_),
            ..
        } = slot
        else {
            return Ok(None);
        };
        if spell_combat.native_death_events.is_none() || spell_combat.source_death_completed {
            return Ok(None);
        }
        committed
            .lethal()
            .map(|record| {
                Self::native_death_notice_for_record(
                    actor,
                    record,
                    committed.lethal_owner_time_ms(),
                    at_us,
                )
            })
            .transpose()
    }
    pub(super) fn reserve_native_death_capacity(
        &mut self,
        incoming: &[SourceCreatureDeathNotice],
    ) -> Result<(), CarrierError> {
        let mut pending = Vec::new();
        pending
            .try_reserve(MAX_PENDING_NATIVE_DEATHS)
            .map_err(|_| CarrierError::AllocationFailed)?;
        pending.extend(self.native_deaths.iter().cloned());
        for &slot in &self.occupied {
            let state = match &self.slots[slot as usize] {
                Slot::Occupied { spell_combat, .. }
                | Slot::CreatureOccupied { spell_combat, .. } => spell_combat,
                _ => continue,
            };
            if let Some(reservation) = &state.pending_owner {
                for notice in reservation.native_death_events() {
                    if !pending.contains(notice) {
                        if pending.len() == MAX_PENDING_NATIVE_DEATHS {
                            return Err(CarrierError::CapacityExceeded);
                        }
                        pending.push(notice.clone());
                    }
                }
            }
        }
        for notice in incoming {
            if !pending.contains(notice) {
                if pending.len() == MAX_PENDING_NATIVE_DEATHS {
                    return Err(CarrierError::CapacityExceeded);
                }
                pending.push(notice.clone());
            }
        }
        self.native_deaths
            .try_reserve(pending.len().saturating_sub(self.native_deaths.len()))
            .map_err(|_| CarrierError::AllocationFailed)
    }
    pub(super) fn install_native_death_events(
        &mut self,
        incoming: impl IntoIterator<Item = SourceCreatureDeathNotice>,
    ) {
        for notice in incoming {
            if !self.native_deaths.contains(&notice) {
                self.native_deaths.push(notice);
            }
        }
    }
}

/// Literal Creature:getKillers(true) data, separate from master reward credit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePlayerDamageShare {
    pub actor: ExactActorRef,
    pub character: CharacterId,
    pub total: u64,
    pub last_damage_at_ms: Option<u64>,
}
impl ChannelRuntimeV1 {
    pub fn source_player_damage_shares(
        &self,
        actor: ExactActorRef,
    ) -> Result<Vec<SourcePlayerDamageShare>, CarrierError> {
        self.owner_fence()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied {
            damage_contributors,
            ..
        } = &self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        Ok(damage_contributors
            .entries
            .iter()
            .filter_map(|entry| entry.source_player)
            .collect())
    }
}
