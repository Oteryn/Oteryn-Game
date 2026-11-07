//! Source Cake speech uses bounded native CHAT egress, with live actors checked first.
use super::*;
use oteryn_protocol_oteryn::world_spatial::ActorPosition;

impl ChatRuntime {
    pub(in crate::gameplay_transport) fn pumpkin_skinning_speech(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
    ) {
        if runtime.owner_fence().is_err() || runtime.player_control_facts(actor, session).is_err() {
            return;
        }
        let Ok(sessions) = self.sessions.lock() else {
            return;
        };
        let Some(name) = sessions.get(&session).map(|s| s.name.clone()) else {
            return;
        };
        drop(sessions);
        // Source player MonsterSay is ordinary item speech. Optional mount voices
        // (ambient288) do not own its delivery; use current chat/scope/viewport like House items.
        let Ok(position) = runtime.read_actor_position(actor) else {
            return;
        };
        let position = position.position();
        let (Some(generation), Some(floor), Ok(origin)) = (
            NonZeroU64::new(actor.actor_local_generation()),
            crate::movement::interest::tibia_floor(position.floor),
            crate::movement::interest::VisibilityPosition::new(
                position.x,
                position.y,
                position.floor,
            ),
        ) else {
            return;
        };
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        for (listener, state) in sessions.iter_mut() {
            let Ok(Some((recipient, heard))) = runtime.positioned_player_for_session(*listener)
            else {
                continue;
            };
            if recipient.runtime_scope() != actor.runtime_scope()
                || recipient.scope_generation() != actor.scope_generation()
            {
                continue;
            }
            let heard = heard.position();
            if heard.floor != position.floor {
                continue;
            }
            let Ok(observer) =
                crate::movement::interest::VisibilityPosition::new(heard.x, heard.y, heard.floor)
            else {
                continue;
            };
            if !crate::movement::interest::VisibilitySettings::REFERENCE.can_see(observer, origin) {
                continue;
            }
            state.egress.push(ChatLine::Local {
                speaker: ChatSpeaker {
                    identity: actor.placement_identity(),
                    generation,
                },
                speaker_name: name.clone(),
                mode: state.scripted_item_mode(),
                text: "Happy Halloween!".into(),
                position: ActorPosition {
                    x: position.x,
                    y: position.y,
                    floor,
                },
            });
        }
    }

    pub(in crate::gameplay_transport) fn cake_bite_speech(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
    ) {
        if runtime.owner_fence().is_err() || runtime.player_control_facts(actor, session).is_err() {
            return;
        }
        let Ok(sessions) = self.sessions.lock() else {
            return;
        };
        let Some(name) = sessions.get(&session).map(|s| s.name.clone()) else {
            return;
        };
        drop(sessions);
        self.cake_speech(runtime, actor, &name, "Mmmm!");
    }

    pub(in crate::gameplay_transport) fn santa_candy_speech(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
    ) {
        if runtime.owner_fence().is_err() || runtime.player_control_facts(actor, session).is_err() {
            return;
        }
        let Ok(sessions) = self.sessions.lock() else {
            return;
        };
        let Some(name) = sessions.get(&session).map(|s| s.name.clone()) else {
            return;
        };
        drop(sessions);
        self.cake_speech(runtime, actor, &name, "Munch.");
    }

    pub(in crate::gameplay_transport) fn cake_golem_crunch(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
    ) {
        if runtime.owner_fence().is_err()
            || !runtime.matches_live_creature_identity(actor, b"oteryn:creature.cake_golem")
            || !runtime
                .companion_snapshot(actor)
                .is_ok_and(|s| s.state.master.is_none())
        {
            return;
        }
        self.cake_speech(runtime, actor, "Cake Golem", "CRUNSH!");
    }

    fn cake_speech(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        name: &str,
        text: &str,
    ) {
        let (Ok(position), Some(generation)) = (
            runtime.read_actor_position(actor),
            NonZeroU64::new(actor.actor_local_generation()),
        ) else {
            return;
        };
        let position = position.position();
        let Some(floor) = crate::movement::interest::tibia_floor(position.floor) else {
            return;
        };
        let Ok(origin) = crate::movement::interest::VisibilityPosition::new(
            position.x,
            position.y,
            position.floor,
        ) else {
            return;
        };
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        for (session, state) in sessions.iter_mut() {
            let Ok(Some((recipient, at))) = runtime.positioned_player_for_session(*session) else {
                continue;
            };
            if recipient.runtime_scope() != actor.runtime_scope()
                || recipient.scope_generation() != actor.scope_generation()
            {
                continue;
            }
            let at = at.position();
            if at.floor != position.floor {
                continue;
            }
            let Ok(observer) =
                crate::movement::interest::VisibilityPosition::new(at.x, at.y, at.floor)
            else {
                continue;
            };
            if !crate::movement::interest::VisibilitySettings::REFERENCE.can_see(observer, origin) {
                continue;
            }
            state.egress.push(ChatLine::Local {
                speaker: ChatSpeaker {
                    identity: actor.placement_identity(),
                    generation,
                },
                speaker_name: name.to_owned(),
                mode: state.scripted_item_mode(),
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
