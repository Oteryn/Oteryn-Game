use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, house_podium_display::Request};
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    view: Option<&crate::gameplay_transport::item_view::SessionItemView>,
    command: ClientCommandView<'_>,
) -> Dispatch {
    let Some(request) = Request::decode(command.payload) else {
        return Dispatch::Reply(CommandStatus::Rejected, Vec::new());
    };
    let Some(request) = crate::gameplay_transport::house_podium_display::resolve(request, view)
    else {
        return Dispatch::Reply(CommandStatus::Rejected, Vec::new());
    };
    match authority.house_podium_display(*admitted, request).await {
        Some(reply) => Dispatch::Reply(CommandStatus::Accepted, reply),
        None => Dispatch::Reply(CommandStatus::Rejected, Vec::new()),
    }
}
