//! TASKBOARD-1 weekly Task Board: query, select and claim one kill task.
use crate::charm_wire::{
    CyclopediaWireError as Error, push_message_field, push_varint_field, read_bytes, read_varint,
    set_once,
};

pub const CAPABILITY_TASK_BOARD_V1: u32 = 72;
pub const COMMAND_TYPE_TASK_BOARD: u32 = 89;
/// `TASKBOARD1-RL-01`.
pub const MAX_REQUEST_BYTES: usize = 32;
/// `TASKBOARD1-RL-02`.
pub const MAX_REPLY_BYTES: usize = 1024;
/// `TASKBOARD1-RL-03`: kill tasks on one board.
pub const MAX_TASKS: usize = 5;
const MAX_RACE_BYTES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardRequest {
    /// 0 query, 1 select weekly, 2 claim weekly, 3 select bounty, 4 claim bounty.
    pub action: u8,
    /// The 1-based task number; 0 for a query.
    pub task: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardTask {
    /// The Creature race key the task counts.
    pub race: String,
    pub required: i64,
    /// Database time of the last claim, Unix seconds; 0 when never claimed.
    pub claimed_at_unix: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardReply {
    /// The store refused the select or claim; the board is unchanged.
    pub refused: bool,
    /// The active task's number; 0 when none.
    pub selected: u8,
    pub kills: i64,
    pub points: i64,
    pub tasks: Vec<TaskBoardTask>,
    pub bounty_selected: u8,
    pub bounty_kills: i64,
    pub bounty_points: i64,
    pub reroll_tokens: u8,
    pub next_difficulty: u8,
    pub talisman_owned: bool,
    pub talisman_level: u8,
}

fn valid_race(race: &str) -> bool {
    race.len() <= MAX_RACE_BYTES
        && race.strip_prefix("oteryn:").is_some_and(|tail| {
            !tail.is_empty()
                && tail
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
        })
}

pub fn encode_task_board_request(r: &TaskBoardRequest) -> Result<Vec<u8>, Error> {
    let task_ok = match r.action {
        0 | 5 | 6 => r.task == 0,
        1 | 2 => (1..=MAX_TASKS).contains(&usize::from(r.task)),
        3 | 4 => (1..=12).contains(&usize::from(r.task)),
        7 => (1..=4).contains(&usize::from(r.task)),
        _ => false,
    };
    if !task_ok {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, u64::from(r.action));
    push_varint_field(&mut out, 2, u64::from(r.task));
    Ok(out)
}

pub fn decode_task_board_request(bytes: &[u8]) -> Result<TaskBoardRequest, Error> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut action, mut task) = (None, None);
    let mut at = 0;
    while at < bytes.len() {
        let slot = match read_varint(bytes, &mut at)? {
            8 => &mut action,
            16 => &mut task,
            _ => return Err(Error::Malformed),
        };
        set_once(
            slot,
            u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
        )?;
    }
    let r = TaskBoardRequest {
        action: action.ok_or(Error::Malformed)?,
        task: task.ok_or(Error::Malformed)?,
    };
    encode_task_board_request(&r)?;
    Ok(r)
}

pub fn encode_task_board_reply(r: &TaskBoardReply) -> Result<Vec<u8>, Error> {
    if r.tasks.len() > MAX_TASKS {
        return Err(Error::LimitExceeded);
    }
    let kills_ok = match usize::from(r.selected).checked_sub(1) {
        None => r.kills == 0,
        Some(index) => r
            .tasks
            .get(index)
            .is_some_and(|task| (0..=task.required).contains(&r.kills)),
    };
    if !kills_ok
        || r.points < 0
        || r.bounty_selected > 12
        || r.bounty_kills < 0
        || r.bounty_points < 0
        || r.reroll_tokens > 10
        || r.next_difficulty > 4
        || r.talisman_level > 10
        || r.tasks
            .iter()
            .any(|task| !valid_race(&task.race) || task.required <= 0 || task.claimed_at_unix < 0)
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, u64::from(r.refused));
    push_varint_field(&mut out, 2, u64::from(r.selected));
    push_varint_field(&mut out, 3, r.kills as u64);
    push_varint_field(&mut out, 4, r.points as u64);
    for task in &r.tasks {
        let mut entry = Vec::new();
        push_message_field(&mut entry, 1, task.race.as_bytes());
        push_varint_field(&mut entry, 2, task.required as u64);
        push_varint_field(&mut entry, 3, task.claimed_at_unix as u64);
        push_message_field(&mut out, 5, &entry);
    }
    push_varint_field(&mut out, 6, u64::from(r.bounty_selected));
    push_varint_field(&mut out, 7, r.bounty_kills as u64);
    push_varint_field(&mut out, 8, r.bounty_points as u64);
    push_varint_field(&mut out, 9, u64::from(r.reroll_tokens));
    push_varint_field(&mut out, 10, u64::from(r.next_difficulty));
    push_varint_field(&mut out, 11, u64::from(r.talisman_owned));
    push_varint_field(&mut out, 12, u64::from(r.talisman_level));
    if out.len() > MAX_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}

fn decode_task(bytes: &[u8]) -> Result<TaskBoardTask, Error> {
    let (mut race, mut required, mut claimed_at) = (None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            10 => set_once(
                &mut race,
                std::str::from_utf8(read_bytes(bytes, &mut at)?)
                    .map_err(|_| Error::Malformed)?
                    .to_owned(),
            )?,
            16 => set_once(&mut required, read_i64(bytes, &mut at)?)?,
            24 => set_once(&mut claimed_at, read_i64(bytes, &mut at)?)?,
            _ => return Err(Error::Malformed),
        }
    }
    Ok(TaskBoardTask {
        race: race.ok_or(Error::Malformed)?,
        required: required.ok_or(Error::Malformed)?,
        claimed_at_unix: claimed_at.ok_or(Error::Malformed)?,
    })
}

fn read_i64(bytes: &[u8], at: &mut usize) -> Result<i64, Error> {
    i64::try_from(read_varint(bytes, at)?).map_err(|_| Error::Malformed)
}

pub fn decode_task_board_reply(bytes: &[u8]) -> Result<TaskBoardReply, Error> {
    if bytes.len() > MAX_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut refused, mut selected, mut kills, mut points) = (None, None, None, None);
    let (mut bounty_selected, mut bounty_kills, mut bounty_points) = (None, None, None);
    let (mut reroll_tokens, mut next_difficulty, mut talisman_owned, mut talisman_level) =
        (None, None, None, None);
    let mut tasks = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(
                &mut refused,
                match read_varint(bytes, &mut at)? {
                    0 => false,
                    1 => true,
                    _ => return Err(Error::Malformed),
                },
            )?,
            16 => set_once(
                &mut selected,
                u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            24 => set_once(&mut kills, read_i64(bytes, &mut at)?)?,
            32 => set_once(&mut points, read_i64(bytes, &mut at)?)?,
            42 => {
                if tasks.len() == MAX_TASKS {
                    return Err(Error::LimitExceeded);
                }
                tasks.push(decode_task(read_bytes(bytes, &mut at)?)?);
            }
            48 => set_once(
                &mut bounty_selected,
                u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            56 => set_once(&mut bounty_kills, read_i64(bytes, &mut at)?)?,
            64 => set_once(&mut bounty_points, read_i64(bytes, &mut at)?)?,
            72 => set_once(
                &mut reroll_tokens,
                u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            80 => set_once(
                &mut next_difficulty,
                u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            88 => set_once(
                &mut talisman_owned,
                match read_varint(bytes, &mut at)? {
                    0 => false,
                    1 => true,
                    _ => return Err(Error::Malformed),
                },
            )?,
            96 => set_once(
                &mut talisman_level,
                u8::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    let r = TaskBoardReply {
        refused: refused.ok_or(Error::Malformed)?,
        selected: selected.ok_or(Error::Malformed)?,
        kills: kills.ok_or(Error::Malformed)?,
        points: points.ok_or(Error::Malformed)?,
        tasks,
        bounty_selected: bounty_selected.ok_or(Error::Malformed)?,
        bounty_kills: bounty_kills.ok_or(Error::Malformed)?,
        bounty_points: bounty_points.ok_or(Error::Malformed)?,
        reroll_tokens: reroll_tokens.ok_or(Error::Malformed)?,
        next_difficulty: next_difficulty.ok_or(Error::Malformed)?,
        talisman_owned: talisman_owned.ok_or(Error::Malformed)?,
        talisman_level: talisman_level.ok_or(Error::Malformed)?,
    };
    // Strict: only the canonical encoding (field order, minimal varints) decodes.
    if encode_task_board_reply(&r)? != bytes {
        return Err(Error::Malformed);
    }
    Ok(r)
}

#[cfg(test)]
#[path = "task_board_tests.rs"]
mod tests;
