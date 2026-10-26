//! The connection retains capability admission, framing and command replay.
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let (status, bytes) =
        super::super::super::podium::dispatch(authority, admitted, command).await?;
    Some(Dispatch::Reply(status, bytes))
}
