//! Npc shop command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{
    AdmittedSession, ExactActorRef, FreshAdmissionAuthority, NpcIntentDisposition, NpcUpdate,
    SessionItemView, UseCommand, item_move,
};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::ClientCommandView;
use oteryn_protocol_oteryn::npc_shop::{decode_npc_shop_intent, encode_npc_shop_result};
use transport::npc_shop::NpcShopStep;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: &mut Option<SessionItemView>,
) -> Option<Dispatch> {
    Some({
        // NPC-TRADE-SELLALL-1: a malformed intent is REJECTED. LOOK answers the text alone; a
        // committed sell-all answers OK with its units and gold, then the inventory delta
        // (domain 9) when the session shows items; an unknown outcome ends the connection before
        // the CommandId is sequenced, so the resend replays.
        let step = match decode_npc_shop_intent(command.payload) {
            Ok(intent) => {
                authority
                    .npc_shop(
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
            Err(_) => NpcShopStep::rejected(),
        };
        let payload = encode_npc_shop_result(&step.result()).ok()?;
        match step {
            NpcShopStep::Looked(_) => {
                Dispatch::NpcShop(CommandStatus::Accepted, payload, NpcUpdate::default(), Vec::new())
            }
            NpcShopStep::Sold { .. } => {
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
                Dispatch::NpcShop(CommandStatus::Accepted, payload, NpcUpdate::default(), deltas)
            }
            NpcShopStep::Refused(refusal) => Dispatch::NpcShop(
                if refusal.disposition == NpcIntentDisposition::Rejected {
                    CommandStatus::Rejected
                } else {
                    CommandStatus::Accepted
                },
                payload,
                refusal.update,
                Vec::new(),
            ),
            NpcShopStep::Unknown => return None,
        }
    })
}
