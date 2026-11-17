use super::DepotSearchPanel;
use oteryn_client_runtime::cui3::depot_search::{self, Request};
use oteryn_client_runtime::server_mechanic::{
    CommandStatus, ServerMechanicCommand, ServerMechanicResponse,
};
pub fn command(payload: Vec<u8>) -> ServerMechanicCommand {
    ServerMechanicCommand {
        capability: 153,
        command_type: 182,
        payload,
    }
}
pub fn search(p: &mut DepotSearchPanel) -> Option<ServerMechanicCommand> {
    let payload = depot_search::encode_request(Request::Search {
        client_id: p.item_id,
    })
    .ok()
    .or_else(|| {
        p.feedback = Some(tr!("Enter a valid item id").into());
        None
    })?;
    request(p, command(payload))
}
pub fn detail(p: &mut DepotSearchPanel) -> Option<ServerMechanicCommand> {
    let payload = depot_search::encode_request(Request::Detail {
        client_id: p.item_id,
        tier: p.tier,
    })
    .ok()?;
    request(p, command(payload))
}
pub fn request(
    p: &mut DepotSearchPanel,
    c: ServerMechanicCommand,
) -> Option<ServerMechanicCommand> {
    if !p.selected || p.pending {
        return None;
    }
    p.pending = true;
    p.feedback = None;
    Some(c)
}
pub fn apply_result(p: &mut DepotSearchPanel, r: &ServerMechanicResponse) {
    p.pending = false;
    if !matches!(
        r.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        p.feedback = Some(lfmt!("Server refused depot search: {:?}", r.status));
        return;
    }
    match depot_search::decode_reply(&r.payload) {
        Ok(v) => {
            p.reply = Some(v);
            p.feedback = None
        }
        Err(_) => p.feedback = Some(tr!("The server returned invalid depot results").into()),
    }
}
#[cfg(test)]
// Direct fixture construction keeps the wire assertions readable.
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use oteryn_client_runtime::cui3::depot_search::{Outcome, Reply, SearchResult, TierCount};
    #[test]
    fn controls_encode_reply_and_refusal() {
        let mut p = DepotSearchPanel {
            selected: true,
            item_id: 100,
            ..Default::default()
        };
        let c = search(&mut p).unwrap();
        assert_eq!(
            depot_search::decode_request(&c.payload).unwrap(),
            Request::Search { client_id: 100 }
        );
        let wire = depot_search::encode_reply(&Reply {
            outcome: Outcome::Found,
            retrieved: 0,
            results: vec![SearchResult {
                client_id: 100,
                depot: 2,
                stash: 0,
                inbox: 0,
                tiers: vec![TierCount { tier: 0, count: 2 }],
            }],
        })
        .unwrap();
        apply_result(
            &mut p,
            &ServerMechanicResponse {
                command_id: 1,
                status: CommandStatus::Accepted,
                server_sequence: 1,
                payload: wire,
            },
        );
        assert_eq!(p.reply.as_ref().unwrap().results[0].depot, 2);
        apply_result(
            &mut p,
            &ServerMechanicResponse {
                command_id: 2,
                status: CommandStatus::Rejected,
                server_sequence: 2,
                payload: vec![],
            },
        );
        assert!(p.feedback.unwrap().contains("refused"));
    }
}
