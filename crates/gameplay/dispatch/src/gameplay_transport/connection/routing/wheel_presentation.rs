use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, wheel_presentation as wire};
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    a: &A,
    s: &AdmittedSession,
    c: ClientCommandView<'_>,
) -> Dispatch {
    let Some(r) = wire::Request::decode(c.payload) else {
        return Dispatch::Reply(CommandStatus::Rejected, vec![]);
    };
    match a.wheel_presentation_query(*s, r).await {
        Some(p) => Dispatch::Reply(CommandStatus::Accepted, p),
        None => Dispatch::Reply(CommandStatus::Rejected, vec![]),
    }
}
