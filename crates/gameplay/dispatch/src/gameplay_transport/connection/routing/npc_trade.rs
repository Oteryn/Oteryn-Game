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
        let bulk =
            command.command_type == oteryn_protocol_oteryn::npc_purchase::COMMAND_TYPE_NPC_PURCHASE;
        let mut text = None;
        let step = if bulk {
            match oteryn_protocol_oteryn::npc_purchase::decode_npc_purchase(command.payload) {
                Ok(intent) => {
                    let (step, message) = authority
                        .npc_purchase(
                            actor,
                            UseCommand {
                                game_session_id: admitted.game_session_id,
                                command_id: command.command_id,
                                item_fence: admitted.item_fence,
                            },
                            intent,
                        )
                        .await;
                    text = message;
                    step
                }
                Err(_) => NpcTradeStep::Refused(transport::npc_command::NpcTradeRefusal::only(
                    NpcIntentDisposition::Rejected,
                )),
            }
        } else {
            match decode_npc_trade_intent(command.payload) {
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
            }
        };
        let dispatch = match step {
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
        };
        if bulk {
            match dispatch {
                Dispatch::NpcTrade(disposition, update, deltas) => Dispatch::NpcShop(
                    if disposition == NpcIntentDisposition::Ok {
                        crate::foundation::CommandStatus::Accepted
                    } else {
                        crate::foundation::CommandStatus::Rejected
                    },
                    text.unwrap_or_else(|| "Sorry, not possible.".into())
                        .into_bytes(),
                    update,
                    deltas,
                ),
                _ => return None,
            }
        } else {
            dispatch
        }
    })
}

/// Bulk purchases can change an already open Obtain container or create nested bags.
/// Refresh through the same container-view owner used after ordinary item moves.
pub(in super::super) async fn route_purchase<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: &mut Option<SessionItemView>,
    container_views: &mut Option<super::super::ContainerViewState>,
) -> Option<Dispatch> {
    let mut dispatch = route(authority, actor, admitted, command, item_view).await?;
    if let Dispatch::NpcShop(crate::foundation::CommandStatus::Accepted, _, _, deltas) =
        &mut dispatch
        && let (Some(views), Some(view)) = (container_views.as_mut(), item_view.as_mut())
    {
        let refreshed = transport::container_view::refresh::refresh(
            authority,
            actor,
            admitted.item_fence,
            views,
            view,
        )
        .await
        .ok()?;
        admitted.continuity.item_view = view.continuity();
        deltas.extend(refreshed);
    }
    Some(dispatch)
}
