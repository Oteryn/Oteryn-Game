//! Keep carried NPC consumption behind its own fenced owner, without a ground-write permit.
use super::*;

pub(super) async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    _permit: &super::super::quest_state::npc_program::NpcItemPermit,
    input: &Input,
    transaction: &str,
) -> Result<(), DurabilityError> {
    // Ground consumption requires the spell owner's channel permit.
    location_queries(&input.kind)?;
    mutate_input(tx, input, transaction).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn npc_carried_owner_cannot_authorize_ground_mutations() {
        assert!(super::location_queries("ground").is_err());
        for kind in ["backpack", "container", "nested", "equipment"] {
            assert!(super::location_queries(kind).is_ok(), "{kind}");
        }
    }
}
