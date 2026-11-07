use super::super::{AdmittedSession, FreshAdmissionAuthority, SessionItemView, SessionVisibility};
use super::Dispatch;
use oteryn_protocol_oteryn::inspection::{self as wire, Reply, Request};
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
    visibility: &Option<SessionVisibility>,
    views: &Option<SessionItemView>,
) -> Dispatch {
    let Ok(request) = wire::decode_request(command.payload) else {
        return Dispatch::Unregistered;
    };
    let mut reply = if !admitted.continuity.look.admit(tokio::time::Instant::now()) {
        Reply::refused(5)
    } else if matches!(request,Request::Player(entity)|Request::Permission {target:Some(entity),..} if !visibility.as_ref().is_some_and(|v|v.shows(entity)))
    {
        Reply::refused(3)
    } else {
        authority
            .inspection_command(*admitted, command.command_id, request, views.as_ref())
            .await
    };
    reply
        .requests
        .retain(|entity| visibility.as_ref().is_some_and(|view| view.shows(*entity)));
    match wire::encode_reply(&reply) {
        Ok(bytes) => Dispatch::Reply(CommandStatus::Accepted, bytes),
        Err(_) => Dispatch::Unregistered,
    }
}
