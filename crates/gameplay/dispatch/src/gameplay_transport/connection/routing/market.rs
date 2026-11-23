use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::market_details::{self, Query, Reply};
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let details = command.command_type == market_details::COMMAND_TYPE_MARKET_DETAILS;
    if details != market_details::is_query(command.payload) {
        let bytes = if details {
            market_details::encode_reply(&Reply::Refused { code: 9 }).ok()?
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
    if details
        && matches!(market_details::decode_query(command.payload), Ok(Query::Statistics { .. }))
        && !admitted.continuity.market_statistics.admit(tokio::time::Instant::now())
    {
        let bytes = market_details::encode_reply(&Reply::Refused {
            code: market_details::STATISTICS_RATE_LIMITED,
        })
        .ok()?;
        return Some(Dispatch::Reply(CommandStatus::Accepted, bytes));
    }
    let bytes = authority
        .market_command(*admitted, command.command_id, command.payload)
        .await?;
    Some(Dispatch::Reply(CommandStatus::Accepted, bytes))
}
