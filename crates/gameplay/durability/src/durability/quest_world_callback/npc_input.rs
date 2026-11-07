//! NPC hand-ins own carried inputs; ground mutation retains its spell-lane gate.
use super::{DurabilityError, Input, apply_input_rows};

pub(super) async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    _permit: &crate::durability::quest_state::npc_program::NpcItemPermit,
    input: &Input,
    transaction: &str,
) -> Result<(), DurabilityError> {
    if input.kind == "ground" {
        return Err(DurabilityError::InvalidStoredState);
    }
    apply_input_rows(tx, input, transaction).await
}
