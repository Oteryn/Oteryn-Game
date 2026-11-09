//! Bound imported callbacks to their own tracks while reusing the existing NPC state owner.
use super::BoundQuest;
use crate::quest::reward_callbacks::{StateCondition, StateMutation, StateOperation};

pub(super) fn valid(quest: &BoundQuest, operation: &StateOperation) -> bool {
    if !operation.valid() {
        return false;
    }
    let Some(track) = quest
        .tracks
        .iter()
        .find(|track| track.key == operation.track)
    else {
        return false;
    };
    let bounds = match operation.from {
        StateCondition::Between { minimum, maximum } => {
            if minimum < track.minimum || maximum > track.maximum {
                return false;
            }
            Some((minimum, maximum))
        }
        StateCondition::Any => None,
        StateCondition::ElapsedAtLeast { .. } => {
            if track.minimum < 0 {
                return false;
            }
            None
        }
    };
    match operation.effect {
        StateMutation::Add { value } => match bounds {
            Some((minimum, maximum)) => {
                minimum
                    .checked_add(value)
                    .is_some_and(|after| (track.minimum..=track.maximum).contains(&after))
                    && maximum
                        .checked_add(value)
                        .is_some_and(|after| (track.minimum..=track.maximum).contains(&after))
            }
            // Unconditional and elapsed predicates may read without changing the track.
            None => value == 0,
        },
        StateMutation::AddAtLeast {
            value,
            minimum: floor,
        } => {
            (track.minimum..=track.maximum).contains(&floor)
                && match bounds {
                    Some((minimum, maximum)) => [minimum, maximum].into_iter().all(|before| {
                        before.checked_add(value).is_some_and(|after| {
                            (track.minimum..=track.maximum).contains(&after.max(floor))
                        })
                    }),
                    None => value == 0 && floor <= track.minimum,
                }
        }
        // Only the existing SQL transaction clock stamps a native timestamp track.
        StateMutation::SetNow => track.minimum == 0 && track.maximum == i64::MAX,
    }
}
