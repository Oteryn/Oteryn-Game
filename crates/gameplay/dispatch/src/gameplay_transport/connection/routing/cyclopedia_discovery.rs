//! MAPDISC Cyclopedia discovery data: a read-only reply; a refused read has an empty rejected
//! result.
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    match authority
        .cyclopedia_discovery_query(*admitted, command.command_id, command.payload)
        .await
    {
        Some(bytes) => Dispatch::Reply(CommandStatus::Accepted, bytes),
        None => Dispatch::Reply(CommandStatus::Rejected, Vec::new()),
    }
}
