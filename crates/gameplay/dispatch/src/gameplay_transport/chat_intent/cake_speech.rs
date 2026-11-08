//! Crystal data-global/scripts/quests/a_piece_of_cake/movements_extraction_spot.lua:38
//! speaks from the real extracted monster, before its administrative removal.
use super::*;

impl ChatRuntime {
    pub(in crate::gameplay_transport) fn publish_cake_extraction_speech(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        position: crate::foundation::MovementLocalPosition,
    ) {
        if runtime.owner_fence().is_err()
            || !runtime.matches_live_creature_identity(actor, b"oteryn:creature.cake_golem")
            || !runtime
                .read_actor_position(actor)
                .is_ok_and(|current| current.position() == position)
        {
            return;
        }
        let (Some(generation), Ok(floor), Ok(text)) = (
            NonZeroU64::new(actor.actor_local_generation()),
            u8::try_from(position.floor),
            ChatText::parse("CRUNSH!"),
        ) else {
            return;
        };
        let at = ChatPosition {
            x: position.x,
            y: position.y,
            floor,
        };
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let listeners = sessions
            .keys()
            .filter_map(|session| {
                let (_, current) = runtime.positioned_player_for_session(*session).ok()??;
                let pos = current.position();
                Some((
                    *session,
                    ChatPosition {
                        x: pos.x,
                        y: pos.y,
                        floor: u8::try_from(pos.floor).ok()?,
                    },
                ))
            })
            .collect::<Vec<_>>();
        let speaker = ChatSpeaker {
            identity: actor.placement_identity(),
            generation,
        };
        for (listener, heard) in local_listeners(SpeechMode::Say, at, listeners) {
            if let Some(entry) = sessions.get_mut(&listener) {
                let mut line = local_line(speaker, "Cake Golem", SpeechMode::Say, &text, at, heard);
                if let ChatLine::Local { mode, .. } = &mut line {
                    *mode = ChatSpeechMode::MonsterSay;
                }
                entry.egress.push(line);
            }
        }
    }
}
