use super::{ArenaWireError as Error, push_varint_field};
use crate::charm_wire::read_uint32_fields;

pub const COMMAND_TYPE_ARENA_MATCH: u32 = 436;
pub const MAX_MATCH_REQUEST_BYTES: usize = 32;
pub const MAX_MATCH_RESULT_BYTES: usize = 128;

/// A player can request progression; only the server can record a defeated opponent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArenaAction {
    Status,
    Enter,
    Advance,
    Leave,
    /// Categories: 1 present, 2 supplies, 3 weapon, 4 trophy. Only weapons have choices 2/3.
    ClaimReward {
        rank: u8,
        category: u8,
        choice: u8,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArenaState {
    pub completed_rank: u8,
    pub active_rank: u8,
    pub pit: u8,
    /// 0 idle, 1 fighting, 2 next pit ready, 3 reward room.
    pub phase: u8,
    pub remaining_seconds: u32,
    /// One mask per rank; bits 0..3 are present, supplies, weapon and trophy respectively.
    pub claimed_masks: [u8; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ArenaDisposition {
    Applied = 1,
    WrongState = 2,
    InsufficientGold = 3,
    AlreadyClaimed = 4,
    InventoryFull = 5,
    TimedOut = 6,
    Busy = 7,
    InvalidReward = 8,
    AllChallengesCompleted = 9,
    ContentUnavailable = 10,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArenaResult {
    pub disposition: ArenaDisposition,
    pub state: ArenaState,
}

pub fn encode_match_request(action: ArenaAction) -> Result<Vec<u8>, Error> {
    let fields = match action {
        ArenaAction::Status => [1, 0, 0, 0],
        ArenaAction::Enter => [2, 0, 0, 0],
        ArenaAction::Advance => [3, 0, 0, 0],
        ArenaAction::Leave => [4, 0, 0, 0],
        ArenaAction::ClaimReward {
            rank,
            category,
            choice,
        } => {
            if !valid_reward(rank, category, choice) {
                return Err(Error::Malformed);
            }
            [5, u32::from(rank), u32::from(category), u32::from(choice)]
        }
    };
    Ok(encode_fields(&fields))
}

pub fn decode_match_request(payload: &[u8]) -> Result<ArenaAction, Error> {
    if payload.len() > MAX_MATCH_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    let [action, rank, category, choice] = read_uint32_fields::<4>(payload)?;
    if action != 5 && [rank, category, choice] != [0; 3] {
        return Err(Error::Malformed);
    }
    match action {
        1 => Ok(ArenaAction::Status),
        2 => Ok(ArenaAction::Enter),
        3 => Ok(ArenaAction::Advance),
        4 => Ok(ArenaAction::Leave),
        5 => {
            let rank = byte(rank)?;
            let category = byte(category)?;
            let choice = byte(choice)?;
            if !valid_reward(rank, category, choice) {
                return Err(Error::Malformed);
            }
            Ok(ArenaAction::ClaimReward {
                rank,
                category,
                choice,
            })
        }
        _ => Err(Error::Malformed),
    }
}

pub fn encode_match_result(result: &ArenaResult) -> Result<Vec<u8>, Error> {
    validate_state(&result.state)?;
    let state = result.state;
    Ok(encode_fields(&[
        result.disposition as u32,
        u32::from(state.completed_rank),
        u32::from(state.active_rank),
        u32::from(state.pit),
        u32::from(state.phase),
        state.remaining_seconds,
        u32::from(state.claimed_masks[0]),
        u32::from(state.claimed_masks[1]),
        u32::from(state.claimed_masks[2]),
    ]))
}

pub fn decode_match_result(payload: &[u8]) -> Result<ArenaResult, Error> {
    if payload.len() > MAX_MATCH_RESULT_BYTES {
        return Err(Error::LimitExceeded);
    }
    let [
        disposition,
        completed,
        active,
        pit,
        phase,
        remaining,
        greenhorn,
        scrapper,
        warlord,
    ] = read_uint32_fields::<9>(payload)?;
    let disposition = match disposition {
        1 => ArenaDisposition::Applied,
        2 => ArenaDisposition::WrongState,
        3 => ArenaDisposition::InsufficientGold,
        4 => ArenaDisposition::AlreadyClaimed,
        5 => ArenaDisposition::InventoryFull,
        6 => ArenaDisposition::TimedOut,
        7 => ArenaDisposition::Busy,
        8 => ArenaDisposition::InvalidReward,
        9 => ArenaDisposition::AllChallengesCompleted,
        10 => ArenaDisposition::ContentUnavailable,
        _ => return Err(Error::Malformed),
    };
    let state = ArenaState {
        completed_rank: byte(completed)?,
        active_rank: byte(active)?,
        pit: byte(pit)?,
        phase: byte(phase)?,
        remaining_seconds: remaining,
        claimed_masks: [byte(greenhorn)?, byte(scrapper)?, byte(warlord)?],
    };
    validate_state(&state)?;
    Ok(ArenaResult { disposition, state })
}

fn encode_fields(fields: &[u32]) -> Vec<u8> {
    let mut out = Vec::new();
    for (index, value) in fields.iter().enumerate() {
        if *value != 0 {
            push_varint_field(&mut out, index as u64 + 1, u64::from(*value));
        }
    }
    out
}

fn byte(value: u32) -> Result<u8, Error> {
    u8::try_from(value).map_err(|_| Error::Malformed)
}

fn valid_reward(rank: u8, category: u8, choice: u8) -> bool {
    (1..=3).contains(&rank)
        && (1..=4).contains(&category)
        && if category == 3 {
            (1..=3).contains(&choice)
        } else {
            choice == 1
        }
}

fn validate_state(state: &ArenaState) -> Result<(), Error> {
    if state.completed_rank > 3
        || state.active_rank > 3
        || state.pit > 10
        || state.phase > 3
        || state.remaining_seconds > 600
        || state.claimed_masks.iter().any(|mask| *mask > 15)
    {
        return Err(Error::Malformed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_actions_roundtrip_and_never_accept_client_kill_reports() -> Result<(), Error> {
        for action in [
            ArenaAction::Status,
            ArenaAction::Enter,
            ArenaAction::Advance,
            ArenaAction::Leave,
        ] {
            assert_eq!(
                decode_match_request(&encode_match_request(action)?)?,
                action
            );
        }
        for rank in 1..=3 {
            for category in 1..=4 {
                for choice in 1..=if category == 3 { 3 } else { 1 } {
                    let action = ArenaAction::ClaimReward {
                        rank,
                        category,
                        choice,
                    };
                    assert_eq!(
                        decode_match_request(&encode_match_request(action)?)?,
                        action
                    );
                }
            }
        }
        assert_eq!(decode_match_request(&[8, 6]), Err(Error::Malformed));
        Ok(())
    }

    #[test]
    fn match_requests_refuse_unknown_repeated_wrong_wire_and_excess_fields() {
        for payload in [
            &[][..],
            &[8, 0],
            &[8, 255, 1],
            &[8, 1, 8, 1],
            &[8, 1, 40, 1],
            &[10, 0],
            &[8],
            &[8, 2, 16, 1],
            &[8, 5, 16, 1, 24, 1, 32, 2],
        ] {
            assert_eq!(decode_match_request(payload), Err(Error::Malformed));
        }
        assert_eq!(
            decode_match_request(&[0; MAX_MATCH_REQUEST_BYTES + 1]),
            Err(Error::LimitExceeded)
        );
        for (rank, category, choice) in [
            (0, 1, 1),
            (4, 1, 1),
            (1, 0, 1),
            (1, 5, 1),
            (1, 3, 0),
            (1, 3, 4),
            (1, 4, 2),
        ] {
            assert_eq!(
                encode_match_request(ArenaAction::ClaimReward {
                    rank,
                    category,
                    choice
                }),
                Err(Error::Malformed)
            );
        }
    }

    #[test]
    fn match_results_preserve_state_for_accepted_and_refused_replays() -> Result<(), Error> {
        for disposition in [
            ArenaDisposition::Applied,
            ArenaDisposition::WrongState,
            ArenaDisposition::InsufficientGold,
            ArenaDisposition::AlreadyClaimed,
            ArenaDisposition::InventoryFull,
            ArenaDisposition::TimedOut,
            ArenaDisposition::Busy,
            ArenaDisposition::InvalidReward,
            ArenaDisposition::AllChallengesCompleted,
            ArenaDisposition::ContentUnavailable,
        ] {
            let result = ArenaResult {
                disposition,
                state: ArenaState {
                    completed_rank: 2,
                    active_rank: 3,
                    pit: 10,
                    phase: 2,
                    remaining_seconds: 600,
                    claimed_masks: [15, 7, 0],
                },
            };
            assert_eq!(decode_match_result(&encode_match_result(&result)?)?, result);
        }
        Ok(())
    }

    #[test]
    fn match_results_refuse_malformed_states_and_byte_overflow() {
        for payload in [
            &[][..],
            &[8, 11],
            &[8, 1, 16, 4],
            &[8, 1, 24, 4],
            &[8, 1, 32, 11],
            &[8, 1, 40, 4],
            &[8, 1, 48, 217, 4],
            &[8, 1, 56, 16],
            &[8, 1, 64, 128, 2],
            &[8, 1, 8, 1],
            &[8, 1, 80, 1],
        ] {
            assert_eq!(decode_match_result(payload), Err(Error::Malformed));
        }
        assert_eq!(
            decode_match_result(&[0; MAX_MATCH_RESULT_BYTES + 1]),
            Err(Error::LimitExceeded)
        );
    }
}
