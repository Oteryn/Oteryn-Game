//! Npc trade command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{
    AdmittedSession, ExactActorRef, FreshAdmissionAuthority, NpcIntentDisposition, NpcTradeStep,
    NpcUpdate, SessionItemView, UseCommand, decode_npc_trade_intent, item_move,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: &mut Option<SessionItemView>,
) -> Option<Dispatch> {
    Some({
        // NPC-TRADE-2: a malformed intent is REJECTED. A committed trade answers OK and the
        // inventory delta (domain 9) follows when the session shows items; an unknown outcome
        // ends the connection before the CommandId is sequenced, so the resend replays.
        let step = match decode_npc_trade_intent(command.payload) {
            Ok(intent) => {
                authority
                    .npc_trade(
                        actor,
                        UseCommand {
                            game_session_id: admitted.game_session_id,
                            command_id: command.command_id,
                            item_fence: admitted.item_fence,
                        },
                        intent,
                    )
                    .await
            }
            Err(_) => NpcTradeStep::Refused(transport::npc_command::NpcTradeRefusal::only(
                NpcIntentDisposition::Rejected,
            )),
        };
        match step {
            NpcTradeStep::Traded => {
                let mut deltas = Vec::new();
                if let Some(view) = item_view.as_mut() {
                    let inventory = item_move::observe_inventory(
                        authority,
                        actor,
                        admitted.game_session_id,
                        view,
                    )
                    .await?;
                    let Ok(delta) = view.inventory_committed(inventory) else {
                        return None;
                    };
                    admitted.continuity.item_view = view.continuity();
                    deltas.extend(delta);
                }
                Dispatch::NpcTrade(NpcIntentDisposition::Ok, NpcUpdate::default(), deltas)
            }
            NpcTradeStep::Refused(refusal) => {
                Dispatch::NpcTrade(refusal.disposition, refusal.update, Vec::new())
            }
            NpcTradeStep::Unknown => return None,
        }
    })
}
