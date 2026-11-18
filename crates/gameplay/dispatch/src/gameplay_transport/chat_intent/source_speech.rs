//! The source portal speaks privately as each current spectator at the room centre.
use super::*;

impl ChatRuntime {
    pub(in crate::gameplay_transport) fn publish_rotten_massive_warning(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        position: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    ) {
        if runtime.player_control_facts(actor, session).is_err() {
            return;
        }
        let Some(generation) = NonZeroU64::new(actor.actor_local_generation()) else {
            return;
        };
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let Some(entry) = sessions.get_mut(&session) else {
            return;
        };
        entry.egress.push(ChatLine::Local {
            speaker: ChatSpeaker { identity: actor.placement_identity(), generation },
            speaker_name: entry.name.clone(),
            text: "the discharge of energy weakened the resistance of the boss shortly! PREPARE FOR 2 MASSIVE ATTACKS ON THE MAIN TARGET!".into(),
            mode: ChatSpeechMode::MonsterSay,
            position,
        });
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn oberon_reply(
        &self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        text: &str,
        now_seconds: u64,
    ) {
        let chat_position = |at: crate::foundation::MovementLocalPosition| {
            Some(ChatPosition {
                x: at.x,
                y: at.y,
                floor: u8::try_from(at.floor).ok()?,
            })
        };
        let Some(speaker) = runtime
            .read_actor_position(actor)
            .ok()
            .and_then(|position| chat_position(position.position()))
        else {
            return;
        };
        let answers = oteryn_gameplay_encounters::bosses::phase_controller::oberon::say(
            runtime,
            text,
            now_seconds,
            |at| {
                chat_position(at).is_some_and(|listener| {
                    crate::chat::hears(SpeechMode::Say, speaker, listener).is_some()
                })
            },
        );
        if let Some(map) = self.bundle_map() {
            for (boss, text) in answers {
                let Some(position) = runtime.read_actor_position(boss).ok().and_then(|at| {
                    let at = at.position();
                    Some(crate::map::overlay::TilePos {
                        x: u16::try_from(at.x).ok()?,
                        y: u16::try_from(at.y).ok()?,
                        floor: i8::try_from(at.floor.checked_neg()?).ok()?,
                    })
                }) else {
                    continue;
                };
                let captured_name = oteryn_protocol_oteryn::entity_names::EntityDisplayName::new(
                    "Grand Master Oberon",
                )
                .ok();
                map.facts
                    .emit_movement_speech(crate::map::facts::MovementSpeech {
                        speaker: boss,
                        position,
                        talk_type: 36,
                        text: text.into(),
                        captured_name,
                    });
            }
        }
    }
}
