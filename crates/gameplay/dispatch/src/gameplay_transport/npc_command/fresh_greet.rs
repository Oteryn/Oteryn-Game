//! A fresh source callback is witnessed by the actual conversation owner.
//! This private witness is deliberately neither Clone nor Copy.
use super::*;

pub(in crate::gameplay_transport) struct FreshNpcGreet {
    session: GameSessionId,
    actor: ExactActorRef,
    npc: EntityRef,
    context: (u64, u64, String),
}
impl NpcConversations {
    pub(in crate::gameplay_transport) fn talk_with_fresh_greet(
        &self,
        chat: &ChatRuntime,
        runtime: &ChannelRuntimeV1,
        (actor, session): (ExactActorRef, GameSessionId),
        intent: &NpcTalkIntent,
        now: SemanticTimeMicros,
        items: Option<&ItemDefinitionIndex>,
    ) -> (NpcTalkOutcome, Option<FreshNpcGreet>) {
        let previous = chat.npc_conversation_generation(session);
        let outcome = self.talk_in(chat, runtime, actor, session, intent, now, items);
        let witness = (outcome.disposition == NpcIntentDisposition::Ok)
            .then(|| self.current_npc(chat, runtime, actor, session, intent))
            .flatten()
            .filter(|context| previous != Some(context.1))
            .map(|context| FreshNpcGreet {
                session,
                actor,
                npc: intent.npc_actor,
                context,
            });
        (outcome, witness)
    }
}
impl FreshNpcGreet {
    // Consume this witness only into a source-program request. Repeated greetings
    // cannot reconstruct it: the chat owner does not advance joined for them.
    pub(super) fn consume(
        self,
        conversations: &NpcConversations,
        chat: &ChatRuntime,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        intent: &NpcTalkIntent,
    ) -> Option<(u64, u64, String)> {
        (self.session == session
            && self.actor == actor
            && self.npc == intent.npc_actor
            && conversations.current_npc(chat, runtime, actor, session, intent)
                == Some(self.context.clone()))
        .then_some(self.context)
    }
}

impl FreshNpcGreet {
    pub(super) fn into_source_context(
        self,
        conversations: &NpcConversations,
        chat: &ChatRuntime,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        command: super::super::connection::UseCommand,
        intent: &NpcTalkIntent,
    ) -> Option<crate::durability::npc_cosmetics::NpcSourceGreetContext> {
        let item_fence = command.item_fence?;
        if item_fence.game_session_id != command.game_session_id {
            return None;
        }
        let context = self.consume(
            conversations,
            chat,
            runtime,
            actor,
            command.game_session_id,
            intent,
        )?;
        let position = runtime.read_actor_position(actor).ok()?;
        Some(crate::durability::npc_cosmetics::NpcSourceGreetContext {
            game_session_id: command.game_session_id,
            connection_generation: item_fence.connection_generation.get(),
            command_id: command.command_id,
            actor_identity: actor.placement_identity(),
            npc_identity: intent.npc_actor.identity,
            npc_generation: intent.npc_actor.generation,
            catalogue_revision: context.0.to_string(),
            conversation_generation: context.1,
            movement_revision: position.revision(),
        })
    }
}
