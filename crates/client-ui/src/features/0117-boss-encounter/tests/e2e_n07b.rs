//! N07b replay: two server BOSS_COOLDOWNS replies applied by the production reducer
//! `apply_result` (`protocol.rs`) and projected by the Boss cooldown tracker text `cooldown_text`
//! (`view.rs`). Requests are built by the production `request`.
use super::*;
use oteryn_client_runtime::boss_encounter::{BossCooldownEntry, encode_bosstiary_list};
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

type Error = Box<dyn std::error::Error>;
const BOSS: &str = "oteryn:boss.faceless_bane";
const HEADING: &str = "Boss cooldown tracker \u{2014} Ctrl+Alt+Shift+J\r\n";

fn captured() -> Result<(Vec<u8>, Vec<u8>, String), Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok((
            std::fs::read(root.join("n07b-cooldowns.bin"))?,
            std::fs::read(root.join("n07b-cooldowns-later.bin"))?,
            std::fs::read_to_string(root.join("n07b-expected.txt"))?,
        ));
    }
    let cooldown = |at| BossCooldownEntry {
        boss_key: BOSS.into(),
        available_at_unix_ms: at,
    };
    let encode = |now, entry| {
        encode_bosstiary_list(now, &[entry], false, &[], 0, false)
            .map_err(|error| format!("{error:?}"))
    };
    Ok((
        encode(30_000, cooldown(90_000))?,
        encode(31_500, cooldown(90_000))?,
        format!("now=30000\npoints=0\ncooldowns={BOSS}=90000\nmore=false\nnow_later=31500\n"),
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

/// The text the tracker must show for the server-observed cooldowns, `elapsed` ms after receipt.
fn tracker(expected: &str, now: u64, elapsed: u64) -> Result<String, Error> {
    let mut text = HEADING.to_owned();
    let listed = value(expected, "cooldowns")?;
    if listed.is_empty() {
        text.push_str("No cooldown records on this page\r\n");
    }
    for entry in listed.split(',').filter(|entry| !entry.is_empty()) {
        let (key, deadline) = entry.rsplit_once('=').ok_or("cooldown entry")?;
        let remaining = deadline
            .parse::<u64>()?
            .saturating_sub(now)
            .saturating_sub(elapsed);
        if remaining == 0 {
            text.push_str(&format!("{key}: Ready \u{2014} refresh to confirm\r\n"));
        } else {
            text.push_str(&format!("{key}: {remaining} ms remaining\r\n"));
        }
    }
    if value(expected, "more")? == "true" {
        text.push_str("More records available: Next page\r\n");
    }
    Ok(text)
}

#[test]
fn e2e_n07b_replays_cooldown_snapshots_into_the_tracker() -> Result<(), Error> {
    let (first, later, expected) = captured()?;
    let now: u64 = value(&expected, "now")?.parse()?;
    let now_later: u64 = value(&expected, "now_later")?.parse()?;
    let mut panel = BossEncounterPanel {
        selected: true,
        ..BossEncounterPanel::default()
    };
    let query = request(&mut panel, Intent::Refresh).ok_or("cooldown query")?;
    assert_eq!((query.capability, query.kind), (117, 133));
    assert!(apply_result(
        &mut panel,
        &result(CommandStatus::Accepted, &first)
    ));
    let snapshot = panel.snapshot.clone().ok_or("snapshot absent")?;
    assert_eq!(snapshot.now, now);
    assert_eq!(snapshot.points.to_string(), value(&expected, "points")?);
    let listed: Vec<String> = snapshot
        .cooldowns
        .iter()
        .map(|(key, at)| format!("{key}={at}"))
        .collect();
    assert_eq!(listed.join(","), value(&expected, "cooldowns")?);
    assert!(panel.cooldown_received_at.is_some());
    assert_eq!(cooldown_text(&panel, 0), tracker(&expected, now, 0)?);
    assert_eq!(cooldown_text(&panel, 5), tracker(&expected, now, 5)?);
    assert_eq!(
        cooldown_text(&panel, u64::MAX),
        tracker(&expected, now, u64::MAX)?
    );

    // The second reply moves the server clock; the tracker recomputes against it.
    request(&mut panel, Intent::Refresh).ok_or("second cooldown query")?;
    assert!(apply_result(
        &mut panel,
        &result(CommandStatus::Accepted, &later)
    ));
    assert_eq!(panel.snapshot.as_ref().map(|s| s.now), Some(now_later));
    assert!(now_later > now);
    assert_eq!(cooldown_text(&panel, 0), tracker(&expected, now_later, 0)?);

    // An undecodable reply and a refusal keep the confirmed snapshot and its text.
    let (shown, text) = (panel.snapshot.clone(), cooldown_text(&panel, 0));
    for (status, payload, feedback) in [
        (
            CommandStatus::Accepted,
            &[0xff][..],
            "The server returned invalid boss status",
        ),
        (
            CommandStatus::Rejected,
            &[][..],
            "The server refused the boss status query",
        ),
    ] {
        request(&mut panel, Intent::Refresh).ok_or("repeat cooldown query")?;
        assert!(!apply_result(&mut panel, &result(status, payload)));
        assert_eq!(panel.feedback, Some(feedback));
        assert_eq!(panel.snapshot, shown);
        assert_eq!(cooldown_text(&panel, 0), format!("{text}{feedback}"));
    }
    Ok(())
}
