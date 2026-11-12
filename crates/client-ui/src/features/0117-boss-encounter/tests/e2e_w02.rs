//! W02 replay: the BOSS_COOLDOWNS reply and the Faceless Bane BOSS_LEVER reply applied by the
//! production client reducer `apply_result` (`protocol.rs`), projected by `panel_text`
//! (`view.rs`). Requests are built by the production `request`.
use super::*;
use oteryn_client_runtime::boss_encounter::{
    BossActivityRefusal, BossActivityReply, BossCooldownEntry, encode_boss_activity_reply,
    encode_boss_key, encode_bosstiary_list,
};
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

type Error = Box<dyn std::error::Error>;
const BOSS: &str = "oteryn:boss.faceless_bane";

fn captured() -> Result<(Vec<u8>, Vec<u8>, String), Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok((
            std::fs::read(root.join("w02-cooldowns.bin"))?,
            std::fs::read(root.join("w02-lever.bin"))?,
            std::fs::read_to_string(root.join("w02-expected.txt"))?,
        ));
    }
    let cooldown = BossCooldownEntry {
        boss_key: BOSS.into(),
        available_at_unix_ms: 90_000,
    };
    Ok((
        encode_bosstiary_list(30_000, &[cooldown], false, &[], 0, false)
            .map_err(|error| format!("{error:?}"))?,
        encode_boss_activity_reply(&BossActivityReply::Refused(BossActivityRefusal::Position))
            .map_err(|error| format!("{error:?}"))?,
        format!(
            "now=30000\npoints=0\ncooldowns={BOSS}=90000\nlever_status=Rejected\nactivity=refused:Position\n"
        ),
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
fn e2e_w02_replays_cooldowns_and_lever_reply() -> Result<(), Error> {
    let (cooldowns, lever, expected) = captured()?;
    let mut panel = BossEncounterPanel {
        selected: true,
        ..BossEncounterPanel::default()
    };
    let query = request(&mut panel, Intent::Refresh).ok_or("cooldown query")?;
    assert_eq!((query.capability, query.kind), (117, 133));
    assert!(apply_result(
        &mut panel,
        &result(CommandStatus::Accepted, &cooldowns)
    ));
    let snapshot = panel.snapshot.clone().ok_or("snapshot absent")?;
    assert_eq!(snapshot.now.to_string(), value(&expected, "now")?);
    assert_eq!(snapshot.points.to_string(), value(&expected, "points")?);
    let listed: Vec<String> = snapshot
        .cooldowns
        .iter()
        .map(|(key, at)| format!("{key}={at}"))
        .collect();
    assert_eq!(listed.join(","), value(&expected, "cooldowns")?);
    let text = panel_text(&panel);
    assert!(text.contains(&format!(
        "Boss points: {}\nServer time: {}\n",
        snapshot.points, snapshot.now
    )));
    for (key, at) in &snapshot.cooldowns {
        let kills = snapshot
            .progress
            .iter()
            .find(|(id, _)| id == key)
            .map(|(_, kills)| format!(": {kills} kills"))
            .unwrap_or_default();
        let row = format!(
            "  {key}{kills} | {} ms remaining",
            at.saturating_sub(snapshot.now)
        );
        assert!(
            text.lines().any(|line| line == row),
            "missing {row}: {text}"
        );
    }

    panel.fields[0] = BOSS.into();
    let enter = request(&mut panel, Intent::Enter).ok_or("lever request")?;
    assert_eq!(enter.kind, 178);
    assert_eq!(
        enter.payload,
        encode_boss_key(BOSS).map_err(|error| format!("{error:?}"))?
    );
    let status = match value(&expected, "lever_status")? {
        "Accepted" => CommandStatus::Accepted,
        "Rejected" => CommandStatus::Rejected,
        other => return Err(format!("unexpected lever status {other}").into()),
    };
    assert!(apply_result(&mut panel, &result(status, &lever)));
    let activity = value(&expected, "activity")?;
    match activity.split_once(':') {
        Some(("refused", reason)) => {
            let summary = match reason {
                "Level" => "A party member does not meet the level requirement.".into(),
                "PartyCount" => "The party does not meet this encounter's size requirement.".into(),
                "Position" => "A party member is not on a required entry tile.".into(),
                _ => format!("Boss request refused: {reason}"),
            };
            assert_eq!(panel.summary, summary);
            assert_eq!(panel.feedback, Some("The server refused the boss request"));
        }
        Some(("accepted", "None")) => assert_eq!(panel.summary, "Boss return accepted"),
        Some(("accepted", deadline)) => {
            let deadline = deadline.trim_start_matches("Some(").trim_end_matches(')');
            assert_eq!(
                panel.summary,
                format!("Boss entry accepted. Deadline: {deadline}")
            );
        }
        _ => return Err(format!("unexpected activity {activity}").into()),
    }

    // An undecodable lever reply and an empty rejection change neither summary nor cooldowns.
    let (summary, shown) = (panel.summary.clone(), panel.snapshot.clone());
    for (status, payload) in [
        (CommandStatus::Accepted, &[0xff][..]),
        (CommandStatus::Rejected, &[][..]),
    ] {
        request(&mut panel, Intent::Enter).ok_or("repeat lever request")?;
        assert!(!apply_result(&mut panel, &result(status, payload)));
        assert_eq!(panel.summary, summary);
        assert_eq!(panel.snapshot, shown);
    }
    Ok(())
}
