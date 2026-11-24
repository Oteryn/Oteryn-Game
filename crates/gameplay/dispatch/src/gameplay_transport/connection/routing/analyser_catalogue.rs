use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::analyser::CAPABILITY_ANALYSER_V1;
use oteryn_protocol_oteryn::analyser_catalogue::{
    CAPABILITY_ANALYSER_NPC_PRICES_V1, COMMAND_TYPE_ANALYSER_NPC_PRICES, decode_request,
};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Dispatch {
    let npc_prices = command.command_type == COMMAND_TYPE_ANALYSER_NPC_PRICES;
    if npc_prices
        && !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_ANALYSER_NPC_PRICES_V1)
    {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    }
    if !admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_ANALYSER_V1)
    {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    }
    let Ok(ids) = decode_request(command.payload) else {
        return Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new());
    };
    let reply = if npc_prices {
        authority.analyser_npc_prices_query(*admitted, &ids).await
    } else {
        authority.analyser_catalogue_query(*admitted, &ids).await
    };
    match reply {
        Some(payload) => Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload),
        None => Dispatch::Reply(crate::foundation::CommandStatus::Rejected, Vec::new()),
    }
}

#[cfg(test)]
#[path = "analyser_catalogue/tests.rs"]
mod tests;
