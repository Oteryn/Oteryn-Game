//! W18 named-House adapter of the SV8 magic-effect-only presentation codec.
//! Reservations precede the durable item await; only fresh committed receipts publish.
use crate::foundation::{
    ExactActorRef, GameSessionId, MovementLocalPosition, RuntimeScopeRefV1,
    ScopeOwnershipGeneration,
};
use oteryn_protocol_oteryn::presentation::{
    MAX_PRESENTATION_EVENTS, PresentationEvent, PresentationEventKind, PresentationSource,
};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub(in crate::gameplay_transport) struct Journal {
    revision: u64,
    reserved: u64,
    events: VecDeque<Entry>,
}
struct Entry {
    revision: u64,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    actor: ExactActorRef,
    position: MovementLocalPosition,
    effect: u16,
    binding: ([u8; 32], [u8; 32], u64),
}
pub(in crate::gameplay_transport) struct Reservation {
    journal: Arc<Mutex<Journal>>,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    actor: ExactActorRef,
    position: MovementLocalPosition,
    cues: Vec<u16>,
    binding: ([u8; 32], [u8; 32], u64),
    live: bool,
}
impl Journal {
    pub(super) fn reserve(
        journal: &Arc<Mutex<Self>>,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        actor: ExactActorRef,
        position: MovementLocalPosition,
        cues: Vec<u16>,
        binding: ([u8; 32], [u8; 32], u64),
    ) -> Option<Reservation> {
        if cues.len() > MAX_PRESENTATION_EVENTS || cues.contains(&0) {
            return None;
        }
        let mut held = journal.lock().ok()?;
        let reserved = held.reserved.checked_add(u64::try_from(cues.len()).ok()?)?;
        held.revision.checked_add(reserved)?;
        // This journal is bounded at MAX_PRESENTATION_EVENTS even while producers await DB.
        let need = MAX_PRESENTATION_EVENTS.saturating_sub(held.events.len());
        held.events.try_reserve(need).ok()?;
        held.reserved = reserved;
        drop(held);
        Some(Reservation {
            journal: Arc::clone(journal),
            scope,
            generation,
            actor,
            position,
            cues,
            binding,
            live: true,
        })
    }
    pub(in crate::gameplay_transport) fn observe(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        after: u64,
        binding: ([u8; 32], [u8; 32], u64),
    ) -> Option<(u64, Vec<PresentationEvent>)> {
        let observer = runtime.house_actor_state(actor, session).ok()?;
        if after == u64::MAX {
            return Some((self.revision, Vec::new()));
        }
        let observer_at = crate::movement::interest::VisibilityPosition::new(
            observer.position.x,
            observer.position.y,
            observer.position.floor,
        )
        .ok()?;
        let mut events = Vec::new();
        for event in self.events.iter().filter(|entry| {
            entry.revision > after
                && entry.scope == observer.scope
                && entry.generation == observer.generation
                && entry.binding == binding
                && entry.position.floor == observer.position.floor
        }) {
            let target = crate::movement::interest::VisibilityPosition::new(
                event.position.x,
                event.position.y,
                event.position.floor,
            )
            .ok()?;
            if !crate::movement::interest::VisibilitySettings::REFERENCE
                .can_see(observer_at, target)
            {
                continue;
            }
            events.push(PresentationEvent {
                decision_ordinal: u32::try_from(events.len() + 1).ok()?,
                emission_ordinal: 1,
                kind: PresentationEventKind::MagicEffect {
                    position: ActorPosition {
                        x: event.position.x,
                        y: event.position.y,
                        floor: crate::movement::interest::tibia_floor(event.position.floor)?,
                    },
                    effect_id: u32::from(event.effect),
                    source: if event.actor == actor {
                        PresentationSource::Own
                    } else {
                        PresentationSource::Others
                    },
                },
            });
        }
        Some((self.revision, events))
    }
}
impl Reservation {
    pub(super) fn commit(mut self) {
        if let Ok(mut held) = self.journal.lock() {
            for cue in &self.cues {
                // reserve proved revision+all outstanding cue counts is representable.
                held.revision += 1;
                let revision = held.revision;
                if held.events.len() == MAX_PRESENTATION_EVENTS {
                    held.events.pop_front();
                }
                held.events.push_back(Entry {
                    revision,
                    scope: self.scope,
                    generation: self.generation,
                    actor: self.actor,
                    position: self.position,
                    effect: *cue,
                    binding: self.binding,
                });
            }
            held.reserved -= self.cues.len() as u64;
            self.live = false;
        }
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        if self.live
            && let Ok(mut held) = self.journal.lock()
        {
            held.reserved -= self.cues.len() as u64;
        }
    }
}
