//! Party accounting is available only with both analyser and party capabilities.
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::analyser_party::decode_request;
pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    view: &mut crate::gameplay_transport::party_command::SessionPartyView,
) -> Dispatch {
    if ![10, 42]
        .iter()
        .all(|cap| admitted.continuity.selected_capabilities.contains(*cap))
    {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    }
    let Ok(prices) = decode_request(command.payload) else {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    };
    match authority
        .analyser_party_query(*admitted, command.command_id, &prices, view)
        .await
    {
        Some(payload) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload),
        None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
    }
}

#[cfg(test)]
#[path = "analyser_party/tests.rs"]
mod tests;
