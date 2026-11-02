//! Typed personal boss reward commands sent through the admitted session.
pub use oteryn_session::{CommandStatus, boss_rewards};
use oteryn_session::{Session, SessionError, SessionStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BossRewardsCommand {
    Browse(boss_rewards::BrowseRequest),
    Take(boss_rewards::TakeRequest),
    CollectAll(boss_rewards::CollectAllRequest),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BossRewardsReply {
    Browse(boss_rewards::BrowseReply),
    Collect(boss_rewards::CollectReply),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BossRewardsResponse {
    pub status: CommandStatus,
    pub reply: Option<BossRewardsReply>,
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    command: &BossRewardsCommand,
) -> Result<BossRewardsResponse, SessionError> {
    let (kind, payload) = match command {
        BossRewardsCommand::Browse(value) => (
            boss_rewards::COMMAND_TYPE_BOSS_REWARD_BROWSE,
            boss_rewards::encode_browse_request(value),
        ),
        BossRewardsCommand::Take(value) => (
            boss_rewards::COMMAND_TYPE_BOSS_REWARD_TAKE,
            boss_rewards::encode_take_request(value),
        ),
        BossRewardsCommand::CollectAll(value) => (
            boss_rewards::COMMAND_TYPE_BOSS_REWARD_COLLECT_ALL,
            boss_rewards::encode_collect_all_request(value),
        ),
    };
    let payload = payload.map_err(|_| SessionError::SessionUnusable)?;
    let result = session
        .gated_command(boss_rewards::CAPABILITY_BOSS_REWARDS_V1, kind, &payload)
        .await?;
    let reply = matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    )
    .then(|| match command {
        BossRewardsCommand::Browse(_) => {
            boss_rewards::decode_browse_reply(&result.payload).map(BossRewardsReply::Browse)
        }
        BossRewardsCommand::Take(_) | BossRewardsCommand::CollectAll(_) => {
            boss_rewards::decode_collect_reply(&result.payload).map(BossRewardsReply::Collect)
        }
    })
    .transpose()
    .map_err(|_| SessionError::SessionUnusable)?;
    Ok(BossRewardsResponse {
        status: result.status,
        reply,
    })
}
