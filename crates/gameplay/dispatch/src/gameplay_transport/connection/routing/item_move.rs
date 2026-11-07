//! Item move command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{
    AdmittedSession, CAPABILITY_CONTAINER_TREE_V1, CAPABILITY_ITEM_EQUIP_DROP_V1,
    CAPABILITY_WORLD_INTERACTION_V1, ContainerViewState, ExactActorRef, FreshAdmissionAuthority,
    ItemMoveOutcome, ItemMoveSelection, ItemMoveStep, SelectedCapabilities, SessionItemView,
    UseCommand, item_move,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: &mut Option<SessionItemView>,
    container_views: &mut Option<ContainerViewState>,
    selected_capabilities: SelectedCapabilities,
) -> Option<Dispatch> {
    Some({
        // ITEM-MOVE-1 (WIRE-0 §5): replay first, then an entry of the open corpse into the
        // main backpack. An unknown outcome ends the connection before the CommandId is
        // sequenced, so the resent command is answered by replay.
        match item_view.as_mut() {
            Some(view) => {
                let selection = ItemMoveSelection {
                    equip_drop: selected_capabilities.contains(CAPABILITY_ITEM_EQUIP_DROP_V1),
                    container_tree: selected_capabilities.contains(CAPABILITY_CONTAINER_TREE_V1),
                    world_interaction: selected_capabilities
                        .contains(CAPABILITY_WORLD_INTERACTION_V1),
                };
                // Foundation has already classified replay. Observe the reservation before
                // legacy receipt reads can yield to the partner's observation cadence.
                if let Ok(intent) = oteryn_protocol_oteryn::item_view::decode_item_move_intent_for(
                    command.payload,
                    selection,
                ) && !authority
                    .player_trade_item_available(
                        actor,
                        UseCommand {
                            game_session_id: admitted.game_session_id,
                            command_id: command.command_id,
                            item_fence: admitted.item_fence,
                        },
                        view.resolve(intent.source),
                    )
                    .await?
                {
                    return Some(Dispatch::ItemMove(
                        ItemMoveOutcome::Rejected,
                        Vec::new(),
                        None,
                    ));
                }
                let step = item_move::item_move(
                    authority,
                    actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: command.command_id,
                        item_fence: admitted.item_fence,
                    },
                    command.payload,
                    selection,
                    selected_capabilities.contains(
                        oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_STACK_COUNT_V1,
                    ),
                    view,
                )
                .await;
                admitted.continuity.item_view = view.continuity();
                match step {
                    ItemMoveStep::Result(outcome, mut deltas) => {
                        // ITEM-LOOP-1: the open container views show the committed tree.
                        if let (ItemMoveOutcome::Moved, Some(views)) =
                            (outcome, container_views.as_mut())
                        {
                            let refreshed = transport::container_view::refresh::refresh(
                                authority,
                                actor,
                                admitted.item_fence,
                                views,
                                view,
                            )
                            .await;
                            admitted.continuity.item_view = view.continuity();
                            let Ok(delta) = refreshed else {
                                return None;
                            };
                            deltas.extend(delta);
                        }
                        Dispatch::ItemMove(outcome, deltas, None)
                    }
                    ItemMoveStep::Refused(outcome, detail) => {
                        Dispatch::ItemMove(outcome, Vec::new(), Some(detail))
                    }
                    ItemMoveStep::Disconnect => {
                        return None;
                    }
                }
            }
            None => Dispatch::Unregistered,
        }
    })
}
