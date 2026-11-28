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

const FIXTURE_SESSION: &str = "00000000-0000-4000-8000-000000000001";

/// One captured `ServerMechanicResponse` envelope (`n03-list-<index>.json`, `n03-retrieve.json`):
/// the writer's `game_session_id`, `command_id`, `server_sequence`, numeric `status` and raw
/// `payload` bytes.
struct Envelope {
    session: String,
    response: ServerMechanicResponse,
}

struct Captured {
    session: String,
    pages: Vec<Envelope>,
    retrieve: Envelope,
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

fn is_uuid(value: &str) -> bool {
    let parts = value.split('-').collect::<Vec<_>>();
    parts.len() == 5
        && parts
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(part, width)| part.len() == width && part.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn status(value: u64) -> Option<CommandStatus> {
    match value {
        1 => Some(CommandStatus::Accepted),
        2 => Some(CommandStatus::Rejected),
        3 => Some(CommandStatus::DuplicateReplay),
        4 => Some(CommandStatus::DuplicateOutcomeExpired),
        _ => None,
    }
}

fn envelope(path: &std::path::Path) -> Result<Envelope, Error> {
    let name = path.display();
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let positive = |key: &str| -> Result<u64, Error> {
        value[key]
            .as_u64()
            .filter(|number| *number > 0)
            .ok_or_else(|| format!("{name}: {key} is not a positive integer").into())
    };
    let session = value["game_session_id"]
        .as_str()
        .filter(|id| is_uuid(id))
        .ok_or_else(|| format!("{name}: game_session_id is not a UUID"))?
        .to_owned();
    let payload = value["payload"]
        .as_array()
        .ok_or_else(|| format!("{name}: payload is not a byte array"))?
        .iter()
        .map(|byte| byte.as_u64().and_then(|byte| u8::try_from(byte).ok()))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| format!("{name}: payload is not a byte array"))?;
    Ok(Envelope {
        session,
        response: ServerMechanicResponse {
            command_id: positive("command_id")?,
            server_sequence: positive("server_sequence")?,
            status: value["status"]
                .as_u64()
                .and_then(status)
                .ok_or_else(|| format!("{name}: unknown status"))?,
            payload,
        },
    })
}

fn fixture() -> Result<Captured, Error> {
    let envelope = |index: u64, payload| Envelope {
        session: FIXTURE_SESSION.into(),
        response: ServerMechanicResponse {
            command_id: 0x41 + index,
            server_sequence: 0x900 + 2 * index,
            status: CommandStatus::Accepted,
            payload,
        },
    };
    let tiers = vec![TierCount { tier: 0, count: 3 }];
    Ok(Captured {
        session: FIXTURE_SESSION.into(),
        pages: vec![
            envelope(
                0,
                encode(&Reply {
                    outcome: Outcome::More,
                    retrieved: 0,
                    results: (100..164)
                        .map(|id| row(id, 1, vec![TierCount { tier: 0, count: 1 }]))
                        .collect(),
                })?,
            ),
            envelope(
                1,
                encode(&Reply {
                    outcome: Outcome::Listed,
                    retrieved: 0,
                    results: vec![row(2854, 3, tiers)],
                })?,
            ),
        ],
        retrieve: envelope(
            2,
            encode(&Reply {
                outcome: Outcome::Retrieved,
                retrieved: 1,
                results: vec![],
            })?,
        ),
        item: 2854,
        row: "3|0|0|0x3".into(),
        retrieved: 1,
    })
}

/// Loads the capture under `root`: the envelopes must all carry the session named by
/// `n03-expected.txt`, and their command IDs and server sequences strictly increase in
/// capture order (list pages, then the retrieve).
fn captured_from(root: &std::path::Path) -> Result<Captured, Error> {
    let expected = std::fs::read_to_string(root.join("n03-expected.txt"))?;
    let value = |key: &str| -> Result<String, Error> {
        expected
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .map(str::to_owned)
            .ok_or_else(|| format!("n03-expected.txt lacks {key}").into())
    };
    let session = value("session")?;
    if !is_uuid(&session) {
        return Err(format!("n03-expected.txt session {session} is not a UUID").into());
    }
    let count: usize = value("pages")?.parse()?;
    let pages = (0..count)
        .map(|index| envelope(&root.join(format!("n03-list-{index}.json"))))
        .collect::<Result<Vec<_>, _>>()?;
    let retrieve = envelope(&root.join("n03-retrieve.json"))?;
    let mut previous = (0, 0);
    for captured in pages.iter().chain(std::iter::once(&retrieve)) {
        if captured.session != session {
            return Err(format!("envelope session {} is not {session}", captured.session).into());
        }
        if captured.response.status != CommandStatus::Accepted {
            return Err(format!("envelope status {:?} is not Accepted", captured.response.status).into());
        }
        let current = (captured.response.command_id, captured.response.server_sequence);
        if current.0 <= previous.0 || current.1 <= previous.1 {
            return Err(format!(
                "command_id/server_sequence {current:?} do not increase after {previous:?}"
            )
            .into());
        }
        previous = current;
    }
    Ok(Captured {
        session,
        pages,
        retrieve,
        item: value("item")?.parse()?,
        row: value("row")?,
        retrieved: value("retrieve")?
            .split('|')
            .nth(1)
            .ok_or("retrieved count")?
            .parse()?,
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

fn replay(captured: &Captured) -> Result<(), Error> {
    let (link, mut receiver, outbox) = play_channel();
    outbox.depot_search_initialize(true);
    let inputs = Inputs::default();

    activate(&link, &inputs, 5);
    assert_eq!(sent(&mut receiver)?, Request::Open { after: 0 });
    let mut previous = None;
    for page in &captured.pages {
        if let Some(after) = previous {
            activate(&link, &inputs, 6);
            assert_eq!(sent(&mut receiver)?, Request::Open { after });
        }
        outbox.depot_search_result(&page.response);
        let panel = link.depot_search_panel();
        assert!(!panel.pending);
        let reply = panel.reply.as_ref().ok_or("a captured page left no reply")?;
        previous = reply.results.last().map(|r| r.client_id);
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
    outbox.depot_search_result(&captured.retrieve.response);
    let panel = link.depot_search_panel();
    assert!(!panel.pending);
    let reply = panel.reply.clone().ok_or("no retrieve reply")?;
    assert_eq!((reply.outcome, reply.retrieved), (Outcome::Retrieved, captured.retrieved));
    Ok(())
}

#[test]
fn e2e_n03_depot_list_retrieve_one_and_refusals() -> Result<(), Error> {
    match std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        Some(root) => {
            let captured = captured_from(std::path::Path::new(&root))?;
            replay(&captured)?;
            depot_search_malformed_payload_and_refusal_keep_the_last_reply()?;
            println!("N03_NATIVE_REPLAY session={}", captured.session);
            Ok(())
        }
        None => replay(&fixture()?),
    }
}

#[test]
fn depot_search_malformed_payload_and_refusal_keep_the_last_reply() -> Result<(), Error> {
    let (link, mut receiver, outbox) = play_channel();
    outbox.depot_search_initialize(true);
    activate(&link, &Inputs::default(), 5);
    assert_eq!(sent(&mut receiver)?, Request::Open { after: 0 });
    let listed = Reply {
        outcome: Outcome::Listed,
        retrieved: 0,
        results: vec![row(2854, 1, vec![TierCount { tier: 0, count: 1 }])],
    };
    outbox.depot_search_result(&response(1, CommandStatus::Accepted, encode(&listed)?));
    assert_eq!(link.depot_search_panel().reply.as_ref(), Some(&listed));
    // The errors follow a Retrieved reply, as in the live journey.
    let mut inputs = Inputs::default();
    inputs.0[2] = "0".into();
    activate(&link, &inputs, 7);
    assert_eq!(
        sent(&mut receiver)?,
        Request::Retrieve { client_id: 2854, all: false }
    );
    let retrieved = Reply {
        outcome: Outcome::Retrieved,
        retrieved: 1,
        results: vec![],
    };
    outbox.depot_search_result(&response(2, CommandStatus::Accepted, encode(&retrieved)?));
    let panel = link.depot_search_panel();
    assert_eq!(panel.reply.as_ref(), Some(&retrieved));
    for (command_id, payload, status, feedback) in [
        (3, vec![0xff, 0xff], CommandStatus::Accepted, "The server returned invalid depot results"),
        (4, Vec::new(), CommandStatus::Rejected, "Server refused depot search: Rejected"),
    ] {
        activate(&link, &Inputs::default(), 5);
        assert_eq!(sent(&mut receiver)?, Request::Open { after: 0 });
        outbox.depot_search_result(&response(command_id, status, payload));
        let after = link.depot_search_panel();
        assert_eq!(after.feedback.as_deref(), Some(feedback));
        assert_eq!(after.reply, panel.reply);
        assert!(!after.pending);
    }
    Ok(())
}

fn write_capture(captured: &Captured) -> Result<tempfile::TempDir, Error> {
    let root = tempfile::tempdir()?;
    let json = |envelope: &Envelope| {
        serde_json::to_vec(&serde_json::json!({
            "game_session_id": envelope.session,
            "command_id": envelope.response.command_id,
            "server_sequence": envelope.response.server_sequence,
            "status": envelope.response.status as u64,
            "payload": envelope.response.payload,
        }))
    };
    for (index, page) in captured.pages.iter().enumerate() {
        std::fs::write(root.path().join(format!("n03-list-{index}.json")), json(page)?)?;
    }
    std::fs::write(root.path().join("n03-retrieve.json"), json(&captured.retrieve)?)?;
    std::fs::write(
        root.path().join("n03-expected.txt"),
        format!(
            "session={}\nitem={}\npages={}\nlisted=Listed\nrow={}\nretrieve=Retrieved|{}\n",
            captured.session,
            captured.item,
            captured.pages.len(),
            captured.row,
            captured.retrieved
        ),
    )?;
    Ok(root)
}

#[test]
fn e2e_n03_replays_captured_envelopes() -> Result<(), Error> {
    let root = write_capture(&fixture()?)?;
    let captured = captured_from(root.path())?;
    assert_eq!(captured.session, FIXTURE_SESSION);
    assert_eq!(captured.retrieve.response.server_sequence, 0x904);
    replay(&captured)
}

#[test]
fn e2e_n03_rejects_inconsistent_capture_identity() -> Result<(), Error> {
    type Mutation = fn(&mut Captured);
    let cases: [(&str, Mutation); 5] = [
        ("refused page", |c| c.pages[0].response.status = CommandStatus::Rejected),
        ("foreign session", |c| c.retrieve.session = "00000000-0000-4000-8000-000000000002".into()),
        ("non-UUID session", |c| c.pages[0].session = "session-1".into()),
        ("repeated command_id", |c| {
            c.retrieve.response.command_id = c.pages[1].response.command_id;
        }),
        ("repeated server_sequence", |c| {
            c.pages[1].response.server_sequence = c.pages[0].response.server_sequence;
        }),
    ];
    for (case, mutate) in cases {
        let mut captured = fixture()?;
        mutate(&mut captured);
        let root = write_capture(&captured)?;
        assert!(captured_from(root.path()).is_err(), "{case} was accepted");
    }
    Ok(())
}

#[test]
fn depot_actions_parse_only_their_required_fields() -> Result<(), Error> {
    for detail in [false, true] {
        let (link, mut receiver, outbox) = play_channel();
        outbox.depot_search_initialize(true);
        let mut inputs = Inputs::default();
        if detail {
            activate(&link, &inputs, 5);
            assert_eq!(sent(&mut receiver)?, Request::Open { after: 0 });
            outbox.depot_search_result(&response(1, CommandStatus::Accepted, encode(&Reply {
                outcome: Outcome::Listed, retrieved: 0,
                results: vec![row(2854, 1, vec![TierCount { tier: 2, count: 1 }])],
            })?));
            inputs.0[0].clear();
            inputs.0[3] = "2".into();
            activate(&link, &inputs, 4);
            assert_eq!(sent(&mut receiver)?, Request::Detail { client_id: 2854, tier: 2 });
        } else {
            inputs.0[3].clear();
            activate(&link, &inputs, 1);
            assert_eq!(sent(&mut receiver)?, Request::Search { client_id: 100 });
        }
    }
    Ok(())
}
