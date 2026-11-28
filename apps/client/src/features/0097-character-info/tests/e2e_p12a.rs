//! P12a replay: server CHARACTER_INFO replies (QuerySelf, then the refused Select of an unearned
//! title) applied by the production `command` and `apply` in `protocol.rs`.
use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

type Error = Box<dyn std::error::Error>;

struct Captured {
    query: Vec<u8>,
    after: Vec<u8>,
    refused: Vec<u8>,
    expected: String,
}

/// The documented CHARACTER-INFO-1 reply layout, used only when no capture is present.
fn reply(values: [i64; 8], deaths: u16, kills: u16, earned: &[u8], selected: u8) -> Vec<u8> {
    let mut bytes = vec![0];
    for value in values {
        bytes.extend(value.to_le_bytes());
    }
    bytes.extend(deaths.to_le_bytes());
    bytes.extend(kills.to_le_bytes());
    bytes.push(earned.len() as u8);
    bytes.extend(earned);
    bytes.push(selected);
    bytes
}

fn fixture() -> Captured {
    let query = reply([37, 120_500, 4, 6, 12, 3, 1, 0], 1, 0, &[1, 2], 2);
    Captured {
        after: query.clone(),
        refused: Vec::new(),
        query,
        expected: "values=37,120500,4,6,12,3,1,0\ndeaths=1\nkills=0\nearned=1,2\nselected=2\nrefused_title=3\nsession=00000000-0000-7000-8000-000000000001\nquery_result=1,1,Accepted\nrefused_result=2,2,Rejected\nafter_result=3,3,Accepted\n".into(),
    }
}

fn captured() -> Result<Captured, Error> {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(fixture());
    };
    let path = std::path::Path::new(&path);
    Ok(Captured {
        query: std::fs::read(path.join("p12a-query.bin"))?,
        after: std::fs::read(path.join("p12a-query-after.bin"))?,
        refused: std::fs::read(path.join("p12a-refused.bin"))?,
        expected: std::fs::read_to_string(path.join("p12a-expected.txt"))?,
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent").into())
}

fn title_name(id: u8) -> Result<&'static str, Error> {
    Ok(wire::CharacterTitle::from_id(id).ok_or("unknown title id")?.name())
}

/// The summary text the player must see, composed from the server-observed values.
fn visible(e: &str) -> Result<String, Error> {
    let v: Vec<&str> = value(e, "values")?.split(',').collect();
    let earned: Vec<&str> = value(e, "earned")?.split(',').filter(|s| !s.is_empty()).collect();
    let names = earned
        .iter()
        .map(|id| title_name(id.parse()?))
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    let selected = match value(e, "selected")? {
        "0" => "None",
        id => title_name(id.parse()?)?,
    };
    Ok(format!(
        "Level {} | Experience {}\r\nAchievements {} | Appearances {}\r\nInventory {} | Depot {} | Stash {} | Inbox {}\r\nPage deaths {} | PvP kills {}\r\nTitles: {names}\r\nSelected: {selected}",
        v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], value(e, "deaths")?, value(e, "kills")?
    ))
}

fn pend(panel: &mut Panel, intent: Intent) -> Result<FeatureCommand, Error> {
    let command = command(panel, intent)?;
    panel.pending = true;
    panel.pending_kind = command.kind;
    Ok(command)
}

fn captured_result(metadata: &str, payload: Vec<u8>) -> Result<GatedCommandResult, Error> {
    let fields: Vec<_> = metadata.split(',').collect();
    let [command_id, server_sequence, status] = fields.as_slice() else {
        return Err("invalid P12a result metadata".into());
    };
    let status = match *status {
        "Accepted" => CommandStatus::Accepted,
        "Rejected" => CommandStatus::Rejected,
        _ => return Err("unexpected P12a result status".into()),
    };
    Ok(GatedCommandResult {
        command_id: command_id.parse()?,
        status,
        server_sequence: server_sequence.parse()?,
        payload,
    })
}

#[test]
fn e2e_p12a_replays_titles_and_refused_unearned_select() -> Result<(), Error> {
    let captured = captured()?;
    let e = captured.expected.as_str();
    assert!(!value(e, "session")?.is_empty());
    let before = captured_result(value(e, "query_result")?, captured.query)?;
    let refused = captured_result(value(e, "refused_result")?, captured.refused)?;
    let after = captured_result(value(e, "after_result")?, captured.after)?;
    assert_eq!(before.status, CommandStatus::Accepted);
    assert_eq!(refused.status, CommandStatus::Rejected);
    assert!(refused.payload.is_empty());
    assert_eq!(after.status, CommandStatus::Accepted);
    assert_eq!(before.command_id.checked_add(1), Some(refused.command_id));
    assert_eq!(refused.command_id.checked_add(1), Some(after.command_id));
    assert!(before.server_sequence < refused.server_sequence);
    assert!(refused.server_sequence < after.server_sequence);
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };

    let query = pend(&mut panel, Intent::Query)?;
    assert_eq!(query.payload, [0, 0, 0, 50]);
    apply(&mut panel, &query, &before);
    assert_eq!(panel.feedback, "Character information updated");
    let confirmed = visible(e)?;
    assert_eq!(panel.summary, confirmed);

    // Selecting a title the character has not earned is refused and the summary is untouched.
    panel.fields[3] = value(e, "refused_title")?.to_owned();
    let select = pend(&mut panel, Intent::Select)?;
    assert_eq!(select.payload, [3, value(e, "refused_title")?.parse()?]);
    apply(&mut panel, &select, &refused);
    assert_eq!(panel.feedback, "The server refused the request");
    assert_eq!(panel.summary, confirmed);
    assert!(!panel.pending);

    // The follow-up query shows the same earned set and selected title.
    let query = pend(&mut panel, Intent::Query)?;
    apply(&mut panel, &query, &after);
    assert_eq!(panel.summary, confirmed);

    // A reply with trailing bytes is rejected and the confirmed summary stays.
    let mut trailing = before;
    trailing.payload.push(0);
    let query = pend(&mut panel, Intent::Query)?;
    apply(&mut panel, &query, &trailing);
    assert_eq!(panel.feedback, "Invalid character information reply");
    assert_eq!(panel.summary, confirmed);
    Ok(())
}
