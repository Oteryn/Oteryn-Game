//! W11 replay: the Soul War / Rotten Blood taint projection and the Goshnar's Cruelty lever reply
//! applied by the production client reducer `apply_result` (`protocol.rs`), projected by
//! `panel_text` (`view.rs`). Requests are built by the production `request`.
use super::*;
use oteryn_client_runtime::boss_encounter::encode_boss_key;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

type Error = Box<dyn std::error::Error>;
type CapturedFrame = (u64, u64, String, Vec<u8>);
type CapturedTaints = (u8, u8, u8, bool);
type Capture = (String, [CapturedFrame; 3], CapturedTaints);
const BOSS: &str = "oteryn:creature.goshnar_s_cruelty";

fn result(
    (command_id, server_sequence, observed_status, payload): CapturedFrame,
) -> Result<GatedCommandResult, Error> {
    Ok(GatedCommandResult {
        command_id,
        server_sequence,
        status: status(&observed_status)?,
        payload,
    })
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
    // The aggregate client suite leaves W11 replay to node boot.
    let Some(run_session) = std::env::var_os("W11_CAPTURE_SESSION") else {
        return Ok(());
    };
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("W11 capture directory missing")?;
    let (session, [query_frame, lever_frame, malformed_frame], expected): Capture =
        serde_json::from_slice(&std::fs::read(
            std::path::Path::new(&root).join("w11-live.json"),
        )?)?;
    assert_eq!(Some(session.as_str()), run_session.to_str());
    let [taints, lever, malformed] = [
        result(query_frame)?,
        result(lever_frame)?,
        result(malformed_frame)?,
    ];
    assert!(taints.command_id > 0 && taints.server_sequence > 0);
    assert!(taints.command_id < lever.command_id && lever.command_id < malformed.command_id);
    assert!(
        taints.server_sequence < lever.server_sequence
            && lever.server_sequence < malformed.server_sequence
    );
    assert_eq!(taints.status, CommandStatus::Accepted);
    assert_eq!(lever.status, CommandStatus::Rejected);
    assert_eq!(malformed.status, CommandStatus::Rejected);
    assert!(malformed.payload.is_empty());
    let mut panel = BossEncounterPanel {
        selected: true,
        ..BossEncounterPanel::default()
    };
    let query = request(&mut panel, Intent::Refresh).ok_or("taint query")?;
    assert_eq!((query.capability, query.kind), (117, 133));
    assert_eq!(query.payload, [16, 1]);
    assert!(panel.snapshot.is_none());
    assert!(apply_result(&mut panel, &taints));
    let taint = panel
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.taints)
        .ok_or("taints absent")?;
    assert_eq!(
        (
            taint.soul_war,
            taint.rotten_blood,
            taint.rotten_blood_flags,
            taint.rotten_blood_final
        ),
        expected
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
    let before = panel.snapshot.clone();
    assert_ne!(
        panel.summary,
        "A party member does not meet the level requirement."
    );
    assert!(apply_result(&mut panel, &lever));
    assert_eq!(
        panel.summary,
        "A party member does not meet the level requirement."
    );
    assert_eq!(panel.feedback, Some("The server refused the boss request"));
    assert_eq!(panel.snapshot, before);

    // A malformed lever is rejected without a payload and leaves the summary and taints as shown.
    let (summary, snapshot) = (panel.summary.clone(), panel.snapshot.clone());
    request(&mut panel, Intent::Enter).ok_or("second lever request")?;
    assert!(!apply_result(&mut panel, &malformed));
    assert_eq!(panel.feedback, Some("The server refused the boss request"));
    assert_eq!(panel.summary, summary);
    assert_eq!(panel.snapshot, snapshot);
    Ok(())
}
