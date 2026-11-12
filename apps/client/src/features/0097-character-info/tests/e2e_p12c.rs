//! P12c replay: server CHARACTER_INFO replies (full and paged QuerySelf, then the refused
//! QueryVisible of another character) applied by the production `command` and `apply` in
//! `protocol.rs`.
use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

type Error = Box<dyn std::error::Error>;

struct Captured {
    query: Vec<u8>,
    paged: Vec<u8>,
    expected: String,
}

/// The documented CHARACTER-INFO-1 reply layout, used only when no capture is present.
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
        query: reply([37, 120_500, 4, 6, 12, 3, 1, 0], 3, 2),
        paged: reply([37, 120_500, 4, 6, 12, 3, 1, 0], 1, 0),
        expected: "query=values=37,120500,4,6,12,3,1,0|deaths=3|kills=2\npaged=values=37,120500,4,6,12,3,1,0|deaths=1|kills=0\nvisible_target=0123456789abcdef0123456789abcdef\nrefused_status=Rejected\n".into(),
    }
}

fn captured() -> Result<Captured, Error> {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(fixture());
    };
    let path = std::path::Path::new(&path);
    Ok(Captured {
        query: std::fs::read(path.join("p12c-query.bin"))?,
        paged: std::fs::read(path.join("p12c-paged.bin"))?,
        expected: std::fs::read_to_string(path.join("p12c-expected.txt"))?,
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
        v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7],
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
fn e2e_p12c_replays_paged_summary_and_refused_visible_query() -> Result<(), Error> {
    let captured = captured()?;
    let e = captured.expected.as_str();
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };

    let query = pend(&mut panel, Intent::Query)?;
    assert_eq!(query.payload, [0, 0, 0, 50]);
    apply(&mut panel, &query, &result(CommandStatus::Accepted, &captured.query));
    assert_eq!(panel.feedback, "Character information updated");
    assert_eq!(panel.summary, visible(value(e, "query")?)?);

    panel.fields[0] = "1".into();
    panel.fields[1] = "1".into();
    let paged = pend(&mut panel, Intent::Query)?;
    assert_eq!(paged.payload, [0, 1, 0, 1]);
    apply(&mut panel, &paged, &result(CommandStatus::Accepted, &captured.paged));
    let confirmed = visible(value(e, "paged")?)?;
    assert_eq!(panel.summary, confirmed);

    // Another character's summary is refused without consent: no payload, summary unchanged.
    panel.fields[0] = "0".into();
    panel.fields[2] = value(e, "visible_target")?.to_owned();
    let visible_query = pend(&mut panel, Intent::Visible)?;
    let mut expected_payload = vec![1];
    for at in 0..16 {
        expected_payload.push(u8::from_str_radix(&value(e, "visible_target")?[at * 2..at * 2 + 2], 16)?);
    }
    expected_payload.extend([0, 0, 1]);
    assert_eq!(visible_query.payload, expected_payload);
    assert_eq!(value(e, "refused_status")?, "Rejected");
    apply(&mut panel, &visible_query, &result(CommandStatus::Rejected, &[]));
    assert_eq!(panel.feedback, "The server refused the request");
    assert_eq!(panel.summary, confirmed);

    // A paged reply whose history count exceeds the requested limit is rejected as invalid.
    panel.fields[1] = "1".into();
    let paged = pend(&mut panel, Intent::Query)?;
    let mut over_limit = captured.paged.clone();
    over_limit[65..67].copy_from_slice(&2_u16.to_le_bytes());
    apply(&mut panel, &paged, &result(CommandStatus::Accepted, &over_limit));
    assert_eq!(panel.feedback, "Invalid character information reply");
    assert_eq!(panel.summary, confirmed);
    Ok(())
}
