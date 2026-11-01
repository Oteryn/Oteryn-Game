use super::*;
use oteryn_client_runtime::stash::ItemHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Refresh,
    Previous,
    Next,
    Stow(ItemHandle),
    Withdraw { row: usize, amount: u32 },
}

pub fn request(panel: &mut StashPanel, intent: Intent) -> Option<StashCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    let (page, action) = match intent {
        Intent::Refresh => (panel.page, stash::StashAction::Browse),
        Intent::Previous if panel.page > 0 => (panel.page - 1, stash::StashAction::Browse),
        Intent::Next if panel.has_more => (panel.page + 1, stash::StashAction::Browse),
        Intent::Stow(handle) => (panel.page, stash::StashAction::Stow(handle)),
        Intent::Withdraw { row, amount } => {
            let row = panel.rows.get(row)?.handle;
            (panel.page, stash::StashAction::Withdraw { row, amount })
        }
        _ => return None,
    };
    let command = StashCommand::new(stash::StashIntent { page, action })?;
    panel.pending = true;
    panel.feedback = None;
    Some(command)
}

pub fn apply_result(panel: &mut StashPanel, response: &StashResponse) {
    panel.pending = false;
    panel.page = response.reply.page;
    panel.has_more = response.reply.has_more;
    panel.rows.clone_from(&response.reply.rows);
    panel.row = panel.row.filter(|row| *row < panel.rows.len());
    let message = response.reply.message();
    panel.feedback = (!message.is_empty()).then_some(message);
}
