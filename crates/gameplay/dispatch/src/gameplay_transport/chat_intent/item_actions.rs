//! Sourced automatic item speech uses the existing bounded local CHAT egress.
use super::*;
use crate::chat::Heard;
use oteryn_gameplay_items::novelty_items::{self, NoveltyEffects, NoveltyKind};

fn item_listeners<K>(
    effects: &NoveltyEffects,
    origin: ChatPosition,
    listeners: impl IntoIterator<Item = (K, ChatPosition)>,
) -> Vec<(K, Heard)> {
    if effects.dice_value.is_none() {
        return local_listeners(SpeechMode::Say, origin, listeners);
    }
    let Some(range) = novelty_items::rules()
        .and_then(|rules| {
            rules.iter().find(|rule| {
                rule.kind == NoveltyKind::Dice && rule.item_ids.contains(&effects.source_item_id)
            })
        })
        .and_then(|rule| rule.dice_speech_range)
    else {
        return Vec::new();
    };
    // die.lua:11 explicitly selects X +/-3, with omitted Y bounds. The Lua
    // binding defaults Y to0 and Spectators::find substitutes the source viewport.
    // This explicit recipient set must not then be narrowed by ordinary SAY range.
    listeners
        .into_iter()
        .filter_map(|(listener, position)| {
            let dx = i64::from(position.x) - i64::from(origin.x);
            let dy = i64::from(position.y) - i64::from(origin.y);
            (position.floor == origin.floor
                && dx >= -i64::from(range[0])
                && dx <= i64::from(range[1])
                && dy >= -i64::from(range[2])
                && dy <= i64::from(range[3]))
            .then_some((listener, Heard::Text))
        })
        .collect()
}

impl ChatRuntime {
    /// Crystal 201189f `data/scripts/actions/objects/wall_mirror.lua:17-24`:
    /// sourced automatic speech comes from the player, inside its actual House.
    pub(in crate::gameplay_transport) fn publish_house_decoration_speech(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        speech: &str,
    ) {
        self.publish_house_speech(runtime, actor, session, speech, false);
    }

    fn publish_house_speech(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        speech: &str,
        private: bool,
    ) {
        if runtime.owner_fence().is_err()
            || !runtime
                .house_player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return;
        }
        let Ok(house) = runtime.house_actor_state(actor, session) else {
            return;
        };
        let (Some(generation), Ok(floor), Ok(text)) = (
            NonZeroU64::new(actor.actor_local_generation()),
            u8::try_from(house.position.floor),
            ChatText::parse(speech),
        ) else {
            return;
        };
        let at = ChatPosition {
            x: house.position.x,
            y: house.position.y,
            floor,
        };
        let Ok(visible) = runtime.house_visible_players(actor, session) else {
            return;
        };
        let listeners = visible.into_iter().filter_map(|player| {
            let (listener, current) = runtime.house_observer(player.actor).ok()??;
            if private && listener != session {
                return None;
            }
            Some((
                listener,
                ChatPosition {
                    x: current.position.x,
                    y: current.position.y,
                    floor: u8::try_from(current.position.floor).ok()?,
                },
            ))
        });
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let Some(speaker) = sessions.get(&session) else {
            return;
        };
        let name = speaker.name.clone();
        let speaker_id = ChatSpeaker {
            identity: actor.placement_identity(),
            generation,
        };
        for (listener, heard) in local_listeners(SpeechMode::Say, at, listeners) {
            if let Some(entry) = sessions.get_mut(&listener) {
                let mut line = local_line(speaker_id, &name, SpeechMode::Say, &text, at, heard);
                // Crystal data/scripts/actions/objects/wall_mirror.lua:17-22
                // uses TALKTYPE_MONSTER_SAY while keeping local SAY listeners.
                if let ChatLine::Local { mode, .. } = &mut line {
                    *mode = ChatSpeechMode::MonsterSay;
                }
                entry.egress.push(line);
            }
        }
    }

    /// Automatic NPC speech shares the bounded CHAT egress, without player ingress limits
    /// or the player-only uppercase transformation for YELL.
    pub(in crate::gameplay_transport) fn publish_npc_voice(
        &self,
        runtime: &ChannelRuntimeV1,
        proposal: &crate::gameplay_transport::npc_voice::NpcVoiceProposal,
    ) {
        if runtime.owner_fence().is_err() || runtime.npc_key(proposal.occurrence.actor).is_none() {
            return;
        }
        let visible = runtime.visible_entities();
        let Some(npc) = visible
            .npcs
            .iter()
            .find(|npc| npc.actor == proposal.occurrence.actor)
        else {
            return;
        };
        let at = npc.position;
        if at.x != proposal.position.x
            || at.y != proposal.position.y
            || at.floor != i16::from(proposal.position.floor)
        {
            return;
        }
        let Ok(line) = proposal.chat_line() else {
            return;
        };
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let listeners = sessions
            .keys()
            .filter_map(|session| {
                let (_, position) = runtime.positioned_player_for_session(*session).ok()??;
                let at = position.position();
                Some((
                    *session,
                    ChatPosition {
                        x: at.x,
                        y: at.y,
                        floor: u8::try_from(at.floor).ok()?,
                    },
                ))
            })
            .collect::<Vec<_>>();
        for (listener, _) in local_listeners(proposal.mode(), proposal.position, listeners) {
            if let Some(entry) = sessions.get_mut(&listener) {
                entry.egress.push(line.clone());
            }
        }
    }

    pub(in crate::gameplay_transport) fn publish_item_speech(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        effects: &NoveltyEffects,
    ) {
        if runtime.house_actor_state(actor, session).is_ok() {
            if effects.dice_value.is_none()
                && let Some(speech) = effects.speech.as_deref()
            {
                if effects.private_speech {
                    self.publish_house_speech(runtime, actor, session, speech, true);
                } else {
                    self.publish_house_decoration_speech(runtime, actor, session, speech);
                }
            }
            return;
        }
        if runtime.player_control_facts(actor, session).is_err() {
            return;
        }
        let visible = runtime.visible_entities();
        let Some(speaker) = visible.players.iter().find(|entry| entry.actor == actor) else {
            return;
        };
        let (x, y, source_floor) = effects
            .ground
            .as_ref()
            .filter(|_| effects.dice_value.is_some())
            .map_or(
                (
                    speaker.position.x,
                    speaker.position.y,
                    speaker.position.floor,
                ),
                |ground| (ground.x, ground.y, ground.floor),
            );
        let (Some(generation), Ok(floor)) = (
            NonZeroU64::new(speaker.generation),
            u8::try_from(source_floor),
        ) else {
            return;
        };
        let at = ChatPosition { x, y, floor };
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let Some(entry) = sessions.get(&session) else {
            return;
        };
        let name = entry.name.clone();
        let speech = effects
            .dice_value
            .map(|value| format!("{name} rolled a {value}."))
            .or_else(|| effects.speech.clone());
        let Some(speech) = speech else {
            return;
        };
        let Ok(text) = ChatText::parse(&speech) else {
            return;
        };
        let speaker_id = ChatSpeaker {
            identity: actor.placement_identity(),
            generation,
        };
        let listeners = sessions
            .keys()
            .filter_map(|listener| {
                if effects.private_speech && *listener != session {
                    return None;
                }
                let (_, position) = runtime.positioned_player_for_session(*listener).ok()??;
                let position = position.position();
                Some((
                    *listener,
                    ChatPosition {
                        x: position.x,
                        y: position.y,
                        floor: u8::try_from(position.floor).ok()?,
                    },
                ))
            })
            .collect::<Vec<_>>();
        for (listener, heard) in item_listeners(effects, at, listeners) {
            if let Some(entry) = sessions.get_mut(&listener) {
                entry.egress.push(local_line(
                    speaker_id,
                    &name,
                    SpeechMode::Say,
                    &text,
                    at,
                    heard,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        reason = "the fixture requires the exact captured dice rule"
    )]
    use super::*;

    #[test]
    fn dice_uses_source_x_bounds_and_default_y_viewport_without_say_clipping() {
        let rule = novelty_items::rule_for_key("oteryn:item.tibia.i5792").expect("source die");
        assert_eq!(rule.dice_speech_range, Some([3, 3, 11, 11]));
        let selected = novelty_items::select(rule, 5792, |_, _| Some(0)).expect("source roll");
        let origin = ChatPosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        let listeners = [
            (
                1,
                ChatPosition {
                    x: 97,
                    y: 89,
                    floor: 7,
                },
            ),
            (
                2,
                ChatPosition {
                    x: 103,
                    y: 111,
                    floor: 7,
                },
            ),
            (
                3,
                ChatPosition {
                    x: 104,
                    y: 100,
                    floor: 7,
                },
            ),
            (
                4,
                ChatPosition {
                    x: 100,
                    y: 112,
                    floor: 7,
                },
            ),
            (
                5,
                ChatPosition {
                    x: 100,
                    y: 100,
                    floor: 8,
                },
            ),
        ];
        assert_eq!(
            item_listeners(&selected.effects, origin, listeners),
            vec![(1, Heard::Text), (2, Heard::Text)]
        );
    }
}
