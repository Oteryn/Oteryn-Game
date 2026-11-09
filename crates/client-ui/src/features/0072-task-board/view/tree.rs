//! Visual paths and reward state from the full Task Board reply, without local progression.
use super::TaskBoardPanel;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    pub parent: Option<usize>,
    pub path: Option<u8>,
    pub label: String,
}
fn node(rows: &mut Vec<TreeNode>, parent: Option<usize>, path: Option<u8>, label: String) -> usize {
    let index = rows.len();
    rows.push(TreeNode {
        parent,
        path,
        label,
    });
    index
}
pub fn tree_nodes(panel: &TaskBoardPanel) -> Vec<TreeNode> {
    let mut rows = Vec::new();
    let Some(reply) = &panel.weekly else {
        node(
            &mut rows,
            None,
            None,
            if panel.selected {
                "Refresh the board to load upgrades and rewards"
            } else {
                "Task Board unavailable in this session"
            }
            .into(),
        );
        return rows;
    };
    let rewards = node(&mut rows, None, None, tr!("Rewards").into());
    node(
        &mut rows,
        Some(rewards),
        None,
        format!(
            "Weekly reset (automatic): {} task points + {} Soulseals",
            reply.weekly.pending_points, reply.weekly.pending_soulseals
        ),
    );
    if let Some(active) = &reply.bounty.active {
        node(
            &mut rows,
            Some(rewards),
            None,
            format!(
                "Bounty: {} / {} — {}",
                reply.bounty.current,
                active.required,
                if reply.bounty.state == 3 {
                    tr!("ready to claim")
                } else {
                    tr!("in progress")
                }
            ),
        );
        node(
            &mut rows,
            Some(rewards),
            None,
            format!(
                "Claim bounty: {} XP + {} bounty points",
                active.experience, active.points
            ),
        );
    } else {
        node(
            &mut rows,
            Some(rewards),
            None,
            "No active bounty reward to claim".into(),
        );
    }
    let talisman = node(&mut rows, None, None, "Bounty Talisman".into());
    // Accepted task_board_full::TALISMAN_PURCHASE_GOLD profile; the request never sends a price.
    node(
        &mut rows,
        Some(talisman),
        None,
        "Purchase: 5000 gold; server validates funds".into(),
    );
    node(
        &mut rows,
        Some(talisman),
        None,
        "Upgrade buys the next level of the selected path".into(),
    );
    for (index, name) in [
        tr!("Damage"),
        tr!("Life leech"),
        tr!("Loot"),
        "Double Bestiary chance",
    ]
    .iter()
    .enumerate()
    {
        let value = &reply.talisman[index];
        let path = u8::try_from(index).ok();
        let root = node(
            &mut rows,
            Some(talisman),
            path,
            format!("{name} — level {} / 355", value.level),
        );
        node(
            &mut rows,
            Some(root),
            path,
            format!(
                "Current server bonus: {}.{:02}%",
                value.bonus / 100,
                value.bonus % 100
            ),
        );
        if value.level < 355 {
            node(
                &mut rows,
                Some(root),
                path,
                format!(
                    "Next level {}: {} bounty points",
                    value.level + 1,
                    value.next_cost
                ),
            );
            if value.level < 354 {
                node(
                    &mut rows,
                    Some(root),
                    path,
                    "Later levels: quote refreshed after each upgrade".into(),
                );
            }
        } else {
            node(&mut rows, Some(root), path, "Maximum level reached".into());
        }
    }
    rows
}
pub(super) fn refusal_message(code: u8) -> Option<&'static str> {
    Some(match code {
        0 => return None,
        1 => "The server refused: invalid rule input",
        2 => "The server refused: task or bounty is not in the required state",
        3 => "The server refused: level too low for this difficulty",
        4 => "The server refused: insufficient points, tokens, or rule resources",
        5 => "The server refused: free-reroll cooldown has not elapsed",
        6 => "The server refused: progression or claim limit reached",
        7 => "The server refused: this permanent reward is already owned",
        8 => "The server refused: the base outfit is required for this addon",
        9 => "The server refused: invalid random draw; refresh the board",
        10 => "The server refused: insufficient admitted creatures or deliveries",
        11 => "The server refused: reward arithmetic limit reached",
        12 => "The server refused: legacy weekly claims are retired; weekly payout is automatic",
        13 => "The server refused: insufficient deliverable items",
        14 => "The server refused: insufficient gold",
        15 => "The server refused: item or decoration-kit delivery unavailable",
        16 => "The server refused: insufficient inventory capacity",
        17 => "The server refused: cosmetic reward mapping unavailable",
        _ => tr!("The server refused the Task Board action"),
    })
}
