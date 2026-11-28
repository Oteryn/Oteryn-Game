//! Replay only the live frames captured for this qualification run.
use super::protocol::{FullIntent, apply_full_result, full_request};
use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

#[test]
fn e2e_p23c_replays_hunting_task_query_and_select() -> Result<(), Box<dyn std::error::Error>> {
    let Some(root) = std::env::var_os("OTERYN_P23C_CAPTURE_DIR") else {
        return Ok(());
    };
    let root = std::path::PathBuf::from(root);
    let capture: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(root.join("p23c.json"))?)?;
    assert_eq!(
        capture["run"].as_str(),
        root.file_name().and_then(|name| name.to_str())
    );
    assert!(
        capture["session"]
            .as_str()
            .is_some_and(|session| !session.is_empty())
    );
    let commands = capture["commands"]
        .as_array()
        .ok_or("capture commands missing")?;
    assert_eq!(commands.len(), 4);
    let mut panel = TaskBoardPanel {
        selected: true,
        ..TaskBoardPanel::default()
    };
    let mut command_id = 0;
    let mut server_sequence = 0;
    let mut views = Vec::new();
    for (frame, intent) in commands.iter().zip([
        FullIntent::Refresh,
        FullIntent::StartWeekly(0),
        FullIntent::OpenBounty,
        FullIntent::SelectBounty(0),
    ]) {
        let request = full_request(&mut panel, intent).ok_or("client command missing")?;
        let captured_request: Vec<u8> = serde_json::from_value(frame["request"].clone())?;
        assert_eq!(request.payload, captured_request);
        let next_command = frame["command"].as_u64().ok_or("missing command ID")?;
        let next_sequence = frame["sequence"]
            .as_u64()
            .ok_or("missing server sequence")?;
        assert!(next_command > command_id && next_sequence > server_sequence);
        command_id = next_command;
        server_sequence = next_sequence;
        assert_eq!(frame["status"].as_str(), Some("Accepted"));
        apply_full_result(
            &mut panel,
            &GatedCommandResult {
                command_id,
                server_sequence,
                status: CommandStatus::Accepted,
                payload: serde_json::from_value(frame["payload"].clone())?,
            },
        );
        let view = panel.weekly.clone().ok_or("client board not updated")?;
        assert!(!panel.pending);
        assert_eq!(view.refusal, 0);
        views.push(view);
    }
    let [query, weekly, opened, selected] = views.as_slice() else {
        return Err("missing replay views".into());
    };
    assert!(query.bounty.active.is_none());
    assert!(!query.weekly.started);
    assert!(weekly.weekly.started);
    assert_eq!(weekly.weekly.difficulty, 0);
    assert!(!weekly.weekly.kills.is_empty());
    assert_eq!(weekly.weekly.pending_points, 0);
    assert_eq!(weekly.points, query.points);
    assert!(opened.bounty.active.is_none());
    assert_eq!(weekly.bounty.state, 0);
    assert!(weekly.bounty.offers.is_empty());
    assert_eq!(opened.bounty.state, 1);
    let offer = opened
        .bounty
        .offers
        .first()
        .ok_or("server offered no bounty")?;
    assert_eq!(selected.bounty.state, 2);
    assert_eq!(selected.bounty.selected, 1);
    assert_eq!(selected.bounty.current, 0);
    assert_eq!(selected.bounty.active.as_ref(), Some(offer));
    assert_eq!(selected.weekly, weekly.weekly);
    assert_eq!(
        (selected.points, selected.bounty_points),
        (query.points, query.bounty_points)
    );
    let rendered = super::view::panel_text(selected, None);
    assert!(rendered.contains(&format!("Pending points: {}", query.weekly.pending_points)));
    assert!(rendered.contains(&format!(
        "{} kills, {} XP, {} points",
        offer.required, offer.experience, offer.points
    )));
    println!(
        "P23C_REPLAY verified run={}",
        root.file_name()
            .and_then(|name| name.to_str())
            .ok_or("invalid capture directory")?
    );
    Ok(())
}
