//! Atomic spell mutations of cloned physical carrier slots. No independent HP engine.
//! Player vitals/conditions remain in their existing owner; its sealed preflight is joined by
//! the gameplay composition while both owners are locked, before any physical replacement.

use super::runtime_actor_spell_types::{
    AvatarState, CombatBatchReceipt, EffectReceipt, Error, MAX_BINDING_BYTES, MAX_COMMAND_RECEIPTS,
    MAX_EFFECTS, MonsterAiState, OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect,
    PlayerBatchProof,
};
use super::*;
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Data in the actual actor slot, never a parallel reservation/health map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OwnerSpellReservation {
    caster: ExactActorRef,
    command: CommandRef,
    phase: u16,
    canonical: Arc<[u8]>,
    destination: Option<(ExactActorRef, MovementLocalPosition)>,
    deferred: Option<super::runtime_actor_spell_types::DeferredCommitAuthority>,
}
impl OwnerSpellReservation {
    pub(super) fn destination(&self) -> Option<(ExactActorRef, MovementLocalPosition)> {
        self.destination
    }
    fn matches(&self, batch: &OwnerCombatBatch, binding: &[u8]) -> bool {
        self.caster == batch.caster
            && self.command == batch.command
            && self.phase == batch.deferred.as_ref().map_or(0, |p| p.phase_ordinal())
            && self.canonical.as_ref() == binding
            && self.deferred == batch.deferred
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BatchRecord {
    character: CharacterId,
    lease: u64,
    command: CommandRef,
    phase: u16,
    original_batch: OwnerCombatBatch,
    receipt: CombatBatchReceipt,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ActorCombatState {
    /// Actual same-session recovery window, set only after committed reentry.
    pub(super) reentry: Option<(u64, u64)>,
    pub(super) pending_owner: Option<Arc<OwnerSpellReservation>>,
    pub(super) pending_source:
        Option<Arc<super::runtime_actor_source_reservation::SourceReservationIdentity>>,
    pub(super) party: super::runtime_actor_party::PartyActorState,
    /// Only creature slots use this field; player maxima belong to PlayerSpellState.
    pub(crate) maximum_health: i64,
    pub(crate) monster_ai: Option<MonsterAiState>,
    pub(crate) paralysed: bool,
    records: Vec<BatchRecord>,
    high_water: Option<(CharacterId, u64, GameSessionId, u64, u16)>,
    deferred_high_water: u64,
}

impl ActorCombatState {
    pub(crate) fn creature(maximum_health: i64) -> Self {
        Self {
            maximum_health,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StagedSpellBatch {
    reservation: Option<Arc<OwnerSpellReservation>>,
    batch: OwnerCombatBatch,
    originals: Vec<(usize, Slot)>,
    replacements: Vec<(usize, Slot)>,
    companion_touch_indices: Vec<usize>,
    relocation: Option<QualifiedSpellRelocation>,
    player_effects: Vec<OwnerCombatEffect>,
    player_preflight_bound: bool,
    receipt: CombatBatchReceipt,
}

/// Full source relocation decision captured only from the world owner's sealed proof.
#[derive(Debug, Clone)]
pub(crate) struct QualifiedSpellRelocation {
    actor: ExactActorRef,
    expected: MovementPositionSnapshot,
    destination: MovementLocalPosition,
    binding: String,
}
impl QualifiedSpellRelocation {
    pub(crate) fn bind<P: SpellRelocationProof>(
        batch: &mut OwnerCombatBatch,
        proof: P,
    ) -> Result<Self, Error> {
        let (actor, expected, destination) = proof.parts();
        if actor.0 != expected.0.actor_ref {
            return Err(Error::InvalidBatch);
        }
        let binding = format!("source-relocation-r21:{actor:?}:{expected:?}:{destination:?}");
        let mut value: serde_json::Value =
            serde_json::from_slice(&batch.binding).map_err(|_| Error::InvalidBatch)?;
        let object = value.as_object_mut().ok_or(Error::InvalidBatch)?;
        if object.contains_key("source_relocation") {
            return Err(Error::InvalidBatch);
        }
        object.insert(
            "source_relocation".into(),
            serde_json::Value::String(binding.clone()),
        );
        let bytes = serde_json::to_vec(&value).map_err(|_| Error::InvalidBatch)?;
        if bytes.len() > MAX_BINDING_BYTES {
            return Err(Error::BindingTooLarge);
        }
        batch.binding = bytes;
        Ok(Self {
            actor,
            expected,
            destination,
            binding,
        })
    }
}

/// Explicit source owner touches carry no damage/heal. Their full immutable snapshots
/// are part of the original command identity; actual slots are compared independently.
#[derive(Debug, Clone)]
pub(crate) struct QualifiedCompanionTouches {
    snapshots: Vec<super::runtime_actor_companion::CompanionSnapshot>,
    binding: String,
}
impl QualifiedCompanionTouches {
    pub(crate) fn bind(
        batch: &mut OwnerCombatBatch,
        snapshots: &[super::runtime_actor_companion::CompanionSnapshot],
    ) -> Result<Self, Error> {
        if snapshots.len() > MAX_EFFECTS {
            return Err(Error::TooManyEffects);
        }
        for (index, snapshot) in snapshots.iter().enumerate() {
            if snapshots[..index]
                .iter()
                .any(|prior| prior.actor == snapshot.actor)
            {
                return Err(Error::InvalidBatch);
            }
        }
        let binding = format!("companion-owner-touches-r21:{snapshots:?}");
        let mut value: serde_json::Value =
            serde_json::from_slice(&batch.binding).map_err(|_| Error::InvalidBatch)?;
        let object = value.as_object_mut().ok_or(Error::InvalidBatch)?;
        if object.contains_key("source_companion_touches") {
            return Err(Error::InvalidBatch);
        }
        object.insert(
            "source_companion_touches".into(),
            serde_json::Value::String(binding.clone()),
        );
        let next = serde_json::to_vec(&value).map_err(|_| Error::InvalidBatch)?;
        if next.len() > MAX_BINDING_BYTES {
            return Err(Error::BindingTooLarge);
        }
        batch.binding = next;
        Ok(Self {
            snapshots: snapshots.to_vec(),
            binding,
        })
    }
}

impl StagedSpellBatch {
    pub(crate) fn batch(&self) -> &OwnerCombatBatch {
        &self.batch
    }
    pub(crate) fn player_effects(&self) -> &[OwnerCombatEffect] {
        &self.player_effects
    }
    pub(crate) fn will_apply(&self) -> bool {
        self.receipt.applied
    }

    /// Binding evidence alone is not current authority. The real composition must first call
    /// preflight.validate_current(runtime, states) under the same held owner locks, and install
    /// its already-staged states infallibly only after an applied physical receipt.
    pub(crate) fn bind_player_preflight(
        &mut self,
        preflight: &impl PlayerBatchProof,
    ) -> Result<(), Error> {
        if !preflight.matches_batch(&self.batch) {
            return Err(Error::CommandConflict);
        }
        self.player_preflight_bound = true;
        Ok(())
    }
}

fn state(slot: &Slot) -> Result<&ActorCombatState, Error> {
    match slot {
        Slot::Occupied { spell_combat, .. } | Slot::CreatureOccupied { spell_combat, .. } => {
            Ok(spell_combat)
        }
        _ => Err(Error::Owner(CarrierError::StaleActorGeneration)),
    }
}
fn state_mut(slot: &mut Slot) -> Result<&mut ActorCombatState, Error> {
    match slot {
        Slot::Occupied { spell_combat, .. } | Slot::CreatureOccupied { spell_combat, .. } => {
            Ok(spell_combat)
        }
        _ => Err(Error::Owner(CarrierError::StaleActorGeneration)),
    }
}

struct Binding(Vec<u8>);
impl Binding {
    fn bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if self
            .0
            .len()
            .checked_add(bytes.len())
            .is_none_or(|n| n > MAX_BINDING_BYTES)
        {
            return Err(Error::BindingTooLarge);
        }
        self.0
            .try_reserve(bytes.len())
            .map_err(|_| Error::AllocationFailed)?;
        self.0.extend_from_slice(bytes);
        Ok(())
    }
    fn number(&mut self, value: u64) -> Result<(), Error> {
        self.bytes(&value.to_be_bytes())
    }
    fn atom(&mut self, value: &[u8]) -> Result<(), Error> {
        self.number(u64::try_from(value.len()).map_err(|_| Error::BindingTooLarge)?)?;
        self.bytes(value)
    }
    fn actor(&mut self, actor: ExactActorRef) -> Result<(), Error> {
        let a = actor.0;
        self.bytes(a.world_id.as_bytes())?;
        self.bytes(a.channel_id.as_bytes())?;
        self.number(a.scope_generation.get())?;
        self.number(u64::from(a.actor_local_id.0))?;
        self.number(a.actor_local_generation.0)
    }
}

fn canonical_binding(batch: &OwnerCombatBatch) -> Result<Vec<u8>, Error> {
    let mut out = Binding(Vec::new());
    out.atom(b"oteryn:physical-spell-batch:candidate-v1")?;
    out.actor(batch.caster)?;
    out.bytes(batch.attacker.as_bytes())?;
    out.number(batch.current_lease_generation)?;
    out.bytes(batch.command.game_session_id().as_bytes())?;
    out.number(batch.command.command_id().get())?;
    out.atom(batch.occurrence.id.as_bytes())?;
    for revision in &batch.occurrence.revisions {
        out.atom(revision.as_bytes())?;
    }
    out.atom(&batch.binding)?;
    out.number(batch.now_ms)?;
    if let Some(anchor) = &batch.anchor {
        out.number(1)?;
        out.number(anchor.expected_revision)?;
        out.number(anchor.next_revision)?;
        out.number(u64::from(anchor.paid_mana))?;
        out.number(u64::from(anchor.paid_soul))?;
        out.number(anchor.cooldown_deadlines.len() as u64)?;
        for (key, deadline) in &anchor.cooldown_deadlines {
            out.atom(key.as_bytes())?;
            out.number(*deadline)?;
        }
    } else {
        out.number(0)?;
    }
    out.number(batch.effects.len() as u64)?;
    for effect in &batch.effects {
        out.actor(effect.target)?;
        out.number(u64::from(effect.sub_ordinal))?;
        match &effect.change {
            OwnerCombatChange::Damage {
                target_atom,
                magnitude,
            }
            | OwnerCombatChange::Heal {
                target_atom,
                magnitude,
            } => {
                out.number(
                    if matches!(effect.change, OwnerCombatChange::Damage { .. }) {
                        1
                    } else {
                        2
                    },
                )?;
                out.atom(target_atom.as_bytes())?;
                out.bytes(&magnitude.to_be_bytes())?;
            }
            OwnerCombatChange::Avatar(AvatarState {
                expires_ms,
                outfit_look_type,
                incoming_reduction_percent,
                critical_chance_percent,
                critical_extra_percentage_points,
            }) => {
                out.number(3)?;
                for value in [
                    *expires_ms,
                    u64::from(*outfit_look_type),
                    u64::from(*incoming_reduction_percent),
                    u64::from(*critical_chance_percent),
                    u64::from(*critical_extra_percentage_points),
                ] {
                    out.number(value)?;
                }
            }
            OwnerCombatChange::ManaShield(shield) => {
                out.number(4)?;
                out.number(u64::from(shield.capacity))?;
                out.number(shield.expires_ms)?;
            }
            OwnerCombatChange::ConsumeManaShield {
                expected_capacity,
                amount,
            } => {
                out.number(5)?;
                out.number(u64::from(*expected_capacity))?;
                out.number(u64::from(*amount))?;
            }
            OwnerCombatChange::MonsterAi(ai) => {
                out.number(6)?;
                if let Some((distance, expires)) = ai.forced_distance {
                    out.number(1)?;
                    out.number(u64::from(distance))?;
                    out.number(expires)?;
                } else {
                    out.number(0)?;
                }
                if let Some((actor, expires)) = ai.challenged_to {
                    out.number(1)?;
                    out.actor(actor)?;
                    out.number(expires)?;
                } else {
                    out.number(0)?;
                }
            }
            OwnerCombatChange::PlayerConditions { expected, next } => {
                out.number(9)?;
                out.atom(b"player-condition-owner-debug-r21")?;
                out.atom(format!("{expected:?}").as_bytes())?;
                out.atom(format!("{next:?}").as_bytes())?;
            }
            OwnerCombatChange::DispelParalysis => {
                out.number(7)?;
            }
            OwnerCombatChange::CompanionMaster(assignment) => {
                out.number(10)?;
                out.atom(b"companion-master-owner-debug-r21")?;
                out.atom(format!("{assignment:?}").as_bytes())?;
            }
            OwnerCombatChange::CompanionConditions(update) => {
                out.number(8)?;
                // Versioned deterministic receipt capture of typed actual owner data;
                // never a Lua body or an executable authoring payload.
                out.atom(b"companion-condition-owner-debug-r21")?;
                out.atom(format!("{:?}", update.expected).as_bytes())?;
                out.atom(format!("{:?}", update.next).as_bytes())?;
            }
        }
    }
    Ok(out.0)
}

fn validate_batch(batch: &OwnerCombatBatch) -> Result<(), Error> {
    if batch.current_lease_generation == 0
        || batch.binding.is_empty()
        || (batch.effects.is_empty() && batch.anchor.is_none())
    {
        return Err(Error::InvalidBatch);
    }
    if batch.effects.len() > MAX_EFFECTS {
        return Err(Error::TooManyEffects);
    }
    if batch.binding.len() > MAX_BINDING_BYTES {
        return Err(Error::BindingTooLarge);
    }
    if batch
        .effects
        .iter()
        .any(|e| usize::from(e.sub_ordinal) >= MAX_EFFECTS)
        || batch
            .effects
            .windows(2)
            .any(|w| w[0].sub_ordinal >= w[1].sub_ordinal)
    {
        return Err(Error::InvalidBatch);
    }
    if let Some(anchor) = &batch.anchor {
        if anchor.expected_revision.checked_add(1) != Some(anchor.next_revision)
            || anchor.cooldown_deadlines.len() > 256
            || anchor
                .cooldown_deadlines
                .iter()
                .any(|(key, _)| key.is_empty() || key.contains('\0'))
            || anchor
                .cooldown_deadlines
                .windows(2)
                .any(|w| w[0].0 >= w[1].0)
        {
            return Err(Error::InvalidAnchor);
        }
    }
    Ok(())
}

impl ChannelRuntimeV1 {
    /// Every actual actor mutation outside the owning pending batch checks this guard.
    pub(crate) fn assert_actor_spell_unreserved(
        &self,
        actor: ExactActorRef,
    ) -> Result<(), CarrierError> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        if !matches!(&self.carrier.slots[index],Slot::Occupied{generation,..}|Slot::CreatureOccupied{generation,..}
            if *generation==actor.0.actor_local_generation.0)
        {
            return Err(CarrierError::StaleActorGeneration);
        }
        if state(&self.carrier.slots[index])
            .map_err(|_| CarrierError::StaleActorGeneration)?
            .pending_source
            .is_some()
            || state(&self.carrier.slots[index])
                .map_err(|_| CarrierError::StaleActorGeneration)?
                .pending_owner
                .is_some()
        {
            return Err(CarrierError::PlanConflict);
        }
        Ok(())
    }
    /// Invalid/stale actor references never advertise an available mutation lane.
    pub(crate) fn actor_spell_reserved(&self, actor: ExactActorRef) -> bool {
        self.assert_actor_spell_unreserved(actor).is_err()
    }
    /// Reserve the actual touched slots after all owner seals/capacity have been staged,
    /// before SQL may COMMIT. Existing identical reservation is retained across retries.
    pub(crate) fn reserve_spell_batch(
        &mut self,
        staged: &mut StagedSpellBatch,
    ) -> Result<(), Error> {
        self.validate_staged_spell_batch(staged)?;
        if !staged.will_apply() {
            return Ok(());
        }
        let binding = canonical_binding(&staged.batch)?;
        let reservation = match &staged.reservation {
            Some(existing) if existing.matches(&staged.batch, &binding) => existing.clone(),
            Some(_) => return Err(Error::CommandConflict),
            None => Arc::new(OwnerSpellReservation {
                caster: staged.batch.caster,
                command: staged.batch.command,
                phase: staged
                    .batch
                    .deferred
                    .as_ref()
                    .map_or(0, |p| p.phase_ordinal()),
                canonical: Arc::from(binding),
                destination: staged.relocation.as_ref().map(|r| (r.actor, r.destination)),
                deferred: staged.batch.deferred.clone(),
            }),
        };
        for (_, original) in &staged.originals {
            if state(original)?.pending_source.is_some() {
                return Err(Error::SnapshotChanged);
            }
            if state(original)?
                .pending_owner
                .as_ref()
                .is_some_and(|r| r != &reservation)
            {
                return Err(Error::SnapshotChanged);
            }
        }
        // All references and states were validated above. This installs reservation data
        // only, no HP/condition/payment changes and no fallible work after the first slot.
        for (index, original) in &mut staged.originals {
            state_mut(&mut self.carrier.slots[*index])
                .expect("validated touched actual slot")
                .pending_owner = Some(reservation.clone());
            state_mut(original)
                .expect("validated original slot")
                .pending_owner = Some(reservation.clone());
        }
        staged.reservation = Some(reservation);
        Ok(())
    }
    /// Caller must have independently proven the source transaction did not COMMIT.
    /// Unknown outcomes retain reservations; dropping a staged data object never releases.
    pub(crate) fn release_definitely_uncommitted_spell_batch(
        &mut self,
        staged: &StagedSpellBatch,
    ) -> Result<(), Error> {
        self.validate_staged_spell_batch(staged)?;
        let reservation = staged.reservation.as_ref().ok_or(Error::InvalidBatch)?;
        for (_, original) in &staged.originals {
            if state(original)?.pending_owner.as_ref() != Some(reservation) {
                return Err(Error::SnapshotChanged);
            }
        }
        for (index, _) in &staged.originals {
            state_mut(&mut self.carrier.slots[*index])
                .expect("validated reserved actor")
                .pending_owner = None;
        }
        Ok(())
    }

    /// Exact existing physical target identity; never infer a spawn's atom from its prototype.
    pub(crate) fn creature_spell_target_atom(&self, actor: ExactActorRef) -> Result<String, Error> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied {
            target_identity, ..
        } = &self.carrier.slots[index]
        else {
            return Err(Error::InvalidBatch);
        };
        String::from_utf8(target_identity.to_vec()).map_err(|_| Error::InvalidBatch)
    }
    /// Resolve retained normalized input before a caller reads a new clock or draws RNG.
    /// The existing bounded caster ledger is the sole retention owner, not a second cache.
    pub(crate) fn retained_spell_batch(
        &self,
        caster: ExactActorRef,
        character: CharacterId,
        lease_generation: u64,
        command: CommandRef,
    ) -> Result<Option<OwnerCombatBatch>, Error> {
        // Historical input is readable while control is lost. This read grants no
        // mutation authority; fresh commands and deferred commits retain their
        // separate control and sealed-timer qualification below.
        self.player_control_facts(caster, command.game_session_id())?;
        if lease_generation == 0 {
            return Err(Error::InvalidBatch);
        }
        let index = self.carrier.validate_ref(&self.continuity, caster.0)?;
        let owner = state(&self.carrier.slots[index])?;
        if let Some(record) = owner
            .records
            .iter()
            .find(|r| r.command == command && r.phase == 0)
        {
            if record.character != character || record.lease != lease_generation {
                return Err(Error::CommandConflict);
            }
            return Ok(Some(record.original_batch.clone()));
        }
        if let Some((before_character, before_lease, before_session, sequence, _)) =
            owner.high_water
        {
            if before_character != character
                || lease_generation < before_lease
                || (lease_generation == before_lease && before_session != command.game_session_id())
            {
                return Err(Error::SupersededSession);
            }
            if lease_generation == before_lease && command.command_id().get() <= sequence {
                return Err(Error::StaleCommand);
            }
        }
        Ok(None)
    }

    /// The sealed source touch list reserves actual companion metadata in the same
    /// physical batch, without inventing an execution effect or a second entity map.
    pub(crate) fn stage_spell_batch_with_companion_touches(
        &self,
        batch: &OwnerCombatBatch,
        touches: &QualifiedCompanionTouches,
    ) -> Result<StagedSpellBatch, Error> {
        let value: serde_json::Value =
            serde_json::from_slice(&batch.binding).map_err(|_| Error::InvalidBatch)?;
        if value
            .get("source_companion_touches")
            .and_then(|v| v.as_str())
            != Some(touches.binding.as_str())
        {
            return Err(Error::CommandConflict);
        }
        let mut staged = self.stage_spell_batch(batch)?;
        if !staged.will_apply() {
            return Ok(staged);
        }
        let canonical = canonical_binding(batch)?;
        for snapshot in &touches.snapshots {
            self.validate_companion_snapshot(snapshot)?;
            let index = self
                .carrier
                .validate_ref(&self.continuity, snapshot.actor.0)?;
            let original = &self.carrier.slots[index];
            if state(original)?.pending_source.is_some() {
                return Err(Error::SnapshotChanged);
            }
            if state(original)?
                .pending_owner
                .as_ref()
                .is_some_and(|p| !p.matches(batch, &canonical))
            {
                return Err(Error::SnapshotChanged);
            }
            if !staged.originals.iter().any(|(i, _)| *i == index) {
                staged.companion_touch_indices.push(index);
                staged.originals.push((index, original.clone()));
                let mut replacement = original.clone();
                state_mut(&mut replacement)?.pending_owner = None;
                staged.replacements.push((index, replacement));
            }
        }
        staged.originals.sort_unstable_by_key(|(index, _)| *index);
        staged
            .replacements
            .sort_unstable_by_key(|(index, _)| *index);
        Ok(staged)
    }

    /// Used only by the source composition after a genuine source COMMIT and full
    /// source successor preflight, with uninterrupted runtime/player owner locks.
    /// Releases metadata-only touches for immediate source party/despawn installation;
    /// retains the caster/payment reservation and the full original command identity.
    pub(crate) fn release_companion_touches_for_source_commit(
        &mut self,
        staged: &mut StagedSpellBatch,
    ) -> Result<(), Error> {
        self.validate_staged_spell_batch(staged)?;
        let reservation = staged.reservation.as_ref().ok_or(Error::InvalidBatch)?;
        for index in &staged.companion_touch_indices {
            if state(&self.carrier.slots[*index])?.pending_owner.as_ref() != Some(reservation) {
                return Err(Error::SnapshotChanged);
            }
        }
        // All fallible checks precede the first actual reservation change. The source
        // composition installs its preallocated metadata successors in this owner turn.
        for index in &staged.companion_touch_indices {
            state_mut(&mut self.carrier.slots[*index])
                .expect("validated touched slot")
                .pending_owner = None;
        }
        staged
            .originals
            .retain(|(index, _)| !staged.companion_touch_indices.contains(index));
        staged
            .replacements
            .retain(|(index, _)| !staged.companion_touch_indices.contains(index));
        staged.companion_touch_indices.clear();
        Ok(())
    }

    pub(crate) fn stage_spell_batch_with_relocation(
        &self,
        batch: &OwnerCombatBatch,
        relocation: &QualifiedSpellRelocation,
    ) -> Result<StagedSpellBatch, Error> {
        let value: serde_json::Value =
            serde_json::from_slice(&batch.binding).map_err(|_| Error::InvalidBatch)?;
        if value.get("source_relocation").and_then(|v| v.as_str())
            != Some(relocation.binding.as_str())
        {
            return Err(Error::CommandConflict);
        }
        let mut staged = self.stage_spell_batch_inner(batch)?;
        if !staged.will_apply() {
            return Ok(staged);
        }
        let index = self
            .carrier
            .validate_ref(&self.continuity, relocation.actor.0)?;
        if self.read_actor_position(relocation.actor)? != relocation.expected {
            return Err(Error::SnapshotChanged);
        }
        if self.position_occupied_by_other(relocation.actor, relocation.destination)? {
            return Err(Error::SnapshotChanged);
        }
        let original = &self.carrier.slots[index];
        let Slot::Occupied {
            committed: true,
            lifecycle,
            ..
        } = original
        else {
            return Err(Error::InvalidBatch);
        };
        if lifecycle.control_loss.is_some() {
            return Err(Error::InvalidBatch);
        }
        let original_state = state(original)?;
        let canonical = canonical_binding(batch)?;
        if original_state.pending_source.is_some()
            || original_state
                .pending_owner
                .as_ref()
                .is_some_and(|p| !p.matches(batch, &canonical))
        {
            return Err(Error::SnapshotChanged);
        }
        if !staged.originals.iter().any(|(i, _)| *i == index) {
            staged.originals.push((index, original.clone()));
            let mut replacement = original.clone();
            state_mut(&mut replacement)?.pending_owner = None;
            staged.replacements.push((index, replacement));
        }
        let (_, slot) = staged
            .replacements
            .iter_mut()
            .find(|(i, _)| *i == index)
            .ok_or(Error::InvalidBatch)?;
        let Slot::Occupied {
            position: Some(position),
            ..
        } = slot
        else {
            return Err(Error::InvalidBatch);
        };
        if *position != relocation.expected.0.version {
            return Err(Error::SnapshotChanged);
        }
        position.revision = position
            .revision
            .checked_add(1)
            .ok_or(Error::InvalidBatch)?;
        position.position = LocalPosition {
            x: relocation.destination.x,
            y: relocation.destination.y,
            floor: relocation.destination.floor,
        };
        // Facing is the genuine current facing; vertical/rope casts do not invent a turn.
        staged.relocation = Some(relocation.clone());
        Ok(staged)
    }

    pub(crate) fn stage_spell_batch(
        &self,
        batch: &OwnerCombatBatch,
    ) -> Result<StagedSpellBatch, Error> {
        let staged = self.stage_spell_batch_inner(batch)?;
        if staged.will_apply()
            && serde_json::from_slice::<serde_json::Value>(&batch.binding)
                .ok()
                .is_some_and(|v| v.get("source_relocation").is_some())
        {
            return Err(Error::InvalidBatch);
        }
        Ok(staged)
    }
    fn qualified_deferred_batch(&self, batch: &OwnerCombatBatch) -> bool {
        batch.deferred.as_ref().is_some_and(|authority| {
            authority.phase_ordinal() != 0
                && authority.matches(
                    batch,
                    super::super::RuntimeScopeRefV1::channel(
                        self.binding.world_id,
                        self.binding.channel_id,
                    ),
                    self.binding.scope_generation,
                )
        })
    }

    fn stage_spell_batch_inner(&self, batch: &OwnerCombatBatch) -> Result<StagedSpellBatch, Error> {
        validate_batch(batch)?;
        let caster_facts =
            self.player_control_facts(batch.caster, batch.command.game_session_id())?;
        // Loss rejects fresh input, while a timer phase already admitted after
        // its original committed cast remains prospective history. The actual
        // scheduler is the sole producer of this token; merely setting a phase
        // or retaining a description is insufficient. All ordinary current
        // actor/session, reservation and deferred replay checks still follow.
        let qualified_deferred = self.qualified_deferred_batch(batch);
        if caster_facts.control_loss.is_some() && !qualified_deferred {
            return Err(Error::InvalidBatch);
        }
        let caster_index = self
            .carrier
            .validate_ref(&self.continuity, batch.caster.0)?;
        let binding = canonical_binding(batch)?;
        let phase = batch
            .deferred
            .as_ref()
            .map_or(0, |authority| authority.phase_ordinal());
        let caster_state = state(&self.carrier.slots[caster_index])?;
        if caster_state.pending_source.is_some() {
            return Err(Error::SnapshotChanged);
        }
        if caster_state
            .pending_owner
            .as_ref()
            .is_some_and(|r| !r.matches(batch, &binding))
        {
            return Err(Error::SnapshotChanged);
        }
        if let Some(prior) = caster_state
            .records
            .iter()
            .find(|r| r.command == batch.command && r.phase == phase)
        {
            if prior.character != batch.attacker
                || prior.lease != batch.current_lease_generation
                || canonical_binding(&prior.original_batch)? != binding
                || prior.original_batch.deferred != batch.deferred
            {
                return Err(Error::CommandConflict);
            }
            let mut receipt = prior.receipt.clone();
            receipt.applied = false;
            return Ok(StagedSpellBatch {
                reservation: None,
                batch: batch.clone(),
                originals: vec![(caster_index, self.carrier.slots[caster_index].clone())],
                replacements: vec![],
                companion_touch_indices: vec![],
                relocation: None,
                player_effects: vec![],
                player_preflight_bound: true,
                receipt,
            });
        }
        let deferred = if let Some(authority) = &batch.deferred {
            if !authority.matches(
                batch,
                super::super::RuntimeScopeRefV1::channel(
                    self.binding.world_id,
                    self.binding.channel_id,
                ),
                self.binding.scope_generation,
            ) || authority.admission_sequence() <= caster_state.deferred_high_water
            {
                return Err(Error::StaleCommand);
            }
            true
        } else {
            false
        };
        if let Some((character, lease, session, sequence, ordinal)) = caster_state.high_water {
            if character != batch.attacker
                || batch.current_lease_generation < lease
                || (batch.current_lease_generation == lease
                    && batch.command.game_session_id() != session)
            {
                return Err(Error::SupersededSession);
            }
            if !deferred
                && batch.current_lease_generation == lease
                && (batch.command.command_id().get(), phase) <= (sequence, ordinal)
            {
                return Err(Error::StaleCommand);
            }
        }
        // Only data is cloned. The original independently current continuity is borrowed by
        // every carrier operation below; no namespace grant/capability is cloned or reissued.
        let mut next = self.carrier.clone();
        state_mut(&mut next.slots[caster_index])?.pending_owner = None;
        let mut touched = vec![caster_index];
        let mut player_effects = Vec::new();
        let mut effect_receipts = Vec::new();
        player_effects
            .try_reserve(batch.effects.len())
            .map_err(|_| Error::AllocationFailed)?;
        effect_receipts
            .try_reserve(batch.effects.len())
            .map_err(|_| Error::AllocationFailed)?;
        let digest: [u8; 32] = Sha256::digest(&binding).into();
        for effect in &batch.effects {
            let index = self
                .carrier
                .validate_ref(&self.continuity, effect.target.0)?;
            if !matches!(&self.carrier.slots[index],Slot::Occupied{generation,..}|Slot::CreatureOccupied{generation,..}
                if *generation==effect.target.0.actor_local_generation.0)
            {
                return Err(Error::Owner(CarrierError::StaleActorGeneration));
            }
            if state(&self.carrier.slots[index])?.pending_source.is_some() {
                return Err(Error::SnapshotChanged);
            }
            if state(&self.carrier.slots[index])?
                .pending_owner
                .as_ref()
                .is_some_and(|r| !r.matches(batch, &binding))
            {
                return Err(Error::SnapshotChanged);
            }
            // Only the data clone loses its matched own reservation. The actual
            // original remains sealed while raw commit validates this proposed successor.
            state_mut(&mut next.slots[index])?.pending_owner = None;
            if !touched.contains(&index) {
                touched.push(index);
            }
            let mut health_receipt = None;
            if matches!(next.slots[index], Slot::Occupied { .. }) {
                let Slot::Occupied {
                    game_session_id: Some(_),
                    committed: true,
                    ..
                } = &next.slots[index]
                else {
                    return Err(Error::InvalidBatch);
                };
                if matches!(effect.change, OwnerCombatChange::MonsterAi(_)) {
                    return Err(Error::InvalidCondition);
                }
                player_effects.push(effect.clone());
            } else {
                match &effect.change {
                    OwnerCombatChange::CompanionMaster(assignment) => {
                        if assignment.actor() != effect.target
                            || assignment.owner() != batch.caster
                            || assignment.session() != batch.command.game_session_id()
                            || batch.deferred.is_some()
                        {
                            return Err(Error::InvalidCondition);
                        }
                        self.validate_companion_assignment(assignment)?;
                        let Slot::CreatureOccupied {
                            companion: Some(companion),
                            ..
                        } = &mut next.slots[index]
                        else {
                            return Err(Error::InvalidCondition);
                        };
                        companion.master = Some(super::runtime_actor_companion::CompanionMaster {
                            actor: assignment.owner(),
                            session: assignment.session(),
                        });
                    }
                    OwnerCombatChange::CompanionConditions(update) => {
                        if update.expected.actor != effect.target {
                            return Err(Error::InvalidCondition);
                        }
                        self.validate_companion_snapshot(&update.expected)?;
                        let before = &update.expected.state;
                        let after = &update.next;
                        if before.policy != after.policy
                            || before.master != after.master
                            || before.creation_speed_delta != after.creation_speed_delta
                            || before.outfit_look_type != after.outfit_look_type
                            || before.object_look_type != after.object_look_type
                            || before.expires_at_unix != after.expires_at_unix
                            || before.lifecycle_epoch != after.lifecycle_epoch
                            || before.party_protection != after.party_protection
                            || before.created_at_us != after.created_at_us
                            || before.familiar_defense != after.familiar_defense
                        {
                            return Err(Error::InvalidCondition);
                        }
                        let Slot::CreatureOccupied {
                            companion: Some(companion),
                            ..
                        } = &mut next.slots[index]
                        else {
                            return Err(Error::InvalidCondition);
                        };
                        companion.conditions = after.conditions.clone();
                    }
                    OwnerCombatChange::Damage {
                        target_atom,
                        magnitude,
                    } => {
                        let mut hit_binding = digest.to_vec();
                        hit_binding.extend_from_slice(&effect.sub_ordinal.to_be_bytes());
                        health_receipt = Some(next.commit_creature_damage_inner_bounded(
                            &self.continuity,
                            effect.target.0,
                            OwnerDamageCommand {
                                target: target_atom.as_bytes(),
                                occurrence: &[],
                                binding: &hit_binding,
                                damage: *magnitude,
                            },
                            Some(AttackerCommand::new(
                                batch.attacker,
                                batch.current_lease_generation,
                                batch.command,
                                effect.sub_ordinal,
                            )),
                            false,
                            MAX_EFFECTS as u16,
                            deferred,
                        )?);
                    }
                    OwnerCombatChange::Heal {
                        target_atom,
                        magnitude,
                    } => {
                        let Slot::CreatureOccupied {
                            target_identity,
                            health,
                            spell_combat,
                            ..
                        } = &mut next.slots[index]
                        else {
                            return Err(Error::InvalidBatch);
                        };
                        if target_identity.as_ref() != target_atom.as_bytes() {
                            return Err(Error::Owner(CarrierError::CreatureTargetMismatch));
                        }
                        if *magnitude <= 0 {
                            return Err(Error::InvalidMagnitude);
                        }
                        if *health <= 0 || spell_combat.maximum_health <= 0 {
                            return Err(Error::Owner(CarrierError::CreatureNotActionable));
                        }
                        let before = *health;
                        *health = before
                            .saturating_add(*magnitude)
                            .min(spell_combat.maximum_health);
                        health_receipt = Some(OwnerDamageResult {
                            applied: true,
                            health_before: before,
                            health_after: *health,
                        });
                    }
                    OwnerCombatChange::MonsterAi(ai) => {
                        if !matches!(
                            next.slots[index],
                            Slot::CreatureOccupied { health: 1.., .. }
                        ) {
                            return Err(Error::Owner(CarrierError::CreatureNotActionable));
                        }
                        if ai.forced_distance.is_none() && ai.challenged_to.is_none() {
                            return Err(Error::InvalidCondition);
                        }
                        if ai
                            .forced_distance
                            .is_some_and(|(_, expires)| expires <= batch.now_ms)
                        {
                            return Err(Error::InvalidCondition);
                        }
                        if let Some((challenged, expires)) = ai.challenged_to {
                            self.carrier.validate_ref(&self.continuity, challenged.0)?;
                            if expires <= batch.now_ms {
                                return Err(Error::InvalidCondition);
                            }
                        }
                        let current = state_mut(&mut next.slots[index])?;
                        let mut merged = current.monster_ai.clone().unwrap_or(MonsterAiState {
                            forced_distance: None,
                            challenged_to: None,
                        });
                        if ai.forced_distance.is_some() {
                            merged.forced_distance = ai.forced_distance;
                        }
                        if ai.challenged_to.is_some() {
                            merged.challenged_to = ai.challenged_to;
                        }
                        current.monster_ai = Some(merged);
                    }
                    OwnerCombatChange::DispelParalysis => {
                        state_mut(&mut next.slots[index])?.paralysed = false;
                    }
                    _ => return Err(Error::InvalidCondition),
                }
            }
            effect_receipts.push(EffectReceipt {
                target: effect.target,
                sub_ordinal: effect.sub_ordinal,
                health: health_receipt,
            });
        }
        let receipt = CombatBatchReceipt {
            applied: true,
            caster: batch.caster,
            command: batch.command,
            effects: effect_receipts,
        };
        let updated_caster = state_mut(&mut next.slots[caster_index])?;
        updated_caster
            .records
            .try_reserve(1)
            .map_err(|_| Error::AllocationFailed)?;
        if updated_caster.records.len() >= MAX_COMMAND_RECEIPTS {
            updated_caster.records.remove(0);
        }
        updated_caster.records.push(BatchRecord {
            character: batch.attacker,
            lease: batch.current_lease_generation,
            command: batch.command,
            phase,
            original_batch: batch.clone(),
            receipt: receipt.clone(),
        });
        let last = batch.effects.last().map_or(0, |e| e.sub_ordinal);
        let mark = (
            batch.attacker,
            batch.current_lease_generation,
            batch.command.game_session_id(),
            batch.command.command_id().get(),
            last,
        );
        if updated_caster
            .high_water
            .is_none_or(|old| (mark.1, mark.3, mark.4) > (old.1, old.3, old.4))
        {
            updated_caster.high_water = Some(mark);
        }
        if let Some(authority) = &batch.deferred {
            updated_caster.deferred_high_water = authority.admission_sequence();
        }
        touched.sort_unstable();
        let originals = touched
            .iter()
            .map(|i| (*i, self.carrier.slots[*i].clone()))
            .collect();
        let replacements = touched
            .iter()
            .map(|i| {
                let mut slot = next.slots[*i].clone();
                state_mut(&mut slot)
                    .expect("qualified touched actor")
                    .pending_owner = None;
                (*i, slot)
            })
            .collect();
        Ok(StagedSpellBatch {
            reservation: caster_state.pending_owner.clone(),
            batch: batch.clone(),
            originals,
            replacements,
            companion_touch_indices: vec![],
            relocation: None,
            player_preflight_bound: player_effects.is_empty() && batch.anchor.is_none(),
            player_effects,
            receipt,
        })
    }

    pub(crate) fn commit_spell_batch(
        &mut self,
        staged: StagedSpellBatch,
    ) -> Result<CombatBatchReceipt, Error> {
        self.validate_staged_spell_batch(&staged)?;
        if !staged.player_preflight_bound {
            return Err(Error::PlayerVitalsOwnerRequired);
        }
        // No fallible calculation, allocation or callbacks follow the first replacement.
        for (index, replacement) in staged.replacements {
            self.carrier.slots[index] = replacement;
        }
        Ok(staged.receipt)
    }

    /// Borrow-only compare of the already allocated real slots, also used before consuming
    /// a receipt-qualified training successor. This check grants no detached authority.
    pub(crate) fn validate_staged_spell_batch(
        &self,
        staged: &StagedSpellBatch,
    ) -> Result<(), Error> {
        // Independent current continuity and actual player session are checked again here.
        self.carrier.validate_current_continuity(&self.continuity)?;
        let facts =
            self.player_control_facts(staged.batch.caster, staged.batch.command.game_session_id())?;
        if facts.control_loss.is_some() && !self.qualified_deferred_batch(&staged.batch) {
            return Err(Error::InvalidBatch);
        }
        for (index, original) in &staged.originals {
            if self.carrier.slots.get(*index) != Some(original) {
                return Err(Error::SnapshotChanged);
            }
        }
        Ok(())
    }

    pub(crate) fn creature_combat_facts(
        &self,
        actor: ExactActorRef,
        now_ms: u64,
    ) -> Result<CreatureCombatFacts, Error> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied {
            health,
            spell_combat,
            ..
        } = &self.carrier.slots[index]
        else {
            return Err(Error::Owner(CarrierError::NotCreature));
        };
        let mut ai = spell_combat.monster_ai.clone();
        if let Some(ai) = &mut ai {
            if ai
                .forced_distance
                .is_some_and(|(_, expiry)| now_ms >= expiry)
            {
                ai.forced_distance = None;
            }
            if ai.challenged_to.is_some_and(|(_, expiry)| now_ms >= expiry) {
                ai.challenged_to = None;
            }
        }
        Ok(CreatureCombatFacts {
            health: *health,
            maximum_health: spell_combat.maximum_health,
            monster_ai: ai,
            paralysed: spell_combat.paralysed,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreatureCombatFacts {
    pub(crate) health: i64,
    pub(crate) maximum_health: i64,
    pub(crate) monster_ai: Option<MonsterAiState>,
    pub(crate) paralysed: bool,
}

#[cfg(test)]
#[path = "runtime_actor_spell_tests.rs"]
mod tests;
