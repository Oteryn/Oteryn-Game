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
    depot_search_selected: bool,
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
        depot_search_selected: true,
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
            return Err(format!(
                "envelope status {:?} is not Accepted",
                captured.response.status
            )
            .into());
        }
        let current = (
            captured.response.command_id,
            captured.response.server_sequence,
        );
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
        depot_search_selected: value("depot_search_selected")?.parse()?,
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
    decode_command(receiver.try_recv()?)
}

fn decode_command(command: PlayCommand) -> Result<Request, Error> {
    let PlayCommand::ServerMechanic(command) = command else {
        return Err("depot command was not a server mechanic".into());
    };
    assert_eq!((command.capability, command.command_type), (153, 182));
    depot_search::decode_request(&command.payload).map_err(|error| format!("{error:?}").into())
}

async fn sent_from_session(
    receiver: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result<Request, Error> {
    let command = tokio::time::timeout(std::time::Duration::from_secs(3), receiver.recv())
        .await?
        .ok_or("depot session closed before sending the command")?;
    decode_command(command)
}

struct CapturePeer {
    selected: bool,
    requests: tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    replies: tokio::sync::mpsc::UnboundedReceiver<ServerMechanicResponse>,
}

impl crate::play::Stepper for CapturePeer {
    fn server_mechanic_selected(&self, capability: u32) -> bool {
        self.selected && capability == 153
    }

    async fn server_mechanic(
        &mut self,
        request: &oteryn_client_runtime::server_mechanic::ServerMechanicCommand,
    ) -> Result<ServerMechanicResponse, oteryn_session::SessionError> {
        self.requests
            .send(PlayCommand::ServerMechanic(request.clone()))
            .map_err(|_| oteryn_session::SessionError::SessionUnusable)?;
        self.replies
            .recv()
            .await
            .ok_or(oteryn_session::SessionError::SessionUnusable)
    }

    async fn serve(
        &mut self,
        duration: std::time::Duration,
    ) -> Result<(), oteryn_session::SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }
    fn drain(&mut self) -> Vec<oteryn_session::SessionEvent> {
        vec![]
    }
    async fn step(
        &mut self,
        _: oteryn_session::StepDirection,
    ) -> Result<oteryn_session::StepOutcome, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }
    async fn combat(
        &mut self,
        _: crate::combat_input::CombatCommand,
    ) -> Result<crate::combat_input::CombatReport, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }
    async fn logout(
        &mut self,
    ) -> Result<Option<oteryn_session::LogoutResult>, oteryn_session::SessionError> {
        Ok(None)
    }
}

async fn wait_until(mut ready: impl FnMut() -> bool) -> Result<(), Error> {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while !ready() {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
    })
    .await
    .map_err(|_| "depot capability or response never reached the panel".into())
}

fn replay(captured: &Captured) -> Result<(), Error> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn_scoped(scope, || {
                let run = || -> Result<(), Error> {
                    oteryn_client_runtime::ClientRuntime::new()?
                        .block_on(replay_session(captured))?
                };
                run().map_err(|error| error.to_string())
            })?
            .join()
            .map_err(|_| "N03 replay thread panicked")?
            .map_err(Into::into)
    })
}

async fn replay_session(captured: &Captured) -> Result<(), Error> {
    let (link, commands, outbox) = play_channel();
    let (requests, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let (replies, incoming) = tokio::sync::mpsc::unbounded_channel();
    let _task = tokio::spawn(crate::play::run_session(
        CapturePeer {
            selected: captured.depot_search_selected,
            requests,
            replies: incoming,
        },
        commands,
        outbox,
    ));
    wait_until(|| link.depot_search_panel().selected).await?;
    #[cfg(bevy_shell)]
    let mut ui = crate::bevy_shell::DepotSearchReplayUi::new(crate::play::PlayLink {
        owns_session: false,
        commands: link.commands.clone(),
        mailbox: link.mailbox.clone(),
    })?;
    let inputs = Inputs::default();
    macro_rules! press {
        ($label:expr, $inputs:expr, $action:expr) => {{
            #[cfg(bevy_shell)]
            {
                let _ = $inputs;
                ui.press($label)?;
            }
            #[cfg(not(bevy_shell))]
            activate(&link, $inputs, $action);
        }};
    }

    press!("List depot holdings", &inputs, 5);
    assert_eq!(
        sent_from_session(&mut receiver).await?,
        Request::Open { after: 0 }
    );
    let mut previous = None;
    for (index, page) in captured.pages.iter().enumerate() {
        if let Some(after) = previous {
            press!("Next page", &inputs, 6);
            assert_eq!(
                sent_from_session(&mut receiver).await?,
                Request::Open { after }
            );
        }
        let expected = depot_search::decode_reply(&page.response.payload)
            .map_err(|error| format!("invalid captured holdings page: {error:?}"))?;
        let terminal = index + 1 == captured.pages.len();
        let mut cursor = previous.unwrap_or(0);
        for row in &expected.results {
            if row.client_id <= cursor {
                return Err(format!("holdings page {index} overlaps or is out of order").into());
            }
            cursor = row.client_id;
        }
        if expected.outcome
            != if terminal {
                Outcome::Listed
            } else {
                Outcome::More
            }
            || expected.retrieved != 0
            || (!terminal && expected.results.is_empty())
        {
            return Err(format!("invalid continuation on holdings page {index}").into());
        }
        replies.send(page.response.clone())?;
        wait_until(|| !link.depot_search_panel().pending).await?;
        let panel = link.depot_search_panel();
        assert!(!panel.pending);
        let reply = panel
            .reply
            .as_ref()
            .ok_or("a captured page left no reply")?;
        assert_eq!(reply, &expected, "panel did not show captured page {index}");
        previous = Some(cursor);
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
    press!(
        &format!("Retrieve one of item {}", captured.item),
        &inputs,
        7
    );
    assert_eq!(
        sent_from_session(&mut receiver).await?,
        Request::Retrieve {
            client_id: captured.item,
            all: false
        }
    );
    replies.send(captured.retrieve.response.clone())?;
    wait_until(|| !link.depot_search_panel().pending).await?;
    let panel = link.depot_search_panel();
    assert!(!panel.pending);
    let reply = panel.reply.clone().ok_or("no retrieve reply")?;
    assert_eq!(
        (reply.outcome, reply.retrieved),
        (Outcome::Retrieved, captured.retrieved)
    );
    assert!(
        presentation(&panel)
            .lines()
            .any(|line| line == format!("Retrieved: {}", captured.retrieved))
    );
    #[cfg(bevy_shell)]
    assert!(ui.shows_line(&format!("Retrieved: {}", captured.retrieved)));
    for (command_id, payload, status, feedback) in [
        (
            1,
            vec![0xff, 0xff],
            CommandStatus::Accepted,
            "The server returned invalid depot results",
        ),
        (
            2,
            Vec::new(),
            CommandStatus::Rejected,
            "Server refused depot search: Rejected",
        ),
    ] {
        press!("List depot holdings", &Inputs::default(), 5);
        assert_eq!(
            sent_from_session(&mut receiver).await?,
            Request::Open { after: 0 }
        );
        replies.send(response(
            captured.retrieve.response.command_id + command_id,
            status,
            payload,
        ))?;
        wait_until(|| !link.depot_search_panel().pending).await?;
        let after = link.depot_search_panel();
        assert_eq!(after.feedback.as_deref(), Some(feedback));
        assert_eq!(after.reply, panel.reply);
        assert!(!after.pending);
        #[cfg(bevy_shell)]
        {
            assert!(ui.shows_line(feedback));
            assert!(ui.shows_line(&format!("Retrieved: {}", captured.retrieved)));
        }
    }
    Ok(())
}

#[test]
fn e2e_n03_depot_list_retrieve_one_and_refusals() -> Result<(), Error> {
    match std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        Some(root) => {
            if !cfg!(bevy_shell) {
                return Err("N03 live captures require the shipped Bevy controls".into());
            }
            let captured = captured_from(std::path::Path::new(&root))?;
            replay(&captured)?;
            println!("N03_NATIVE_REPLAY session={}", captured.session);
            Ok(())
        }
        None => replay(&fixture()?),
    }
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
        std::fs::write(
            root.path().join(format!("n03-list-{index}.json")),
            json(page)?,
        )?;
    }
    std::fs::write(
        root.path().join("n03-retrieve.json"),
        json(&captured.retrieve)?,
    )?;
    std::fs::write(
        root.path().join("n03-expected.txt"),
        format!(
            "session={}\ndepot_search_selected={}\nitem={}\npages={}\nlisted=Listed\nrow={}\nretrieve=Retrieved|{}\n",
            captured.session,
            captured.depot_search_selected,
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
fn e2e_n03_checks_each_page_and_rejects_overlapping_cursor() -> Result<(), Error> {
    let mut captured = fixture()?;
    let middle = |start| {
        encode(&Reply {
            outcome: Outcome::More,
            retrieved: 0,
            results: (start..start + 64)
                .map(|id| row(id, 1, vec![TierCount { tier: 0, count: 1 }]))
                .collect(),
        })
    };
    captured.pages.insert(
        1,
        Envelope {
            session: captured.session.clone(),
            response: response(0x42, CommandStatus::Accepted, middle(164)?),
        },
    );
    replay(&captured)?;
    captured.pages[1].response.payload = middle(163)?;
    assert!(replay(&captured).is_err());
    Ok(())
}

#[test]
fn e2e_n03_rejects_inconsistent_capture_identity() -> Result<(), Error> {
    type Mutation = fn(&mut Captured);
    let cases: [(&str, Mutation); 5] = [
        ("refused page", |c| {
            c.pages[0].response.status = CommandStatus::Rejected
        }),
        ("foreign session", |c| {
            c.retrieve.session = "00000000-0000-4000-8000-000000000002".into()
        }),
        ("non-UUID session", |c| {
            c.pages[0].session = "session-1".into()
        }),
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
            outbox.depot_search_result(&response(
                1,
                CommandStatus::Accepted,
                encode(&Reply {
                    outcome: Outcome::Listed,
                    retrieved: 0,
                    results: vec![row(2854, 1, vec![TierCount { tier: 2, count: 1 }])],
                })?,
            ));
            inputs.0[0].clear();
            inputs.0[3] = "2".into();
            activate(&link, &inputs, 4);
            assert_eq!(
                sent(&mut receiver)?,
                Request::Detail {
                    client_id: 2854,
                    tier: 2
                }
            );
        } else {
            inputs.0[3].clear();
            activate(&link, &inputs, 1);
            assert_eq!(sent(&mut receiver)?, Request::Search { client_id: 100 });
        }
    }
    Ok(())
}
