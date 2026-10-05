//! The Channel's bounded presentation outbox. Source cue identities are data,
//! never combat authority. Preparation binds exact activated spell, positions
//! and normalized batch before RNG/SQL commit; replay does not enqueue twice.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use crate::content::native_gameplay::NativeGameplayState;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
use crate::spell::Execution;
use crate::spell::combat_batch::{CombatBatchReceipt, OwnerCombatBatch};
use crate::spell::native::CompiledNativeSpell;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, num::NonZeroU32, sync::OnceLock};

#[path = "spell_presentations_familiar.rs"]
mod familiar;
pub(crate) const MAX_PRESENTATIONS: usize = 512;
pub(crate) const MAX_CAST_PRESENTATIONS: usize = 256;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cue {
    Effect(u16),
    Projectile(u16),
    Sound(u16),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CueRequest {
    pub(crate) binding: String,
    /// Exact positioned source or target; there is no opaque client address.
    pub(crate) actor: ExactActorRef,
}
/// Qualification describes a real placed map cell. Current owner authority
/// is independently checked again at outbox installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QualifiedCueTile {
    position: MovementLocalPosition,
    content: [u8; 32],
    map: [u8; 32],
    frame: [u8; 32],
}
impl QualifiedCueTile {
    pub(crate) fn from_owners(
        room: &crate::content::QualifiedNativeEntryRoom,
        runtime: &ChannelRuntimeV1,
        position: MovementLocalPosition,
    ) -> Result<Self, Error> {
        crate::spell::world_items_execution::SpellGroundTarget::from_native_owner(
            room, runtime, position,
        )
        .map_err(|_| Error::UnqualifiedSource)?;
        let pin = runtime.content_pin();
        Ok(Self {
            position,
            content: pin.server_artifact_digest(),
            map: pin.map_revision_digest(),
            frame: pin.frame_binding_digest(),
        })
    }
    /// Source mutable cells are qualified by their real initialized Item/Map
    /// owner in the current SQL pass, instead of a second static lookup.
    pub(crate) fn from_current_tile(
        room: &crate::content::QualifiedNativeEntryRoom,
        runtime: &ChannelRuntimeV1,
        fact: &crate::spell::world_execution::QualifiedCombatTileFact<'_>,
    ) -> Result<Self, Error> {
        if !fact.matches_runtime(runtime) {
            return Err(Error::UnqualifiedSource);
        }
        crate::spell::world_items_execution::SpellGroundTarget::for_native_tile_read(
            room,
            runtime,
            fact.position(),
        )
        .map_err(|_| Error::UnqualifiedSource)?;
        let pin = runtime.content_pin();
        Ok(Self {
            position: fact.position(),
            content: pin.server_artifact_digest(),
            map: pin.map_revision_digest(),
            frame: pin.frame_binding_digest(),
        })
    }
    /// A source-captured absent cell can receive the source failure/success cue;
    /// it cannot become a ground mint or collision capability.
    pub(crate) fn from_known_absent_cell(
        room: &crate::content::QualifiedNativeEntryRoom,
        runtime: &ChannelRuntimeV1,
        position: MovementLocalPosition,
    ) -> Result<Self, Error> {
        let cells = room.movement_cells();
        let cell = crate::content::LogicalCell {
            x: position.x,
            y: position.y,
            z: i32::from(position.floor),
        };
        if !matches!(
            cells.spell_tiles().lookup(cells.scope(), cell),
            Err(crate::content::SpellTileLookupError::Absent)
        ) {
            return Err(Error::UnqualifiedSource);
        }
        crate::spell::world_items_execution::SpellGroundTarget::for_native_tile_read(
            room, runtime, position,
        )
        .map_err(|_| Error::UnqualifiedSource)?;
        let pin = runtime.content_pin();
        Ok(Self {
            position,
            content: pin.server_artifact_digest(),
            map: pin.map_revision_digest(),
            frame: pin.frame_binding_digest(),
        })
    }
    fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
        let pin = runtime.content_pin();
        self.content == pin.server_artifact_digest()
            && self.map == pin.map_revision_digest()
            && self.frame == pin.frame_binding_digest()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CueTarget {
    Actor(ExactActorRef),
    Tile(QualifiedCueTile),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocatedCueRequest {
    pub(crate) binding: String,
    pub(crate) target: CueTarget,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Presentation {
    pub(crate) source_binding: String,
    pub(crate) cue: Cue,
    pub(crate) actor: Option<ExactActorRef>,
    pub(crate) position: MovementLocalPosition,
}
/// Retained source of an actual applied owner decision. Familiar decisions deliberately have
/// no player CommandRef; their sealed physical receipt identifies the independent occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PresentationCause {
    Cast {
        caster: ExactActorRef,
        origin: MovementLocalPosition,
        occurrence: crate::spell::combat_batch::SpellOccurrenceBinding,
    },
    FamiliarDefense {
        actor: ExactActorRef,
        origin: MovementLocalPosition,
        profile: [u8; 32],
        ordinal: u64,
    },
}
impl PresentationCause {
    pub(crate) fn actor(&self) -> ExactActorRef {
        match self {
            Self::Cast { caster, .. } => *caster,
            Self::FamiliarDefense { actor, .. } => *actor,
        }
    }
    pub(crate) fn origin(&self) -> MovementLocalPosition {
        match self {
            Self::Cast { origin, .. } | Self::FamiliarDefense { origin, .. } => *origin,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommittedPresentation {
    pub(crate) event: Presentation,
    pub(crate) cause: std::sync::Arc<PresentationCause>,
    /// Channel owner commit order, never a command id or a random draw.
    pub(crate) decision_ordinal: u64,
    pub(crate) emission_ordinal: u16,
    pub(crate) emission_sequence: u64,
}
#[derive(Debug)]
pub(crate) struct SpellPresentationOwner {
    content: [u8; 32],
    scope: crate::foundation::RuntimeScopeRefV1,
    generation: crate::foundation::ScopeOwnershipGeneration,
    pending: Vec<CommittedPresentation>,
    held: Vec<PresentationHold>,
    committed_decisions: u64,
    emitted_events: u64,
}
#[derive(Debug, PartialEq, Eq)]
struct PresentationHold {
    caster: ExactActorRef,
    command: crate::foundation::CommandRef,
    seal: [u8; 32],
    count: usize,
}
fn hold_for(prepared: &PreparedPresentation) -> PresentationHold {
    PresentationHold {
        caster: prepared.batch.caster,
        command: prepared.batch.command,
        seal: Sha256::digest(&prepared.batch.binding).into(),
        count: prepared.events.len(),
    }
}
#[derive(Debug, Clone)]
pub(crate) struct PreparedPresentation {
    content: [u8; 32],
    scope: crate::foundation::RuntimeScopeRefV1,
    generation: crate::foundation::ScopeOwnershipGeneration,
    batch: OwnerCombatBatch,
    events: Vec<Presentation>,
    tiles: Vec<QualifiedCueTile>,
    source_origin: MovementLocalPosition,
    cause: std::sync::Arc<PresentationCause>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Error {
    StaleOwner,
    Capacity,
    UnqualifiedSource,
    InvalidBatch,
}
impl SpellPresentationOwner {
    pub(crate) fn new(runtime: &ChannelRuntimeV1) -> Self {
        let binding = runtime.binding();
        Self {
            content: runtime.content_pin().server_artifact_digest(),
            scope: crate::foundation::RuntimeScopeRefV1::channel(
                binding.world_id(),
                binding.channel_id(),
            ),
            generation: binding.scope_generation(),
            pending: Vec::new(),
            held: Vec::new(),
            committed_decisions: 0,
            emitted_events: 0,
        }
    }
    /// Call with the entire source maximum before any random draw; never clip.
    pub(crate) fn reserve_before_draw(
        &mut self,
        runtime: &ChannelRuntimeV1,
        count: usize,
    ) -> Result<(), Error> {
        self.validate_owner(runtime)?;
        let held: usize = self.held.iter().map(|hold| hold.count).sum();
        // Reserve numerical headroom with the actual retained queue obligations before RNG.
        if self
            .committed_decisions
            .checked_add(self.held.len() as u64 + 1)
            .is_none()
            || self
                .emitted_events
                .checked_add((held + count) as u64)
                .is_none()
        {
            return Err(Error::Capacity);
        }
        if count > MAX_CAST_PRESENTATIONS
            || count > MAX_PRESENTATIONS.saturating_sub(self.pending.len() + held)
            || self.held.len() >= MAX_PRESENTATIONS
        {
            return Err(Error::Capacity);
        }
        self.pending
            .try_reserve(held + count)
            .map_err(|_| Error::Capacity)
    }
    fn validate_owner(&self, runtime: &ChannelRuntimeV1) -> Result<(), Error> {
        let binding = runtime.binding();
        if runtime.owner_fence().is_err()
            || self.content != runtime.content_pin().server_artifact_digest()
            || self.scope
                != crate::foundation::RuntimeScopeRefV1::channel(
                    binding.world_id(),
                    binding.channel_id(),
                )
            || self.generation != binding.scope_generation()
        {
            return Err(Error::StaleOwner);
        }
        Ok(())
    }
    /// The caller incorporates the normalized cue bytes into the same original
    /// source intent used by physical/durable replay classification.
    pub(crate) fn prepare(
        &mut self,
        runtime: &ChannelRuntimeV1,
        active: &NativeGameplayState,
        index: NonZeroU32,
        profile: &CompiledNativeSpell,
        batch: &mut OwnerCombatBatch,
        requests: Vec<CueRequest>,
    ) -> Result<PreparedPresentation, Error> {
        let requests = requests
            .into_iter()
            .map(|r| LocatedCueRequest {
                binding: r.binding,
                target: CueTarget::Actor(r.actor),
            })
            .collect();
        self.prepare_positions(runtime, active, index, profile, batch, requests)
    }
    pub(crate) fn prepare_positions(
        &mut self,
        runtime: &ChannelRuntimeV1,
        active: &NativeGameplayState,
        index: NonZeroU32,
        profile: &CompiledNativeSpell,
        batch: &mut OwnerCombatBatch,
        requests: Vec<LocatedCueRequest>,
    ) -> Result<PreparedPresentation, Error> {
        self.reserve_before_draw(runtime, requests.len())?;
        if active.source_digest() != self.content {
            return Err(Error::UnqualifiedSource);
        }
        let Some((definition, true)) = active.spell_book().source_indexed(index) else {
            return Err(Error::UnqualifiedSource);
        };
        if !matches!(&definition.execution, Execution::NativeProfile(actual) if actual == profile) {
            return Err(Error::UnqualifiedSource);
        }
        let mut allowed = BTreeSet::new();
        collect_bindings(profile.spell(), &mut allowed);
        collect_bindings(profile.dependencies(), &mut allowed);
        if profile.spell()["execution"]["native_behavior"]["key"] == "companion_haste"
            && profile.spell()["execution"]["native_behavior"]["parameters"]["caster_effect"]
                == "magic_green"
        {
            allowed.insert("appearance:effect/magic_green");
        }
        let acquisition = &profile.spell()["execution"]["native_behavior"];
        if acquisition["key"] == "acquire_summon"
            && acquisition["parameters"]["visuals"]["success"] == "magic_blue"
        {
            allowed.insert("appearance:effect/magic_blue");
            if acquisition["parameters"]["source"] == "named_creature"
                && acquisition["parameters"]["visuals"]["created_teleport"] == true
            {
                allowed.insert("appearance:effect/teleport");
            }
        }
        self.prepare_qualified_positions(runtime, batch, requests, allowed)
    }
    pub(crate) fn prepare_source_definition(
        &mut self,
        runtime: &ChannelRuntimeV1,
        active: &NativeGameplayState,
        index: NonZeroU32,
        definition: &crate::spell::SpellDefinition,
        batch: &mut OwnerCombatBatch,
        requests: Vec<LocatedCueRequest>,
    ) -> Result<PreparedPresentation, Error> {
        self.reserve_before_draw(runtime, requests.len())?;
        if active.source_digest() != self.content
            || active.spell_book().indexed(index) != Some(definition)
        {
            return Err(Error::UnqualifiedSource);
        }
        if let Execution::NativeProfile(profile) = &definition.execution {
            return self.prepare_positions(runtime, active, index, profile, batch, requests);
        }
        let source = definition
            .authored
            .as_ref()
            .ok_or(Error::UnqualifiedSource)?;
        let mut allowed = BTreeSet::new();
        fn add<'a>(value: &'a Option<String>, allowed: &mut BTreeSet<&'a str>) {
            if let Some(value) = value
                && resolve_source_cue(value).is_some()
            {
                allowed.insert(value.as_str());
            }
        }
        if let Some(cast) = &source.header.presentation {
            add(&cast.cast_cue, &mut allowed);
            add(&cast.impact_cue, &mut allowed);
        }
        if let Some(conjure) = &source.header.execution.conjure {
            add(&conjure.effect_asset_binding, &mut allowed);
        }
        if let Some(chain) = &definition.chain {
            add(&chain.asset_binding, &mut allowed);
        }
        for effect in &source.dependencies.effects {
            if let Some(presentation) = &effect.presentation {
                add(&presentation.impact_asset_binding, &mut allowed);
                add(&presentation.projectile_asset_binding, &mut allowed);
                add(&presentation.caster_effect_asset_binding, &mut allowed);
            }
        }
        self.prepare_qualified_positions(runtime, batch, requests, allowed)
    }
    fn prepare_qualified_positions(
        &mut self,
        runtime: &ChannelRuntimeV1,
        batch: &mut OwnerCombatBatch,
        requests: Vec<LocatedCueRequest>,
        allowed: BTreeSet<&str>,
    ) -> Result<PreparedPresentation, Error> {
        let source_origin = runtime
            .read_actor_position(batch.caster)
            .map_err(|_| Error::StaleOwner)?
            .position();
        let mut events = Vec::new();
        events
            .try_reserve_exact(requests.len())
            .map_err(|_| Error::Capacity)?;
        let mut tiles = Vec::new();
        tiles
            .try_reserve_exact(requests.len())
            .map_err(|_| Error::Capacity)?;
        for request in requests {
            if !allowed.contains(request.binding.as_str()) {
                return Err(Error::UnqualifiedSource);
            }
            let cue = resolve_source_cue(&request.binding).ok_or(Error::UnqualifiedSource)?;
            let (actor, position) = match request.target {
                CueTarget::Actor(actor) => (
                    Some(actor),
                    runtime
                        .read_actor_position(actor)
                        .map_err(|_| Error::StaleOwner)?
                        .position(),
                ),
                CueTarget::Tile(tile) => {
                    if !tile.current(runtime) {
                        return Err(Error::StaleOwner);
                    }
                    let position = tile.position;
                    tiles.push(tile);
                    (None, position)
                }
            };
            events.push(Presentation {
                source_binding: request.binding,
                cue,
                actor,
                position,
            });
        }
        let mut encoded = canonical_events(&events)?;
        encoded.extend_from_slice(&batch.caster.placement_identity());
        encoded.extend_from_slice(&source_origin.x.to_be_bytes());
        encoded.extend_from_slice(&source_origin.y.to_be_bytes());
        encoded.extend_from_slice(&source_origin.floor.to_be_bytes());
        let mut source: serde_json::Value =
            serde_json::from_slice(&batch.binding).map_err(|_| Error::InvalidBatch)?;
        let object = source.as_object_mut().ok_or(Error::InvalidBatch)?;
        if object.contains_key("source_presentation_bytes") {
            return Err(Error::InvalidBatch);
        }
        object.insert(
            "source_presentation_bytes".into(),
            serde_json::Value::Array(encoded.into_iter().map(|v| v.into()).collect()),
        );
        let binding = serde_json::to_vec(&source).map_err(|_| Error::InvalidBatch)?;
        if binding.len() > 131_072 {
            return Err(Error::Capacity);
        }
        batch.binding = binding;
        let cause = std::sync::Arc::new(PresentationCause::Cast {
            caster: batch.caster,
            origin: source_origin,
            occurrence: batch.occurrence.clone(),
        });
        Ok(PreparedPresentation {
            content: self.content,
            scope: self.scope,
            generation: self.generation,
            batch: batch.clone(),
            events,
            tiles,
            source_origin,
            cause,
        })
    }
    /// Register the entire original queue obligation before the durable mutation.
    /// Unrelated casts and publication may advance the queue while this COMMIT is unknown.
    pub(crate) fn hold_prepared_before_sql(
        &mut self,
        runtime: &ChannelRuntimeV1,
        prepared: &PreparedPresentation,
        batch: &OwnerCombatBatch,
    ) -> Result<(), Error> {
        self.validate_source_prepared(runtime, prepared, batch)?;
        let hold = hold_for(prepared);
        if self.held.contains(&hold) {
            return Ok(());
        }
        if self
            .held
            .iter()
            .any(|value| value.caster == hold.caster && value.command == hold.command)
        {
            return Err(Error::InvalidBatch);
        }
        self.reserve_before_draw(runtime, hold.count)?;
        self.held.try_reserve(1).map_err(|_| Error::Capacity)?;
        self.held.push(hold);
        Ok(())
    }
    pub(crate) fn release_definitely_uncommitted(&mut self, prepared: &PreparedPresentation) {
        let hold = hold_for(prepared);
        if let Some(index) = self.held.iter().position(|value| value == &hold) {
            self.held.remove(index);
        }
    }
    pub(crate) fn validate_prepared(
        &self,
        runtime: &ChannelRuntimeV1,
        prepared: &PreparedPresentation,
        batch: &OwnerCombatBatch,
    ) -> Result<(), Error> {
        self.validate_source_prepared(runtime, prepared, batch)?;
        if !self.held.contains(&hold_for(prepared)) {
            return Err(Error::InvalidBatch);
        }
        Ok(())
    }
    fn validate_source_prepared(
        &self,
        runtime: &ChannelRuntimeV1,
        prepared: &PreparedPresentation,
        batch: &OwnerCombatBatch,
    ) -> Result<(), Error> {
        self.validate_owner(runtime)?;
        if prepared.content != self.content
            || prepared.scope != self.scope
            || prepared.generation != self.generation
            || &prepared.batch != batch
        {
            return Err(Error::InvalidBatch);
        }
        if runtime
            .read_actor_position(batch.caster)
            .map_err(|_| Error::StaleOwner)?
            .position()
            != prepared.source_origin
        {
            return Err(Error::StaleOwner);
        }
        if prepared.events.len() > MAX_PRESENTATIONS.saturating_sub(self.pending.len())
            || prepared.events.len() > self.pending.capacity().saturating_sub(self.pending.len())
        {
            return Err(Error::Capacity);
        }
        if prepared.tiles.iter().any(|tile| !tile.current(runtime)) {
            return Err(Error::StaleOwner);
        }
        for event in &prepared.events {
            if let Some(actor) = event.actor
                && runtime
                    .read_actor_position(actor)
                    .map_err(|_| Error::StaleOwner)?
                    .position()
                    != event.position
            {
                return Err(Error::StaleOwner);
            }
        }
        Ok(())
    }
    /// Only after validation in the same uninterrupted owner turn. The receipt
    /// supplies history, while current runtime authority was resolved separately.
    #[allow(
        clippy::expect_used,
        reason = "post-validation commit invariant; a fallible exit here would leave a partial owner write"
    )]
    pub(crate) fn install_preflighted(
        &mut self,
        prepared: PreparedPresentation,
        receipt: &CombatBatchReceipt,
    ) {
        assert_eq!(receipt.caster, prepared.batch.caster);
        assert_eq!(receipt.command, prepared.batch.command);
        let hold = hold_for(&prepared);
        let index = self
            .held
            .iter()
            .position(|value| value == &hold)
            .expect("prevalidated original queue hold");
        self.held.remove(index);
        if receipt.applied {
            assert!(prepared.events.len() <= self.pending.capacity() - self.pending.len());
            self.install_committed_events(prepared.cause, prepared.events);
        }
    }
    #[allow(
        clippy::expect_used,
        reason = "post-validation commit invariant; a fallible exit here would leave a partial owner write"
    )]
    fn install_committed_events(
        &mut self,
        cause: std::sync::Arc<PresentationCause>,
        events: impl IntoIterator<Item = Presentation>,
    ) {
        self.committed_decisions = self
            .committed_decisions
            .checked_add(1)
            .expect("preflighted decision headroom");
        for (ordinal, event) in events.into_iter().enumerate() {
            self.emitted_events = self
                .emitted_events
                .checked_add(1)
                .expect("preflighted emission headroom");
            self.pending.push(CommittedPresentation {
                event,
                cause: cause.clone(),
                decision_ordinal: self.committed_decisions,
                emission_ordinal: u16::try_from(ordinal).expect("bounded source event count"),
                emission_sequence: self.emitted_events,
            });
        }
    }
    /// A current Channel publisher takes the actual outbox; wire delivery still
    /// requires the independently selected protocol presentation capability.
    pub(crate) fn take_for_current_publisher(
        &mut self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Vec<CommittedPresentation>, Error> {
        self.validate_owner(runtime)?;
        let mut delivered = Vec::new();
        delivered
            .try_reserve_exact(self.pending.len())
            .map_err(|_| Error::Capacity)?;
        delivered.append(&mut self.pending);
        Ok(delivered)
    }
}
fn collect_bindings<'a>(value: &'a serde_json::Value, into: &mut BTreeSet<&'a str>) {
    match value {
        serde_json::Value::Object(object) => {
            for value in object.values() {
                collect_bindings(value, into);
            }
        }
        serde_json::Value::Array(array) => {
            for value in array {
                collect_bindings(value, into);
            }
        }
        serde_json::Value::String(value) if resolve_source_cue(value).is_some() => {
            into.insert(value);
        }
        _ => {}
    }
}
fn canonical_events(events: &[Presentation]) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    out.try_reserve(events.len().saturating_mul(256).saturating_add(32))
        .map_err(|_| Error::Capacity)?;
    out.extend_from_slice(b"OTERYN_SOURCE_CUES_V1\0");
    out.extend_from_slice(&(events.len() as u16).to_be_bytes());
    for event in events {
        let length = u16::try_from(event.source_binding.len()).map_err(|_| Error::Capacity)?;
        out.extend_from_slice(&length.to_be_bytes());
        out.extend_from_slice(event.source_binding.as_bytes());
        let (kind, value) = match event.cue {
            Cue::Effect(v) => (1, v),
            Cue::Projectile(v) => (2, v),
            Cue::Sound(v) => (3, v),
        };
        out.push(kind);
        out.extend_from_slice(&value.to_be_bytes());
        match event.actor {
            Some(actor) => {
                out.push(1);
                out.extend_from_slice(&actor.placement_identity());
            }
            None => {
                out.push(0);
            }
        }
        out.extend_from_slice(&event.position.x.to_be_bytes());
        out.extend_from_slice(&event.position.y.to_be_bytes());
        out.extend_from_slice(&event.position.floor.to_be_bytes());
    }
    Ok(out)
}
#[derive(Deserialize)]
struct CueDocument {
    records: Vec<CueRecord>,
}
#[derive(Deserialize)]
struct CueRecord {
    alias: String,
    kind: String,
    value: u16,
}
#[allow(
    clippy::expect_used,
    reason = "post-validation commit invariant; a fallible exit here would leave a partial owner write"
)]
fn resolve_source_cue(alias: &str) -> Option<Cue> {
    static REGISTRY: OnceLock<Vec<CueRecord>> = OnceLock::new();
    let records = REGISTRY.get_or_init(|| {
        serde_json::from_slice::<CueDocument>(include_bytes!(
            "../../../../tools/content-schema/native-gameplay/spell-cues.json"
        ))
        .expect("compiled source-qualified cue policy")
        .records
    });
    let record = records.iter().find(|record| record.alias == alias)?;
    match record.kind.as_str() {
        "effect" => Some(Cue::Effect(record.value)),
        "projectile" => Some(Cue::Projectile(record.value)),
        "sound" => Some(Cue::Sound(record.value)),
        _ => None,
    }
}

#[cfg(test)]
#[path = "spell_presentations_tests.rs"]
pub(super) mod tests;
