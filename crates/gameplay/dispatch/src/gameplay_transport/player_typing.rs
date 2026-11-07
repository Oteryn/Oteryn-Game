//! S19: ephemeral typing broadcasts under the existing Channel and chat owners.
//!
//! Crystal 201189f8 game.cpp:13074-13084 selects player spectators, including the
//! speaker, with multifloor geometry from map/spectators.cpp:103-116,157-160,180-184.
use std::collections::HashMap;
use std::num::NonZeroU64;
use std::sync::Mutex;

use oteryn_protocol_oteryn::chat::ChatSpeaker;
use oteryn_protocol_oteryn::player_typing::{CAPABILITY_PLAYER_TYPING_V1, TypingUpdate};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;

use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};

macro_rules! authority_methods {
    () => {
        fn player_typing_admit(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
        ) -> impl std::future::Future<Output = ()> {
            async {}
        }
        fn player_typing(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _typing: bool,
        ) -> impl std::future::Future<Output = bool> {
            async { false }
        }
        fn player_typing_next(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
        ) -> impl std::future::Future<Output = Option<oteryn_protocol_oteryn::player_typing::TypingUpdate>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;

macro_rules! composed_methods {
    () => {
        async fn player_typing_admit(
            &self,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
        ) {
            let Some(actor) = admitted.runtime_actor else {
                return;
            };
            let runtime = self.runtime.lock().await;
            self.states.chat.typing.admit(
                &runtime,
                actor,
                admitted.game_session_id,
                admitted.continuity.connection_generation,
                admitted
                    .continuity
                    .selected_capabilities
                    .contains(oteryn_protocol_oteryn::player_typing::CAPABILITY_PLAYER_TYPING_V1),
            );
        }
        async fn player_typing(
            &self,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
            typing: bool,
        ) -> bool {
            $crate::gameplay_transport::player_typing::execute(self, admitted, typing).await
        }
        async fn player_typing_next(
            &self,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
        ) -> Option<oteryn_protocol_oteryn::player_typing::TypingUpdate> {
            self.states.chat.typing.next(
                admitted.game_session_id,
                admitted.continuity.connection_generation,
            )
        }
    };
}
pub(super) use composed_methods;

struct TypingSession {
    connection_generation: u64,
    selected: bool,
    /// At most one unsent state per selected live speaker. The session capacity
    /// therefore bounds this map, even if a recipient stops consuming updates.
    pending: HashMap<GameSessionId, TypingUpdate>,
}

#[derive(Default)]
pub(super) struct TypingRuntime(Mutex<HashMap<GameSessionId, TypingSession>>);

impl TypingRuntime {
    /// The caller holds the Channel runtime lock through selection. Terminal
    /// retirement removes the actor under that lock before forgetting this entry.
    pub(super) fn admit(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        generation: u64,
        selected: bool,
    ) {
        if runtime
            .player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
        {
            self.select(session, generation, selected);
        }
    }

    fn select(&self, session: GameSessionId, generation: u64, selected: bool) {
        if generation == 0 {
            return;
        }
        let Ok(mut sessions) = self.0.lock() else {
            return;
        };
        if sessions
            .get(&session)
            .is_some_and(|entry| entry.connection_generation > generation)
        {
            return;
        }
        sessions.remove(&session);
        for entry in sessions.values_mut() {
            entry.pending.remove(&session);
        }
        // An unselected admission still fences delayed older snapshots. One
        // entry per current admitted actor stays bounded until terminal forget.
        sessions.insert(
            session,
            TypingSession {
                connection_generation: generation,
                selected,
                pending: HashMap::new(),
            },
        );
    }

    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut sessions) = self.0.lock() {
            sessions.remove(&session);
            for entry in sessions.values_mut() {
                entry.pending.remove(&session);
            }
        }
    }

    pub(super) fn next(&self, session: GameSessionId, generation: u64) -> Option<TypingUpdate> {
        let mut sessions = self.0.lock().ok()?;
        let entry = sessions.get_mut(&session)?;
        if !entry.selected || entry.connection_generation != generation {
            return None;
        }
        let speaker = entry.pending.keys().next().copied()?;
        entry.pending.remove(&speaker)
    }

    /// Called under the Channel runtime lock. No lock here crosses an await.
    pub(super) fn broadcast(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        generation: u64,
        typing: bool,
    ) -> bool {
        if !runtime
            .player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return false;
        }
        let Some((bound_actor, position)) = runtime
            .positioned_player_for_session(session)
            .ok()
            .flatten()
        else {
            return false;
        };
        if bound_actor != actor {
            return false;
        }
        let Some(actor_generation) = runtime
            .visible_entities()
            .players
            .iter()
            .find(|entry| entry.actor == actor)
            .and_then(|entry| NonZeroU64::new(entry.generation))
        else {
            return false;
        };
        let position = position.position();
        let source = ActorPosition {
            x: position.x,
            y: position.y,
            floor: position.floor,
        };
        let update = TypingUpdate {
            speaker: ChatSpeaker {
                identity: actor.placement_identity(),
                generation: actor_generation,
            },
            typing,
        };
        let Ok(mut sessions) = self.0.lock() else {
            return false;
        };
        if !sessions
            .get(&session)
            .is_some_and(|entry| entry.selected && entry.connection_generation == generation)
        {
            return false;
        }
        for (recipient, entry) in sessions.iter_mut() {
            if !entry.selected {
                continue;
            }
            let Some((_, position)) = runtime
                .positioned_player_for_session(*recipient)
                .ok()
                .flatten()
            else {
                continue;
            };
            let position = position.position();
            if observes(
                source,
                ActorPosition {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                },
            ) {
                entry.pending.insert(session, update);
            }
        }
        true
    }
}

/// Donor map_const.hpp:20-27: viewport (8+3,6+5), surface 7, floor span 2,
/// 16 floors. Spectators use the speaker's floor, not the observer's viewport.
fn observes(source: ActorPosition, observer: ActorPosition) -> bool {
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
        && (i64::from(observer.x) - offset - i64::from(source.x)).abs() <= 11
        && (i64::from(observer.y) - offset - i64::from(source.y)).abs() <= 11
}

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    typing: bool,
) -> bool {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_PLAYER_TYPING_V1)
    {
        return false;
    }
    let Some(actor) = admitted.runtime_actor else {
        return false;
    };
    let runtime = owner.runtime.lock().await;
    owner.states.chat.typing.broadcast(
        &runtime,
        actor,
        admitted.game_session_id,
        admitted.continuity.connection_generation,
        typing,
    )
}

#[cfg(test)]
#[path = "player_typing_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "player_typing_wire_tests.rs"]
mod wire_tests;
