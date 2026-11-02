//! Current NPC identity, conversation and range shared by Bank and Premium services.
use super::*;

impl NpcConversations {
    pub(in crate::gameplay_transport) fn current_npc(
        &self,
        chat: &ChatRuntime,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        intent: &NpcTalkIntent,
    ) -> Option<(u64, u64, String)> {
        let speaker = speaker_position(runtime, actor, session)?;
        let npc = live_npc(runtime, actor, session, intent.npc_actor)?;
        if chat.engaged(session) != Some(npc.actor)
            || speaker.floor != npc.position.floor
            || speaker.x.abs_diff(npc.position.x) > TALK_RANGE_TILES
            || speaker.y.abs_diff(npc.position.y) > TALK_RANGE_TILES
        {
            return None;
        }
        let book = chat.npc_talk()?;
        let key = npc.key.as_str();
        if book.get(key).is_none()
            && !key.starts_with("oteryn:npc.hireling-bank:")
            && crate::durability::house_hireling::npc_identity(key).is_none()
        {
            return None;
        }
        Some((
            book.catalogue_revision(),
            chat.npc_conversation_generation(session)?,
            key.to_owned(),
        ))
    }
}

#[derive(Clone, Copy)]
pub(super) struct Position {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}
pub(super) struct LiveNpc {
    pub actor: ExactActorRef,
    pub position: Position,
    pub key: String,
}

pub(super) fn speaker_position(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) -> Option<Position> {
    if let Ok(state) = runtime.house_actor_state(actor, session) {
        return Some(Position {
            x: state.position.x,
            y: state.position.y,
            floor: state.position.floor,
        });
    }
    runtime
        .visible_entities()
        .players
        .into_iter()
        .find(|entry| entry.actor == actor)
        .map(|entry| Position {
            x: entry.position.x,
            y: entry.position.y,
            floor: entry.position.floor,
        })
}

/// A House observer sees only its scoped native NPC slots. Channel keys cannot opt into House.
pub(super) fn live_npc(
    runtime: &ChannelRuntimeV1,
    observer: ExactActorRef,
    session: GameSessionId,
    named: EntityRef,
) -> Option<LiveNpc> {
    if runtime.house_actor_state(observer, session).is_ok() {
        let npc = runtime
            .house_hireling_entities(observer, session)
            .ok()?
            .into_iter()
            .find(|npc| {
                npc.actor.placement_identity() == named.identity
                    && npc.actor.actor_local_generation() == named.generation
            })?;
        let key = runtime
            .house_hireling_key(observer, session, npc.actor)?
            .to_owned();
        return Some(LiveNpc {
            actor: npc.actor,
            key,
            position: Position {
                x: npc.position.x,
                y: npc.position.y,
                floor: npc.position.floor,
            },
        });
    }
    let visible = runtime.visible_entities();
    let npc = visible.npcs.into_iter().find(|npc| {
        npc.actor.placement_identity() == named.identity && npc.generation == named.generation
    })?;
    Some(LiveNpc {
        actor: npc.actor,
        key: runtime.npc_key(npc.actor)?.to_owned(),
        position: Position {
            x: npc.position.x,
            y: npc.position.y,
            floor: npc.position.floor,
        },
    })
}
