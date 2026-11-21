//! Bind paid timestamp terms without changing the transaction-clock operation.
use super::{QuestEffect, QuestEffectKind, StateOperation};

pub(super) fn stamp(operation: &StateOperation, binding: Option<[u8; 32]>) -> QuestEffect {
    let mut effect = operation.quest_effect();
    if let Some(binding) = binding
        && matches!(effect.effect, QuestEffectKind::SetNow)
    {
        effect.effect = QuestEffectKind::SetNowWithBinding { binding };
    }
    effect
}
