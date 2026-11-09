use super::*;
use oteryn_client_runtime::stash::ItemHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Refresh,
    Filter,
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
        Intent::Filter => (0, stash::StashAction::Browse),
        Intent::Refresh => (panel.page, stash::StashAction::Browse),
        Intent::Previous if panel.page > 0 => (panel.page - 1, stash::StashAction::Browse),
        Intent::Next if panel.has_more => (panel.page + 1, stash::StashAction::Browse),
        Intent::Stow(handle) => {
            if !panel.carried.iter().any(|item| item.handle == handle) {
                panel.feedback = Some(tr!("The item changed; select a carried item again").into());
                return None;
            }
            (panel.page, stash::StashAction::Stow(handle))
        }
        Intent::Withdraw { row, amount } => {
            let row = panel.rows.get(row)?.handle;
            (panel.page, stash::StashAction::Withdraw { row, amount })
        }
        _ => return None,
    };
    let Some(command) = StashCommand::filtered(
        stash::StashIntent { page, action },
        Some(stash::StashFilter {
            search: panel.query.trim().into(),
            category: panel.category.trim().into(),
            trader: panel.trader.trim().into(),
            stock_order: panel.sort == Sort::Stock,
        }),
    ) else {
        panel.feedback = Some(
            "Each filter must contain at most 128 UTF-8 bytes and no control characters".into(),
        );
        return None;
    };
    panel.pending = true;
    panel.feedback = None;
    Some(command)
}

pub fn apply_result(panel: &mut StashPanel, response: &StashResponse) {
    panel.pending = false;
    panel.server_filtered = true;
    panel.page = response.reply.page;
    panel.has_more = response.reply.has_more;
    panel.rows.clone_from(&response.reply.rows);
    panel.row = panel.row.filter(|row| *row < panel.rows.len());
    let message = response.reply.message();
    panel.feedback = (!message.is_empty()).then_some(message);
}

/// Resolve a click only while the displayed carried list still matches live items.
pub fn carried_intent(
    panel: &StashPanel,
    displayed: &[oteryn_client_runtime::depot::ItemEntry],
    row: usize,
) -> Option<Intent> {
    if displayed != panel.carried {
        return None;
    }
    panel.carried.get(row).map(|item| Intent::Stow(item.handle))
}
