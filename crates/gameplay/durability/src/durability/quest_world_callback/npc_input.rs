//! NPC hand-ins own carried inputs; ground mutation retains its spell-lane gate.
use super::{DurabilityError, Input};

pub(super) async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    _permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    fence: super::CurrentCharacterGameplayFence,
    input: &Input,
    transaction: &str,
) -> Result<(), DurabilityError> {
    if input.kind == "ground" {
        return Err(DurabilityError::InvalidStoredState);
    }
    super::apply_input(tx, _permit, fence, input, transaction).await
}
