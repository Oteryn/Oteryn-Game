//! House-owned automatic item speech; current scope and visibility are checked per listener.
use super::*;
use oteryn_gameplay_items::novelty_items::NoveltyEffects;
impl ChatRuntime {
    pub(in crate::gameplay_transport) fn publish_house_item_speech(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        effects: &NoveltyEffects,
    ) {
        let Some(text) = effects.speech.as_ref() else {
            return;
        };
        let Ok(origin) = runtime.house_actor_state(actor, session) else {
            return;
        };
        let position = effects.ground.as_ref().map_or(origin.position, |at| {
            crate::foundation::LocalPosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            }
        });
        if position.floor != origin.position.floor
            || position.x.abs_diff(origin.position.x) > 1
            || position.y.abs_diff(origin.position.y) > 1
        {
            return;
        }
        if ChatText::parse(text).is_err() {
            return;
        }
        let (Some(generation), Ok(at), Some(floor)) = (
            NonZeroU64::new(actor.actor_local_generation()),
            crate::movement::interest::VisibilityPosition::new(
                position.x,
                position.y,
                position.floor,
            ),
            crate::movement::interest::tibia_floor(position.floor),
        ) else {
            return;
        };
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let Some(name) = sessions.get(&session).map(|state| state.name.clone()) else {
            return;
        };
        for (listener, state) in sessions.iter_mut() {
            if effects.private_speech && *listener != session {
                continue;
            }
            let Ok(Some((_, heard))) = runtime.house_player_for_session(*listener) else {
                continue;
            };
            if heard.scope != origin.scope
                || heard.generation != origin.generation
                || heard.position.floor != origin.position.floor
            {
                continue;
            }
            // Crystal decay_to.lua489-492 explicitly uses a3x3 spectator box.
            if matches!(effects.source_item_id, 23443 | 23708)
                && (heard.position.x.abs_diff(position.x) > 3
                    || heard.position.y.abs_diff(position.y) > 3)
            {
                continue;
            }
            let Ok(listener_position) = crate::movement::interest::VisibilityPosition::new(
                heard.position.x,
                heard.position.y,
                heard.position.floor,
            ) else {
                continue;
            };
            if !crate::movement::interest::VisibilitySettings::REFERENCE
                .can_see(listener_position, at)
            {
                continue;
            }
            state.egress.push(ChatLine::Local {
                speaker: ChatSpeaker {
                    identity: actor.placement_identity(),
                    generation,
                },
                speaker_name: name.clone(),
                mode: state.scripted_item_mode(),
                text: text.clone(),
                position: oteryn_protocol_oteryn::world_spatial::ActorPosition {
                    x: position.x,
                    y: position.y,
                    floor,
                },
            });
        }
    }
}
