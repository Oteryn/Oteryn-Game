//! Party accounting is available only with both analyser and party capabilities.
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::analyser_party::{
    CAPABILITY_ANALYSER_PARTY_INTERVAL_V1, CAPABILITY_ANALYSER_PARTY_VALUATION_V1,
    COMMAND_TYPE_ANALYSER_PARTY_QUERY, COMMAND_TYPE_ANALYSER_PARTY_VALUATION, HuntRequest,
};
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
    let required = match command.command_type {
        COMMAND_TYPE_ANALYSER_PARTY_QUERY => 10,
        COMMAND_TYPE_ANALYSER_PARTY_VALUATION => CAPABILITY_ANALYSER_PARTY_VALUATION_V1,
        _ => CAPABILITY_ANALYSER_PARTY_INTERVAL_V1,
    };
    if !admitted.continuity.selected_capabilities.contains(required) {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    }
    let Ok(request) = HuntRequest::decode(command.command_type, command.payload) else {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    };
    match authority
        .analyser_party_query(
            *admitted,
            command.command_id,
            command.command_type,
            request,
            view,
        )
        .await
    {
        Some(payload) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload),
        None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
    }
}

#[cfg(test)]
#[path = "analyser_party/tests.rs"]
mod tests;
