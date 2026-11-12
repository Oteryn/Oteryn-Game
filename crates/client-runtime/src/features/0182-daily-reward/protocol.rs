//! Typed one-click daily reward v2 commands and bounded reply decoding.
pub use oteryn_session::{CommandStatus, daily_reward};
use oteryn_session::{Session, SessionError, SessionStream};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DailyRewardCommand {
    Query,
    Claim,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DailyRewardReply {
    Wall(daily_reward::DailyRewardWall),
    Result(daily_reward::DailyRewardResult),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyRewardResponse {
    pub status: CommandStatus,
    pub reply: Option<DailyRewardReply>,
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    command: &DailyRewardCommand,
) -> Result<DailyRewardResponse, SessionError> {
    // Opening the wall also attempts the server-owned shrine interaction. The
    // server checks reach and generation; a refused USE still permits viewing.
    if *command == DailyRewardCommand::Query
        && session.is_selected(daily_reward::CAPABILITY_DAILY_REWARD_V2)
    {
        session
            .use_object(b"oteryn:placement/daily-reward-shrine", 0)
            .await?;
    }
    let command_type = match command {
        DailyRewardCommand::Query => daily_reward::COMMAND_TYPE_DAILY_REWARD_QUERY,
        DailyRewardCommand::Claim => daily_reward::COMMAND_TYPE_DAILY_REWARD_CLAIM,
    };
    let result = session
        .gated_command(daily_reward::CAPABILITY_DAILY_REWARD_V2, command_type, &[])
        .await?;
    let reply = (matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) || (*command == DailyRewardCommand::Claim && !result.payload.is_empty()))
    .then(|| match command {
        DailyRewardCommand::Query => decode_wall(&result.payload).map(DailyRewardReply::Wall),
        DailyRewardCommand::Claim => decode_result(&result.payload).map(DailyRewardReply::Result),
    })
    .transpose()?;
    Ok(DailyRewardResponse {
        status: result.status,
        reply,
    })
}

fn varint(bytes: &[u8], at: &mut usize) -> Result<u64, SessionError> {
    let mut value = 0_u64;
    for shift in (0..=63).step_by(7) {
        let byte = *bytes.get(*at).ok_or(SessionError::SessionUnusable)?;
        *at += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(SessionError::SessionUnusable)
}
fn decode_wall(bytes: &[u8]) -> Result<daily_reward::DailyRewardWall, SessionError> {
    if bytes.len() > daily_reward::MAX_DAILY_REWARD_VIEW_BYTES {
        return Err(SessionError::SessionUnusable);
    }
    let mut at = 0;
    let mut values = [None; 5];
    while at < bytes.len() {
        let key = varint(bytes, &mut at)?;
        if key & 7 != 0 || !(1..=5).contains(&(key >> 3)) {
            return Err(SessionError::SessionUnusable);
        }
        let slot = &mut values[(key >> 3) as usize - 1];
        if slot.is_some() {
            return Err(SessionError::SessionUnusable);
        }
        *slot = Some(varint(bytes, &mut at)?);
    }
    let [
        Some(day),
        Some(streak),
        Some(jokers),
        Some(claimed),
        Some(enabled),
    ] = values
    else {
        return Err(SessionError::SessionUnusable);
    };
    let wall = daily_reward::DailyRewardWall {
        lane_day: u8::try_from(day).map_err(|_| SessionError::SessionUnusable)?,
        streak: u32::try_from(streak).map_err(|_| SessionError::SessionUnusable)?,
        jokers: u8::try_from(jokers).map_err(|_| SessionError::SessionUnusable)?,
        claimed: bool_value(claimed)?,
        claim_enabled: bool_value(enabled)?,
    };
    wall.encode().map_err(|_| SessionError::SessionUnusable)?;
    Ok(wall)
}
/// Decodes the command-specific server payload using the bounded v2 contract.
pub fn decode_reply(
    command: DailyRewardCommand,
    bytes: &[u8],
) -> Result<DailyRewardReply, SessionError> {
    match command {
        DailyRewardCommand::Query => decode_wall(bytes).map(DailyRewardReply::Wall),
        DailyRewardCommand::Claim => decode_result(bytes).map(DailyRewardReply::Result),
    }
}
fn bool_value(value: u64) -> Result<bool, SessionError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(SessionError::SessionUnusable),
    }
}
fn decode_result(bytes: &[u8]) -> Result<daily_reward::DailyRewardResult, SessionError> {
    if bytes.len() > daily_reward::MAX_DAILY_REWARD_RESULT_BYTES {
        return Err(SessionError::SessionUnusable);
    }
    let mut at = 0;
    if varint(bytes, &mut at)? != 8 {
        return Err(SessionError::SessionUnusable);
    }
    let disposition =
        u8::try_from(varint(bytes, &mut at)?).map_err(|_| SessionError::SessionUnusable)?;
    let transaction_id = if at == bytes.len() {
        None
    } else {
        if varint(bytes, &mut at)? != 18 || varint(bytes, &mut at)? != 16 || bytes.len() - at != 16
        {
            return Err(SessionError::SessionUnusable);
        }
        let mut id = [0; 16];
        id.copy_from_slice(&bytes[at..]);
        Some(id)
    };
    let result = daily_reward::DailyRewardResult {
        disposition,
        transaction_id,
    };
    result.encode().map_err(|_| SessionError::SessionUnusable)?;
    Ok(result)
}
