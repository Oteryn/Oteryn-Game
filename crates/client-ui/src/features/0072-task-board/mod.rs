//! Server-owned weekly Task Board panel.
pub use oteryn_client_runtime::task_board::{
    CommandStatus, TaskBoardCommand, TaskBoardResponse, task_board,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Refresh,
    Select(u8),
    Claim(u8),
    UpgradeTalisman(u8),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskBoardPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub state: Option<task_board::TaskBoardReply>,
    pub feedback: Option<&'static str>,
}

pub fn request(panel: &mut TaskBoardPanel, intent: Intent) -> Option<TaskBoardCommand> {
    if !panel.selected {
        panel.feedback = Some("Task Board is unavailable in this session");
        return None;
    }
    if panel.pending {
        panel.feedback = Some("Waiting for the server");
        return None;
    }
    let (action, task) = match intent {
        Intent::Refresh => (0, 0),
        Intent::Select(task) => (1, task),
        Intent::Claim(task) => (2, task),
        Intent::UpgradeTalisman(path) => (3, path),
    };
    let value = task_board::TaskBoardRequest { action, task };
    task_board::encode_task_board_request(&value).ok()?;
    panel.pending = true;
    panel.feedback = None;
    Some(TaskBoardCommand(value))
}

pub fn apply_result(panel: &mut TaskBoardPanel, response: &TaskBoardResponse) {
    panel.pending = false;
    panel.feedback = Some(if response.reply.refused {
        "The server refused the Task Board action"
    } else {
        "The server confirmed the Task Board action"
    });
    panel.state = Some(response.reply.clone());
}

#[must_use]
pub fn panel_text(panel: &TaskBoardPanel) -> String {
    let mut text = String::from("Task Board\n");
    if let Some(state) = &panel.state {
        text.push_str(&format!(
            "Points: {}  Bounty points: {}\n",
            state.points, state.bounty_points
        ));
        for (index, task) in state.tasks.iter().enumerate() {
            let progress = if state.selected == (index + 1) as u8 {
                state.kills
            } else {
                0
            };
            text.push_str(&format!(
                "{}. {}: {}/{}  last claim: {}\n",
                index + 1,
                task.race,
                progress,
                task.required,
                task.claimed_at_unix
            ));
        }
        for path in 0..task_board::TALISMAN_PATHS {
            text.push_str(&format!(
                "Talisman {}: level {}, bonus {}, next cost {}\n",
                path + 1,
                state.talisman_levels[path],
                state.talisman_bonuses[path],
                state.talisman_next_costs[path]
            ));
        }
    } else {
        text.push_str("Refresh to load the board\n");
    }
    if let Some(feedback) = panel.feedback {
        text.push_str(feedback);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    fn reply(refused: bool) -> task_board::TaskBoardReply {
        task_board::TaskBoardReply {
            refused,
            selected: 1,
            kills: 3,
            points: 9,
            bounty_points: 4,
            talisman_levels: [1, 2, 3, 4],
            talisman_bonuses: [2, 3, 4, 5],
            talisman_next_costs: [10, 20, 30, 40],
            tasks: vec![task_board::TaskBoardTask {
                race: "oteryn:rat".into(),
                required: 10,
                claimed_at_unix: 0,
            }],
        }
    }
    #[test]
    fn requests_every_wire_action_and_renders_reply_and_refusal() -> Result<(), &'static str> {
        let mut panel = TaskBoardPanel {
            selected: true,
            ..Default::default()
        };
        for (intent, action, task) in [
            (Intent::Refresh, 0, 0),
            (Intent::Select(1), 1, 1),
            (Intent::Claim(1), 2, 1),
            (Intent::UpgradeTalisman(4), 3, 4),
        ] {
            let command = request(&mut panel, intent).ok_or("valid command")?;
            assert_eq!((command.0.action, command.0.task), (action, task));
            panel.pending = false;
        }
        apply_result(
            &mut panel,
            &TaskBoardResponse {
                status: CommandStatus::Accepted,
                reply: reply(true),
            },
        );
        let text = panel_text(&panel);
        assert!(text.contains("oteryn:rat: 3/10"));
        assert!(text.contains("server refused"));
        Ok(())
    }
}
