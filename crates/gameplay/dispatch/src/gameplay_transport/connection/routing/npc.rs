//! Npc command body; predicate and connection control flow stay in `connection.rs`.

use super::super::{
    AdmittedSession, BufferedStep, ExactActorRef, FreshAdmissionAuthority, NpcTalkIntent,
    NpcUpdate, UseCommand, WireEntityRef, WorldSpatialObservation, decode_npc_talk_intent,
    newhaven_travel_transport,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
    buffered: &mut Option<BufferedStep>,
    npc_relocation: &mut Option<WorldSpatialObservation>,
) -> Option<Dispatch> {
    Some({
        // NPC-CMD-1a: a malformed intent is REJECTED, but it still spends the session's
        // NPC0-RL-04 allowance: it reaches the limiter as an intent with no text.
        let intent = decode_npc_talk_intent(command.payload).unwrap_or(NpcTalkIntent {
            npc_actor: WireEntityRef {
                identity: Default::default(),
                generation: 0,
            },
            text: String::new(),
        });
        let before_npc = authority.observe(actor).await;
        let outcome = authority
            .npc_talk(
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                },
                intent,
            )
            .await;
        if outcome.unknown {
            return None;
        }
        *npc_relocation =
            newhaven_travel_transport::retire_buffer(authority, actor, before_npc, buffered).await;
        Dispatch::Npc(
            outcome.disposition,
            NpcUpdate {
                conversation: outcome.conversation,
                window: outcome.window,
            },
            outcome.inventory_changed,
        )
    })
}
