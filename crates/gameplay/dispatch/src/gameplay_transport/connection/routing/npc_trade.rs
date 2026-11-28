//! Npc trade command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{
    AdmittedSession, ExactActorRef, FreshAdmissionAuthority, NpcIntentDisposition, NpcTradeStep,
    NpcUpdate, SessionItemView, UseCommand, item_move,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;
use oteryn_protocol_oteryn::npc_service::{
    CAPABILITY_NPC_PURCHASE_ATTRIBUTION_V1, NpcTradeSide,
};

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
        let options = admitted.continuity.selected_capabilities.contains(
            oteryn_protocol_oteryn::npc_shop::CAPABILITY_NPC_SHOP_OPTIONS_V1,
        );
        let selected = admitted.continuity.selected_capabilities
            .contains(CAPABILITY_NPC_PURCHASE_ATTRIBUTION_V1);
        let mut attribute_purchase = false;
        let step = match oteryn_protocol_oteryn::npc_service::decode_npc_trade_intent_for(command.payload, options) {
            Ok((intent, _))
                if intent.quantity > 100
                    && !admitted.continuity.selected_capabilities.contains(501) =>
            {
                NpcTradeStep::Refused(transport::npc_command::NpcTradeRefusal::only(
                    NpcIntentDisposition::Rejected,
                ))
            }
            Ok((intent, packed)) => {
                attribute_purchase = selected && intent.side == NpcTradeSide::Buy;
                if attribute_purchase && item_view.as_ref().is_some_and(SessionItemView::npc_purchase_attribution_full) {
                    // Preserve every already committed ID while the inventory read is busy.
                    return Some(Dispatch::NpcTrade(
                        NpcIntentDisposition::RateLimited,
                        NpcUpdate::default(),
                        Vec::new(),
                    ));
                }
                authority
                    .npc_trade_with_delivery(
                        actor,
                        UseCommand {
                            game_session_id: admitted.game_session_id,
                            command_id: command.command_id,
                            item_fence: admitted.item_fence,
                        },
                        intent,
                        packed,
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
                    if attribute_purchase {
                        view.record_npc_purchase(command.command_id);
                    }
                    if let Some(inventory) = item_move::observe_inventory(
                        authority,
                        actor,
                        admitted.game_session_id,
                        view,
                    )
                    .await
                    {
                        let Ok(delta) = view.inventory_committed(inventory) else {
                            return None;
                        };
                        admitted.continuity.item_view = view.continuity();
                        deltas.extend(delta);
                    } else {
                        view.inventory_refresh_pending = true;
                    }
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
