use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let details =
        command.command_type == oteryn_protocol_oteryn::market_details::COMMAND_TYPE_MARKET_DETAILS;
    if details != oteryn_protocol_oteryn::market_details::is_query(command.payload) {
        let bytes = if details {
            oteryn_protocol_oteryn::market_details::encode_reply(
                &oteryn_protocol_oteryn::market_details::Reply::Refused { code: 9 },
            )
            .ok()?
        } else {
            oteryn_protocol_oteryn::market::encode_reply(
                &oteryn_protocol_oteryn::market::MarketReply {
                    action: 0,
                    code: 9,
                    offer: [0; 16],
                    rows: Vec::new(),
                },
            )
            .ok()?
        };
        return Some(Dispatch::Reply(CommandStatus::Accepted, bytes));
    }
    let bytes = authority
        .market_command(*admitted, command.command_id, command.payload)
        .await?;
    Some(Dispatch::Reply(CommandStatus::Accepted, bytes))
}
