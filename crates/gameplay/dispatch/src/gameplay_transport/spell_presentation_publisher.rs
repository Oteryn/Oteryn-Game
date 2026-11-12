//! Production sound fan-out and retained SPELL-PRESENT-0 publisher candidate.
//! It consumes the actual Channel outbox,
//! canonical same-TX current-session source and the existing VIS-2 index/settings. It does
//! not install a capability, emit a protocol envelope, query a second spatial index, or
//! infer a frame conversion for native floors outside the existing VIS-2 0..15 contract.
#![allow(
    dead_code,
    reason = "candidate transport helpers retained alongside the production sound publisher"
)]
use super::spell_presentations::{
    CommittedPresentation, Cue, PresentationCause, SpellPresentationOwner,
};
use crate::durability::{
    DurabilityRoot,
    spell_item_transaction::{SpellItemError, SpellItemScopeAuthority},
};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::movement::interest::{
    InterestIndex, VisibilityPosition, VisibilitySettings, tibia_floor,
};
use oteryn_protocol_oteryn::{spell_presentation_candidate as wire, world_spatial::ActorPosition};
use sqlx::{Postgres, Transaction};
use std::collections::VecDeque;
#[derive(Debug, Clone, PartialEq, Eq)]
struct ScopeContext {
    scope: crate::foundation::RuntimeScopeRefV1,
    generation: u64,
    content: [u8; 32],
    map: [u8; 32],
    frame: [u8; 32],
}
fn context(runtime: &ChannelRuntimeV1) -> Result<ScopeContext, Error> {
    runtime.owner_fence().map_err(|_| Error::Stale)?;
    let b = runtime.binding();
    let p = runtime.content_pin();
    Ok(ScopeContext {
        scope: b.runtime_scope(),
        generation: b.scope_generation().get(),
        content: p.server_artifact_digest(),
        map: p.map_revision_digest(),
        frame: p.frame_binding_digest(),
    })
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Stale,
    UnknownFrame,
    InvalidIndex,
    Bound,
    Source,
    Wire,
}
/// A turn is captured once, then fanned out to all independently current observers.
/// Source keys and occurrence provenance stay here; only cue/client ids enter encoded bytes.
#[derive(Debug)]
pub struct CandidatePresentationTurn {
    context: ScopeContext,
    sync: u64,
    events: Vec<CommittedPresentation>,
}
impl CandidatePresentationTurn {
    pub fn take(
        owner: &mut SpellPresentationOwner,
        runtime: &ChannelRuntimeV1,
        sync: u64,
    ) -> Result<Self, Error> {
        if sync == 0 {
            return Err(Error::Bound);
        }
        let context = context(runtime)?;
        let events = owner
            .take_for_current_publisher(runtime)
            .map_err(|_| Error::Stale)?;
        Ok(Self {
            context,
            sync,
            events,
        })
    }
}
pub struct ObserverSource<'a> {
    pub root: &'a DurabilityRoot,
    pub scope: &'a SpellItemScopeAuthority,
    pub runtime: &'a ChannelRuntimeV1,
    pub index: &'a InterestIndex,
    pub settings: VisibilitySettings,
    pub actor: ExactActorRef,
    pub session: GameSessionId,
}
#[derive(Debug)]
struct Observer {
    context: ScopeContext,
    actor: ExactActorRef,
    session: GameSessionId,
    connection: u64,
    position: MovementLocalPosition,
    visible: VisibilityPosition,
}
fn owned_observer(
    runtime: &ChannelRuntimeV1,
    index: &InterestIndex,
    actor: ExactActorRef,
    session: GameSessionId,
    connection: u64,
) -> Result<Observer, Error> {
    if connection == 0 {
        return Err(Error::Stale);
    }
    runtime
        .player_control_facts(actor, session)
        .map_err(|_| Error::Stale)?;
    let position = runtime
        .read_actor_position(actor)
        .map_err(|_| Error::Stale)?
        .position();
    let visible = view(position)?;
    let entry = index
        .get(&actor.placement_identity())
        .ok_or(Error::InvalidIndex)?;
    if entry.position != visible {
        return Err(Error::InvalidIndex);
    }
    Ok(Observer {
        context: context(runtime)?,
        actor,
        session,
        connection,
        position,
        visible,
    })
}
async fn current_observer(
    tx: &mut Transaction<'_, Postgres>,
    source: &ObserverSource<'_>,
) -> Result<Observer, SpellItemError> {
    crate::durability::admission_journal::party_target_binding::current_colocated_party_character_for_scope_in_transaction(
  tx,source.root,source.scope,source.runtime,source.actor,source.session,
 ).await?.ok_or(SpellItemError::Rejected("presentation current canonical observer unavailable"))?;
    // The canonical factory above locked and checked this exact current row independently.
    let generation = crate::durability::admission_journal::party_target_binding::read_connection_generation_for_share_in_transaction(
        tx, source.session,
    ).await?;
    let generation = generation
        .parse()
        .map_err(|_| SpellItemError::Rejected("presentation connection generation"))?;
    owned_observer(
        source.runtime,
        source.index,
        source.actor,
        source.session,
        generation,
    )
    .map_err(|_| SpellItemError::Rejected("presentation current VIS2 observer unavailable"))
}
fn view(p: MovementLocalPosition) -> Result<VisibilityPosition, Error> {
    VisibilityPosition::new(p.x, p.y, p.floor).map_err(|_| Error::UnknownFrame)
}
fn pos(p: MovementLocalPosition) -> ActorPosition {
    ActorPosition {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}
/// D87's floor distance and projected plane distance, on Tibia floors so a native floor ranks
/// like its Tibia floor (see [`VisibilityPosition`]).
fn plane_distance(
    observer: MovementLocalPosition,
    event: MovementLocalPosition,
) -> Result<(i64, i64), Error> {
    let floor = |p: MovementLocalPosition| {
        tibia_floor(p.floor)
            .map(i64::from)
            .ok_or(Error::UnknownFrame)
    };
    let floor_offset = floor(observer)? - floor(event)?;
    let dx = (i64::from(event.x) - floor_offset - i64::from(observer.x)).abs();
    let dy = (i64::from(event.y) - floor_offset - i64::from(observer.y)).abs();
    Ok((floor_offset.abs(), dx.max(dy)))
}
fn selected_events(
    turn: &CandidatePresentationTurn,
    observer: &Observer,
    settings: VisibilitySettings,
) -> Result<Vec<wire::Event>, Error> {
    if turn.context != observer.context {
        return Err(Error::Stale);
    }
    let mut selected = Vec::new();
    for event in &turn.events {
        let target = view(event.event.position)?;
        let origin = view(event.cause.origin())?;
        let visible = match event.event.cue {
            Cue::Projectile(_) => {
                settings.can_see(observer.visible, target)
                    || settings.can_see(observer.visible, origin)
            }
            _ => settings.can_see(observer.visible, target),
        };
        if !visible {
            continue;
        }
        let own = event.cause.actor() == observer.actor;
        let source = if own {
            wire::Source::Own
        } else {
            match event.cause.as_ref() {
                PresentationCause::Cast { .. } => wire::Source::Others,
                PresentationCause::FamiliarDefense { .. } => wire::Source::Creatures,
            }
        };
        let body = match event.event.cue {
            Cue::Effect(id) => wire::Body::Effect {
                position: pos(event.event.position),
                id,
                source,
            },
            Cue::Projectile(id) => wire::Body::Missile {
                from: pos(event.cause.origin()),
                to: pos(event.event.position),
                id,
                source,
            },
            Cue::Sound(id) => wire::Body::Sound {
                position: pos(event.event.position),
                id,
                source,
                secondary: None,
            },
        };
        // D87's projected plane distance, then actual owner decision/emission order. Tile events
        // have no entity identity, so no invented identity or container iteration tie-break is used.
        let (df, plane) = plane_distance(observer.position, event.event.position)?;
        selected.push((
            (
                !own,
                df,
                plane,
                event.decision_ordinal,
                event.emission_ordinal,
            ),
            wire::Event {
                decision_ordinal: event.decision_ordinal,
                emission_ordinal: event.emission_ordinal,
                body,
            },
        ));
    }
    selected.sort_by_key(|(key, _)| *key);
    selected.truncate(wire::MAX_EVENTS);
    // Stable source order on delivery; retention priority affects selection, not owner order.
    selected.sort_by_key(|(_, event)| (event.decision_ordinal, event.emission_ordinal));
    Ok(selected.into_iter().map(|(_, event)| event).collect())
}
#[derive(Debug)]
pub struct CandidateBatch {
    pub base_revision: u64,
    pub revision: u64,
    pub bytes: Vec<u8>,
}
/// Candidate-only egress owner. Existing snapshot begin/commit fields drive its barrier;
/// no caller 'is-current' boolean or fabricated current-session fact is accepted.
#[derive(Debug, Default)]
pub struct CandidateSessionPublisher {
    binding: Option<(ScopeContext, GameSessionId, u64)>,
    revision: u64,
    pending: VecDeque<CandidateBatch>,
    snapshot: Option<u64>,
    last_turn: Option<(ScopeContext, u64, Vec<u64>)>,
    condition_revision: u64,
    condition_pending: VecDeque<(
        CandidateBatch,
        super::condition_snapshots::PreparedConditionSnapshot,
    )>,
    last_condition: Option<oteryn_protocol_oteryn::actor_condition_snapshot_candidate::Snapshot>,
}
impl CandidateSessionPublisher {
    pub fn from_retained_revision(revision: u64) -> Self {
        Self {
            revision,
            ..Self::default()
        }
    }
    pub fn snapshot_begin(
        &mut self,
        begin: &oteryn_protocol_oteryn::SnapshotBeginFields,
    ) -> Result<(), Error> {
        if begin.snapshot_id == 0 || self.snapshot.is_some() {
            return Err(Error::Stale);
        }
        self.snapshot = Some(begin.snapshot_id);
        self.pending.clear();
        self.condition_pending.clear();
        self.last_condition = None;
        Ok(())
    }
    pub fn snapshot_commit(&mut self, id: u64) -> Result<(), Error> {
        if self.snapshot != Some(id) {
            return Err(Error::Stale);
        }
        self.snapshot = None;
        Ok(())
    }
    pub async fn publish_current(
        &mut self,
        tx: &mut Transaction<'_, Postgres>,
        source: &ObserverSource<'_>,
        turn: &CandidatePresentationTurn,
    ) -> Result<bool, SpellItemError> {
        let observer = current_observer(tx, source).await?;
        self.publish_qualified(&observer, source.settings, turn)
            .map_err(|_| SpellItemError::Rejected("presentation candidate delivery qualification"))
    }
    /// Current condition snapshots have their own candidate body and queue. The
    /// canonical current observer and existing VIS2 audience remain authoritative.
    /// No routing number, renderer support or signed-frame conversion is assumed.
    pub async fn publish_current_conditions(
        &mut self,
        tx: &mut Transaction<'_, Postgres>,
        source: &ObserverSource<'_>,
        prepared: &super::condition_snapshots::PreparedConditionSnapshot,
        states: &super::actor_spell::ChannelSpellStates,
        current: &crate::spell::owned_cast_facts::OwnedCastFacts,
        now_us: u64,
    ) -> Result<bool, SpellItemError> {
        let observer = current_observer(tx, source).await?;
        prepared
            .validate_current(source.runtime, states, current, now_us)
            .map_err(|_| SpellItemError::Rejected("condition snapshot current subject changed"))?;
        let snapshot = prepared.snapshot();
        if observer.context.content != snapshot.content
            || observer.context.map != snapshot.map
            || observer.context.frame != snapshot.frame
        {
            return Err(SpellItemError::Rejected(
                "condition snapshot active frame changed",
            ));
        }
        let subject = view(MovementLocalPosition {
            x: snapshot.x,
            y: snapshot.y,
            floor: snapshot.floor,
        })
        .map_err(|_| {
            SpellItemError::Rejected("condition snapshot native frame not qualified by VIS2")
        })?;
        if source
            .index
            .get(&snapshot.entity.identity)
            .is_none_or(|entry| entry.position != subject)
            || !source.settings.can_see(observer.visible, subject)
        {
            return Err(SpellItemError::Rejected(
                "condition snapshot current audience unavailable",
            ));
        }
        let binding = (
            observer.context.clone(),
            observer.session,
            observer.connection,
        );
        if self.binding.as_ref() != Some(&binding) {
            self.pending.clear();
            self.condition_pending.clear();
            self.last_condition = None;
            self.binding = Some(binding);
        }
        if self.snapshot.is_some() || self.condition_pending.len() >= 2 {
            return Ok(false);
        }
        if let Some(previous) = &self.last_condition {
            let mut effective = previous.clone();
            effective.owner_time_us = snapshot.owner_time_us;
            if &effective == snapshot {
                return Ok(false);
            }
        }
        let revision = self
            .condition_revision
            .checked_add(1)
            .ok_or(SpellItemError::Rejected(
                "condition snapshot revision exhausted",
            ))?;
        self.condition_pending
            .try_reserve(1)
            .map_err(|_| SpellItemError::Rejected("condition snapshot queue capacity"))?;
        self.condition_pending.push_back((
            CandidateBatch {
                base_revision: self.condition_revision,
                revision,
                bytes: prepared.bytes().to_vec(),
            },
            prepared.clone(),
        ));
        self.condition_revision = revision;
        self.last_condition = Some(snapshot.clone());
        Ok(true)
    }
    /// The transport re-reads both observer and subject independently; a queued
    /// pre-expiry light/appearance body cannot authorize a later stale send.
    pub async fn pop_current_condition_transport(
        &mut self,
        tx: &mut Transaction<'_, Postgres>,
        source: &ObserverSource<'_>,
        states: &super::actor_spell::ChannelSpellStates,
        current: &crate::spell::owned_cast_facts::OwnedCastFacts,
        now_us: u64,
    ) -> Result<Option<CandidateBatch>, SpellItemError> {
        let observer = match current_observer(tx, source).await {
            Ok(value) => value,
            Err(error) => {
                self.condition_pending.clear();
                self.last_condition = None;
                return Err(error);
            }
        };
        if self.binding.as_ref()
            != Some(&(
                observer.context.clone(),
                observer.session,
                observer.connection,
            ))
            || self.snapshot.is_some()
        {
            self.condition_pending.clear();
            self.last_condition = None;
            return Ok(None);
        }
        let Some((_, proof)) = self.condition_pending.front() else {
            return Ok(None);
        };
        let s = proof.snapshot();
        let position = view(MovementLocalPosition {
            x: s.x,
            y: s.y,
            floor: s.floor,
        })
        .map_err(|_| {
            SpellItemError::Rejected("condition transport native frame not qualified by VIS2")
        })?;
        if proof
            .validate_current(source.runtime, states, current, now_us)
            .is_err()
            || source
                .index
                .get(&s.entity.identity)
                .is_none_or(|entry| entry.position != position)
            || !source.settings.can_see(observer.visible, position)
        {
            self.condition_pending.clear();
            self.last_condition = None;
            return Err(SpellItemError::Rejected(
                "condition transport current subject or audience changed",
            ));
        }
        Ok(self.condition_pending.pop_front().map(|(batch, _)| batch))
    }
    fn publish_qualified(
        &mut self,
        observer: &Observer,
        settings: VisibilitySettings,
        turn: &CandidatePresentationTurn,
    ) -> Result<bool, Error> {
        if turn.context != observer.context {
            return Err(Error::Stale);
        }
        let sequences: Vec<_> = turn
            .events
            .iter()
            .map(|event| event.emission_sequence)
            .collect();
        if let Some((previous, sync, original)) = &self.last_turn
            && previous == &turn.context
        {
            if turn.sync < *sync {
                return Err(Error::Stale);
            }
            if turn.sync == *sync {
                return if *original == sequences {
                    Ok(false)
                } else {
                    Err(Error::Stale)
                };
            }
        }
        self.last_turn = Some((turn.context.clone(), turn.sync, sequences));
        let binding = (
            observer.context.clone(),
            observer.session,
            observer.connection,
        );
        if self.binding.as_ref() != Some(&binding) {
            self.pending.clear();
            self.condition_pending.clear();
            self.last_condition = None;
            self.binding = Some(binding);
        }
        if self.snapshot.is_some() || self.pending.len() >= 2 {
            return Ok(false);
        }
        let events = selected_events(turn, observer, settings)?;
        if events.is_empty() {
            return Ok(false);
        }
        let bytes = wire::encode_batch(&wire::Batch {
            content: observer.context.content,
            sync_unit: turn.sync,
            events,
        })
        .map_err(|_| Error::Wire)?;
        let revision = self.revision.checked_add(1).ok_or(Error::Bound)?;
        self.pending.push_back(CandidateBatch {
            base_revision: self.revision,
            revision,
            bytes,
        });
        self.revision = revision;
        Ok(true)
    }
    pub async fn pop_for_current_transport(
        &mut self,
        tx: &mut Transaction<'_, Postgres>,
        source: &ObserverSource<'_>,
    ) -> Result<Option<CandidateBatch>, SpellItemError> {
        let observer = match current_observer(tx, source).await {
            Ok(observer) => observer,
            Err(error) => {
                self.pending.clear();
                return Err(error);
            }
        };
        Ok(self.pop_qualified(&observer))
    }
    fn pop_qualified(&mut self, observer: &Observer) -> Option<CandidateBatch> {
        let binding = (
            observer.context.clone(),
            observer.session,
            observer.connection,
        );
        if self.binding.as_ref() != Some(&binding) || self.snapshot.is_some() {
            self.pending.clear();
            self.condition_pending.clear();
            self.last_condition = None;
            return None;
        }
        self.pending.pop_front()
    }
}
#[path = "ambient_sounds.rs"]
pub(super) mod ambient_sounds;

#[cfg(test)]
#[path = "spell_presentation_publisher_tests.rs"]
mod tests;

#[path = "spell_presentation_cooldowns.rs"]
mod cooldowns;

/// Bounded production fan-out journal. Connections keep independent cursors.
#[derive(Debug, Default)]
pub struct SoundPublisher {
    revision: u64,
    binding: Option<ScopeContext>,
    sounds: VecDeque<SoundEntry>,
    ambient: std::collections::BTreeMap<GameSessionId, (ExactActorRef, u8, u16, u64)>,
}
#[derive(Debug)]
struct SoundEntry {
    revision: u64,
    actor: ExactActorRef,
    position: MovementLocalPosition,
    cue: u16,
    secondary: u16,
    creature: bool,
    effect: bool,
    private: bool,
    house_scope: Option<(crate::foundation::RuntimeScopeRefV1, u64)>,
}
impl SoundPublisher {
    /// Channel outbox ownership does not change when its caster enters a House
    /// before the publisher drains the committed events.
    pub fn record(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        position: MovementLocalPosition,
        cue: u16,
        secondary: u16,
    ) {
        let Ok(binding) = context(runtime) else {
            return;
        };
        if self.binding.as_ref() != Some(&binding) {
            self.sounds.clear();
            self.ambient.clear();
            self.binding = Some(binding);
        }
        let (cue, secondary) = if cue == 0 {
            (secondary, 0)
        } else {
            (cue, secondary)
        };
        if cue == 0 {
            return;
        }
        let Some(revision) = self.revision.checked_add(1) else {
            return;
        };
        self.revision = revision;
        if self.sounds.len() == oteryn_protocol_oteryn::presentation::MAX_PRESENTATION_EVENTS {
            self.sounds.pop_front();
        }
        self.sounds.push_back(SoundEntry {
            revision,
            actor,
            position,
            cue,
            secondary,
            creature: runtime.companion_snapshot_including_dead(actor).is_ok(),
            effect: false,
            private: false,
            house_scope: None,
        });
    }
    pub(super) fn record_item_effect(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        position: MovementLocalPosition,
        cue: u16,
        private: bool,
    ) {
        if cue == 0 {
            return;
        }
        let before = self.revision;
        self.record(runtime, actor, position, cue, 0);
        if self.revision != before
            && let Some(entry) = self.sounds.back_mut()
        {
            entry.effect = true;
            entry.private = private;
        }
    }
    /// Crystal 201189f `data-global/scripts/actions/other/fish_tank.lua:3-5` emits at
    /// the item tile. The existing journal retains its actual named-House owner.
    pub(super) fn record_house_item_effect(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        position: MovementLocalPosition,
        cue: u16,
    ) {
        if !runtime
            .house_player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return;
        }
        let Ok(house) = runtime.house_actor_state(actor, session) else {
            return;
        };
        let before = self.revision;
        self.record_item_effect(runtime, actor, position, cue, false);
        if self.revision != before
            && let Some(entry) = self.sounds.back_mut()
        {
            entry.house_scope = Some((house.scope, house.generation.get()));
        }
    }
    pub fn capture(
        &mut self,
        owner: &mut SpellPresentationOwner,
        runtime: &ChannelRuntimeV1,
    ) -> Result<(), Error> {
        for event in owner
            .take_for_current_publisher(runtime)
            .map_err(|_| Error::Stale)?
        {
            if let Cue::Sound(cue) = event.event.cue {
                self.record(runtime, event.cause.actor(), event.event.position, cue, 0);
            }
        }
        Ok(())
    }
    /// The two independently negotiated domains retain their own bounds: at most
    /// MAX_PRESENTATION_EVENTS positional sounds plus one private ambient replacement.
    pub fn observe(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        after: u64,
        selected: bool,
    ) -> Option<(
        u64,
        Vec<oteryn_protocol_oteryn::presentation::PresentationEvent>,
    )> {
        use oteryn_protocol_oteryn::presentation::{
            PresentationEvent, PresentationEventKind, PresentationSource,
        };
        if !selected {
            return Some((after, Vec::new()));
        }
        if self
            .binding
            .as_ref()
            .is_some_and(|binding| context(runtime).ok().as_ref() != Some(binding))
        {
            return Some((self.revision, Vec::new()));
        }
        runtime.owner_fence().ok()?;
        let (position, house_scope) = if let Ok(house) = runtime.house_actor_state(actor, session) {
            runtime.house_player_control_facts(actor, session).ok()?;
            (
                MovementLocalPosition {
                    x: house.position.x,
                    y: house.position.y,
                    floor: house.position.floor,
                },
                Some((house.scope, house.generation.get())),
            )
        } else {
            runtime.player_control_facts(actor, session).ok()?;
            (runtime.read_actor_position(actor).ok()?.position(), None)
        };
        let observer = view(position).ok()?;
        let mut events = Vec::new();
        // The current private loop survives overflow of the transient journal.
        if let Some((subject, _, cue, revision)) = self.ambient.get(&session)
            && house_scope.is_none()
            && *subject == actor
            && *revision > after
        {
            events.push(PresentationEvent {
                decision_ordinal: 1,
                emission_ordinal: 1,
                kind: PresentationEventKind::AmbientSound {
                    cue_id: u32::from(*cue),
                },
            });
        }
        for sound in self.sounds.iter().filter(|sound| sound.revision > after) {
            if sound.house_scope != house_scope || (sound.private && sound.actor != actor) {
                continue;
            }
            let Ok(target) = view(sound.position) else {
                continue;
            };
            if !VisibilitySettings::REFERENCE.can_see(observer, target) {
                continue;
            }
            let source = if sound.actor == actor {
                PresentationSource::Own
            } else if sound.creature {
                PresentationSource::Creatures
            } else {
                PresentationSource::Others
            };
            events.push(PresentationEvent {
                decision_ordinal: u32::try_from(events.len() + 1).ok()?,
                emission_ordinal: 1,
                kind: if sound.effect {
                    PresentationEventKind::MagicEffect {
                        position: ActorPosition {
                            floor: tibia_floor(sound.position.floor)?,
                            ..pos(sound.position)
                        },
                        effect_id: u32::from(sound.cue),
                        source,
                    }
                } else {
                    PresentationEventKind::Sound {
                        position: ActorPosition {
                            floor: tibia_floor(sound.position.floor)?,
                            ..pos(sound.position)
                        },
                        cue_id: u32::from(sound.cue),
                        source,
                        secondary_cue_id: u32::from(sound.secondary),
                    }
                },
            });
        }
        Some((self.revision, events))
    }
}
