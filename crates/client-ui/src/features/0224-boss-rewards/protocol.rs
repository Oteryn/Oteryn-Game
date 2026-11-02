use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Browse,
    Take,
    CollectGroup,
    CollectAll,
}

pub fn request(panel: &mut BossRewardsPanel, intent: Intent) -> Option<BossRewardsCommand> {
    if !panel.selected {
        panel.feedback = Some("Boss rewards are unavailable in this session");
        return None;
    }
    if panel.pending {
        panel.feedback = Some("Waiting for the server");
        return None;
    }
    let command = match intent {
        Intent::Browse => BossRewardsCommand::Browse(boss_rewards::BrowseRequest {
            death: panel.death,
            after_ordinal: panel.after_ordinal,
        }),
        Intent::Take => BossRewardsCommand::Take(boss_rewards::TakeRequest {
            death: panel.death,
            ordinal: panel.selected_ordinal,
        }),
        Intent::CollectGroup => BossRewardsCommand::CollectAll(boss_rewards::CollectAllRequest {
            death: Some(panel.death),
        }),
        Intent::CollectAll => {
            BossRewardsCommand::CollectAll(boss_rewards::CollectAllRequest { death: None })
        }
    };
    let valid = match &command {
        BossRewardsCommand::Browse(v) => boss_rewards::encode_browse_request(v).is_ok(),
        BossRewardsCommand::Take(v) => boss_rewards::encode_take_request(v).is_ok(),
        BossRewardsCommand::CollectAll(v) => boss_rewards::encode_collect_all_request(v).is_ok(),
    };
    if !valid {
        panel.feedback = Some("Enter a valid nonzero death id and reward ordinal");
        return None;
    }
    panel.pending = true;
    panel.feedback = None;
    Some(command)
}

pub fn apply_result(panel: &mut BossRewardsPanel, response: &BossRewardsResponse) -> bool {
    panel.pending = false;
    if !matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.feedback = Some("The server refused the boss reward action");
        return false;
    }
    let Some(reply) = &response.reply else {
        panel.feedback = Some("The server returned no boss reward state");
        return false;
    };
    let status = match reply {
        BossRewardsReply::Browse(value) => {
            panel.death = value.death;
            panel.browse = Some(value.clone());
            value.status
        }
        BossRewardsReply::Collect(value) => value.status,
    };
    panel.feedback = match status {
        boss_rewards::BossRewardStatus::Ok => None,
        boss_rewards::BossRewardStatus::Empty => Some("No live rewards remain"),
        boss_rewards::BossRewardStatus::Expired => Some("These rewards expired"),
        boss_rewards::BossRewardStatus::NoRoom => Some("There is no room in your backpack"),
        boss_rewards::BossRewardStatus::OutOfReach => Some("Move next to a qualified reward chest"),
        boss_rewards::BossRewardStatus::Busy => Some("The reward owner is busy; try again"),
        boss_rewards::BossRewardStatus::Rejected => {
            Some("The server rejected this reward identity")
        }
        boss_rewards::BossRewardStatus::Unavailable => {
            Some("Boss rewards are temporarily unavailable")
        }
        boss_rewards::BossRewardStatus::ContentUnavailable => {
            Some("Reward item content is unavailable")
        }
    };
    status == boss_rewards::BossRewardStatus::Ok
}
