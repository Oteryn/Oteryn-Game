//! Replay-bound Barbarian Test booze outcomes, Crystal action_mead.lua:6-29.
use super::QuestRefusal;

/// Locked source snapshot: questline, mission01, successful sips, total sips.
/// Roll is supplied by the durable owner, never by a gameplay client.
pub fn apply(mut state: [i64; 4], roll: u8) -> Result<[i64; 4], QuestRefusal> {
    if !(1..=5).contains(&roll) {
        return Err(QuestRefusal::NotSupported);
    }
    if state[0] == 2 && state[3] <= 20 {
        state[2] = if roll > 1 {
            state[2].checked_add(1).ok_or(QuestRefusal::OutOfRange)?
        } else {
            0
        };
        if state[2] == 9 {
            state[0] = 3;
            state[1] = 3;
            return Ok(state);
        }
        state[3] = state[3].checked_add(1).ok_or(QuestRefusal::OutOfRange)?;
    } else if state[3] > 20 {
        state[0] = 1;
        state[1] = 1;
    } else {
        return Err(QuestRefusal::StageMismatch);
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn m47_mead_source_boundaries() {
        assert_eq!(apply([2, 2, 8, 20], 5), Ok([3, 3, 9, 20]));
        assert_eq!(apply([2, 2, 8, 20], 1), Ok([2, 2, 0, 21]));
        assert_eq!(apply([2, 2, 0, 21], 5), Ok([1, 1, 0, 21]));
        for roll in 2..=5 {
            assert_eq!(apply([2, 2, -1, -1], roll), Ok([2, 2, 0, 0]));
        }
        assert!(apply([1, 1, 0, 0], 5).is_err());
        assert!(apply([2, 2, 0, 0], 0).is_err());
        assert!(apply([2, 2, 0, 0], 6).is_err());
    }
}
