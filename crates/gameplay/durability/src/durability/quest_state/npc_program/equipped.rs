//! Equipment admission borrows the existing locked equipment custody owner.
use super::*;
use crate::durability::character_equipment;

pub(super) async fn inventory(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    command: CommandRef,
    digest: [u8; 32],
) -> StoredResult<NpcInventorySnapshot> {
    let authority = character_equipment::assert_current_equipment_in_transaction(
        tx,
        recovery,
        node,
        &fence.item_fence(),
        Some(command),
        digest,
    )
    .await
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let equipment = character_equipment::read_equipment_in_transaction(tx, &authority)
        .await
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let mut inventory = NpcInventorySnapshot::default();
    for item in &equipment.items {
        // Durable GAME-ITEM-01 semantic slot codes, never protocol enum values.
        let slot = match item.slot {
            1 => "head",
            7 => "necklace",
            _ => continue,
        };
        inventory
            .equipped
            .insert(slot.into(), item.definition.production_key.clone());
    }
    Ok(inventory)
}

impl DurabilityRoot {
    /// Isolated engine fixture; retains digest admission and every production fence.
    #[cfg(feature = "test-support")]
    #[allow(
        clippy::too_many_arguments,
        reason = "fixture preserves the production source transaction inputs"
    )]
    pub async fn commit_fixture_npc_program<
        C: super::super::super::item_loop_move::ItemLoopContent,
    >(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &super::super::super::spell_owner_commit::SpellLanePermit,
        fence: CurrentCharacterGameplayFence,
        request: NpcProgramRequest,
        catalogue: Arc<QuestStateCatalogue>,
        items: &C,
        source: &NpcRuleCatalog,
    ) -> Result<NpcProgramCommit> {
        self.commit_npc_program_source(
            authority, node, permit, fence, request, catalogue, items, source, None,
        )
        .await
    }
}
