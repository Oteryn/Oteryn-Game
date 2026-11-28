use super::*;
use oteryn_client_runtime::stash::ItemHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Refresh,
    Filter,
    Previous,
    Next,
    Stow(ItemHandle),
    StowAction(stash::StashAction),
    Withdraw { row: usize, amount: u32 },
}

pub fn request(panel: &mut StashPanel, intent: Intent) -> Option<StashCommand> {
    request_with_stackability(panel, intent, |_| None)
}

pub fn request_stow(
    panel: &mut StashPanel,
    handle: ItemHandle,
    stackable: impl FnOnce(u16) -> Option<bool>,
) -> Option<StashCommand> {
    request_stow_action(panel, stash::StashAction::Stow(handle), stackable)
}

pub fn request_stow_action(
    panel: &mut StashPanel,
    action: stash::StashAction,
    stackable: impl FnOnce(u16) -> Option<bool>,
) -> Option<StashCommand> {
    request_with_stackability(panel, Intent::StowAction(action), stackable)
}

fn request_with_stackability(
    panel: &mut StashPanel,
    intent: Intent,
    stackable: impl FnOnce(u16) -> Option<bool>,
) -> Option<StashCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    let intent = match intent {
        Intent::Stow(handle) => Intent::StowAction(stash::StashAction::Stow(handle)),
        intent => intent,
    };
    let (page, action) = match intent {
        Intent::Filter => (0, stash::StashAction::Browse),
        Intent::Refresh => (panel.page, stash::StashAction::Browse),
        Intent::Previous if panel.page > 0 => (panel.page - 1, stash::StashAction::Browse),
        Intent::Next if panel.has_more => (panel.page + 1, stash::StashAction::Browse),
        Intent::StowAction(action) => {
            if action.needs_stash_actions() && !panel.actions_selected {
                return None;
            }
            let handle = action.stow_handle()?;
            let Some(item) = panel.carried.iter().find(|item| item.handle == handle) else {
                panel.feedback = Some(tr!("The item changed; select a carried item again").into());
                return None;
            };
            // Without appearance facts, only a visible multi-unit stack proves eligibility.
            let eligible = stackable(item.appearance_id).unwrap_or_else(|| item.count.get() > 1);
            if let stash::StashAction::StowQuantity { amount, .. } = action
                && !(1..=item.count.get()).contains(&amount)
            {
                return None;
            }
            if !eligible && !matches!(action, stash::StashAction::StowContents(_)) {
                panel.outcome = Some(stash::StashOutcome::Unsupported);
                panel.feedback =
                    Some("Only stackable items can be stowed in the Supply Stash.".into());
                return None;
            }
            (panel.page, action)
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
    panel.outcome = None;
    panel.feedback = None;
    Some(command)
}

pub fn apply_result(panel: &mut StashPanel, response: &StashResponse) {
    panel.pending = false;
    panel.server_filtered = true;
    panel.outcome = Some(response.reply.outcome);
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
