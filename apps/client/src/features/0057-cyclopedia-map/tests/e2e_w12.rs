//! W12 replay: the Cyclopedia map QUERY, FIND and START replies applied by the production
//! client reducer `apply` (`protocol.rs`); requests are built by the production `command`.
use super::*;
use oteryn_client_runtime::raw_feature::CommandStatus;

type Error = Box<dyn std::error::Error>;

struct Replies {
    query: GatedCommandResult,
    find: GatedCommandResult,
    start: GatedCommandResult,
    refused: GatedCommandResult,
    expected: String,
}

fn replies() -> Result<Replies, Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root).join("w12");
        let read = |name, status| -> Result<GatedCommandResult, Error> {
            let frame = std::fs::read(root.join(name))?;
            let Some((header, payload)) = frame.split_at_checked(16) else {
                return Err("truncated live capture".into());
            };
            Ok(GatedCommandResult {
                command_id: u64::from_le_bytes(header[..8].try_into()?),
                server_sequence: u64::from_le_bytes(header[8..].try_into()?),
                status,
                payload: payload.to_vec(),
            })
        };
        let replies = Replies {
            query: read("w12-query.bin", CommandStatus::Accepted)?,
            find: read("w12-find.bin", CommandStatus::Accepted)?,
            start: read("w12-start.bin", CommandStatus::Accepted)?,
            refused: read("w12-refused.bin", CommandStatus::Rejected)?,
            expected: std::fs::read_to_string(root.join("w12-expected.txt"))?,
        };
        let binding = format!(
            "session={} sequence={}",
            value(&replies.expected, "session")?,
            replies.refused.server_sequence
        );
        assert_eq!(std::env::var("W12_CAPTURE_BINDING")?, binding);
        for (before, after) in [
            (&replies.query, &replies.find),
            (&replies.find, &replies.start),
            (&replies.start, &replies.refused),
        ] {
            assert_eq!(after.command_id, before.command_id + 1);
            assert!(after.server_sequence > before.server_sequence);
        }
        return Ok(replies);
    }
    let encode = |code, revision, found, visible: Vec<(u16, u16, i8)>| {
        wire::encode_wire_result(&wire::MapResult {
            code,
            revision,
            discovered: false,
            found,
            area_percent: 0,
            pool: 12,
            epoch: 3,
            improved: false,
            visible,
        })
        .map_err(|error| format!("{error:?}"))
    };
    let payload = encode(0, 4, 0, vec![])?;
    let expected = ["query", "find", "start"].map(|prefix| format!(
        "{prefix}_code=0\n{prefix}_revision=4\n{prefix}_discovered=false\n{prefix}_found=0\n{prefix}_percent=0\n{prefix}_pool=12\n{prefix}_epoch=3\n{prefix}_improved=false\n{prefix}_visible=[]\n"
    )).concat();
    Ok(Replies {
        query: result(CommandStatus::Accepted, &payload),
        find: result(CommandStatus::Accepted, &payload),
        start: result(CommandStatus::Accepted, &payload),
        refused: result(CommandStatus::Rejected, &[]),
        expected,
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

fn result(status: CommandStatus, payload: &[u8]) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload: payload.to_vec(),
    }
}

fn pending(kind: u32) -> Panel {
    Panel {
        selected: true,
        open: true,
        pending: true,
        pending_kind: kind,
        ..Panel::default()
    }
}

fn summary(expected: &str, prefix: &str) -> Result<String, Error> {
    let field = |name: &str| value(expected, &format!("{prefix}_{name}"));
    Ok(format!(
        "Current exploration area | Revision {} | Discovered: {}\r\nFound {}/7 | Area {}% | Pool {} | Epoch {} | Improved: {}\r\nVisible points: {}",
        field("revision")?,
        field("discovered")?,
        field("found")?,
        field("percent")?,
        field("pool")?,
        field("epoch")?,
        field("improved")?,
        field("visible")?,
    ))
}

#[test]
fn e2e_w12_replays_map_query_find_and_start() -> Result<(), Error> {
    let replies = replies()?;
    let expected = replies.expected.as_str();
    let blank = Panel {
        selected: true,
        ..Panel::default()
    };
    let query = command(&blank, Intent::Query)?;
    assert_eq!((query.capability, query.kind), (57, 72));
    assert!(query.payload.is_empty());
    let find = command(&blank, Intent::Find)?;
    assert_eq!((find.capability, find.kind), (57, 73));
    assert_eq!(find.payload, [3, 1, 0, 0, 0, 0, 0, 0, 0]);
    let start = command(&blank, Intent::Start)?;
    assert_eq!(start.payload, [1, 1, 0, 0, 0, 0, 0, 0, 0]);

    // QUERY: the confirmed server state is shown verbatim.
    assert_eq!(value(expected, "query_code")?, "0");
    let mut panel = pending(72);
    apply(&mut panel, &query, &replies.query);
    assert!(!panel.pending);
    assert_eq!(panel.feedback, "Cyclopedia map updated");
    let confirmed = summary(expected, "query")?;
    assert_eq!(panel.summary, confirmed);

    for (request, response, prefix) in [
        (&find, &replies.find, "find"),
        (&start, &replies.start, "start"),
    ] {
        assert_eq!(value(expected, &format!("{prefix}_code"))?, "0");
        panel.pending = true;
        panel.pending_kind = 73;
        apply(&mut panel, request, response);
        assert!(!panel.pending);
        assert_eq!(panel.feedback, "Cyclopedia map updated");
        assert_eq!(panel.summary, summary(expected, prefix)?);
        assert_eq!(panel.summary, confirmed);
    }

    assert!(replies.refused.payload.is_empty());
    panel.pending = true;
    panel.pending_kind = 73;
    let mut malformed = start;
    malformed.payload[1] = 2;
    apply(&mut panel, &malformed, &replies.refused);
    assert!(!panel.pending);
    assert_eq!(panel.summary, confirmed);
    assert_eq!(panel.feedback, "The server refused the request");

    // A truncated payload is a decoder negative, independent of the live replay.
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        let payload = &replies.start.payload;
        panel.pending = true;
        panel.pending_kind = 72;
        apply(
            &mut panel,
            &query,
            &result(CommandStatus::Accepted, &payload[..payload.len() - 1]),
        );
        assert!(!panel.pending);
        assert_eq!(panel.summary, confirmed);
        assert_eq!(panel.feedback, "Invalid cyclopedia map reply");
    }
    Ok(())
}
