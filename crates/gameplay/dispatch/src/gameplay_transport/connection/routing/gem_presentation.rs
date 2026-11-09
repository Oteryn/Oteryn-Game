use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus, gem_presentation as wire};
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    a: &A,
    s: &AdmittedSession,
    c: ClientCommandView<'_>,
) -> Dispatch {
    let Some(r) = wire::Request::decode(c.payload) else {
        return Dispatch::Reply(CommandStatus::Rejected, vec![]);
    };
    match a.gem_presentation_query(*s, r).await {
        Some(b) => Dispatch::Reply(CommandStatus::Accepted, b),
        None => Dispatch::Reply(CommandStatus::Rejected, vec![]),
    }
}
