//! GUILD-CHAT-1 composition on the existing chat relay and Guild owner.
use super::connection::AdmittedSession;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{ChannelRuntimeV1, GameSessionId, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::chat::guild_room::CAPABILITY_CHAT_GUILD_ROOM_V1;
use oteryn_protocol_oteryn::chat::{ChatDisposition, ChatIntentResult};
use std::collections::HashMap;
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

mod admission;
mod dequeue;
mod send;
pub(super) mod wire;

#[derive(Default)]
pub(super) struct GuildChatRuntime {
    bindings: Mutex<
        HashMap<GameSessionId, (CurrentCharacterItemFence, crate::foundation::ExactActorRef)>,
    >,
    /// Aggregate only: no name, text, GuildId or CharacterId is logged.
    drops: AtomicU64,
}
impl GuildChatRuntime {
    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut bindings) = self.bindings.lock() {
            bindings.remove(&session);
        }
    }
    pub(super) fn bind(&self, admitted: AdmittedSession) {
        if let (Some(fence), Some(actor)) = (admitted.item_fence, admitted.runtime_actor)
            && let Ok(mut bindings) = self.bindings.lock()
        {
            bindings.insert(admitted.game_session_id, (fence, actor));
        }
    }
    pub(super) fn negotiated(&self, session: GameSessionId) -> bool {
        self.bindings
            .lock()
            .is_ok_and(|bindings| bindings.contains_key(&session))
    }
    pub(super) fn matches(&self, admitted: &AdmittedSession) -> bool {
        let (Some(fence), Some(actor)) = (admitted.item_fence, admitted.runtime_actor) else {
            return false;
        };
        fence.connection_generation.get() == admitted.continuity.connection_generation
            && self.bindings.lock().is_ok_and(|bindings| {
                bindings.get(&admitted.game_session_id) == Some(&(fence, actor))
            })
    }
    pub(super) fn take_drops(&self) -> u64 {
        self.drops.swap(0, Ordering::Relaxed)
    }
    pub(super) fn dropped(&self) {
        self.drops.fetch_add(1, Ordering::Relaxed);
    }
}

fn result(disposition: ChatDisposition) -> ChatIntentResult {
    ChatIntentResult {
        disposition,
        wait_seconds: 0,
    }
}

fn qualified_fence(
    admitted: &AdmittedSession,
    runtime: &ChannelRuntimeV1,
) -> Option<CurrentCharacterItemFence> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_CHAT_GUILD_ROOM_V1)
    {
        return None;
    }
    let fence = admitted.item_fence?;
    let binding = runtime.binding();
    let actor = admitted.runtime_actor?;
    if runtime.owner_fence().is_err()
        || !runtime
            .player_control_facts(actor, admitted.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
        || fence.game_session_id != admitted.game_session_id
        || fence.connection_generation.get() != admitted.continuity.connection_generation
        || admitted.world_id != binding.world_id()
        || admitted.channel_id != binding.channel_id()
        || fence.runtime_scope
            != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
        || fence.scope_ownership_generation != binding.scope_generation()
    {
        return None;
    }
    Some(fence)
}
