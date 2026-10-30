use super::super::{
    AdmittedSession, ContainerViewState, ExactActorRef, FreshAdmissionAuthority, SessionItemView,
    UseCommand,
};
use super::Dispatch;
use crate::gameplay_transport::{
    container_view::refresh,
    item_move::{self, ItemMoveStep},
};
use oteryn_protocol_oteryn::{
    ClientCommandView, hotkey_equip::Request, item_view::ItemMoveOutcome,
};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: &mut Option<SessionItemView>,
    views: &mut Option<ContainerViewState>,
) -> Option<Dispatch> {
    let Some(request) = Request::decode(command.payload) else {
        return Some(Dispatch::ItemMove(
            ItemMoveOutcome::Rejected,
            Vec::new(),
            None,
        ));
    };
    let (Some(view), Some(views)) = (item_view.as_mut(), views.as_mut()) else {
        return Some(Dispatch::ItemMove(
            ItemMoveOutcome::Rejected,
            Vec::new(),
            None,
        ));
    };
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: command.command_id,
        item_fence: admitted.item_fence,
    };
    let outcome = if admitted
        .continuity
        .selected_capabilities
        .contains(oteryn_protocol_oteryn::hotkey_equip::CAPABILITY_HOTKEY_EQUIP_CARRIED_V1)
    {
        authority
            .hotkey_equip_carried(actor, command, request)
            .await?
    } else {
        authority.hotkey_equip(actor, command, request).await?
    };
    let mut deltas = Vec::new();
    if outcome == ItemMoveOutcome::Moved {
        match item_move::moved(authority, actor, command, view).await {
            ItemMoveStep::Result(_, updated) => deltas.extend(updated),
            ItemMoveStep::Refused(_, _) | ItemMoveStep::Disconnect => return None,
        }
        deltas.extend(
            refresh::refresh(authority, actor, admitted.item_fence, views, view)
                .await
                .ok()?,
        );
    }
    admitted.continuity.item_view = view.continuity();
    Some(Dispatch::ItemMove(outcome, deltas, None))
}
