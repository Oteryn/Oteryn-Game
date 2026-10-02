//! Unallocated SPELL-PRESENT-0 publisher candidate. It consumes the actual Channel outbox,
//! canonical same-TX current-session source and the existing VIS-2 index/settings. It does
//! not install a capability, emit a protocol envelope, query a second spatial index, or
//! infer a frame conversion for native floors outside the existing VIS-2 0..15 contract.
use super::spell_presentations::{
    CommittedPresentation, Cue, PresentationCause, SpellPresentationOwner,
};
use crate::durability::{
    DurabilityRoot,
    spell_item_transaction::{SpellItemError, SpellItemScopeAuthority},
};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::movement::interest::{InterestIndex, VisibilityPosition, VisibilitySettings};
use oteryn_protocol_oteryn::{spell_presentation_candidate as wire, world_spatial::ActorPosition};
use sqlx::{Postgres, Transaction};
use std::collections::VecDeque;
#[derive(Debug, Clone, PartialEq, Eq)]
struct ScopeContext {
    world: [u8; 16],
    channel: [u8; 16],
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
        world: *b.world_id().as_bytes(),
        channel: *b.channel_id().as_bytes(),
        generation: b.scope_generation().get(),
        content: p.server_artifact_digest(),
        map: p.map_revision_digest(),
        frame: p.frame_binding_digest(),
    })
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Error {
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
pub(crate) struct CandidatePresentationTurn {
    context: ScopeContext,
    sync: u64,
    events: Vec<CommittedPresentation>,
}
impl CandidatePresentationTurn {
    pub(crate) fn take(
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
pub(crate) struct ObserverSource<'a> {
    pub(crate) root: &'a DurabilityRoot,
    pub(crate) scope: &'a SpellItemScopeAuthority,
    pub(crate) runtime: &'a ChannelRuntimeV1,
    pub(crate) index: &'a InterestIndex,
    pub(crate) settings: VisibilitySettings,
    pub(crate) actor: ExactActorRef,
    pub(crate) session: GameSessionId,
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
    let generation:String=sqlx::query_scalar("SELECT current_generation::text FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid FOR SHARE")
  .bind(source.session.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
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
        let df = (i64::from(observer.position.floor) - i64::from(event.event.position.floor)).abs();
        let floor_offset =
            i64::from(observer.position.floor) - i64::from(event.event.position.floor);
        let dx =
            (i64::from(event.event.position.x) - floor_offset - i64::from(observer.position.x))
                .abs();
        let dy =
            (i64::from(event.event.position.y) - floor_offset - i64::from(observer.position.y))
                .abs();
        selected.push((
            (
                !own,
                df,
                dx.max(dy),
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
pub(crate) struct CandidateBatch {
    pub(crate) base_revision: u64,
    pub(crate) revision: u64,
    pub(crate) bytes: Vec<u8>,
}
/// Candidate-only egress owner. Existing snapshot begin/commit fields drive its barrier;
/// no caller 'is-current' boolean or fabricated current-session fact is accepted.
#[derive(Debug, Default)]
pub(crate) struct CandidateSessionPublisher {
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
    pub(crate) fn from_retained_revision(revision: u64) -> Self {
        Self {
            revision,
            ..Self::default()
        }
    }
    pub(crate) fn snapshot_begin(
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
    pub(crate) fn snapshot_commit(&mut self, id: u64) -> Result<(), Error> {
        if self.snapshot != Some(id) {
            return Err(Error::Stale);
        }
        self.snapshot = None;
        Ok(())
    }
    pub(crate) async fn publish_current(
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
    pub(crate) async fn publish_current_conditions(
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
    pub(crate) async fn pop_current_condition_transport(
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
        if let Some((previous, sync, original)) = &self.last_turn {
            if previous == &turn.context {
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
    pub(crate) async fn pop_for_current_transport(
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
#[cfg(test)]
#[path = "spell_presentation_publisher_tests.rs"]
mod tests;

#[path = "spell_presentation_cooldowns.rs"]
mod cooldowns;
pub(crate) use cooldowns::{CandidateCooldownPublisher, CooldownRead};
