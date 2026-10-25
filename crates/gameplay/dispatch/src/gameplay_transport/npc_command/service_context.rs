//! Current NPC identity, conversation and range shared by Bank and Premium services.
use super::*;

impl NpcConversations {
    pub(super) fn current_npc(
        &self,
        chat: &ChatRuntime,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        intent: &NpcTalkIntent,
    ) -> Option<(u64, u64, String)> {
        let visible = runtime.visible_entities();
        let speaker = visible
            .players
            .iter()
            .find(|player| player.actor == actor)?;
        let npc = visible.npcs.iter().find(|npc| {
            npc.actor.placement_identity() == intent.npc_actor.identity
                && npc.generation == intent.npc_actor.generation
        })?;
        if chat.engaged(session) != Some(npc.actor)
            || speaker.position.floor != npc.position.floor
            || speaker.position.x.abs_diff(npc.position.x) > TALK_RANGE_TILES
            || speaker.position.y.abs_diff(npc.position.y) > TALK_RANGE_TILES
        {
            return None;
        }
        let book = chat.npc_talk()?;
        book.get(runtime.npc_key(npc.actor)?)?;
        Some((
            book.catalogue_revision(),
            chat.npc_conversation_generation(session)?,
            runtime.npc_key(npc.actor)?.to_owned(),
        ))
    }
}
