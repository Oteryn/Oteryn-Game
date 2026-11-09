use super::{TaskBoardPanel, task_board};
use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};

/// Crystal protocolgame.cpp:3437-3588, mapped by task_board_v1.md to Oteryn actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FullIntent {
    Refresh,
    StartWeekly(u8),
    Deliver(u8),
    OpenBounty,
    SelectBounty(u8),
    ClaimBounty,
    BountyDifficulty(u8),
    Reroll,
    DailyReroll,
    Preferred(u8, String),
    Unwanted(u8, String),
    UnlockList,
    BuyShop(u16),
}

pub fn full_request(panel: &mut TaskBoardPanel, intent: FullIntent) -> Option<FeatureCommand> {
    let (action, selection, race) = match intent {
        FullIntent::Refresh => (4, 0, String::new()),
        FullIntent::StartWeekly(v) => (5, u16::from(v), String::new()),
        FullIntent::Deliver(v) => (6, u16::from(v), String::new()),
        FullIntent::OpenBounty => (7, 0, String::new()),
        FullIntent::SelectBounty(v) => (8, u16::from(v), String::new()),
        FullIntent::ClaimBounty => (9, 0, String::new()),
        FullIntent::BountyDifficulty(v) => (10, u16::from(v), String::new()),
        FullIntent::Reroll => (11, 0, String::new()),
        FullIntent::DailyReroll => (12, 0, String::new()),
        FullIntent::Preferred(v, race) => (13, u16::from(v), race),
        FullIntent::Unwanted(v, race) => (14, u16::from(v), race),
        FullIntent::UnlockList => (15, 0, String::new()),
        FullIntent::BuyShop(v) => (16, v, String::new()),
    };
    command(
        panel,
        task_board::full::Request {
            action,
            selection,
            race,
        },
    )
}

pub(super) fn command(
    panel: &mut TaskBoardPanel,
    request: task_board::full::Request,
) -> Option<FeatureCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    let payload = task_board::full::encode_request(&request).ok()?;
    panel.pending = true;
    panel.feedback = None;
    Some(FeatureCommand {
        capability: task_board::CAPABILITY_TASK_BOARD_V1,
        kind: task_board::COMMAND_TYPE_TASK_BOARD,
        payload,
    })
}

pub fn apply_full_result(panel: &mut TaskBoardPanel, result: &GatedCommandResult) {
    panel.pending = false;
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.feedback = Some(tr!("The server refused the Task Board action"));
        return;
    }
    match task_board::full::decode_reply(&result.payload) {
        Ok(reply) => super::apply_weekly(panel, reply),
        Err(_) => panel.feedback = Some(tr!("The server returned an invalid Task Board reply")),
    }
}
