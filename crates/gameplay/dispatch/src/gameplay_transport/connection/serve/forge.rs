//! Refresh the existing inventory and container views after a Forge item mutation.
use super::*;

pub(super) fn changes_inventory(command: oteryn_protocol_oteryn::ClientCommandView<'_>) -> bool {
    command.command_type == oteryn_protocol_oteryn::item_forge::COMMAND_TYPE_ITEM_FORGE
        && oteryn_protocol_oteryn::item_forge::Request::decode(command.payload)
            .is_some_and(|request| request.operation.changes_inventory())
}

pub(super) async fn refresh<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let Some(view) = session.item_view.as_mut() else {
        return Err(ServeEnd::Disconnected);
    };
    item_use_view_refresh::refresh(
        stream,
        authority,
        session.actor,
        &mut session.admitted,
        view,
        session.container_views.as_mut(),
        &mut session.sequence,
    )
    .await
    .ok_or(ServeEnd::Disconnected)
}
