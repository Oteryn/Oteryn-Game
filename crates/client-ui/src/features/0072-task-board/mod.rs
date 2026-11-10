//! Server-owned weekly Task Board panel.
#[path = "state/fights.rs"]
pub mod fights;
mod input;
#[path = "view/soulseals.rs"]
mod soulseals;
pub use soulseals::{soulseals_key, soulseals_text};
mod protocol;
pub use input::{creature_choices, creature_name, open_key};
#[path = "view/shop_names.rs"]
mod shop_names;
#[path = "protocol/talisman.rs"]
pub mod talisman;
#[path = "view/tree.rs"]
mod tree;
pub use tree::{TreeNode, tree_nodes};
mod view;
pub use oteryn_client_runtime::task_board::{
    CommandStatus, TaskBoardCommand, TaskBoardResponse, task_board,
};
pub use protocol::{FullIntent, apply_full_result, full_request};
pub use view::{shop_offer_choice, shop_offer_text};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Refresh,
    Select(u8),
    Claim(u8),
    UpgradeTalisman(u8),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskBoardPanel {
    pub fights: fights::FightPanel,
    pub selected: bool,
    pub open: bool,
    pub soulseals_open: bool,
    pub weekly_results_selected: bool,
    pub weekly_checked: bool,
    pub weekly_results_open: bool,
    pub weekly_results_seen: Option<u64>,
    pub pending: bool,
    pub state: Option<task_board::TaskBoardReply>,
    pub feedback: Option<&'static str>,
    pub weekly: Option<task_board::full::Reply>,
}

/// The full weekly owner supersedes the legacy select/claim actions.
pub fn weekly_request(
    panel: &mut TaskBoardPanel,
    difficulty: Option<u8>,
) -> Option<task_board::full::Request> {
    if !panel.selected || panel.pending {
        return None;
    }
    let request = match difficulty {
        Some(difficulty) => task_board::full::Request {
            action: 5,
            selection: u16::from(difficulty),
            race: String::new(),
        },
        None => task_board::full::Request::query(),
    };
    task_board::full::encode_request(&request).ok()?;
    panel.pending = true;
    Some(request)
}

pub fn apply_weekly(panel: &mut TaskBoardPanel, reply: task_board::full::Reply) {
    panel.pending = false;
    panel.feedback = tree::refusal_message(reply.refusal);
    if reply.weekly_results.is_none() {
        panel.weekly_results_open = false;
    }
    panel.weekly = Some(reply);
}

pub fn request(panel: &mut TaskBoardPanel, intent: Intent) -> Option<TaskBoardCommand> {
    if !panel.selected {
        panel.feedback = Some(tr!("Task Board is unavailable in this session"));
        return None;
    }
    if panel.pending {
        panel.feedback = Some(tr!("Waiting for the server"));
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
    panel.weekly = None;
    panel.feedback = Some(if response.reply.refused {
        tr!("The server refused the Task Board action")
    } else {
        tr!("The server confirmed the Task Board action")
    });
    panel.state = Some(response.reply.clone());
}

#[must_use]
pub fn panel_text(panel: &TaskBoardPanel) -> String {
    if let Some(reply) = &panel.weekly {
        return view::panel_text(reply, panel.feedback);
    }
    let mut text = String::from(tr!("Task Board\n"));
    if let Some(state) = &panel.state {
        text.push_str(&lfmt!(
            "Points: {}  Bounty points: {}\n",
            state.points,
            state.bounty_points
        ));
        for (index, task) in state.tasks.iter().enumerate() {
            let progress = if state.selected == (index + 1) as u8 {
                state.kills
            } else {
                0
            };
            text.push_str(&lfmt!(
                "{}. {}: {}/{}  last claim: {}\n",
                index + 1,
                task.race,
                progress,
                task.required,
                task.claimed_at_unix
            ));
        }
        for path in 0..task_board::TALISMAN_PATHS {
            text.push_str(&lfmt!(
                "Talisman {}: level {}, bonus {}, next cost {}\n",
                path + 1,
                state.talisman_levels[path],
                state.talisman_bonuses[path],
                state.talisman_next_costs[path]
            ));
        }
    } else {
        text.push_str(tr!("Refresh to load the board\n"));
    }
    if let Some(feedback) = panel.feedback {
        text.push_str(feedback);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weekly_controls_send_full_owner_actions_and_guard_pending() -> Result<(), &'static str> {
        let mut panel = TaskBoardPanel {
            selected: true,
            ..Default::default()
        };
        assert_eq!(
            weekly_request(&mut panel, None),
            Some(task_board::full::Request::query())
        );
        assert!(weekly_request(&mut panel, Some(0)).is_none());
        panel.pending = false;
        assert!(weekly_request(&mut panel, Some(4)).is_none());
        let request = weekly_request(&mut panel, Some(0)).ok_or("weekly start")?;
        assert_eq!((request.action, request.selection), (5, 0));
        Ok(())
    }
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

#[cfg(test)]
#[path = "tests/e2e_p04.rs"]
mod e2e_p04;
#[cfg(test)]
#[path = "tests/e2e_p05.rs"]
mod e2e_p05;
#[cfg(test)]
#[path = "tests/e2e_p23c.rs"]
mod e2e_p23c;
