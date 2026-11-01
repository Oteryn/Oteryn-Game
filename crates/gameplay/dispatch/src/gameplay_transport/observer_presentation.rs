//! PAR-M10: the one server emitter of observer presentation (capability 262), that is world
//! magic effects and monster speech, queued for the players who see them.
//!
//! Crystal 201189f8 `game.cpp:8950-8961` sends a magic effect to the player spectators of the
//! position with multifloor geometry (`map/spectators.cpp:98-190`, `map_const.hpp:20-23`, the
//! S19 typing geometry); `game.cpp:6941-6960` sends a say line (not a yell) to the spectators
//! on the speaker's floor within the client viewport, the existing local Say rule
//! (`foundation::speech::hears`). The events are transient: never stored, never replayed.
use std::collections::{BTreeMap, VecDeque};
use std::num::NonZeroU64;

use oteryn_protocol_oteryn::chat::ChatSpeaker;
use oteryn_protocol_oteryn::observer_presentation::ObserverPresentation;
use oteryn_protocol_oteryn::world_spatial::ActorPosition;

use super::actor_spell::ChannelSpellStates;
use crate::chat::{ChatPosition, SpeechMode, hears};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};

/// The unsent events each observer keeps; an observer that does not drain its queue (one that
/// did not select capability 262) loses the oldest, so the queue never grows past this.
pub(in crate::gameplay_transport) const MAX_PENDING_PRESENTATIONS: usize = 32;
/// `map_const.hpp:22-23`: `MAP_MAX_VIEW_PORT_X/Y` bound a spectator search.
const VIEW_RANGE: i64 = 11;

/// The queued events of each live player session of this Channel.
#[derive(Debug, Default)]
pub(in crate::gameplay_transport) struct ObserverPresentations(
    BTreeMap<GameSessionId, VecDeque<ObserverPresentation>>,
);

impl ObserverPresentations {
    /// The oldest event queued for `session`.
    pub(in crate::gameplay_transport) fn next(
        &mut self,
        session: GameSessionId,
    ) -> Option<ObserverPresentation> {
        let queue = self.0.get_mut(&session)?;
        let next = queue.pop_front();
        if queue.is_empty() {
            self.0.remove(&session);
        }
        next
    }

    fn push(&mut self, session: GameSessionId, event: ObserverPresentation) {
        let queue = self.0.entry(session).or_default();
        if queue.len() == MAX_PENDING_PRESENTATIONS {
            queue.pop_front();
        }
        queue.push_back(event);
    }
}

fn actor_position(position: MovementLocalPosition) -> ActorPosition {
    ActorPosition {
        x: position.x,
        y: position.y,
        floor: position.floor,
    }
}

/// `spectators.cpp:98-160`: whether an observer at `observer` is a multifloor spectator of
/// `source`.
fn sees_effect(source: ActorPosition, observer: ActorPosition) -> bool {
    if !(0..=15).contains(&source.floor) || !(0..=15).contains(&observer.floor) {
        return false;
    }
    let (low, high) = match source.floor {
        8..=15 => ((source.floor - 2).max(0), (source.floor + 2).min(15)),
        6 => (0, 8),
        7 => (0, 9),
        _ => (0, 7),
    };
    let offset = i64::from(source.floor) - i64::from(observer.floor);
    (low..=high).contains(&observer.floor)
        && (i64::from(observer.x) - offset - i64::from(source.x)).abs() <= VIEW_RANGE
        && (i64::from(observer.y) - offset - i64::from(source.y)).abs() <= VIEW_RANGE
}

/// `game.cpp:6956-6958`: whether an observer hears a say line spoken at `source`.
fn hears_say(source: ActorPosition, observer: ActorPosition) -> bool {
    let at = |position: ActorPosition| {
        Some(ChatPosition {
            x: position.x,
            y: position.y,
            floor: u8::try_from(position.floor).ok()?,
        })
    };
    match (at(source), at(observer)) {
        (Some(source), Some(observer)) => hears(SpeechMode::Say, source, observer).is_some(),
        _ => false,
    }
}

impl ChannelSpellStates {
    /// `Position:sendMagicEffect`: queues the effect for every player that sees `at`. Crystal
    /// sends nothing for `CONST_ME_NONE` (0).
    pub(in crate::gameplay_transport) fn present_magic_effect(
        &mut self,
        runtime: &ChannelRuntimeV1,
        at: MovementLocalPosition,
        effect_id: u32,
    ) {
        if effect_id == 0 {
            return;
        }
        let position = actor_position(at);
        self.present(
            runtime,
            position,
            sees_effect,
            &ObserverPresentation::MagicEffect {
                position,
                effect_id,
            },
        );
    }

    /// `Creature:say(text, TALKTYPE_MONSTER_SAY)` by the visible player or creature `speaker`.
    pub(in crate::gameplay_transport) fn present_monster_say(
        &mut self,
        runtime: &ChannelRuntimeV1,
        speaker: ExactActorRef,
        text: &str,
    ) {
        let entities = runtime.visible_entities();
        let Some(visible) = entities
            .players
            .iter()
            .chain(&entities.creatures)
            .find(|entry| entry.actor == speaker)
        else {
            return;
        };
        let Some(generation) = NonZeroU64::new(visible.generation) else {
            return;
        };
        let position = actor_position(visible.position);
        self.present(
            runtime,
            position,
            hears_say,
            &ObserverPresentation::MonsterSay {
                speaker: ChatSpeaker {
                    identity: speaker.placement_identity(),
                    generation,
                },
                position,
                text: text.to_owned(),
            },
        );
    }

    fn present(
        &mut self,
        runtime: &ChannelRuntimeV1,
        source: ActorPosition,
        observes: fn(ActorPosition, ActorPosition) -> bool,
        event: &ObserverPresentation,
    ) {
        let players = runtime.visible_entities().players;
        // A session that left the Channel keeps no queue.
        let live = self
            .actors
            .iter()
            .map(|(_, session, _)| *session)
            .collect::<Vec<_>>();
        self.observer_presentations
            .0
            .retain(|session, _| live.contains(session));
        for (actor, session, _) in &self.actors {
            let Some(observer) = players.iter().find(|entry| entry.actor == *actor) else {
                continue;
            };
            if observes(source, actor_position(observer.position)) {
                self.observer_presentations.push(*session, event.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT: ActorPosition = ActorPosition {
        x: 100,
        y: 100,
        floor: 7,
    };

    #[test]
    fn m10_effects_reach_multifloor_spectators_and_say_only_the_floor_viewport() {
        let at = |x: i32, y: i32, floor: i16| ActorPosition { x, y, floor };
        assert!(sees_effect(AT, at(111, 89, 7)));
        assert!(!sees_effect(AT, at(112, 100, 7)));
        // Floor 7 sees down to floor 9, offset by the floor difference.
        assert!(sees_effect(AT, at(98, 98, 9)));
        assert!(!sees_effect(AT, at(100, 100, 10)));
        assert!(hears_say(AT, at(108, 94, 7)));
        assert!(!hears_say(AT, at(109, 100, 7)));
        assert!(!hears_say(AT, at(100, 107, 7)));
        assert!(!hears_say(AT, at(100, 100, 6)));
    }

    #[test]
    fn m10_an_undrained_observer_keeps_only_the_newest_events()
    -> Result<(), Box<dyn std::error::Error>> {
        // A UUIDv7-shaped session id.
        let Ok(session) =
            GameSessionId::decode(&[1, 0x90, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 3])
        else {
            return Err("session id".into());
        };
        let mut queue = ObserverPresentations::default();
        for effect_id in 1..=u32::try_from(MAX_PENDING_PRESENTATIONS + 2)? {
            queue.push(
                session,
                ObserverPresentation::MagicEffect {
                    position: AT,
                    effect_id,
                },
            );
        }
        assert_eq!(
            queue.next(session),
            Some(ObserverPresentation::MagicEffect {
                position: AT,
                effect_id: 3
            })
        );
        let mut rest = 1;
        while queue.next(session).is_some() {
            rest += 1;
        }
        assert_eq!(rest, MAX_PENDING_PRESENTATIONS);
        assert!(queue.0.is_empty());
        Ok(())
    }
}
