//! World callbacks share the existing transaction-clock state language.
use super::{Callback, QuestEffect, QuestEffectKind, QuestTrack};
use crate::quest::reward_callbacks::{StateCondition, StateMutation, StateOperation};

pub(super) fn mutates(operation: &StateOperation) -> bool {
    !matches!(operation.effect, StateMutation::Add { value: 0 })
}
pub(super) fn refreshes_clock(callback: &Callback) -> bool {
    callback.state_operations.iter().any(|operation| {
        matches!(operation.from, StateCondition::ElapsedAtLeast { seconds } if seconds > 0)
            && matches!(operation.effect, StateMutation::SetNow)
    })
}
pub(super) fn valid(callback: &Callback) -> bool {
    callback.state_operations.iter().all(StateOperation::valid)
        && (callback.state_operations.is_empty() || callback.native_rule.is_none())
}
pub(super) fn extend_tracks(
    callback: &Callback,
    tracks: &mut Vec<QuestTrack>,
) -> Result<(), String> {
    for operation in &callback.state_operations {
        if let Some(track) = tracks.iter().find(|track| track.key == operation.track) {
            if track.quest != callback.quest {
                return Err("world clock track owner mismatch".into());
            }
        } else {
            tracks.push(QuestTrack {
                key: operation.track.clone(),
                quest: callback.quest.clone(),
                initial: -1,
                min: -1,
                max: i64::MAX,
            });
        }
    }
    Ok(())
}
pub(super) fn append_effects(
    callback: &Callback,
    mut binding: Option<[u8; 32]>,
    effects: &mut Vec<QuestEffect>,
) {
    for operation in &callback.state_operations {
        let mut effect = operation.quest_effect();
        if mutates(operation)
            && let Some(value) = binding.take()
        {
            effect.effect = match operation.effect {
                StateMutation::SetNow => QuestEffectKind::SetNowWithBinding { binding: value },
                StateMutation::Add { value: delta } => QuestEffectKind::AddWithBinding {
                    delta,
                    binding: value,
                },
            };
        }
        effects.push(effect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quest::{QuestComparison, admission_binding};

    #[test]
    fn elapsed_clock_binding_cannot_be_stripped_or_retargeted() -> Result<(), String> {
        let mut callback = crate::quest::rotten_blood::altar_callback();
        callback.key = "oteryn:quest-callback/clock-unit".into();
        callback.storage.clear();
        callback.native_rule = None;
        callback.requires_world_admission = true;
        callback.state_operations = vec![StateOperation {
            track: "oteryn:quest-progress/clock-unit/at".into(),
            from: StateCondition::ElapsedAtLeast { seconds: 1_800_000 },
            effect: StateMutation::SetNow,
        }];
        let transition = super::super::primary_transition(&callback)?;
        assert_eq!(
            transition.effects[0].from,
            QuestComparison::ElapsedAtLeast(1_800_000)
        );
        assert!(matches!(
            transition.effects[0].effect,
            QuestEffectKind::SetNowWithBinding { .. }
        ));
        assert!(
            admission_binding::validate(&transition, true).is_err(),
            "an unregistered physical operation must not become a generic write"
        );
        let mut encoded = Vec::new();
        transition.effects[0].effect.encode(&mut encoded);
        assert_eq!(encoded[0], 12);
        let mut conditional = Vec::new();
        QuestEffectKind::SetWhenAtLeast {
            other_effect: 1,
            minimum: 1,
            value: 1,
        }
        .encode(&mut conditional);
        assert_eq!(conditional[0], 11);
        let mut moved = callback.clone();
        moved.selector.position = Some(super::super::Position {
            x: 1,
            y: 2,
            floor: 0,
        });
        assert_ne!(transition, super::super::primary_transition(&moved)?);
        callback.state_operations[0].from = StateCondition::ElapsedAtLeast { seconds: 0 };
        assert!(
            !refreshes_clock(&callback),
            "zero-time predicates must not authorize repeatable MINT"
        );
        callback.state_operations[0].from = StateCondition::Any;
        assert!(!refreshes_clock(&callback));
        Ok(())
    }
}
