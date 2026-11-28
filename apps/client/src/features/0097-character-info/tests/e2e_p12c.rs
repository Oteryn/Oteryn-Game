//! P12c replay: server CHARACTER_INFO replies (full and paged QuerySelf, then the refused
//! QueryVisible of another character) applied by the production `command` and `apply` in
//! `protocol.rs`.
use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

type Error = Box<dyn std::error::Error>;

struct Captured {
    query: GatedCommandResult,
    paged: GatedCommandResult,
    refused: GatedCommandResult,
    allowed: GatedCommandResult,
    durable: GatedCommandResult,
    expected: String,
}

/// A CHARACTER-INFO-1 reply for the reducer unit tests.
fn reply(values: [i64; 8], deaths: u16, kills: u16) -> Vec<u8> {
    let mut bytes = vec![0];
    for value in values {
        bytes.extend(value.to_le_bytes());
    }
    bytes.extend(deaths.to_le_bytes());
    bytes.extend(kills.to_le_bytes());
    bytes.extend([0, 0]);
    bytes
}

fn fixture() -> Captured {
    Captured {
        query: result(CommandStatus::Accepted, &reply([37, 120_500, 4, 6, 12, 3, 1, 0], 1, 0)),
        paged: result(CommandStatus::Accepted, &reply([37, 120_500, 4, 6, 12, 3, 1, 0], 0, 0)),
        refused: result(CommandStatus::Rejected, &[]),
        allowed: result(CommandStatus::Accepted, &reply([1, 0, 0, 0, 15, 0, 0, 0], 0, 0)),
        durable: result(CommandStatus::Accepted, &reply([37, 120_500, 4, 6, 12, 3, 1, 0], 1, 0)),
        expected: "query=values=37,120500,4,6,12,3,1,0|deaths=1|kills=0\npaged=values=37,120500,4,6,12,3,1,0|deaths=0|kills=0\nvisible_target=0123456789abcdef0123456789abcdef\nrefused_status=Rejected\nvisible=values=1,0,0,0,15,0,0,0|deaths=0|kills=0\n".into(),
    }
}

#[derive(serde::Deserialize)]
struct LiveCapture {
    session: String,
    relog_session: String,
    frames: [LiveFrame; 5],
    expected: String,
}

#[derive(serde::Deserialize)]
struct LiveFrame {
    command_id: u64,
    server_sequence: u64,
    status: String,
    payload: Vec<u8>,
}

impl LiveFrame {
    fn into_result(self, status: CommandStatus) -> Result<GatedCommandResult, Error> {
        if self.status != format!("{status:?}") || self.command_id == 0 || self.server_sequence == 0
        {
            return Err("unexpected live P12c command result".into());
        }
        Ok(GatedCommandResult {
            command_id: self.command_id,
            server_sequence: self.server_sequence,
            status,
            payload: self.payload,
        })
    }
}

fn captured() -> Result<Captured, Error> {
    let path = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        .ok_or("P12c live replay requires a capture directory")?;
    let session = std::env::var("E2E_P12C_CAPTURE_SESSION")?;
    let input = std::fs::File::open(std::path::Path::new(&path).join("p12c-capture.json"))?;
    let capture: LiveCapture = serde_json::from_reader(input)?;
    validated_capture(capture, &session)
}

fn validated_capture(capture: LiveCapture, session: &str) -> Result<Captured, Error> {
    if session.is_empty()
        || session != capture.session
        || capture.relog_session.is_empty()
        || capture.relog_session == session
    {
        return Err("P12c capture session mismatch".into());
    }
    let [query, paged, refused, allowed, durable] = capture.frames;
    if query.command_id.checked_add(1) != Some(paged.command_id)
        || paged.command_id.checked_add(1) != Some(refused.command_id)
        || refused.command_id.checked_add(1) != Some(allowed.command_id)
        || !(query.server_sequence < paged.server_sequence
            && paged.server_sequence < refused.server_sequence
            && refused.server_sequence < allowed.server_sequence)
        || !refused.payload.is_empty()
    {
        return Err("P12c capture has invalid ordering or disclosed foreign data".into());
    }
    Ok(Captured {
        query: query.into_result(CommandStatus::Accepted)?,
        paged: paged.into_result(CommandStatus::Accepted)?,
        refused: refused.into_result(CommandStatus::Rejected)?,
        allowed: allowed.into_result(CommandStatus::Accepted)?,
        durable: durable.into_result(CommandStatus::Accepted)?,
        expected: capture.expected,
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent").into())
}

/// The summary text the player must see for a `values=..|deaths=..|kills=..` capture entry.
fn visible(entry: &str) -> Result<String, Error> {
    let [values, deaths, kills]: [&str; 3] = entry
        .split('|')
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "capture summary fields")?;
    let v: Vec<&str> = values.trim_start_matches("values=").split(',').collect();
    Ok(format!(
        "Level {} | Experience {}\r\nAchievements {} | Appearances {}\r\nInventory {} | Depot {} | Stash {} | Inbox {}\r\nPage deaths {} | PvP kills {}\r\nTitles: \r\nSelected: None",
        v[0],
        v[1],
        v[2],
        v[3],
        v[4],
        v[5],
        v[6],
        v[7],
        deaths.trim_start_matches("deaths="),
        kills.trim_start_matches("kills=")
    ))
}

fn pend(panel: &mut Panel, intent: Intent) -> Result<FeatureCommand, Error> {
    let command = command(panel, intent)?;
    panel.pending = true;
    panel.pending_kind = command.kind;
    Ok(command)
}

fn result(status: CommandStatus, payload: &[u8]) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        status,
        server_sequence: 1,
        payload: payload.to_vec(),
    }
}

#[test]
#[ignore = "requires a fresh wire capture from the live P12c row"]
fn e2e_p12c_replays_paged_summary_and_refused_visible_query() -> Result<(), Error> {
    replay(captured()?)
}

#[test]
fn p12c_unit_fixture_exercises_the_reducer() -> Result<(), Error> {
    replay(fixture())
}

#[test]
fn p12c_replay_rejects_a_capture_from_another_session() {
    let frame = |command_id| LiveFrame {
        command_id,
        server_sequence: command_id,
        status: if command_id == 3 {
            "Rejected"
        } else {
            "Accepted"
        }
        .into(),
        payload: Vec::new(),
    };
    let capture = LiveCapture {
        session: "old-session".into(),
        relog_session: "later-session".into(),
        frames: [frame(1), frame(2), frame(3), frame(4), frame(1)],
        expected: fixture().expected,
    };
    assert!(validated_capture(capture, "current-session").is_err());
}

#[test]
fn p12c_replay_rejects_a_page_that_leaves_the_previous_summary() {
    let mut capture = fixture();
    capture.paged.payload[0] = u8::MAX;
    assert!(std::panic::catch_unwind(|| replay(capture)).is_err());
}

#[test]
fn p12c_replay_rejects_repeated_first_page_and_blanket_foreign_denial() {
    let mut repeated = fixture();
    repeated.paged = repeated.query.clone();
    assert!(std::panic::catch_unwind(|| replay(repeated)).is_err());

    let mut denied = fixture();
    denied.allowed = denied.refused.clone();
    assert!(std::panic::catch_unwind(|| replay(denied)).is_err());
}

fn replay(captured: Captured) -> Result<(), Error> {
    let e = captured.expected.as_str();
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };

    let query = pend(&mut panel, Intent::Query)?;
    assert_eq!(query.payload, [0, 0, 0, 50]);
    apply(&mut panel, &query, &captured.query);
    assert_eq!(panel.feedback, "Character information updated");
    let full = visible(value(e, "query")?)?;
    assert_eq!(panel.summary, full);

    panel.fields[0] = "1".into();
    panel.fields[1] = "1".into();
    let paged = pend(&mut panel, Intent::Query)?;
    assert_eq!(paged.payload, [0, 1, 0, 1]);
    apply(&mut panel, &paged, &captured.paged);
    let confirmed = visible(value(e, "paged")?)?;
    assert_eq!(panel.feedback, "Character information updated");
    assert_ne!(confirmed, full);
    assert_eq!(panel.summary, confirmed);

    // Another character's summary is refused without consent: no payload, summary unchanged.
    panel.fields[0] = "0".into();
    panel.fields[2] = value(e, "visible_target")?.to_owned();
    let visible_query = pend(&mut panel, Intent::Visible)?;
    let mut expected_payload = vec![1];
    for at in 0..16 {
        expected_payload.push(u8::from_str_radix(
            &value(e, "visible_target")?[at * 2..at * 2 + 2],
            16,
        )?);
    }
    expected_payload.extend([0, 0, 1]);
    assert_eq!(visible_query.payload, expected_payload);
    assert_eq!(value(e, "refused_status")?, "Rejected");
    apply(&mut panel, &visible_query, &captured.refused);
    assert_eq!(panel.feedback, "The server refused the request");
    assert_eq!(panel.summary, confirmed);

    let consented = pend(&mut panel, Intent::Visible)?;
    assert_eq!(consented.payload, visible_query.payload);
    apply(&mut panel, &consented, &captured.allowed);
    assert_eq!(panel.feedback, "Character information updated");
    let foreign = visible(value(e, "visible")?)?;
    assert_ne!(foreign, full);
    assert_eq!(panel.summary, foreign);

    let mut fresh = Panel {
        selected: true,
        ..Panel::default()
    };
    let relogged = pend(&mut fresh, Intent::Query)?;
    apply(&mut fresh, &relogged, &captured.durable);
    assert_eq!(fresh.feedback, "Character information updated");
    assert_eq!(fresh.summary, full);

    // A paged reply whose history count exceeds the requested limit is rejected as invalid.
    panel.fields[1] = "1".into();
    let paged = pend(&mut panel, Intent::Query)?;
    let mut over_limit = captured.paged;
    over_limit.payload[65..67].copy_from_slice(&2_u16.to_le_bytes());
    apply(&mut panel, &paged, &over_limit);
    assert_eq!(panel.feedback, "Invalid character information reply");
    assert_eq!(panel.summary, foreign);
    Ok(())
}
