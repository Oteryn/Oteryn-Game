//! Deliver the current creature's authored speech through the local chat outbox.
use super::*;
use crate::content::{ProjectV2SpeechMode, ProjectV2Voice};
use crate::foundation::MovementLocalPosition;
use oteryn_protocol_oteryn::chat::{MAX_CHAT_NAME_BYTES, MAX_CHAT_TEXT_BYTES};

impl ChatRuntime {
    pub(in crate::gameplay_transport) fn publish_monster_speech(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        voice: &ProjectV2Voice,
    ) {
        let (Ok(creature), Some(generation)) = (
            runtime.companion_snapshot(actor),
            NonZeroU64::new(actor.actor_local_generation()),
        ) else {
            return;
        };
        let Some(at) = chat_position(creature.position) else {
            return;
        };
        let name = &creature.state.policy.display_name;
        if name.is_empty()
            || name.len() > MAX_CHAT_NAME_BYTES
            || voice.text.is_empty()
            || voice.text.len() > MAX_CHAT_TEXT_BYTES
        {
            return;
        }
        let hearing = match voice.mode {
            ProjectV2SpeechMode::Say => SpeechMode::Say,
            ProjectV2SpeechMode::Yell => SpeechMode::Yell,
        };
        let Ok(listeners) = runtime.positioned_actor_census() else {
            return;
        };
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let speaker = ChatSpeaker {
            identity: actor.placement_identity(),
            generation,
        };
        for (_, listener, session) in listeners {
            let Some(state) = session.and_then(|session| sessions.get_mut(&session)) else {
                continue;
            };
            let Some(listener) = chat_position(listener.position()) else {
                continue;
            };
            if crate::chat::hears(hearing, at, listener).is_none() {
                continue;
            }
            state.egress.push(ChatLine::Local {
                speaker,
                speaker_name: name.clone(),
                // Both modes use the client’s existing monster speech presentation.
                mode: ChatSpeechMode::MonsterSay,
                text: voice.text.clone(),
                position: oteryn_protocol_oteryn::world_spatial::ActorPosition {
                    x: at.x,
                    y: at.y,
                    floor: i16::from(at.floor),
                },
            });
        }
    }
}

fn chat_position(position: MovementLocalPosition) -> Option<ChatPosition> {
    Some(ChatPosition {
        x: position.x,
        y: position.y,
        floor: u8::try_from(crate::movement::interest::tibia_floor(position.floor)?).ok()?,
    })
}
