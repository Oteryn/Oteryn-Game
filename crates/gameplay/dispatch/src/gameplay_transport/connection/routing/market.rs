use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use crate::gameplay_transport::session_ingress::SessionCommandStream;
use oteryn_protocol_oteryn::market::{self as wire, MarketReply};
use oteryn_protocol_oteryn::market_details::{self, Query, Reply};
use oteryn_protocol_oteryn::{ClientCommandView, CommandStatus};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    commands: &SessionCommandStream,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    let details = command.command_type == market_details::COMMAND_TYPE_MARKET_DETAILS;
    if details != market_details::is_query(command.payload) {
        return refusal(details);
    }
    if details
        && matches!(
            market_details::decode_query(command.payload),
            Ok(Query::Statistics { .. })
        )
    {
        match commands.admit_market_statistics(tokio::time::Instant::now()) {
            Some(true) => {}
            Some(false) => {
                let bytes = market_details::encode_reply(&Reply::Refused {
                    code: market_details::STATISTICS_RATE_LIMITED,
                })
                .ok()?;
                return Some(Dispatch::Reply(CommandStatus::Accepted, bytes));
            }
            None => return refusal(details),
        }
    }
    match authority
        .market_command(*admitted, command.command_id, command.payload)
        .await
    {
        Some(bytes) => Some(Dispatch::Reply(CommandStatus::Accepted, bytes)),
        // A market mutation can lose its commit acknowledgement. The authority
        // exposes no error classification for this case.
        None if !details
            && wire::decode_request(command.payload).is_ok_and(|request| request.action >= 5) =>
        {
            None
        }
        None => refusal(details),
    }
}

fn refusal(details: bool) -> Option<Dispatch> {
    let bytes = if details {
        market_details::encode_reply(&Reply::Refused { code: 9 }).ok()?
    } else {
        wire::encode_reply(&MarketReply {
            action: 0,
            code: 9,
            offer: [0; 16],
            rows: Vec::new(),
        })
        .ok()?
    };
    Some(Dispatch::Reply(CommandStatus::Accepted, bytes))
}
