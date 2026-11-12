//! W10 replay: the Hazard query, level selection and locked-level refusal applied by the
//! production client reducer `apply_result` (`protocol.rs`, hazard summary in
//! `protocol/auxiliary.rs`); requests are built by the production `request`.
use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

type Error = Box<dyn std::error::Error>;

fn captured() -> Result<(Vec<u8>, Vec<u8>, String), Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok((
            std::fs::read(root.join("w10-query.bin"))?,
            std::fs::read(root.join("w10-select.bin"))?,
            std::fs::read_to_string(root.join("w10-expected.txt"))?,
        ));
    }
    Ok((
        vec![1, 1],
        vec![1, 1],
        "query_unlocked=1\nquery_selected=1\nselect_unlocked=1\nselect_selected=1\n".into(),
    ))
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

#[test]
fn e2e_w10_replays_hazard_state_selection_and_locked_refusal() -> Result<(), Error> {
    let (query, select, expected) = captured()?;
    let mut panel = BossEncounterPanel {
        selected: true,
        ..BossEncounterPanel::default()
    };
    let command = request(&mut panel, Intent::HazardQuery).ok_or("hazard query request")?;
    assert_eq!(command.capability, 117);
    assert_eq!(
        command.kind,
        oteryn_client_runtime::boss_encounter::hazard::COMMAND_TYPE_HAZARD
    );
    assert!(command.payload.is_empty());
    assert!(apply_result(
        &mut panel,
        &result(CommandStatus::Accepted, &query)
    ));
    let (unlocked, selected) = (
        value(&expected, "query_unlocked")?,
        value(&expected, "query_selected")?,
    );
    assert_eq!(query, [unlocked.parse::<u8>()?, selected.parse::<u8>()?]);
    let confirmed = format!("Hazard: {selected} | Unlocked: {unlocked}");
    assert_eq!(panel.summary, confirmed);
    assert_eq!(panel.feedback, None);

    panel.fields[2] = "1".into();
    let command = request(&mut panel, Intent::HazardSelect).ok_or("hazard select request")?;
    assert_eq!(command.payload, [1]);
    assert!(apply_result(
        &mut panel,
        &result(CommandStatus::Accepted, &select)
    ));
    assert_eq!(
        panel.summary,
        format!(
            "Hazard: {} | Unlocked: {}",
            value(&expected, "select_selected")?,
            value(&expected, "select_unlocked")?
        )
    );
    let confirmed = panel.summary.clone();

    // Level 2 is locked: the server's empty rejection leaves the confirmed hazard state.
    panel.fields[2] = "2".into();
    // The client now fences a level above the confirmed unlocked level before sending anything.
    assert!(request(&mut panel, Intent::HazardSelect).is_none());
    // With no confirmed bounds the request still goes out and the server's rejection is replayed.
    panel.hazard = None;
    let command = request(&mut panel, Intent::HazardSelect).ok_or("locked select request")?;
    assert_eq!(command.payload, [2]);
    assert!(!apply_result(
        &mut panel,
        &result(CommandStatus::Rejected, &[])
    ));
    assert_eq!(
        panel.feedback,
        Some("The server refused the boss status query")
    );
    assert_eq!(panel.summary, confirmed);
    assert!(!panel.pending);

    // A reply whose selected level exceeds the unlocked level is not applied.
    request(&mut panel, Intent::HazardQuery).ok_or("hazard query request")?;
    assert!(!apply_result(
        &mut panel,
        &result(CommandStatus::Accepted, &[1, 2])
    ));
    assert_eq!(
        panel.feedback,
        Some("The server returned invalid boss data")
    );
    assert_eq!(panel.summary, confirmed);
    Ok(())
}
