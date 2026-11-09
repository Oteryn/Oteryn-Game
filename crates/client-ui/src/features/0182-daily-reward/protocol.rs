use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Refresh,
    Claim,
}
pub fn request(panel: &mut DailyRewardPanel, intent: Intent) -> Option<DailyRewardCommand> {
    if !panel.selected {
        panel.feedback = Some(tr!("Daily rewards are unavailable in this session"));
        return None;
    }
    if panel.pending {
        panel.feedback = Some(tr!("Waiting for the server"));
        return None;
    }
    if intent == Intent::Claim && panel.wall.is_none_or(|wall| !wall.claim_enabled) {
        panel.feedback = Some(tr!("The daily reward cannot be claimed now"));
        return None;
    }
    panel.pending = true;
    panel.feedback = None;
    Some(match intent {
        Intent::Refresh => DailyRewardCommand::Query,
        Intent::Claim => DailyRewardCommand::Claim,
    })
}
pub fn apply_result(panel: &mut DailyRewardPanel, response: &DailyRewardResponse) -> bool {
    panel.pending = false;
    if !matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) && !matches!(response.reply, Some(DailyRewardReply::Result(_)))
    {
        panel.feedback = Some(tr!("The server refused the daily reward action"));
        return false;
    }
    match response.reply {
        Some(DailyRewardReply::Wall(wall)) => {
            panel.wall = Some(wall);
            panel.feedback = None;
            true
        }
        Some(DailyRewardReply::Result(result)) => {
            if matches!(result.disposition, 1 | 2)
                && let Some(wall) = &mut panel.wall
            {
                let receipt = Some((wall.lane_day, wall.streak));
                if !panel.claim_receipts.contains(&receipt) {
                    panel.claim_receipts.copy_within(0..7, 1);
                    panel.claim_receipts[0] = receipt;
                }
                wall.claimed = true;
                wall.claim_enabled = false;
            }
            panel.feedback = Some(match result.disposition {
                1 => tr!("Daily reward claimed"),
                2 => tr!("Daily reward already claimed"),
                3 => tr!("Daily reward request was invalid"),
                4 => tr!("Open the daily reward shrine within reach, then refresh"),
                5 => tr!("Daily reward content is unavailable"),
                6 => tr!("Daily reward content changed; refresh before claiming"),
                _ => tr!("Daily reward claim was refused"),
            });
            result.disposition == 1
        }
        None => {
            panel.feedback = Some(tr!("The server returned no daily reward state"));
            false
        }
    }
}

pub fn request_details(
    panel: &mut DailyRewardPanel,
) -> Option<oteryn_client_runtime::raw_feature::FeatureCommand> {
    super::details::request_details(panel)
}

pub fn request_picker(
    panel: &mut DailyRewardPanel,
    selection: Option<&str>,
) -> Option<oteryn_client_runtime::raw_feature::FeatureCommand> {
    super::picker::request_picker(panel, selection)
}

pub fn request_returner(
    panel: &mut DailyRewardPanel,
    claim: bool,
) -> Option<oteryn_client_runtime::raw_feature::FeatureCommand> {
    super::returner_wire::request_returner(panel, claim)
}
