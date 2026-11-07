//! Citizen source introductions use the native programme's guarded topics.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    ordinary: &NpcTalkOutcome,
) -> Option<catalogue::Step> {
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    )?;
    if !matches!(context.2.as_str(), "oteryn:npc.lubo" | "oteryn:npc.hanna") {
        return None;
    }
    catalogue::prepare(admission, actor, command, intent, runtime, ordinary)
}
