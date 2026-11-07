//! Exact source Cupcake speech through the bounded native chat owner.
use super::*;
use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;

fn at(
    runtime: &ChannelRuntimeV1,
    session: GameSessionId,
) -> Option<(ExactActorRef, MovementLocalPosition, RuntimeScopeRefV1, u64)> {
    if let Some((actor, house)) = runtime.house_player_for_session(session).ok()? {
        return Some((
            actor,
            MovementLocalPosition {
                x: house.position.x,
                y: house.position.y,
                floor: house.position.floor,
            },
            house.scope,
            house.generation.get(),
        ));
    }
    let (actor, position) = runtime.positioned_player_for_session(session).ok()??;
    Some((
        actor,
        position.position(),
        runtime.binding().runtime_scope(),
        runtime.binding().scope_generation().get(),
    ))
}

impl ChatRuntime {
    pub(in crate::gameplay_transport) fn cupcake_speech(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
    ) {
        let Some((current, position, scope, generation)) = at(runtime, session) else {
            return;
        };
        if current != actor
            || match scope {
                RuntimeScopeRefV1::Channel { .. } => {
                    if runtime.owner_fence().is_err() {
                        return;
                    }
                    runtime.player_control_facts(actor, session)
                }
                RuntimeScopeRefV1::Instance { .. } => {
                    runtime.house_player_control_facts(actor, session)
                }
            }
            .is_err()
        {
            return;
        }
        let Some(floor) = crate::movement::interest::tibia_floor(position.floor) else {
            return;
        };
        let Some(actor_generation) = NonZeroU64::new(actor.actor_local_generation()) else {
            return;
        };
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let Some(name) = sessions.get(&session).map(|state| state.name.clone()) else {
            return;
        };
        for (listener, state) in sessions.iter_mut() {
            let Some((_, heard, heard_scope, heard_generation)) = at(runtime, *listener) else {
                continue;
            };
            if heard_scope != scope
                || heard_generation != generation
                || heard.floor != position.floor
                || heard.x.abs_diff(position.x) > 8
                || heard.y.abs_diff(position.y) > 6
            {
                continue;
            }
            state.egress.push(ChatLine::Local {
                speaker: ChatSpeaker {
                    identity: actor.placement_identity(),
                    generation: actor_generation,
                },
                speaker_name: name.clone(),
                mode: state.scripted_item_mode(),
                text: "Mmmm.".into(),
                position: ActorPosition {
                    x: position.x,
                    y: position.y,
                    floor,
                },
            });
        }
    }
}
