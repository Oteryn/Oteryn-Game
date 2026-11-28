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
        assert_eq!(
            frame["capability"].as_u64(),
            Some(u64::from(request.capability))
        );
        assert_eq!(frame["kind"].as_u64(), Some(u64::from(request.kind)));
        assert_eq!(request.capability, task_board::CAPABILITY_TASK_BOARD_V1);
        assert_eq!(request.kind, task_board::COMMAND_TYPE_TASK_BOARD);
        let next_command = frame["command"].as_u64().ok_or("missing command ID")?;
        let next_sequence = frame["sequence"]
            .as_u64()
            .ok_or("missing server sequence")?;
        assert!(next_command > command_id && next_sequence > server_sequence);
        command_id = next_command;
        server_sequence = next_sequence;
        assert_eq!(frame["status"].as_str(), Some("Accepted"));
        let payload: Vec<u8> = serde_json::from_value(frame["payload"].clone())?;
        let expected =
            task_board::full::decode_reply(&payload).map_err(|error| format!("{error:?}"))?;
        apply_full_result(
            &mut panel,
            &GatedCommandResult {
                command_id,
                server_sequence,
                status: CommandStatus::Accepted,
                payload,
            },
        );
        let view = panel.weekly.clone().ok_or("client board not updated")?;
        assert!(!panel.pending);
        assert_eq!(view, expected);
        assert_eq!(expected.refusal, 0);
        assert_eq!(panel.feedback, None);
        verify_panel(&panel, &expected);
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
    let valid: Vec<u8> = serde_json::from_value(commands[3]["payload"].clone())?;
    let mut truncated = valid.clone();
    truncated.pop().ok_or("empty captured response")?;
    let mut trailing = valid;
    trailing.push(0);
    for payload in [truncated, trailing] {
        assert!(task_board::full::decode_reply(&payload).is_err());
        panel.pending = true;
        apply_full_result(
            &mut panel,
            &GatedCommandResult {
                command_id: command_id + 1,
                server_sequence: server_sequence + 1,
                status: CommandStatus::Accepted,
                payload,
            },
        );
        assert!(!panel.pending);
        assert_eq!(panel.weekly.as_ref(), Some(selected));
        assert_eq!(
            panel.feedback,
            Some("The server returned an invalid Task Board reply")
        );
        assert!(panel_text(&panel).ends_with("The server returned an invalid Task Board reply"));
    }
    println!(
        "P23C_REPLAY verified run={}",
        root.file_name()
            .and_then(|name| name.to_str())
            .ok_or("invalid capture directory")?
    );
    Ok(())
}

fn verify_panel(panel: &TaskBoardPanel, reply: &task_board::full::Reply) {
    let text = panel_text(panel);
    let name = |race: &str| {
        if race.is_empty() {
            "Any creature".into()
        } else {
            race.trim_start_matches("oteryn:creature.")
                .replace('_', " ")
        }
    };
    assert!(text.starts_with(&format!(
        "Task Board — week {}\nPoints: {}  Bounty points: {}  Soulseals: {}\n",
        reply.period, reply.points, reply.bounty_points, reply.soulseals
    )));
    let weekly_header = format!(
        "Weekly: started {}, difficulty {}, expanded {}, multiplier {}",
        reply.weekly.started,
        reply.weekly.difficulty,
        reply.weekly.expanded,
        reply.weekly.multiplier
    );
    assert!(text.lines().any(|line| line == weekly_header));
    let expected_weekly = format!(
        "Kill XP: {}  Delivery XP: {}  Pending points: {}  Pending soulseals: {}",
        reply.weekly.kill_experience,
        reply.weekly.delivery_experience,
        reply.weekly.pending_points,
        reply.weekly.pending_soulseals
    );
    assert!(text.lines().any(|line| line == expected_weekly));
    let kills: Vec<_> = reply
        .weekly
        .kills
        .iter()
        .enumerate()
        .map(|(i, task)| {
            format!(
                "Kill {}. {}: {}/{}",
                i + 1,
                name(&task.race),
                task.current,
                task.required
            )
        })
        .collect();
    let deliveries: Vec<_> = reply
        .weekly
        .deliveries
        .iter()
        .enumerate()
        .map(|(i, task)| {
            format!(
                "Delivery {}. {}: {}/{}  delivered: {}",
                i + 1,
                task.item,
                task.current,
                task.required,
                task.delivered
            )
        })
        .collect();
    let bounty = &reply.bounty;
    let offers: Vec<_> = bounty
        .offers
        .iter()
        .enumerate()
        .map(|(i, offer)| (i + 1, offer))
        .chain(
            bounty
                .active
                .iter()
                .map(|offer| (usize::from(bounty.selected), offer)),
        )
        .map(|(i, offer)| {
            format!(
                "Bounty {i}. {}: {} kills, {} XP, {} points, grade {}",
                name(&offer.race),
                offer.required,
                offer.experience,
                offer.points,
                ["Ordinary", "Silver", "Gold", "Platinum"][usize::from(offer.grade)]
            )
        })
        .collect();
    for (prefix, expected) in [
        ("Kill ", kills),
        ("Delivery ", deliveries),
        ("Bounty ", offers),
    ] {
        let visible: Vec<_> = text
            .lines()
            .filter(|line| {
                line.strip_prefix(prefix)
                    .is_some_and(|rest| rest.starts_with(|ch: char| ch.is_ascii_digit()))
            })
            .collect();
        assert_eq!(visible, expected);
    }
    let active: Vec<_> = bounty
        .active
        .iter()
        .map(|offer| {
            format!(
                "Active bounty: {} {}/{} (offer {})",
                name(&offer.race),
                bounty.current,
                offer.required,
                bounty.selected
            )
        })
        .collect();
    assert_eq!(
        text.lines()
            .filter(|line| line.starts_with("Active bounty:"))
            .collect::<Vec<_>>(),
        active
    );
}
