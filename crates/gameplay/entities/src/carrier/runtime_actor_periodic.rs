//! Actual condition-store ticks use an owner-input origin, never a player command.
//! The world owner supplies independently qualified standing/PZ facts in this
//! uninterrupted turn. No detached tick or caller damage number is accepted.
type CurrentPlayerPeriodicSource<'a> =
    dyn FnMut(&ChannelRuntimeV1, ExactActorRef, u64) -> Result<Option<u32>, CarrierError> + 'a;

use super::runtime_actor_companion::{CompanionSnapshot, CompanionState};
use super::*;
#[path = "runtime_actor_periodic/health_events.rs"]
mod health_events;
use health_events::PeriodicHealthEvents;
use oteryn_gameplay_foundation::foundation::RuntimeWorkStamp;
use oteryn_gameplay_foundation::foundation::condition::{ConditionTick, TickFacts, TickKind};

/// A field recipe observed by the current map owner. Cursor and HP remain in
/// the creature slot; this value never restores a historical controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureFieldEntry {
    pub definition: oteryn_gameplay_foundation::foundation::condition::ConditionDefinition,
    pub source: Option<ExactActorRef>,
    pub lineage: Option<oteryn_gameplay_foundation::foundation::condition::ConditionLineage>,
    pub occurrence: oteryn_simulation_determinism::DecisionOccurrenceId,
}

#[derive(Debug)]
pub struct PreparedCreaturePeriodicTurn {
    impacts: Vec<CreaturePeriodicImpact>,
    health_events: PeriodicHealthEvents,
    before: CompanionSnapshot,
    original_slot: Slot,
    damage_contributors: Box<DamageContributors>,
    after: CompanionState,
    conditions_after: super::runtime_actor_conditions::ActorConditionState,
    health_after: i64,
    stamp: RuntimeWorkStamp,
    ticks: Vec<ConditionTick<ExactActorRef>>,
    next_ordinal: u64,
    lethal_committed: Option<Box<super::runtime_actor_conditions::CreatureCommitState>>,
    native_death: Option<super::native_source_death::SourceCreatureDeathNotice>,
}
#[derive(Debug)]
pub struct CreaturePeriodicReceipt {
    /// Final per-tick health effects, installed with this transient owner receipt.
    pub impacts: Vec<CreaturePeriodicImpact>,
    pub actor: ExactActorRef,
    pub health_before: i64,
    pub health_after: i64,
    pub ticks: Vec<ConditionTick<ExactActorRef>>,
}
/// Read-only attribution of an applied periodic health effect.
#[derive(Debug)]
pub struct CreaturePeriodicImpact {
    pub source: Option<ExactActorRef>,
    pub element: Option<oteryn_gameplay_domain::domain::condition::DotElement>,
    pub damage: u64,
    pub healing: u64,
}
fn field_entry_is_current(
    previous: (Option<u64>, Option<&CreatureFieldEntry>),
    position_revision: u64,
    field: Option<&CreatureFieldEntry>,
) -> bool {
    previous.0 == Some(position_revision)
        && match (previous.1, field) {
            (Some(previous), Some(current)) => {
                previous.occurrence == current.occurrence
                    && previous.definition == current.definition
            }
            (None, None) => true,
            _ => false,
        }
}

fn apply_field_entry(
    state: &mut CompanionState,
    entry: Option<&CreatureFieldEntry>,
    now_us: u64,
    content_digest: [u8; 32],
) -> Result<Option<u32>, CarrierError> {
    if let Some(field) = entry {
        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(content_digest);
        let facts = oteryn_gameplay_foundation::foundation::condition::ApplicationFacts {
            now: now_us,
            base_speed: u16::try_from(state.policy.base_speed)
                .map_err(|_| CarrierError::PlanConflict)?,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: false,
            decision_root: &root,
            occurrence: field.occurrence,
        };
        use oteryn_gameplay_domain::domain::condition::ConditionRefusal;
        use oteryn_gameplay_foundation::foundation::condition::ConditionSourceKind;
        let result = if let Some(lineage) = &field.lineage {
            state.conditions.apply_with_lineage(
                &field.definition,
                field.source,
                ConditionSourceKind::Field,
                &state.policy.condition_immunities,
                &facts,
                lineage,
            )
        } else {
            state.conditions.apply(
                &field.definition,
                None,
                ConditionSourceKind::Field,
                &state.policy.condition_immunities,
                &facts,
            )
        };
        return match result {
            Ok(applied) => Ok(Some(applied.sequence)),
            Err(
                ConditionRefusal::Immune
                | ConditionRefusal::KeptCurrent
                | ConditionRefusal::ReentryProtected,
            ) => Ok(None),
            Err(_) => Err(CarrierError::PlanConflict),
        };
    }
    Ok(None)
}

impl ChannelRuntimeV1 {
    /// Install entry once per actual position successor, before its due HP
    /// turn. Cancellation between these synchronous owner turns leaves the
    /// initial tick pending in the sole native condition store.
    pub fn commit_creature_field_entry(
        &mut self,
        actor: ExactActorRef,
        now_us: u64,
        field: Option<&CreatureFieldEntry>,
    ) -> Result<(), CarrierError> {
        self.commit_creature_field_entry_inner(actor, now_us, None, field, false)
            .map(drop)
    }

    pub fn commit_creature_field_entry_at(
        &mut self,
        actor: ExactActorRef,
        now_us: u64,
        position_revision: u64,
        field: &CreatureFieldEntry,
    ) -> Result<Option<u32>, CarrierError> {
        self.commit_creature_field_entry_inner(
            actor,
            now_us,
            Some(position_revision),
            Some(field),
            false,
        )
    }

    pub fn commit_creature_spike_trap_entry(
        &mut self,
        actor: ExactActorRef,
        now_us: u64,
        field: &CreatureFieldEntry,
    ) -> Result<(), CarrierError> {
        self.commit_creature_field_entry_inner(actor, now_us, None, Some(field), true)
            .map(drop)
    }

    pub fn creature_field_entry_is_current(
        &self,
        actor: ExactActorRef,
        revision: u64,
        field: &CreatureFieldEntry,
    ) -> Result<bool, CarrierError> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied { committed, .. } = &self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        Ok(field_entry_is_current(
            (
                committed.field_entry_revision,
                committed.field_entry.as_ref(),
            ),
            revision,
            Some(field),
        ))
    }

    fn commit_creature_field_entry_inner(
        &mut self,
        actor: ExactActorRef,
        now_us: u64,
        position_revision: Option<u64>,
        field: Option<&CreatureFieldEntry>,
        spike_trap: bool,
    ) -> Result<Option<u32>, CarrierError> {
        self.assert_actor_spell_unreserved(actor)?;
        let before = self.companion_snapshot(actor)?;
        let position_revision = position_revision.unwrap_or(before.position_revision);
        self.owner_fence()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied { committed, .. } = &self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        let previous = (
            committed.field_entry_revision,
            committed.field_entry.as_ref(),
        );
        let trap = field
            .filter(|_| spike_trap)
            .map(|field| (position_revision, field.occurrence));
        if if spike_trap {
            committed.spike_trap_entry == trap
        } else {
            field_entry_is_current(previous, position_revision, field)
        } {
            return Ok(None);
        }
        self.validate_creature_field_lineage(field)?;
        let mut next = before.state;
        let application = apply_field_entry(
            &mut next,
            field,
            now_us,
            self.content_pin().server_artifact_digest(),
        )?;
        let Slot::CreatureOccupied {
            committed,
            companion,
            ..
        } = &mut self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        *companion = Some(Box::new(next));
        if spike_trap {
            committed.spike_trap_entry = trap;
        } else {
            committed.field_entry_revision = Some(position_revision);
            committed.field_entry = field.cloned();
        }
        Ok(application)
    }

    fn validate_creature_field_lineage(
        &self,
        field: Option<&CreatureFieldEntry>,
    ) -> Result<(), CarrierError> {
        if let Some(field) = field {
            if field.source.is_some() != field.lineage.is_some() {
                return Err(CarrierError::PlanConflict);
            }
            if let Some(lineage) = &field.lineage
                && (lineage.source_content_digest != self.content_pin().server_artifact_digest()
                    || lineage.source_scope_generation != self.binding().scope_generation().get()
                    || field.source.is_none_or(|source| {
                        source.placement_identity() != lineage.source_actor_placement
                            || !self.borrow_exact_actor_lookup().contains(source)
                    }))
            {
                return Err(CarrierError::PlanConflict);
            }
        }
        Ok(())
    }

    /// Preview the sources of accepted immediate hits and residual work on a
    /// cloned store, before the field pass publishes its existing source facts.
    pub fn preview_creature_field_due<'a>(
        &self,
        mut snapshot: CompanionSnapshot,
        now_us: u64,
        contacts: impl Iterator<Item = (u64, &'a CreatureFieldEntry, Option<u32>)>,
        facts: TickFacts,
    ) -> Result<Vec<ConditionTick<ExactActorRef>>, CarrierError> {
        let index = self
            .carrier
            .validate_ref(&self.continuity, snapshot.actor.0)?;
        let Slot::CreatureOccupied { committed, .. } = &self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        let content_digest = self.content_pin().server_artifact_digest();
        let mut previous = (
            committed.field_entry_revision,
            committed.field_entry.as_ref(),
        );
        let mut ticks = Vec::new();
        for (revision, field, applied) in contacts {
            let sequence = if applied.is_some() {
                applied
            } else if field_entry_is_current(previous, revision, Some(field)) {
                None
            } else {
                self.validate_creature_field_lineage(Some(field))?;
                previous = (Some(revision), Some(field));
                apply_field_entry(&mut snapshot.state, Some(field), now_us, content_digest)?
            };
            if let Some(sequence) = sequence {
                ticks.extend(
                    snapshot
                        .state
                        .conditions
                        .take_due_application(now_us, facts, sequence),
                );
                if snapshot.state.conditions.has_pending_application(sequence) {
                    return Ok(ticks);
                }
            }
        }
        ticks.extend(snapshot.state.conditions.take_due(now_us, facts));
        Ok(ticks)
    }

    /// Stages only a source-qualified physical actor's existing condition store.
    /// `facts` belong to the actual world/tile owner, not the attacker or client.
    pub fn stage_creature_periodic_turn(
        &mut self,
        actor: ExactActorRef,
        now_us: u64,
        facts: TickFacts,
    ) -> Result<PreparedCreaturePeriodicTurn, CarrierError> {
        self.stage_creature_periodic_turn_inner(actor, now_us, facts, None, &[], None, None)?
            .ok_or(CarrierError::PlanConflict)
    }
    /// Source snapshots/PlayerSpellState reads are acquired and committed in
    /// one locked Channel-owner call. This is not a detached damage capability.
    pub fn apply_creature_periodic_turn_with_sources(
        &mut self,
        actor: ExactActorRef,
        now_us: u64,
        facts: TickFacts,
        player_source: &mut CurrentPlayerPeriodicSource<'_>,
    ) -> Result<CreaturePeriodicReceipt, CarrierError> {
        let prepared = self
            .stage_creature_periodic_turn_inner(
                actor,
                now_us,
                facts,
                Some(player_source),
                &[],
                None,
                None,
            )?
            .ok_or(CarrierError::PlanConflict)?;
        self.commit_creature_periodic_turn(prepared)
    }

    /// Same periodic owner with current all-role source-name observations. The
    /// opaque observations carry no damage or HP authority and are revalidated
    /// against the actual target and physical cohort before publication.
    pub fn apply_creature_periodic_turn_with_sources_and_health_observations(
        &mut self,
        actor: ExactActorRef,
        now_us: u64,
        facts: TickFacts,
        player_source: &mut CurrentPlayerPeriodicSource<'_>,
        observations: &[super::native_health_change::NativeHealthChangeObservation],
    ) -> Result<CreaturePeriodicReceipt, CarrierError> {
        let prepared = self
            .stage_creature_periodic_turn_inner(
                actor,
                now_us,
                facts,
                Some(player_source),
                observations,
                None,
                None,
            )?
            .ok_or(CarrierError::PlanConflict)?;
        self.commit_creature_periodic_turn(prepared)
    }

    /// A field pass can wait for a genuinely reached busy source. The staged
    /// turn is discarded in that case and owns no consumed ticks or HP writes.
    #[allow(
        clippy::too_many_arguments,
        reason = "the native periodic owner shares the exact field inputs"
    )]
    pub fn apply_creature_field_periodic_turn(
        &mut self,
        actor: ExactActorRef,
        now_us: u64,
        facts: TickFacts,
        player_source: &mut CurrentPlayerPeriodicSource<'_>,
        observations: &[super::native_health_change::NativeHealthChangeObservation],
        application: Option<u32>,
        source_busy: &dyn Fn(&Self, ExactActorRef) -> bool,
    ) -> Result<Option<CreaturePeriodicReceipt>, CarrierError> {
        self.stage_creature_periodic_turn_inner(
            actor,
            now_us,
            facts,
            Some(player_source),
            observations,
            application,
            Some(source_busy),
        )?
        .map(|prepared| self.commit_creature_periodic_turn(prepared))
        .transpose()
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the field pass and ordinary periodic turns share native staging"
    )]
    fn stage_creature_periodic_turn_inner(
        &mut self,
        actor: ExactActorRef,
        now_us: u64,
        facts: TickFacts,
        mut player_source: Option<&mut CurrentPlayerPeriodicSource<'_>>,
        observations: &[super::native_health_change::NativeHealthChangeObservation],
        application: Option<u32>,
        source_busy: Option<&dyn Fn(&Self, ExactActorRef) -> bool>,
    ) -> Result<Option<PreparedCreaturePeriodicTurn>, CarrierError> {
        if self.actor_spell_reserved(actor) {
            return Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict);
        }
        let before = self.companion_snapshot(actor)?;
        if !self.accepts_source_derived_companion_policy(&before)? {
            return Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict);
        }
        if self
            .encounter_hooks()
            .is_some_and(|hooks| hooks.contains(actor))
        {
            return Err(
                oteryn_gameplay_domain::domain::carrier_values::CarrierError::PositionContextMismatch,
            );
        }
        if !before.state.conditions.accepts_time(now_us) {
            return Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict);
        }
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied {
            committed,
            damage_contributors,
            ..
        } = &self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        let soulpit = match &self.carrier.slots[index] {
            Slot::CreatureOccupied { spell_combat, .. } => spell_combat.soulpit,
            _ => return Err(CarrierError::NotCreature),
        };
        let ordinal = committed.next_ordinal;
        let mut contributors_after = damage_contributors.clone();
        contributors_after
            .entries
            .try_reserve(
                COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX
                    .saturating_sub(contributors_after.entries.len()),
            )
            .map_err(|_| CarrierError::AllocationFailed)?;
        let stamp = self.issue_owner_work()?;
        let mut after = before.state.clone();
        let ticks = match application {
            Some(sequence) => after
                .conditions
                .take_due_application(now_us, facts, sequence),
            None => after.conditions.take_due(now_us, facts),
        };
        let mut impacts = Vec::new();
        let mut total = 0i64;
        let mut had_damage = false;
        let mut health_after = before.health;
        let mut health_events = PeriodicHealthEvents::new(self, actor, stamp, observations)?;
        let mut current_type = before.clone();
        for (tick_index, tick) in ticks.iter().enumerate() {
            match tick.kind {
                TickKind::Damage {
                    amount,
                    refused: false,
                    ..
                } => {
                    let mut healing = 0;
                    let mut dealt_percent = 100;
                    let mut attributed = tick.clone();
                    if let Some(source) = tick.provenance.source {
                        if let Some(lineage) = &tick.provenance.lineage
                            && (lineage.source_content_digest
                                != self.content_pin().server_artifact_digest()
                                || lineage.source_actor_placement != source.placement_identity()
                                || lineage.source_scope_generation
                                    != source.scope_generation().get())
                        {
                            return Err(
                                oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict,
                            );
                        }
                        if source_busy.is_some_and(|busy| busy(self, source)) {
                            return Ok(None);
                        }
                        let source_percent = match self.companion_snapshot_including_dead(source) {
                            Ok(snapshot) if snapshot.health <= 0 => None,
                            Ok(snapshot) => {
                                if self
                                    .companion_policy(&snapshot.state.policy.definition_key)?
                                    .as_ref()
                                    != snapshot.state.policy.as_ref()
                                {
                                    return Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict);
                                }
                                self.assert_actor_spell_unreserved(source)?;
                                let position = self.read_actor_position(source)?;
                                if position.context() != self.pinned_movement_context() {
                                    return Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PositionContextMismatch);
                                }
                                use oteryn_gameplay_domain::domain::condition::{
                                    ConditionValues, ConflictKey,
                                };
                                let source_store = self
                                    .actor_conditions_at(source, None, now_us)
                                    .map_err(|_| oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict)?;
                                let native = snapshot
                                    .state
                                    .conditions
                                    .active_at(ConflictKey::Attributes, now_us);
                                let imported =
                                    source_store.active_at(ConflictKey::Attributes, now_us);
                                let chosen = match (native, imported) {
                                    (Some(a), Some(b))
                                        if a.definition() == b.definition()
                                            && a.provenance() == b.provenance() =>
                                    {
                                        Some(a)
                                    }
                                    (Some(_), Some(_)) => return Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict),
                                    (a, b) => a.or(b),
                                };
                                Some(match chosen.map(|a| a.definition().values()) {
                                    Some(ConditionValues::Attributes {
                                        damage_dealt_percent,
                                        ..
                                    }) => damage_dealt_percent,
                                    _ => 100,
                                })
                            }
                            Err(_) => {
                                if let Some(ref mut lookup) = player_source {
                                    lookup(self, source, now_us)?
                                } else if !self.borrow_exact_actor_lookup().contains(source) {
                                    None
                                } else {
                                    return Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict);
                                }
                            }
                        };
                        if let Some(percent) = source_percent {
                            let TickKind::Damage { element, .. } = tick.kind else {
                                return Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict);
                            };
                            if current_type.state.policy.flags.attackable {
                                healing = current_type.state.policy.damage_healing(
                                    periodic_element_key(element),
                                    i64::from(amount),
                                )?;
                            }
                            dealt_percent = percent;
                        }
                        attributed.provenance.source = None;
                        attributed.provenance.lineage = None;
                    } else if tick.provenance.lineage.is_some() {
                        return Err(
                            oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict,
                        );
                    }
                    let adjusted = (u64::from(amount) * u64::from(dealt_percent)) / 100;
                    let adjusted = u32::try_from(adjusted).map_err(|_| {
                        oteryn_gameplay_domain::domain::carrier_values::CarrierError::DamageOverflow
                    })?;
                    use oteryn_gameplay_domain::domain::condition::ConditionValues;
                    use oteryn_gameplay_domain::domain::condition::ConflictKey;
                    let imported = self.actor_conditions_at(actor, None, now_us).map_err(|_| {
                        oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict
                    })?;
                    let a = before
                        .state
                        .conditions
                        .active_at(ConflictKey::Attributes, now_us);
                    let b = imported.active_at(ConflictKey::Attributes, now_us);
                    let chosen =
                        match (a, b) {
                            (Some(a), Some(b))
                                if a.definition() == b.definition()
                                    && a.provenance() == b.provenance() =>
                            {
                                Some(a)
                            }
                            (Some(_), Some(_)) => return Err(
                                oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict,
                            ),
                            (a, b) => a.or(b),
                        };
                    let received = match chosen.map(|a| a.definition().values()) {
                        Some(ConditionValues::Attributes {
                            incoming_reduction_percent,
                            ..
                        }) => 100u32.checked_sub(incoming_reduction_percent).ok_or(
                            oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict,
                        )?,
                        _ => 100,
                    };
                    let damage = finish_condition_tick_with_received(
                        &current_type,
                        &attributed,
                        adjusted,
                        now_us,
                        Some(received),
                    )?;
                    // Source combatBlockHit heals before residual damage; each
                    // real tick caps independently, and lethal never resurrects.
                    let healing = if let Some(tag) = soulpit {
                        tag.incoming_health_change(healing, health_after, before.maximum_health)?
                    } else {
                        healing
                    };
                    let health_before_healing = health_after;
                    health_after = health_after
                        .saturating_add(healing)
                        .min(before.maximum_health);
                    let damage = health_events.apply(
                        self,
                        actor,
                        &mut after,
                        &mut health_after,
                        before.maximum_health,
                        tick,
                        tick_index,
                        damage,
                        now_us,
                    )?;
                    current_type.state.policy = after.policy.clone();
                    let healed = health_after.saturating_sub(health_before_healing);
                    let damage = if let Some(tag) = soulpit {
                        tag.incoming_health_change(damage, health_after, before.maximum_health)?
                    } else {
                        damage
                    };
                    let applied = damage.min(health_after).max(0);
                    had_damage |= applied > 0;
                    if applied > 0 {
                        after.conditions.remove_type(
                            oteryn_gameplay_domain::domain::condition::ConditionType::Invisible,
                        );
                        after.conditions.remove_type(
                            oteryn_gameplay_domain::domain::condition::ConditionType::Status(
                                oteryn_gameplay_domain::domain::condition::StatusKind::Invisible,
                            ),
                        );
                    }
                    self.record_periodic_contributor(
                        &mut contributors_after,
                        tick,
                        applied as u64,
                        ordinal,
                        now_us,
                    )?;
                    if let TickKind::Damage { element, .. } = tick.kind {
                        impacts.push(CreaturePeriodicImpact {
                            source: tick.provenance.source,
                            element: Some(element),
                            damage: u64::try_from(applied).unwrap_or(0),
                            healing: u64::try_from(healed).unwrap_or(0),
                        });
                    }
                    health_after = health_after
                        .checked_sub(damage)
                        .ok_or(
                            oteryn_gameplay_domain::domain::carrier_values::CarrierError::DamageOverflow,
                        )?
                        .max(0);
                    total = before.health.checked_sub(health_after).ok_or(
                        oteryn_gameplay_domain::domain::carrier_values::CarrierError::DamageOverflow,
                    )?;
                    if health_after == 0 {
                        after.conditions.clear_on_death();
                        break;
                    }
                }
                // CONDITIONS-0 §3.2 (amended with §3.5): an agony tick skips mitigation,
                // resistances, attributes and charms; only its commit step runs.
                TickKind::Agony {
                    amount,
                    refused: false,
                } => {
                    let damage = if current_type.state.policy.flags.attackable {
                        i64::from(amount)
                    } else {
                        0
                    };
                    let damage = health_events.apply(
                        self,
                        actor,
                        &mut after,
                        &mut health_after,
                        before.maximum_health,
                        tick,
                        tick_index,
                        damage,
                        now_us,
                    )?;
                    current_type.state.policy = after.policy.clone();
                    let damage = if let Some(tag) = soulpit {
                        tag.incoming_health_change(damage, health_after, before.maximum_health)?
                    } else {
                        damage
                    };
                    had_damage |= damage > 0;
                    if damage > 0 {
                        after.conditions.remove_type(
                            oteryn_gameplay_domain::domain::condition::ConditionType::Invisible,
                        );
                        after.conditions.remove_type(
                            oteryn_gameplay_domain::domain::condition::ConditionType::Status(
                                oteryn_gameplay_domain::domain::condition::StatusKind::Invisible,
                            ),
                        );
                    }
                    self.record_periodic_contributor(
                        &mut contributors_after,
                        tick,
                        damage.min(health_after).max(0) as u64,
                        ordinal,
                        now_us,
                    )?;
                    impacts.push(CreaturePeriodicImpact {
                        source: tick.provenance.source,
                        element: None,
                        damage: u64::try_from(damage.min(health_after).max(0)).unwrap_or(0),
                        healing: 0,
                    });
                    health_after = health_after
                        .checked_sub(damage)
                        .ok_or(
                            oteryn_gameplay_domain::domain::carrier_values::CarrierError::DamageOverflow,
                        )?
                        .max(0);
                    total = before.health.checked_sub(health_after).ok_or(
                        oteryn_gameplay_domain::domain::carrier_values::CarrierError::DamageOverflow,
                    )?;
                    if health_after == 0 {
                        after.conditions.clear_on_death();
                        break;
                    }
                }
                TickKind::Damage { refused: true, .. }
                | TickKind::Agony { refused: true, .. }
                | TickKind::Regeneration {
                    suppressed: true, ..
                }
                | TickKind::SpellRegeneration {
                    suppressed: true, ..
                } => {}
                // Creature regeneration needs its actual source vitals recipe,
                // and is not inferred from the player's regeneration rules.
                _ => {
                    return Err(
                        oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict,
                    );
                }
            }
        }
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied { committed, .. } = &self.carrier.slots[index] else {
            return Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::NotCreature);
        };
        let mut conditions_after = committed.conditions.clone();
        if had_damage {
            conditions_after.remove_type_at(
                oteryn_gameplay_domain::domain::condition::ConditionType::Invisible,
                now_us,
            )?;
            conditions_after.remove_type_at(
                oteryn_gameplay_domain::domain::condition::ConditionType::Status(
                    oteryn_gameplay_domain::domain::condition::StatusKind::Invisible,
                ),
                now_us,
            )?;
        }
        let original_slot = self.carrier.slots[index].clone();
        let ordinal = committed.next_ordinal;
        // One attribution successor: lineage and summon-master ticks were recorded
        // above; this pass adds the bound-session fence and Soul War source players.
        let mut damage_contributors = contributors_after;
        // Attribution is prepared with the same HP/store successor. No new
        // input command is consumed by a condition tick; the existing source
        // stream convention seeds only a zero high-water when none exists.
        damage_contributors
            .entries
            .try_reserve(
                COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX
                    .saturating_sub(damage_contributors.entries.len()),
            )
            .map_err(|_| CarrierError::CommitAllocationFailed)?;
        for impact in impacts.iter().filter(|impact| impact.damage > 0) {
            let Some(source) = impact.source else {
                continue;
            };
            let Ok(source_index) = self.carrier.validate_ref(&self.continuity, source.0) else {
                continue;
            };
            let Slot::Occupied {
                game_session_id: Some(session),
                generation,
                ..
            } = &self.carrier.slots[source_index]
            else {
                continue;
            };
            if *generation != source.0.actor_local_generation.0 {
                continue;
            }
            let lease = self.bound_attacker_lease(source, *session)?;
            let previous = damage_contributors.high_water(lease.character_id());
            if previous.is_some_and(|mark| {
                mark.0 > lease.generation() || (mark.0 == lease.generation() && mark.1 != *session)
            }) {
                return Err(CarrierError::SupersededAttackerSession);
            }
            let context = self.carrier.soul_war_health_context(
                &self.continuity,
                Some(source),
                None,
                Some(now_us / 1_000),
                super::soul_war_player_source::SoulWarPlayerSourceOrigin::Condition,
                self.content_pin(),
            )?;
            damage_contributors.record_source_player(
                lease.character_id(),
                impact.damage,
                Some(&context),
                Some(now_us / 1_000),
            );
        }
        let next_ordinal = if had_damage {
            ordinal.checked_add(1).ok_or(
                oteryn_gameplay_domain::domain::carrier_values::CarrierError::CapacityArithmeticOverflow,
            )?
        } else {
            ordinal
        };
        let lethal_committed = if before.health > 0 && health_after == 0 {
            // The retained lethal source is the real store-issued owner input.
            // Nonlethal replay is fenced by complete store+HP comparison; no
            // unsequenced command receipt is consumed for every DOT pulse.
            let occurrence = format!(
                "owner-tick/{}/{}/{}/{}",
                actor.actor_local_id(),
                actor.actor_local_generation(),
                stamp.generation().get(),
                stamp.ordinal().get()
            );
            let mut binding = occurrence.as_bytes().to_vec();
            binding.push(0);
            binding.extend_from_slice(&actor.placement_identity());
            binding.extend_from_slice(&now_us.to_be_bytes());
            for tick in &ticks {
                binding.extend_from_slice(&tick.sequence.to_be_bytes());
                binding.extend_from_slice(&tick.due.to_be_bytes());
            }
            let Slot::CreatureOccupied {
                target_identity, ..
            } = &original_slot
            else {
                return Err(CarrierError::NotCreature);
            };
            let (_, successor, _) = self.carrier.prepare_creature_damage_inner_bounded(
                &self.continuity,
                actor.0,
                OwnerDamageCommand {
                    target: target_identity,
                    occurrence: occurrence.as_bytes(),
                    binding: &binding,
                    damage: total,
                },
                None,
                false,
                None,
                false,
                Some(now_us / 1000),
                true,
            )?;
            let Some(Slot::CreatureOccupied { committed, .. }) = successor else {
                return Err(CarrierError::PlanConflict);
            };
            Some(committed)
        } else {
            None
        };
        let native_death = if matches!(&original_slot, Slot::CreatureOccupied { spell_combat, .. }
            if spell_combat.native_death_events.is_some()
                && !spell_combat.source_death_completed
                && !spell_combat.restored_splinter_death.as_ref().is_some_and(|original|
                    original.current_for(self) && original.projection(before.actor).is_some()))
        {
            // The lethal receipt is the last entry of the prepared committed ledger.
            lethal_committed
                .as_ref()
                .and_then(|committed| committed.entries.last())
                .map(|record| {
                    ChannelActorCarrier::native_death_notice_for_record(
                        before.actor,
                        record,
                        Some(now_us / 1_000),
                        Some(now_us),
                    )
                })
                .transpose()?
        } else {
            None
        };
        Ok(Some(PreparedCreaturePeriodicTurn {
            damage_contributors,
            impacts,
            health_events,
            before,
            original_slot,
            after,
            conditions_after,
            health_after,
            stamp,
            ticks,
            next_ordinal,
            lethal_committed,
            native_death,
        }))
    }
    fn record_periodic_contributor(
        &self,
        contributors: &mut DamageContributors,
        tick: &ConditionTick<ExactActorRef>,
        damage: u64,
        ordinal: u64,
        now_us: u64,
    ) -> Result<(), CarrierError> {
        if damage == 0 {
            return Ok(());
        }
        let identity = if let Some(lineage) = &tick.provenance.lineage {
            if lineage.source_content_digest != self.content_pin().server_artifact_digest()
                || tick.provenance.source.is_none_or(|source| {
                    lineage.source_actor_placement != source.placement_identity()
                        || lineage.source_scope_generation != source.scope_generation().get()
                })
            {
                return Err(CarrierError::PlanConflict);
            }
            Some((
                CharacterId::decode(&lineage.source_character)
                    .map_err(|_| CarrierError::PlanConflict)?,
                lineage.source_lease_generation,
                GameSessionId::decode(&lineage.source_session)
                    .map_err(|_| CarrierError::PlanConflict)?,
            ))
        } else if let Some(source) = tick.provenance.source {
            let master = self
                .companion_snapshot_including_dead(source)
                .ok()
                .and_then(|s| s.state.master);
            let player = master.map(|m| (m.actor, m.session)).or_else(|| {
                if !self.borrow_exact_actor_lookup().contains(source) {
                    return None;
                }
                let index = self.carrier.validate_ref(&self.continuity, source.0).ok()?;
                match &self.carrier.slots[index] {
                    Slot::Occupied {
                        game_session_id: Some(session),
                        ..
                    } => Some((source, *session)),
                    _ => None,
                }
            });
            player.and_then(|(actor, session)| {
                self.bound_attacker_lease(actor, session)
                    .ok()
                    .map(|lease| (lease.character_id(), lease.generation(), session))
            })
        } else {
            None
        };
        if let Some((character, lease, session)) = identity {
            let mark = match contributors.high_water(character) {
                Some(mark) if mark.0 > lease || (mark.0 == lease && mark.1 != session) => None,
                Some(mark) if mark.0 == lease && mark.1 == session => Some(mark),
                _ => Some((lease, session, 0, 0)),
            };
            contributors.record(character, damage, ordinal, mark, Some(now_us / 1000));
        }
        Ok(())
    }

    /// One real slot mutation installs consumed store state and physical HP.
    pub fn commit_creature_periodic_turn(
        &mut self,
        prepared: PreparedCreaturePeriodicTurn,
    ) -> Result<CreaturePeriodicReceipt, CarrierError> {
        if self.actor_spell_reserved(prepared.before.actor)
            || !self.owner_fence()?.accepts_stamp(prepared.stamp)
            || self.companion_snapshot(prepared.before.actor)? != prepared.before
        {
            return Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict);
        }
        if let Some(observation) = &prepared.health_events.native {
            self.validate_native_health_change_observation(observation)?;
        }
        if let Some(master) = prepared.before.state.master {
            self.carrier
                .player_slot_index(&self.continuity, master.actor.0, master.session)?;
        }
        let index = self
            .carrier
            .validate_ref(&self.continuity, prepared.before.actor.0)?;
        if self.carrier.slots.get(index) != Some(&prepared.original_slot) {
            return Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict);
        }
        // Only this turn's own events need capacity: every pending reservation
        // reserved its own when it was staged, and neither store gives capacity
        // back. Both helpers scan every slot, so calling them with nothing to add
        // made a creature periodic pass quadratic in the creature count.
        if !prepared.health_events.blood.is_empty() {
            self.carrier
                .reserve_cloak_blood_capacity(&prepared.health_events.blood)?;
        }
        if prepared.native_death.is_some() {
            self.carrier
                .reserve_native_death_capacity(prepared.native_death.as_slice())?;
        }
        let Slot::CreatureOccupied {
            health,
            companion,
            committed,
            spell_combat,
            target_identity,
            damage_contributors,
            ..
        } = &mut self.carrier.slots[index]
        else {
            return Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::NotCreature);
        };
        // All receipt admission and allocation happened in the retained successor.
        let next = Box::new(prepared.after);
        if let Some(next_committed) = prepared.lethal_committed {
            *committed = next_committed;
        }
        committed.next_ordinal = prepared.next_ordinal;
        *damage_contributors = prepared.damage_contributors;
        *health = prepared.health_after;
        committed.conditions = prepared.conditions_after;
        if prepared.health_after == 0 {
            committed.conditions.die();
        }
        let blood = prepared
            .health_events
            .install(spell_combat, committed, target_identity);
        *companion = Some(next);
        self.carrier.install_cloak_blood_events(blood);
        self.carrier
            .install_native_death_events(prepared.native_death);
        Ok(CreaturePeriodicReceipt {
            impacts: prepared.impacts,
            actor: prepared.before.actor,
            health_before: prepared.before.health,
            health_after: prepared.health_after,
            ticks: prepared.ticks,
        })
    }
}

fn periodic_element_key(
    element: oteryn_gameplay_domain::domain::condition::DotElement,
) -> &'static str {
    use oteryn_gameplay_domain::domain::condition::DotElement;
    match element {
        DotElement::Poison => "earth",
        DotElement::Fire => "fire",
        DotElement::Energy => "energy",
        DotElement::Bleeding => "physical",
        DotElement::Drown => "drowning",
        DotElement::Freezing => "ice",
        DotElement::Dazzled => "holy",
        DotElement::Cursed => "death",
    }
}
/// ORIGIN_CONDITION has neither armor/defense nor critical/fatal/pierce draws.
/// Attributed ticks additionally require the real live source owner; absence
/// of that qualification cannot silently turn a known caster into no attacker.
fn finish_unattributed_condition_tick(
    target: &CompanionSnapshot,
    tick: &ConditionTick<ExactActorRef>,
    amount: u32,
    now_us: u64,
) -> Result<i64, CarrierError> {
    finish_condition_tick_with_received(target, tick, amount, now_us, None)
}
fn finish_condition_tick_with_received(
    target: &CompanionSnapshot,
    tick: &ConditionTick<ExactActorRef>,
    amount: u32,
    now_us: u64,
    received_override: Option<u32>,
) -> Result<i64, CarrierError> {
    use oteryn_gameplay_domain::domain::condition::{ConditionValues, ConflictKey};
    if tick.provenance.source.is_some() || tick.provenance.lineage.is_some() {
        return Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict);
    }
    let TickKind::Damage { element, .. } = tick.kind else {
        return Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict);
    };
    let key = periodic_element_key(element);
    let policy = &target.state.policy;
    if !policy.flags.attackable || policy.damage_immunities.iter().any(|v| v == key) {
        return Ok(0);
    }
    let mut value = i64::from(amount);
    let mut received_percent = 100;
    if let Some(attributes) = target
        .state
        .conditions
        .active_at(ConflictKey::Attributes, now_us)
        && let ConditionValues::Attributes {
            incoming_reduction_percent,
            ..
        } = attributes.definition().values()
    {
        received_percent = 100 - incoming_reduction_percent;
        value = (value as f64 * f64::from(received_percent) / 100.0).trunc() as i64;
    }
    if let Some(current) = received_override {
        if current > 100 {
            return Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict);
        }
        received_percent = current;
        value = (i64::from(amount) as f64 * f64::from(current) / 100.0).trunc() as i64;
    }
    let mitigation = policy
        .mitigation
        .ok_or(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict)?;
    if mitigation.denominator == 0 || mitigation.numerator < 0 {
        return Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict);
    }
    let mitigation =
        ((mitigation.numerator as f64 / mitigation.denominator as f64) as f32).min(30.0);
    // Source int32 * float happens before promotion by the double /100 literal.
    value = (value as f64 - f64::from(value as f32 * mitigation) / 100.0).trunc() as i64;
    if let Some(resistance) = policy.resistances.iter().find(|v| v.damage_type == key) {
        if resistance.percent.denominator == 0 {
            return Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict);
        }
        let sensitivity =
            100.0 - resistance.percent.numerator as f64 / resistance.percent.denominator as f64;
        let result = (value as f64 * sensitivity / 100.0).round();
        if !result.is_finite() || result >= i64::MAX as f64 {
            return Err(
                oteryn_gameplay_domain::domain::carrier_values::CarrierError::DamageOverflow,
            );
        }
        value = (result as i64).max(0);
    }
    // Canary combatChangeHealth applies the received buff again after
    // combatBlockHit; ORIGIN_CONDITION is retained through both calls.
    value = (value as f64 * f64::from(received_percent) / 100.0).trunc() as i64;
    Ok(value.max(0))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::super::runtime_actor_companion::{CreatureExactRatio, CreatureFlags};
    use super::*;
    use oteryn_gameplay_content::content::creature_policy::{
        CompiledCreaturePolicies, CompiledCreaturePolicy,
    };
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    use {
        oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement},
        oteryn_gameplay_foundation::foundation::condition::{
            ApplicationFacts, ConditionDefinition, ConditionSourceKind,
        },
    };
    fn id(n: u8) -> [u8; 16] {
        [n, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, n]
    }
    fn policy(name: &str) -> CompiledCreaturePolicy {
        CompiledCreaturePolicy {
            definition_key: format!("creature:{name}"),
            definition_revision: "test-1".into(),
            display_name: name.into(),
            maximum_health: 812,
            forge_difficulty: None,
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
            blocked_by_nearby_players: false,
            defense: Some(1),
            armor: Some(10),
            mitigation: Some(CreatureExactRatio {
                numerator: 0,
                denominator: 1,
            }),
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
        owner_with(policy("rat"))
    }
    fn owner_with(rat: CompiledCreaturePolicy) -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
        let world = oteryn_gameplay_foundation::foundation::WorldId::decode(&id(1)).unwrap();
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            oteryn_gameplay_foundation::foundation::ChannelId::decode(&id(2)).unwrap(),
            oteryn_gameplay_foundation::foundation::NodeId::decode(&id(3)).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin::test(world),
        )
        .unwrap();
        let session =
            oteryn_gameplay_foundation::foundation::GameSessionId::decode(&id(4)).unwrap();
        let reserved = runtime.reserve_fresh_session(session).unwrap();
        let actor = runtime.commit_fresh_session(reserved).unwrap();
        runtime.initialize_first_entry_position(actor).unwrap();
        runtime
            .install_companion_policies(
                CompiledCreaturePolicies::from_active_artifact([1; 32], vec![rat]).unwrap(),
            )
            .unwrap();
        (runtime, actor, session)
    }

    fn install_dot(
        runtime: &mut ChannelRuntimeV1,
        creature: ExactActorRef,
        amount: u32,
        repetitions: u32,
    ) {
        let before = runtime.companion_snapshot(creature).unwrap();
        let mut after = before.state.clone();
        let definition = ConditionDefinition::new(
            "test.source.dot",
            1,
            ConditionValues::DamageOverTime {
                element: DotElement::Fire,
                total_min: amount * repetitions,
                total_max: amount * repetitions,
                per_tick: amount,
                interval_ms: 1000,
                delayed: true,
            },
        )
        .unwrap();
        let root = GameplayDecisionRoot::from_bytes([7; 32]);
        after
            .conditions
            .apply(
                &definition,
                None,
                ConditionSourceKind::Field,
                &[],
                &ApplicationFacts {
                    now: 0,
                    base_speed: 220,
                    mana_shield_capacity: 0,
                    target_reentry_protected: false,
                    source_reentry_protected: false,
                    target_is_player: false,
                    decision_root: &root,
                    occurrence: DecisionOccurrenceId::from_bytes([5; 16]),
                },
            )
            .unwrap();
        runtime.compare_companion_state(&before, after).unwrap();
    }
    fn retired_dot_source(reused: bool) {
        let (mut runtime, source, session) = owner();
        let character = CharacterId::decode(&id(9)).unwrap();
        runtime
            .bind_attacker_lease(source, session, CharacterLease::new(character, 1).unwrap())
            .unwrap();
        let creature = runtime
            .admit_pinned_monster_lab_creature(
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
                "creature:rat",
                812,
            )
            .unwrap();
        runtime
            .install_creature_policy(creature, "creature:rat")
            .unwrap();
        let before = runtime.companion_snapshot(creature).unwrap();
        let mut after = before.state.clone();
        let definition = ConditionDefinition::new(
            "test.retired-source.dot",
            1,
            ConditionValues::DamageOverTime {
                element: DotElement::Fire,
                total_min: 10,
                total_max: 10,
                per_tick: 10,
                interval_ms: 1000,
                delayed: true,
            },
        )
        .unwrap();
        let root = GameplayDecisionRoot::from_bytes([7; 32]);
        after
            .conditions
            .apply(
                &definition,
                Some(source),
                ConditionSourceKind::Field,
                &[],
                &ApplicationFacts {
                    now: 0,
                    base_speed: 220,
                    mana_shield_capacity: 0,
                    target_reentry_protected: false,
                    source_reentry_protected: false,
                    target_is_player: false,
                    decision_root: &root,
                    occurrence: DecisionOccurrenceId::from_bytes([5; 16]),
                },
            )
            .unwrap();
        runtime.compare_companion_state(&before, after).unwrap();
        runtime.remove_test_actor(source).unwrap();
        if reused {
            let session = GameSessionId::decode(&id(10)).unwrap();
            let reservation = runtime.reserve_fresh_session(session).unwrap();
            let replacement = runtime.commit_fresh_session(reservation).unwrap();
            runtime
                .initialize_first_entry_position(replacement)
                .unwrap();
            runtime
                .bind_attacker_lease(
                    replacement,
                    session,
                    CharacterLease::new(CharacterId::decode(&id(11)).unwrap(), 1).unwrap(),
                )
                .unwrap();
            assert_eq!(replacement.0.actor_local_id, source.0.actor_local_id);
            assert_ne!(replacement, source);
            let context = runtime
                .carrier
                .soul_war_health_context(
                    &runtime.continuity,
                    Some(source),
                    None,
                    Some(1_000),
                    super::super::soul_war_player_source::SoulWarPlayerSourceOrigin::Condition,
                    runtime.content_pin(),
                )
                .unwrap();
            assert!(matches!(
                context.kind,
                super::super::soul_war_health::HealthSourceKind::Null
            ));
            assert!(context.actor.is_none());
        }
        let prepared = runtime
            .stage_creature_periodic_turn(creature, 1_000_000, TickFacts::default())
            .unwrap();
        let receipt = runtime.commit_creature_periodic_turn(prepared).unwrap();
        assert_eq!(receipt.health_before - receipt.health_after, 10);
        assert_eq!(receipt.impacts.len(), 1);
    }
    #[test]
    fn review9_dot_survives_reused_source_slot() {
        retired_dot_source(true);
    }

    #[test]
    fn periodic_damage_dispels_native_and_companion_invisibility_for_dot_and_agony() {
        use super::super::runtime_actor_conditions::{ActorConditionTransition, ConditionSource};
        use oteryn_gameplay_domain::domain::condition::ConditionType;
        use oteryn_simulation_determinism::SemanticTimeMicros;
        for agony in [false, true] {
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
            let root = GameplayDecisionRoot::from_bytes([7; 32]);
            let invisible = ConditionDefinition::new(
                "test.invisible",
                1,
                ConditionValues::Invisible {
                    duration_ms: 10_000,
                },
            )
            .unwrap();
            let facts = ApplicationFacts {
                now: 0,
                base_speed: 220,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: false,
                decision_root: &root,
                occurrence: DecisionOccurrenceId::from_bytes([6; 16]),
            };
            let plan = runtime
                .prepare_actor_condition(
                    creature,
                    None,
                    ActorConditionTransition::Apply {
                        definition: &invisible,
                        source: ConditionSource {
                            actor: creature,
                            session: None,
                            kind: ConditionSourceKind::SelfUse,
                        },
                        immunities: &[],
                        facts,
                    },
                    SemanticTimeMicros::from_micros(0),
                )
                .unwrap();
            runtime
                .commit_actor_condition(&plan, SemanticTimeMicros::from_micros(0))
                .unwrap();
            let before = runtime.companion_snapshot(creature).unwrap();
            let mut after = before.state.clone();
            after.conditions = runtime.actor_conditions(creature, None).unwrap().clone();
            let values = if agony {
                ConditionValues::Agony {
                    total_min: 100,
                    total_max: 100,
                    per_tick: 100,
                    interval_ms: 1000,
                    delayed: true,
                }
            } else {
                ConditionValues::DamageOverTime {
                    element: DotElement::Fire,
                    total_min: 100,
                    total_max: 100,
                    per_tick: 100,
                    interval_ms: 1000,
                    delayed: true,
                }
            };
            let damage = ConditionDefinition::new("test.damage", 1, values).unwrap();
            after
                .conditions
                .apply(&damage, None, ConditionSourceKind::Field, &[], &facts)
                .unwrap();
            runtime.compare_companion_state(&before, after).unwrap();
            let prepared = runtime
                .stage_creature_periodic_turn(creature, 1_000_000, TickFacts::default())
                .unwrap();
            assert!(
                runtime
                    .actor_conditions(creature, None)
                    .unwrap()
                    .active_at(ConditionType::Invisible.conflict_key(), 1_000_000)
                    .is_some()
            );
            let receipt = runtime.commit_creature_periodic_turn(prepared).unwrap();
            assert!(receipt.health_after < receipt.health_before);
            assert!(
                runtime
                    .actor_conditions(creature, None)
                    .unwrap()
                    .active_at(ConditionType::Invisible.conflict_key(), 1_000_000)
                    .is_none()
            );
            assert!(
                runtime
                    .companion_snapshot(creature)
                    .unwrap()
                    .state
                    .conditions
                    .active_at(ConditionType::Invisible.conflict_key(), 1_000_000)
                    .is_none()
            );
        }
    }

    #[test]
    fn a_periodic_hit_reveals_a_creature_with_timed_status_invisibility() {
        use super::super::runtime_actor_conditions::{ActorConditionTransition, ConditionSource};
        use oteryn_gameplay_domain::domain::condition::StatusKind;
        use oteryn_simulation_determinism::SemanticTimeMicros;
        let (mut runtime, player, session) = owner();
        let creature = runtime
            .create_companion(
                player,
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
        let invisible = ConditionDefinition::new(
            "test.timed-invisible",
            1,
            ConditionValues::TimedStatus {
                kind: StatusKind::Invisible,
                duration_ms: 10_000,
            },
        )
        .unwrap();
        let now = SemanticTimeMicros::from_micros(0);
        let plan = runtime
            .prepare_actor_condition(
                creature,
                None,
                ActorConditionTransition::Apply {
                    definition: &invisible,
                    source: ConditionSource {
                        actor: creature,
                        session: None,
                        kind: ConditionSourceKind::SelfUse,
                    },
                    immunities: &[],
                    facts: ApplicationFacts {
                        now: 0,
                        base_speed: 220,
                        mana_shield_capacity: 0,
                        target_reentry_protected: false,
                        source_reentry_protected: false,
                        target_is_player: false,
                        decision_root: &GameplayDecisionRoot::from_bytes([7; 32]),
                        occurrence: DecisionOccurrenceId::from_bytes([6; 16]),
                    },
                },
                now,
            )
            .unwrap();
        runtime.commit_actor_condition(&plan, now).unwrap();
        let before = runtime.companion_snapshot(creature).unwrap();
        let mut after = before.state.clone();
        after.conditions = runtime.actor_conditions(creature, None).unwrap().clone();
        runtime.compare_companion_state(&before, after).unwrap();
        install_dot(&mut runtime, creature, 100, 1);
        let at = 1_000_000;
        assert!(
            runtime
                .actor_conditions(creature, None)
                .unwrap()
                .invisible_at(at)
        );
        let prepared = runtime
            .stage_creature_periodic_turn(creature, at, TickFacts::default())
            .unwrap();
        let receipt = runtime.commit_creature_periodic_turn(prepared).unwrap();
        assert!(receipt.health_after < receipt.health_before);
        assert!(
            !runtime
                .actor_conditions(creature, None)
                .unwrap()
                .invisible_at(at)
        );
        assert!(
            !runtime
                .companion_snapshot(creature)
                .unwrap()
                .state
                .conditions
                .invisible_at(at)
        );
    }

    #[test]
    fn enrage_periodic_damage_matches_receipt_and_preserves_zero_damage_state() {
        use super::super::soulpit::{SoulpitBossState, SoulpitCreatureTag};
        use oteryn_gameplay_domain::domain::soulpit::{BossAbility, SoulpitStack};
        use oteryn_simulation_determinism::SemanticTimeMicros;
        for (agony, damage) in [
            (false, 1),
            (false, 10),
            (true, 10),
            (false, 1000),
            (true, 1000),
        ] {
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
            let index = runtime
                .carrier
                .validate_ref(&runtime.continuity, creature.0)
                .unwrap();
            let Slot::CreatureOccupied {
                health,
                spell_combat,
                ..
            } = &mut runtime.carrier.slots[index]
            else {
                unreachable!()
            };
            *health = 30;
            spell_combat.maximum_health = 147;
            spell_combat.soulpit = Some(SoulpitCreatureTag {
                handoff_id: id(7),
                wave: 4,
                monster_id: 7,
                stack: SoulpitStack::Boss,
                is_summon: false,
                boss: Some(SoulpitBossState {
                    ability: BossAbility::Enrage,
                    next_oppressor_at_micros: 0,
                    ordinal: 0,
                }),
            });
            if agony {
                let before = runtime.companion_snapshot(creature).unwrap();
                let mut after = before.state.clone();
                let definition = ConditionDefinition::new(
                    "test.agony",
                    1,
                    ConditionValues::Agony {
                        total_min: damage,
                        total_max: damage,
                        per_tick: damage,
                        interval_ms: 1000,
                        delayed: true,
                    },
                )
                .unwrap();
                after
                    .conditions
                    .apply(
                        &definition,
                        None,
                        ConditionSourceKind::Field,
                        &[],
                        &ApplicationFacts {
                            now: 0,
                            base_speed: 220,
                            mana_shield_capacity: 0,
                            target_reentry_protected: false,
                            source_reentry_protected: false,
                            target_is_player: false,
                            decision_root: &GameplayDecisionRoot::from_bytes([7; 32]),
                            occurrence: DecisionOccurrenceId::from_bytes([5; 16]),
                        },
                    )
                    .unwrap();
                runtime.compare_companion_state(&before, after).unwrap();
            } else {
                install_dot(&mut runtime, creature, damage, 1);
            }
            let ordinal_before = if damage == 1 {
                let Slot::CreatureOccupied {
                    health, committed, ..
                } = &mut runtime.carrier.slots[index]
                else {
                    unreachable!()
                };
                *health = 73;
                let ordinal = committed.next_ordinal;
                let invisible = ConditionDefinition::new(
                    "test.enrage.invisible",
                    1,
                    ConditionValues::Invisible { duration_ms: 3000 },
                )
                .unwrap();
                let now = SemanticTimeMicros::from_micros(0);
                let plan = runtime
                    .prepare_actor_condition(
                        creature,
                        None,
                        ActorConditionTransition::Apply {
                            definition: &invisible,
                            source: ConditionSource {
                                actor: creature,
                                session: None,
                                kind: ConditionSourceKind::SelfUse,
                            },
                            immunities: &[],
                            facts: ApplicationFacts {
                                now: 0,
                                base_speed: 220,
                                mana_shield_capacity: 0,
                                target_reentry_protected: false,
                                source_reentry_protected: false,
                                target_is_player: false,
                                decision_root: &GameplayDecisionRoot::from_bytes([7; 32]),
                                occurrence: DecisionOccurrenceId::from_bytes([6; 16]),
                            },
                        },
                        now,
                    )
                    .unwrap();
                runtime.commit_actor_condition(&plan, now).unwrap();
                assert!(
                    runtime
                        .actor_conditions(creature, None)
                        .unwrap()
                        .invisible_at(1_000_000)
                );
                Some(ordinal)
            } else {
                None
            };
            let turn = runtime
                .stage_creature_periodic_turn(creature, 1_000_000, TickFacts::default())
                .unwrap();
            let receipt = runtime.commit_creature_periodic_turn(turn).unwrap();
            if let Some(ordinal) = ordinal_before {
                assert_eq!(receipt.health_after, 73);
                assert_eq!(receipt.impacts.len(), 1);
                assert_eq!(receipt.impacts[0].damage, 0);
                assert!(
                    runtime
                        .actor_conditions(creature, None)
                        .unwrap()
                        .invisible_at(1_000_000)
                );
                let Slot::CreatureOccupied { committed, .. } = &runtime.carrier.slots[index] else {
                    unreachable!()
                };
                assert_eq!(committed.next_ordinal, ordinal);
                continue;
            }
            if damage == 10 {
                assert_eq!(receipt.health_after, 24, "Enrage resolves each tick once");
                assert_eq!(receipt.impacts.iter().map(|i| i.damage).sum::<u64>(), 6);
                continue;
            }
            assert_eq!(receipt.health_after, 0);
            assert_eq!(receipt.impacts.iter().map(|i| i.damage).sum::<u64>(), 30);
            let mut death = runtime.borrow_combat_death();
            let lethal = death.committed_lethal_receipt(creature).unwrap();
            death.project_committed_lethal(lethal).unwrap();
        }
    }

    #[test]
    fn stationary_field_control_changes_do_not_reapply_or_reset_ticks() {
        use oteryn_gameplay_foundation::foundation::condition::ConditionLineage;
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
        let mut field = CreatureFieldEntry {
            definition: ConditionDefinition::new(
                "test.field",
                1,
                ConditionValues::DamageOverTime {
                    element: DotElement::Fire,
                    total_min: 10,
                    total_max: 10,
                    per_tick: 1,
                    interval_ms: 1000,
                    delayed: false,
                },
            )
            .unwrap(),
            source: Some(owner),
            lineage: Some(ConditionLineage {
                source_character: id(9),
                source_session: id(4),
                source_lease_generation: 1,
                source_actor_placement: owner.placement_identity(),
                source_scope_generation: runtime.binding().scope_generation().get(),
                source_content_digest: runtime.content_pin().server_artifact_digest(),
                source_catalog_digest: [1; 32],
                source_creation_command: 1,
            }),
            occurrence: DecisionOccurrenceId::from_bytes([1; 16]),
        };
        runtime
            .commit_creature_field_entry(creature, 0, Some(&field))
            .unwrap();
        let original = runtime.companion_snapshot(creature).unwrap();
        let source = field.source.take();
        let lineage = field.lineage.take();
        runtime
            .commit_creature_field_entry(creature, 100, Some(&field))
            .unwrap();
        assert_eq!(runtime.companion_snapshot(creature).unwrap(), original);
        field.source = source;
        field.lineage = lineage;
        runtime
            .commit_creature_field_entry(creature, 200, Some(&field))
            .unwrap();
        assert_eq!(runtime.companion_snapshot(creature).unwrap(), original);
    }

    #[test]
    fn stationary_creature_admits_new_and_replaced_fields_once_per_field_identity() {
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
        runtime
            .commit_creature_field_entry(creature, 0, None)
            .unwrap();
        for identity in 1..=2 {
            let definition = ConditionDefinition::new(
                &format!("test.field.{identity}"),
                1,
                ConditionValues::DamageOverTime {
                    element: DotElement::Fire,
                    total_min: 10,
                    total_max: 10,
                    per_tick: 1,
                    interval_ms: 1000,
                    delayed: false,
                },
            )
            .unwrap();
            let field = CreatureFieldEntry {
                definition: definition.clone(),
                source: None,
                lineage: None,
                occurrence: DecisionOccurrenceId::from_bytes([identity; 16]),
            };
            runtime
                .commit_creature_field_entry(creature, u64::from(identity), Some(&field))
                .unwrap();
            let current = runtime.companion_snapshot(creature).unwrap();
            assert!(
                current
                    .state
                    .conditions
                    .instances()
                    .iter()
                    .any(|i| i.definition() == &definition)
            );
            runtime
                .commit_creature_field_entry(creature, u64::from(identity) + 1, Some(&field))
                .unwrap();
            assert_eq!(runtime.companion_snapshot(creature).unwrap(), current);
        }
    }

    #[test]
    fn lethal_periodic_tick_evicts_safe_history_and_preserves_original_candidate() {
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
        let character = CharacterId::decode(&id(9)).unwrap();
        runtime
            .bind_attacker_lease(owner, session, CharacterLease::new(character, 1).unwrap())
            .unwrap();
        for sequence in 1..=16 {
            runtime
                .commit_source_damage(
                    creature,
                    owner,
                    session,
                    b"test.history",
                    sequence,
                    1,
                    sequence,
                )
                .unwrap();
        }
        install_dot(&mut runtime, creature, 1000, 1);
        let before = runtime.companion_snapshot(creature).unwrap();
        let prepared = runtime
            .stage_creature_periodic_turn(creature, 1_000_000, TickFacts::default())
            .unwrap();
        assert_eq!(runtime.companion_snapshot(creature).unwrap(), before);
        let receipt = runtime.commit_creature_periodic_turn(prepared).unwrap();
        assert_eq!(receipt.health_after, 0);
        let mut owner = runtime.borrow_combat_death();
        let lethal = owner.committed_lethal_receipt(creature).unwrap();
        owner.project_committed_lethal(lethal).unwrap();
    }

    #[test]
    fn player_periodic_dot_and_agony_damage_updates_reward_and_current_session_shares_once() {
        for agony in [false, true] {
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
            let character = CharacterId::decode(&id(9)).unwrap();
            runtime
                .bind_attacker_lease(owner, session, CharacterLease::new(character, 1).unwrap())
                .unwrap();
            let before = runtime.companion_snapshot(creature).unwrap();
            let mut after = before.state.clone();
            let values = if agony {
                ConditionValues::Agony {
                    total_min: 100,
                    total_max: 100,
                    per_tick: 100,
                    interval_ms: 1000,
                    delayed: true,
                }
            } else {
                ConditionValues::DamageOverTime {
                    element: DotElement::Fire,
                    total_min: 100,
                    total_max: 100,
                    per_tick: 100,
                    interval_ms: 1000,
                    delayed: true,
                }
            };
            let definition = ConditionDefinition::new("test.player.dot", 1, values).unwrap();
            let root = GameplayDecisionRoot::from_bytes([7; 32]);
            after
                .conditions
                .apply(
                    &definition,
                    Some(owner),
                    ConditionSourceKind::Player,
                    &[],
                    &ApplicationFacts {
                        now: 0,
                        base_speed: 220,
                        mana_shield_capacity: 0,
                        target_reentry_protected: false,
                        source_reentry_protected: false,
                        target_is_player: false,
                        decision_root: &root,
                        occurrence: DecisionOccurrenceId::from_bytes([7; 16]),
                    },
                )
                .unwrap();
            runtime.compare_companion_state(&before, after).unwrap();
            let mut source = |r: &ChannelRuntimeV1, actor: ExactActorRef, _: u64| {
                r.player_control_facts(actor, session)?;
                r.bound_attacker_lease(actor, session)?;
                Ok(Some(100))
            };
            let receipt = runtime
                .apply_creature_periodic_turn_with_sources(
                    creature,
                    1_000_000,
                    TickFacts::default(),
                    &mut source,
                )
                .unwrap();
            let damage = receipt.impacts.iter().map(|i| i.damage).sum::<u64>();
            assert!(damage > 0);
            runtime
                .apply_creature_periodic_turn_with_sources(
                    creature,
                    1_000_000,
                    TickFacts::default(),
                    &mut source,
                )
                .unwrap();
            let view = runtime.borrow_combat_death();
            let shares = view.reward_damage_shares(creature).unwrap();
            assert_eq!(shares.len(), 1);
            assert_eq!(shares[0].character, character);
            assert_eq!(shares[0].total, damage);
            let current = view.current_session_damage_shares(creature).unwrap();
            assert_eq!(current.len(), 1);
            assert_eq!(current[0].total, damage);
        }
    }

    #[test]
    fn lethal_periodic_projection_retains_original_owner_timestamp() {
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
        install_dot(&mut runtime, creature, 1000, 1);
        let prepared = runtime
            .stage_creature_periodic_turn(creature, 1_234_567, TickFacts::default())
            .unwrap();
        runtime.commit_creature_periodic_turn(prepared).unwrap();
        let mut view = runtime.borrow_combat_death();
        let lethal = view.committed_lethal_receipt(creature).unwrap();
        view.project_committed_lethal(lethal).unwrap();
        assert_eq!(view.projected_death_at_ms(creature).unwrap(), Some(1234));
    }

    #[test]
    fn agony_on_a_creature_ticks_raw_damage_past_mitigation_and_resistance() {
        let mut tough = policy("rat");
        tough.mitigation = Some(CreatureExactRatio {
            numerator: 30,
            denominator: 1,
        });
        let (mut runtime, owner, session) = owner_with(tough);
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
        let mut after = before.state.clone();
        let definition = ConditionDefinition::new(
            "test.source.agony",
            1,
            ConditionValues::Agony {
                total_min: 300,
                total_max: 300,
                per_tick: 100,
                interval_ms: 1000,
                delayed: true,
            },
        )
        .unwrap();
        let root = GameplayDecisionRoot::from_bytes([7; 32]);
        after
            .conditions
            .apply(
                &definition,
                None,
                ConditionSourceKind::Field,
                &[],
                &ApplicationFacts {
                    now: 0,
                    base_speed: 220,
                    mana_shield_capacity: 0,
                    target_reentry_protected: false,
                    source_reentry_protected: false,
                    target_is_player: false,
                    decision_root: &root,
                    occurrence: DecisionOccurrenceId::from_bytes([5; 16]),
                },
            )
            .unwrap();
        runtime.compare_companion_state(&before, after).unwrap();
        for second in 1..=3 {
            let prepared = runtime
                .stage_creature_periodic_turn(creature, second * 1_000_000, TickFacts::default())
                .unwrap();
            let result = runtime.commit_creature_periodic_turn(prepared).unwrap();
            assert_eq!(result.health_before - result.health_after, 100);
        }
        let current = runtime.companion_snapshot(creature).unwrap();
        assert_eq!(current.health, 512);
        assert!(current.state.conditions.instances().is_empty());
    }
    #[test]
    fn physical_dot_more_than_sixteen_pulses_uses_actual_store_and_refuses_stale_tick_seal() {
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
        install_dot(&mut runtime, creature, 1, 20);
        let first = runtime
            .stage_creature_periodic_turn(creature, 1_000_000, TickFacts::default())
            .unwrap();
        let stale = runtime
            .stage_creature_periodic_turn(creature, 1_000_000, TickFacts::default())
            .unwrap();
        let result = runtime.commit_creature_periodic_turn(first).unwrap();
        assert_eq!((result.health_before, result.health_after), (812, 811));
        assert_eq!(result.ticks.len(), 1);
        assert!(runtime.commit_creature_periodic_turn(stale).is_err());
        for second in 2..=20 {
            let prepared = runtime
                .stage_creature_periodic_turn(creature, second * 1_000_000, TickFacts::default())
                .unwrap();
            runtime.commit_creature_periodic_turn(prepared).unwrap();
        }
        let current = runtime.companion_snapshot(creature).unwrap();
        assert_eq!(current.health, 792);
        assert!(current.state.conditions.instances().is_empty());
    }
    #[test]
    fn actual_periodic_lethal_tick_retains_genuine_corpse_receipt_without_wire_command() {
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
        install_dot(&mut runtime, creature, 1000, 1);
        let prepared = runtime
            .stage_creature_periodic_turn(creature, 1_000_000, TickFacts::default())
            .unwrap();
        let receipt = runtime.commit_creature_periodic_turn(prepared).unwrap();
        assert_eq!(receipt.health_after, 0);
        assert!(
            runtime
                .borrow_combat_death()
                .committed_lethal_receipt(creature)
                .is_ok()
        );
        assert!(
            runtime
                .stage_creature_periodic_turn(creature, 2_000_000, TickFacts::default())
                .is_err()
        );
    }
    #[test]
    fn periodic_source_policy_finisher_uses_mitigation_resistance_and_refuses_unknown() {
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
        install_dot(&mut runtime, creature, 100, 1);
        let mut target = runtime.companion_snapshot(creature).unwrap();
        let tick = target
            .state
            .conditions
            .take_due(1_000_000, TickFacts::default())
            .remove(0);
        let mut policy = (*target.state.policy).clone();
        policy.mitigation = Some(CreatureExactRatio {
            numerator: 10,
            denominator: 1,
        });
        policy.resistances = vec![super::super::runtime_actor_companion::CreatureResistance {
            damage_type: "fire".into(),
            percent: CreatureExactRatio {
                numerator: 50,
                denominator: 1,
            },
        }];
        target.state.policy = std::sync::Arc::new(policy.clone());
        assert_eq!(
            finish_unattributed_condition_tick(&target, &tick, 100, 1_000_000).unwrap(),
            45
        );
        policy.mitigation = None;
        target.state.policy = std::sync::Arc::new(policy.clone());
        assert!(finish_unattributed_condition_tick(&target, &tick, 100, 1_000_000).is_err());
        policy.healing_from_damage =
            vec![super::super::runtime_actor_companion::CreatureResistance {
                damage_type: "fire".into(),
                percent: CreatureExactRatio {
                    numerator: 300,
                    denominator: 1,
                },
            }];
        policy.mitigation = Some(CreatureExactRatio {
            numerator: 10,
            denominator: 1,
        });
        target.state.policy = std::sync::Arc::new(policy.clone());
        assert_eq!(
            finish_unattributed_condition_tick(&target, &tick, 100, 1_000_000).unwrap(),
            45,
            "actual source-free condition damage ignores healingMap without attacker"
        );
        policy.damage_immunities.push("fire".into());
        target.state.policy = std::sync::Arc::new(policy);
        assert_eq!(
            finish_unattributed_condition_tick(&target, &tick, 100, 1_000_000).unwrap(),
            0
        );
        let mut known_source = tick;
        known_source.provenance.source = Some(owner);
        assert!(
            finish_unattributed_condition_tick(&target, &known_source, 100, 1_000_000).is_err()
        );
    }
}
