//! The two closed Santa dolls speak MonsterSay at their actual clicked coordinate.
use super::*;
use oteryn_gameplay_items::novelty_items::NoveltyEffects;
use oteryn_protocol_oteryn::world_spatial::ActorPosition;
impl ChatRuntime {
    pub(in crate::gameplay_transport) fn publish_santa_gift_speech(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        effects: &NoveltyEffects,
    ) {
        if !matches!(effects.source_item_id, 5080 | 5791 | 6511 | 6566)
            || runtime.owner_fence().is_err()
            || runtime.player_control_facts(actor, session).is_err()
        {
            return;
        }
        let Some(text) = effects.speech.as_ref() else {
            return;
        };
        let visible = runtime.visible_entities();
        let Some(speaker) = visible.players.iter().find(|p| p.actor == actor) else {
            return;
        };
        let (x, y, z) = effects.ground.as_ref().map_or(
            (
                speaker.position.x,
                speaker.position.y,
                speaker.position.floor,
            ),
            |p| (p.x, p.y, p.floor),
        );
        let (Some(floor), Some(generation)) = (
            crate::movement::interest::tibia_floor(z),
            NonZeroU64::new(speaker.generation),
        ) else {
            return;
        };
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let Some(name) = sessions.get(&session).map(|s| s.name.clone()) else {
            return;
        };
        for (listener, state) in sessions.iter_mut() {
            if !state.mount_sounds {
                continue;
            }
            let Ok(Some((_, position))) = runtime.positioned_player_for_session(*listener) else {
                continue;
            };
            let at = position.position();
            if at.floor != z || at.x.abs_diff(x) > 8 || at.y.abs_diff(y) > 6 {
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
                position: ActorPosition { x, y, floor },
            });
        }
    }
}
