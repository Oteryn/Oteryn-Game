//! Crystal main@201189f871d23597b82caf88fd611454a2ddefb8:
//! data/scripts/creaturescripts/player/mount_sounds.lua:1-1185 (voices),1188-1211 (8s, 51%, mounted players).
//! Mount voices are speech, not sound-effect ids. Use the existing bounded local chat route.
//! Same-floor viewport delivery: src/game/game.cpp:6957-6969.
use super::ChatRuntime;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::{
    chat::{ChatLine, ChatSpeaker, ChatSpeechMode},
    world_spatial::ActorPosition,
};
use sha2::{Digest, Sha256};

#[path = "mount_voice_lines.rs"]
mod lines;

impl ChatRuntime {
    #[cfg(test)]
    pub(in crate::gameplay_transport) fn select_mount_sounds(
        &self,
        session: GameSessionId,
        selected: bool,
    ) {
        if let Ok(mut sessions) = self.sessions.lock()
            && let Some(state) = sessions.get_mut(&session)
        {
            state.mount_sounds = selected;
        }
    }

    pub(super) fn mount_voice_due(&self, session: GameSessionId, now: u64) -> bool {
        self.sessions.lock().is_ok_and(|sessions| {
            sessions.get(&session).is_some_and(|s| {
                s.mount_sounds
                    && s.last_mount_voice.is_none_or(|last| {
                        now.checked_sub(last)
                            .is_some_and(|delta| delta >= 8_000_000)
                    })
            })
        })
    }

    pub(super) fn mount_voice(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        look: u32,
        now: u64,
        draw: u64,
    ) {
        if runtime.owner_fence().is_err() || runtime.player_control_facts(actor, session).is_err() {
            return;
        }
        let Ok(position) = runtime.read_actor_position(actor) else {
            return;
        };
        let position = position.position();
        let Some(floor) = crate::movement::interest::tibia_floor(position.floor) else {
            return;
        };
        let Some(generation) = std::num::NonZeroU64::new(actor.actor_local_generation()) else {
            return;
        };
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let Some(speaker) = sessions.get_mut(&session) else {
            return;
        };
        if !speaker.mount_sounds {
            return;
        }
        if speaker
            .last_mount_voice
            .is_some_and(|last| now.checked_sub(last).is_none_or(|delta| delta < 8_000_000))
        {
            return;
        }
        speaker.last_mount_voice = Some(now);
        if draw % 100 + 1 < 50 {
            return;
        }
        let voices = lines::voices(look);
        if voices.is_empty() {
            return;
        }
        let text = voices[((draw / 100) % voices.len() as u64) as usize];
        let name = speaker.name.clone();
        for (listener, state) in sessions.iter_mut() {
            if !state.mount_sounds {
                continue;
            }
            let Ok(Some((_, at))) = runtime.positioned_player_for_session(*listener) else {
                continue;
            };
            let at = at.position();
            if at.floor != position.floor
                || at.x.abs_diff(position.x) > 8
                || at.y.abs_diff(position.y) > 6
            {
                continue;
            }
            state.egress.push(ChatLine::Local {
                speaker: ChatSpeaker {
                    identity: actor.placement_identity(),
                    generation,
                },
                speaker_name: name.clone(),
                mode: ChatSpeechMode::MonsterSay,
                text: text.to_owned(),
                position: ActorPosition {
                    x: position.x,
                    y: position.y,
                    floor,
                },
            });
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn emit_mount_voice(&self, admitted: AdmittedSession) {
        let Some(actor) = admitted.runtime_actor else {
            return;
        };
        let now = self.owner_now().get();
        if !self
            .states
            .chat
            .mount_voice_due(admitted.game_session_id, now)
        {
            return;
        }
        // oteryn-choice: optional appearance enrichment has a short deadline so a
        // blocked database read cannot hold the connection's movement/combat cadence.
        let Ok(Some(look)) = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            self.current_mount(actor, admitted.game_session_id),
        )
        .await
        else {
            return;
        };
        // oteryn-choice: a session/bucket draw replaces Lua's process-global random stream.
        let digest = Sha256::new()
            .chain_update(admitted.game_session_id.as_bytes())
            .chain_update((now / 8_000_000).to_le_bytes())
            .finalize();
        let mut bytes = [0; 8];
        bytes.copy_from_slice(&digest[..8]);
        let runtime = self.runtime.lock().await;
        self.states.chat.mount_voice(
            &runtime,
            actor,
            admitted.game_session_id,
            look,
            now,
            u64::from_le_bytes(bytes),
        );
    }
}
