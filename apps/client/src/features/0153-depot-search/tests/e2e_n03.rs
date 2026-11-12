//! N03 replay: the captured depot holdings list and RETRIEVE(one) reply driven through the native
//! controls (`activate_shared`) and the `apply_result` reducer of the production client.
use super::{Inputs, activate, presentation};
use crate::combat_input::PlayCommand;
use crate::play::play_channel;
use oteryn_client_runtime::cui3::depot_search::{
    self, Outcome, Reply, Request, SearchResult, TierCount,
};
use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};

type Error = Box<dyn std::error::Error>;

struct Captured {
    pages: Vec<Vec<u8>>,
    retrieve: Vec<u8>,
    item: u32,
    row: String,
    retrieved: u32,
}

fn response(command_id: u64, status: CommandStatus, payload: Vec<u8>) -> ServerMechanicResponse {
    ServerMechanicResponse {
        command_id,
        status,
        server_sequence: command_id,
        payload,
    }
}

fn encode(reply: &Reply) -> Result<Vec<u8>, Error> {
    depot_search::encode_reply(reply).map_err(|error| format!("{error:?}").into())
}

fn row(client_id: u32, depot: u32, tiers: Vec<TierCount>) -> SearchResult {
    SearchResult {
        client_id,
        depot,
        stash: 0,
        inbox: 0,
        tiers,
    }
}

fn captured() -> Result<Captured, Error> {
    let value = |expected: &str, key: &str| -> Result<String, Error> {
        expected
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .map(str::to_owned)
            .ok_or_else(|| format!("n03-expected.txt lacks {key}").into())
    };
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        let tiers = vec![TierCount { tier: 0, count: 3 }];
        return Ok(Captured {
            pages: vec![
                encode(&Reply {
                    outcome: Outcome::More,
                    retrieved: 0,
                    results: (100..164)
                        .map(|id| row(id, 1, vec![TierCount { tier: 0, count: 1 }]))
                        .collect(),
                })?,
                encode(&Reply {
                    outcome: Outcome::Listed,
                    retrieved: 0,
                    results: vec![row(2854, 3, tiers)],
                })?,
            ],
            retrieve: encode(&Reply {
                outcome: Outcome::Retrieved,
                retrieved: 1,
                results: vec![],
            })?,
            item: 2854,
            row: "3|0|0|0x3".into(),
            retrieved: 1,
        });
    };
    let root = std::path::Path::new(&root);
    let expected = std::fs::read_to_string(root.join("n03-expected.txt"))?;
    let count: usize = value(&expected, "pages")?.parse()?;
    let pages = (0..count)
        .map(|index| std::fs::read(root.join(format!("n03-list-{index}.bin"))))
        .collect::<Result<Vec<_>, _>>()?;
    let retrieve = value(&expected, "retrieve")?;
    Ok(Captured {
        pages,
        retrieve: std::fs::read(root.join("n03-retrieve.bin"))?,
        item: value(&expected, "item")?.parse()?,
        row: value(&expected, "row")?,
        retrieved: retrieve.split('|').nth(1).ok_or("retrieved count")?.parse()?,
    })
}

fn sent(
    receiver: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result<Request, Error> {
    let PlayCommand::ServerMechanic(command) = receiver.try_recv()? else {
        return Err("depot command was not a server mechanic".into());
    };
    assert_eq!((command.capability, command.command_type), (153, 182));
    depot_search::decode_request(&command.payload).map_err(|error| format!("{error:?}").into())
}

#[test]
fn e2e_n03_depot_list_retrieve_one_and_refusals() -> Result<(), Error> {
    let captured = captured()?;
    let (link, mut receiver, outbox) = play_channel();
    outbox.depot_search_initialize(true);
    let inputs = Inputs::default();

    activate(&link, &inputs, 5);
    assert_eq!(sent(&mut receiver)?, Request::Open { after: 0 });
    let mut previous = None;
    for (index, page) in captured.pages.iter().enumerate() {
        if let Some(after) = previous {
            activate(&link, &inputs, 6);
            assert_eq!(sent(&mut receiver)?, Request::Open { after });
        }
        outbox.depot_search_result(&response(index as u64 + 1, CommandStatus::Accepted, page.clone()));
        let panel = link.depot_search_panel();
        assert!(!panel.pending);
        previous = panel.reply.as_ref().and_then(|reply| reply.results.last()).map(|r| r.client_id);
    }

    let panel = link.depot_search_panel();
    let listed = panel.reply.clone().ok_or("no holdings list")?;
    assert_eq!(listed.outcome, Outcome::Listed);
    let position = listed
        .results
        .iter()
        .position(|row| row.client_id == captured.item)
        .ok_or("deposited item missing from the client list")?;
    let found = &listed.results[position];
    let tiers = found
        .tiers
        .iter()
        .map(|tier| format!("{}x{}", tier.tier, tier.count))
        .collect::<Vec<_>>()
        .join(",");
    assert_eq!(
        format!("{}|{}|{}|{tiers}", found.depot, found.stash, found.inbox),
        captured.row
    );
    assert!(presentation(&panel).contains(&format!("depot {}", found.depot)));

    // Retrieve one of the selected row; the reply carries the Retrieved count.
    let mut inputs = Inputs::default();
    inputs.0[2] = position.to_string();
    activate(&link, &inputs, 7);
    assert_eq!(
        sent(&mut receiver)?,
        Request::Retrieve { client_id: captured.item, all: false }
    );
    outbox.depot_search_result(&response(10, CommandStatus::Accepted, captured.retrieve.clone()));
    let panel = link.depot_search_panel();
    let reply = panel.reply.clone().ok_or("no retrieve reply")?;
    assert_eq!((reply.outcome, reply.retrieved), (Outcome::Retrieved, captured.retrieved));

    // A malformed payload and a refusal each leave the last reply untouched.
    for (payload, status, feedback) in [
        (vec![0xff, 0xff], CommandStatus::Accepted, "The server returned invalid depot results"),
        (Vec::new(), CommandStatus::Rejected, "Server refused depot search: Rejected"),
    ] {
        activate(&link, &Inputs::default(), 5);
        assert_eq!(sent(&mut receiver)?, Request::Open { after: 0 });
        outbox.depot_search_result(&response(11, status, payload));
        let after = link.depot_search_panel();
        assert_eq!(after.feedback.as_deref(), Some(feedback));
        assert_eq!(after.reply, panel.reply);
        assert!(!after.pending);
    }
    Ok(())
}
