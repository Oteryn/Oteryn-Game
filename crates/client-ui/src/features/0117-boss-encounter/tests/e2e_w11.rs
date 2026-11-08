//! W11 replay: the Soul War / Rotten Blood taint projection and the Goshnar's Cruelty lever reply
//! applied by the production client reducer `apply_result` (`protocol.rs`), projected by
//! `panel_text` (`view.rs`). Requests are built by the production `request`.
use super::*;
use oteryn_client_runtime::boss_encounter::{
    BossActivityReply, encode_boss_activity_reply, encode_boss_key,
};
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

type Error = Box<dyn std::error::Error>;
const BOSS: &str = "oteryn:creature.goshnar_s_cruelty";

fn captured() -> Result<(Vec<u8>, Vec<u8>, String), Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok((
            std::fs::read(root.join("w11-taints.bin"))?,
            std::fs::read(root.join("w11-lever.bin"))?,
            std::fs::read_to_string(root.join("w11-expected.txt"))?,
        ));
    }
    let mut taints = vec![8, 0];
    taints.extend_from_slice(&[58, 8, 8, 3, 16, 2, 24, 5, 32, 0]);
    let refusal = encode_boss_activity_reply(&BossActivityReply::Refused(
        oteryn_client_runtime::boss_encounter::BossActivityRefusal::Level,
    ))
    .map_err(|error| format!("{error:?}"))?;
    Ok((
        taints,
        refusal,
        "now=0\nsoul_war=3\nrotten_blood=2\nrotten_blood_flags=5\nrotten_blood_final=false\nlever_status=Rejected\nactivity=refused:Level\n".into(),
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

fn status(name: &str) -> Result<CommandStatus, Error> {
    match name {
        "Accepted" => Ok(CommandStatus::Accepted),
        "Rejected" => Ok(CommandStatus::Rejected),
        other => Err(format!("unexpected lever status {other}").into()),
    }
}

#[test]
fn e2e_w11_replays_taint_projection_and_lever_refusal() -> Result<(), Error> {
    let (taints, lever, expected) = captured()?;
    let mut panel = BossEncounterPanel {
        selected: true,
        ..BossEncounterPanel::default()
    };
    let query = request(&mut panel, Intent::Refresh).ok_or("taint query")?;
    assert_eq!((query.capability, query.kind), (117, 133));
    assert_eq!(query.payload, [16, 1]);
    assert!(apply_result(
        &mut panel,
        &result(CommandStatus::Accepted, &taints)
    ));
    let taint = panel
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.taints)
        .ok_or("taints absent")?;
    assert_eq!(taint.soul_war.to_string(), value(&expected, "soul_war")?);
    assert_eq!(
        taint.rotten_blood.to_string(),
        value(&expected, "rotten_blood")?
    );
    assert_eq!(
        taint.rotten_blood_flags.to_string(),
        value(&expected, "rotten_blood_flags")?
    );
    assert_eq!(
        taint.rotten_blood_final.to_string(),
        value(&expected, "rotten_blood_final")?
    );
    let text = panel_text(&panel);
    assert!(text.contains(&format!("Soul War taints: {}\n", taint.soul_war)));
    assert!(text.contains(&format!(
        "Rotten Blood taints: {}; final taint: {}\n",
        taint.rotten_blood,
        if taint.rotten_blood_final {
            "yes"
        } else {
            "no"
        }
    )));
    let defeated: String = ["Ichgahal", "Murcion", "Vemiath", "Chagorz"]
        .iter()
        .enumerate()
        .filter(|(index, _)| taint.rotten_blood_flags & (1 << index) != 0)
        .map(|(_, name)| format!(" {name}"))
        .collect();
    let defeated = if defeated.is_empty() {
        " none".into()
    } else {
        defeated
    };
    assert!(text.contains(&format!("Rotten Blood bosses defeated:{defeated}\n")));

    // The lever reply: a typed refusal (or an accepted handoff) is shown exactly as the server
    // sent it.
    panel.fields[0] = BOSS.into();
    let enter = request(&mut panel, Intent::Enter).ok_or("lever request")?;
    assert_eq!(enter.kind, 178);
    assert_eq!(
        enter.payload,
        encode_boss_key(BOSS).map_err(|error| format!("{error:?}"))?
    );
    assert!(apply_result(
        &mut panel,
        &result(status(value(&expected, "lever_status")?)?, &lever)
    ));
    let activity = value(&expected, "activity")?;
    match activity.split_once(':') {
        Some(("refused", reason)) => {
            assert_eq!(panel.summary, format!("Boss request refused: {reason}"));
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

    // A malformed lever is rejected without a payload and leaves the summary and taints as shown.
    let (summary, snapshot) = (panel.summary.clone(), panel.snapshot.clone());
    request(&mut panel, Intent::Enter).ok_or("second lever request")?;
    assert!(!apply_result(
        &mut panel,
        &result(CommandStatus::Rejected, &[])
    ));
    assert_eq!(panel.feedback, Some("The server refused the boss request"));
    assert_eq!(panel.summary, summary);
    assert_eq!(panel.snapshot, snapshot);
    Ok(())
}
