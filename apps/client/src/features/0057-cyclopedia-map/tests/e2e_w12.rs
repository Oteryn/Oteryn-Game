//! W12 replay: the Cyclopedia map QUERY, FIND and START replies applied by the production
//! client reducer `apply` (`protocol.rs`); requests are built by the production `command`.
use super::*;
use oteryn_client_runtime::raw_feature::CommandStatus;

type Error = Box<dyn std::error::Error>;

struct Replies {
    query: GatedCommandResult,
    progress: GatedCommandResult,
    complete: GatedCommandResult,
    find: GatedCommandResult,
    start: GatedCommandResult,
    refused: GatedCommandResult,
    relog: GatedCommandResult,
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
            progress: read("w12-progress.bin", CommandStatus::Accepted)?,
            complete: read("w12-complete.bin", CommandStatus::Accepted)?,
            relog: read("w12-relog.bin", CommandStatus::Accepted)?,
            find: read("w12-find.bin", CommandStatus::Accepted)?,
            start: read("w12-start.bin", CommandStatus::Accepted)?,
            refused: read("w12-refused.bin", CommandStatus::Rejected)?,
            expected: std::fs::read_to_string(root.join("w12-expected.txt"))?,
        };
        let binding = value(&replies.expected, "binding")?;
        assert_eq!(std::env::var("W12_CAPTURE_BINDING")?, binding);
        for (field, sequence) in [
            ("sequence", replies.refused.server_sequence),
            ("relog_sequence", replies.relog.server_sequence),
        ] {
            assert!(
                binding
                    .split_whitespace()
                    .any(|token| token == format!("{field}={sequence}"))
            );
        }
        for (before, after) in [
            (&replies.query, &replies.progress),
            (&replies.progress, &replies.complete),
            (&replies.complete, &replies.find),
            (&replies.find, &replies.start),
            (&replies.start, &replies.refused),
        ] {
            assert!(after.command_id > before.command_id);
            assert!(after.server_sequence > before.server_sequence);
        }
        return Ok(replies);
    }
    // The local reducer control is used only without the harness's capture directory.
    let mut expected = String::new();
    let mut encode = |prefix, code, revision, found, discovered, percent| -> Result<_, Error> {
        expected.push_str(&format!(
            "{prefix}_code={code}\n{prefix}_revision={revision}\n{prefix}_discovered={discovered}\n{prefix}_found={found}\n{prefix}_percent={percent}\n{prefix}_pool=12\n{prefix}_epoch=3\n{prefix}_improved=false\n{prefix}_visible=[]\n"
        ));
        let payload = wire::encode_wire_result(&wire::MapResult {
            code,
            revision,
            discovered,
            found,
            area_percent: percent,
            pool: 12,
            epoch: 3,
            improved: false,
            visible: vec![],
        })
        .map_err(|error| format!("{error:?}"))?;
        Ok(result(CommandStatus::Accepted, &payload))
    };
    Ok(Replies {
        query: encode("query", 0, 4, 0, false, 0)?,
        progress: encode("progress", 0, 10, 6, false, 0)?,
        complete: encode("complete", 0, 11, 7, true, 25)?,
        find: encode("find", 0, 11, 7, true, 25)?,
        start: encode("start", 3, 11, 7, true, 25)?,
        refused: result(CommandStatus::Rejected, &[]),
        relog: encode("relog", 0, 12, 7, true, 25)?,
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

    let mut panel = pending(72);
    assert!(panel.map.is_none());
    let mut observation = Vec::new();
    for (response, prefix, feedback) in [
        (&replies.query, "query", "Cyclopedia map updated"),
        (&replies.progress, "progress", "Cyclopedia map updated"),
        (
            &replies.complete,
            "complete",
            "Subarea discovered! All seven points of interest found.",
        ),
    ] {
        let state = decode(response)?;
        panel.pending = true;
        panel.pending_kind = 72;
        apply(&mut panel, &query, response);
        assert!(!panel.pending);
        assert_eq!(panel.feedback, feedback);
        assert_eq!(panel.summary, summary(expected, prefix)?);
        assert_eq!(panel.map.as_ref(), Some(&state));
        observation.push(state);
    }
    let fresh_state = decode(&replies.relog)?;
    discovery_transition(
        &observation[0],
        &observation[1],
        &observation[2],
        &fresh_state,
    )?;
    let confirmed = panel.summary.clone();
    let complete = observation.pop().ok_or("missing completion observation")?;

    panel.pending = true;
    panel.pending_kind = 73;
    assert_eq!(decode(&replies.find)?, complete);
    apply(&mut panel, &find, &replies.find);
    assert!(!panel.pending);
    assert_eq!(panel.summary, confirmed);
    assert_eq!(panel.map.as_ref(), Some(&complete));

    let mut already_discovered = complete.clone();
    already_discovered.code = 3;
    assert_eq!(decode(&replies.start)?, already_discovered);
    assert!(replies.refused.payload.is_empty());
    let mut malformed = command(&blank, Intent::Start)?;
    malformed.payload[1] = 2;
    let mut truncated = replies.complete.clone();
    truncated.payload.pop().ok_or("empty completion reply")?;
    for (request, response, feedback) in [
        (&start, &replies.start, "Area already discovered"),
        (
            &malformed,
            &replies.refused,
            "The server refused the request",
        ),
        (&query, &truncated, "Invalid cyclopedia map reply"),
    ] {
        panel.pending = true;
        panel.pending_kind = request.kind;
        apply(&mut panel, request, response);
        assert!(!panel.pending);
        assert_eq!(panel.feedback, feedback);
        assert_eq!(panel.summary, confirmed);
        assert_eq!(panel.map.as_ref(), Some(&complete));
    }

    let mut fresh_panel = pending(72);
    assert!(fresh_panel.map.is_none());
    apply(&mut fresh_panel, &query, &replies.relog);
    assert!(!fresh_panel.pending);
    assert_eq!(fresh_panel.feedback, "Cyclopedia map updated");
    assert_eq!(fresh_panel.summary, summary(expected, "relog")?);
    assert_eq!(fresh_panel.map.as_ref(), Some(&fresh_state));
    Ok(())
}

fn decode(reply: &GatedCommandResult) -> Result<wire::MapResult, Error> {
    wire::decode_result(&reply.payload).map_err(|error| format!("{error:?}").into())
}

fn discovery_transition(
    initial: &wire::MapResult,
    partial: &wire::MapResult,
    complete: &wire::MapResult,
    relog: &wire::MapResult,
) -> Result<(), Error> {
    if initial.code != 0
        || initial.discovered
        || initial.found != 0
        || partial.code != 0
        || partial.discovered
        || partial.found != 6
        || initial.area_percent != partial.area_percent
        || partial.revision <= initial.revision
        || complete.code != 0
        || !complete.discovered
        || complete.found != 7
        || complete.area_percent <= partial.area_percent
        || complete.revision <= partial.revision
        || !complete.visible.is_empty()
    {
        return Err("W12 replay lacks an observed zero-to-seven discovery transition".into());
    }
    let mut expected = complete.clone();
    expected.revision = relog.revision;
    if *relog != expected || relog.revision < complete.revision {
        return Err("W12 replay lacks an exact retained discovery after relog".into());
    }
    Ok(())
}

#[test]
fn w12_audit_rejects_stationary_frames_and_relog_reset() -> Result<(), Error> {
    let fixture = replies()?;
    let initial = decode(&fixture.query)?;
    let partial = decode(&fixture.progress)?;
    let complete = decode(&fixture.complete)?;
    let relog = decode(&fixture.relog)?;
    discovery_transition(&initial, &partial, &complete, &relog)?;
    assert!(discovery_transition(&initial, &initial, &initial, &initial).is_err());
    let mut reset = relog;
    reset.discovered = false;
    reset.found = 0;
    assert!(discovery_transition(&initial, &partial, &complete, &reset).is_err());
    Ok(())
}
