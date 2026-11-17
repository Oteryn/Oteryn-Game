//! Obtain preferences share native custody checks; OPEN uses the existing container view owner.
use super::super::{
    AdmittedSession, ContainerViewState, ExactActorRef, FreshAdmissionAuthority, SessionItemView,
    UseCommand,
};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::{
    ClientCommandView,
    npc_purchase::{ObtainAction, decode_obtain_container},
    quick_loot::encode_quick_loot_result,
};
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
    container_views: &mut Option<ContainerViewState>,
    item_view: &mut Option<SessionItemView>,
) -> Option<Dispatch> {
    let request = match decode_obtain_container(command.payload) {
        Ok(value) => value,
        Err(_) => return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new())),
    };
    let (status, result) = authority
        .obtain_container(
            *admitted,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command.command_id,
                item_fence: admitted.item_fence,
            },
            request,
        )
        .await?;
    let mut deltas = Vec::new();
    if request.action == ObtainAction::Open
        && status == CommandStatus::Accepted
        && let Some(selected) = result.items.first()
        && selected.accepted
        && let (Some(views), Some(view)) = (container_views.as_mut(), item_view.as_mut())
    {
        let key = super::super::ItemKey::Instance(selected.item);
        let observation = authority
            .observe_container_with_house_capacity(
                actor,
                admitted.item_fence,
                key,
                admitted.continuity.selected_capabilities.contains(
                    oteryn_protocol_oteryn::container_tree::CAPABILITY_HOUSE_CONTAINER_CAPACITY_V2,
                ),
            )
            .await;
        let (_, delta) = views
            .apply(
                view,
                super::super::super::container_view::PendingOpen::new_view(key),
                observation,
            )
            .ok()?;
        admitted.continuity.item_view = view.continuity();
        deltas.extend(delta);
    }
    Some(Dispatch::NpcShop(
        status,
        encode_quick_loot_result(&result).ok()?,
        super::super::NpcUpdate::default(),
        deltas,
    ))
}
