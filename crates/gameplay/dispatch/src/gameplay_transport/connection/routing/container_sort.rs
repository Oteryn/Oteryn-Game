use super::super::{AdmittedSession, ContainerViewState, FreshAdmissionAuthority, SessionItemView};
use super::Dispatch;
use crate::gameplay_transport::{container_view::refresh, item_view::ItemKey};
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, container_sort::Request};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
    item_view: &mut Option<SessionItemView>,
    views: &mut Option<ContainerViewState>,
) -> Option<Dispatch> {
    let refused = || Dispatch::Reply(CommandStatus::Rejected, vec![2]);
    let Some(request) = Request::decode(command.payload) else {
        return Some(refused());
    };
    if let Some(replayed) = authority
        .container_sort_replay(*admitted, command.command_id, request)
        .await?
    {
        if !replayed {
            return Some(refused());
        }
        let mut deltas = Vec::new();
        if let (Some(view), Some(views), Some(actor)) =
            (item_view.as_mut(), views.as_mut(), admitted.runtime_actor)
        {
            deltas.extend(
                refresh::refresh(authority, actor, admitted.item_fence, views, view)
                    .await
                    .ok()?,
            );
            admitted.continuity.item_view = view.continuity();
        }
        return Some(Dispatch::ContainerSort(deltas));
    }
    let (Some(view), Some(views), Some(actor)) =
        (item_view.as_mut(), views.as_mut(), admitted.runtime_actor)
    else {
        return Some(refused());
    };
    let Some(ItemKey::Instance(target)) = views.container_for_view(request.view) else {
        return Some(refused());
    };
    if !view.admit_view_command(tokio::time::Instant::now()) {
        return Some(refused());
    }
    admitted.continuity.item_view = view.continuity();
    let result = authority
        .container_sort(*admitted, command.command_id, target, request)
        .await?;
    if !result {
        return Some(refused());
    }
    let deltas = refresh::refresh(authority, actor, admitted.item_fence, views, view)
        .await
        .ok()?;
    admitted.continuity.item_view = view.continuity();
    Some(Dispatch::ContainerSort(deltas.into_iter().collect()))
}
