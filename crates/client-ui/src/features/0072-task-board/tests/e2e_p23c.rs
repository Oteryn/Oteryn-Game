//! P23c replay: the legacy hunting-task board replies (Query, then selecting task 1) applied
//! through the production `request`, `apply_result` and `panel_text`.
//!
//! The reply bytes are decoded by `task_board::decode_task_board_reply`, exactly as the client
//! transport `send` does before it hands the reply to `apply_result`.
use super::*;

type Error = Box<dyn std::error::Error>;

fn value<'a>(text: &'a str, key: &str) -> Result<&'a str, Error> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

fn load() -> Result<(Vec<u8>, Vec<u8>, String), Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok((
            std::fs::read(root.join("p23c-query.bin"))?,
            std::fs::read(root.join("p23c-select.bin"))?,
            std::fs::read_to_string(root.join("p23c-expected.txt"))?,
        ));
    }
    let board = |selected, refused| task_board::TaskBoardReply {
        refused,
        selected,
        kills: 0,
        points: 25,
        bounty_points: 0,
        talisman_levels: [0; task_board::TALISMAN_PATHS],
        talisman_bonuses: [0; task_board::TALISMAN_PATHS],
        talisman_next_costs: [0; task_board::TALISMAN_PATHS],
        tasks: vec![
            task_board::TaskBoardTask {
                race: "oteryn:rat".into(),
                required: 100,
                claimed_at_unix: 0,
            },
            task_board::TaskBoardTask {
                race: "oteryn:cave_rat".into(),
                required: 150,
                claimed_at_unix: 1_700_000_000,
            },
        ],
    };
    let encode = |reply| task_board::encode_task_board_reply(&reply).map_err(|e| format!("{e:?}"));
    Ok((
        encode(board(0, false))?,
        encode(board(1, false))?,
        "points=25\nbounty_points=0\ntask_count=2\ntask_1=oteryn:rat,100,0\ntask_2=oteryn:cave_rat,150,1700000000\nselect_refused=false\nselect_selected=1\nselect_kills=0\nselect_points=25\n".into(),
    ))
}

fn response(bytes: &[u8]) -> Result<TaskBoardResponse, Error> {
    Ok(TaskBoardResponse {
        status: CommandStatus::Accepted,
        reply: task_board::decode_task_board_reply(bytes).map_err(|e| format!("{e:?}"))?,
    })
}

#[test]
fn e2e_p23c_replays_hunting_task_query_and_select() -> Result<(), Error> {
    let (query, select, expected) = load()?;
    let mut panel = TaskBoardPanel {
        selected: true,
        ..TaskBoardPanel::default()
    };
    let command = request(&mut panel, Intent::Refresh).ok_or("refresh request")?;
    assert_eq!((command.0.action, command.0.task), (0, 0));

    // Query: the exact points and every offered task with its required kills and claim time.
    apply_result(&mut panel, &response(&query)?);
    let state = panel.state.clone().ok_or("query state absent")?;
    assert!(!state.refused);
    assert_eq!(state.points, value(&expected, "points")?.parse::<i64>()?);
    assert_eq!(
        state.bounty_points,
        value(&expected, "bounty_points")?.parse::<i64>()?
    );
    let count: usize = value(&expected, "task_count")?.parse()?;
    assert_eq!(state.tasks.len(), count);
    let text = panel_text(&panel);
    assert!(text.starts_with(&format!(
        "Task Board\nPoints: {}  Bounty points: {}\n",
        value(&expected, "points")?,
        value(&expected, "bounty_points")?
    )));
    for index in 1..=count {
        let line = value(&expected, &format!("task_{index}"))?;
        let mut parts = line.split(',');
        let (race, required, claimed) = (
            parts.next().ok_or("race")?,
            parts.next().ok_or("required")?,
            parts.next().ok_or("claimed")?,
        );
        assert_eq!(state.tasks[index - 1].race, race);
        assert_eq!(state.tasks[index - 1].required.to_string(), required);
        assert!(text.contains(&format!(
            "\n{index}. {race}: 0/{required}  last claim: {claimed}\n"
        )));
    }
    assert_eq!(
        panel.feedback,
        Some("The server confirmed the Task Board action")
    );

    // Select task 1: the panel shows the server's refusal or the active task's progress.
    panel.pending = false;
    let command = request(&mut panel, Intent::Select(1)).ok_or("select request")?;
    assert_eq!((command.0.action, command.0.task), (1, 1));
    apply_result(&mut panel, &response(&select)?);
    let after = panel.state.clone().ok_or("select state absent")?;
    assert_eq!(after.refused, value(&expected, "select_refused")? == "true");
    assert_eq!(
        after.selected,
        value(&expected, "select_selected")?.parse::<u8>()?
    );
    assert_eq!(
        after.kills,
        value(&expected, "select_kills")?.parse::<i64>()?
    );
    assert_eq!(
        after.points,
        value(&expected, "select_points")?.parse::<i64>()?
    );
    assert_eq!(
        panel.feedback,
        Some(if after.refused {
            "The server refused the Task Board action"
        } else {
            "The server confirmed the Task Board action"
        })
    );

    // A truncated or trailing-byte payload is not a reply: the transport never reaches
    // `apply_result`, so the displayed board stays as it was.
    let before = panel.clone();
    let mut truncated = select.clone();
    truncated.pop();
    assert!(response(&truncated).is_err());
    let mut trailing = select;
    trailing.push(0);
    assert!(response(&trailing).is_err());
    assert_eq!(panel, before);
    Ok(())
}
