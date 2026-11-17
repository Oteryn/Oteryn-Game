//! Advisory branch classification reads the same bounded custody as the writer.
use super::*;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_transfer::CurrentCharacterItemFence;

/// A current-fenced classification snapshot, never authority to apply an effect.
pub struct NpcProgramInventory {
    npc: String,
    fence: CurrentCharacterItemFence,
    snapshot: NpcInventorySnapshot,
}

impl NpcProgramInventory {
    /// Match classification data to the same NPC and current actor binding.
    pub fn for_fence(
        &self,
        npc: &str,
        fence: CurrentCharacterItemFence,
    ) -> Option<&NpcInventorySnapshot> {
        (self.npc == npc && self.fence == fence).then_some(&self.snapshot)
    }
}

/// These existing source participants cannot publish an optimistic reply/topic.
pub fn requires_current_inventory(request: &NpcProgramRequest, source: &NpcRuleCatalog) -> bool {
    hot_cuisine_m42::owns(request)
        || fathers_burden::owns(request)
        || marlin_trophy::owns(request)
        || source.outfit_exchange_keys(&request.npc).is_some()
        || source.item_exchange_keys(&request.npc).is_some()
        || source.equipped_storage_requirements(&request.npc).is_some()
}

/// Prefetch the supported owners before the runtime/NPC identity recheck.
pub fn inventory_prefetch_required(npc: &str, source: &NpcRuleCatalog) -> bool {
    source.outfit_exchange_keys(npc).is_some()
        || source.item_exchange_keys(npc).is_some()
        || source.equipped_storage_requirements(npc).is_some()
        || matches!(npc, "oteryn:npc.tereban" | "oteryn:npc.pemaret")
        || npc == oteryn_gameplay_quests::quest::hot_cuisine::NPC
}

impl DurabilityRoot {
    /// Resolve only the inventory/equipment facts needed to select a source
    /// branch. The existing receipt owner repeats all facts under its locks.
    #[allow(
        clippy::too_many_arguments,
        reason = "bounded classification borrows the existing NPC fence and custody inputs"
    )]
    pub async fn read_current_npc_program_inventory<C: ItemLoopContent>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        command: CommandRef,
        npc: &str,
        catalogue: &QuestStateCatalogue,
        content: &C,
    ) -> Result<NpcProgramInventory> {
        if command.game_session_id() != fence.game_session_id
            || fence.character_lease_generation == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let source = NpcRuleCatalog::runtime()
            .map_err(|_| CharacterProgressionError::InvalidInput)?
            .admitted_for_npc(npc, catalogue)
            .map_err(|_| CharacterProgressionError::InvalidInput)?
            .ok_or(CharacterProgressionError::InvalidInput)?;
        let plain = source
            .outfit_exchange_keys(npc)
            .or_else(|| source.item_exchange_keys(npc));
        let equipped = source.equipped_storage_requirements(npc).is_some();
        if !inventory_prefetch_required(npc, &source) {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let npc = npc.to_owned();
        let mut participant = content;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut participant, move |holder, deadline, content| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        return Ok(Err(error));
                    }
                    let snapshot = if equipped {
                        super::equipped::inventory(
                            &mut tx,
                            &recovery,
                            &node,
                            fence,
                            command,
                            content.loop_content_digest(),
                        )
                        .await?
                    } else if npc == oteryn_gameplay_quests::quest::hot_cuisine::NPC {
                        let keys = hot_cuisine_m42::keys()
                            .map_err(|_| DurabilityError::InvalidStoredState)?;
                        fathers_burden::inventory_for(&mut tx, fence, &keys).await?
                    } else if let Some(keys) = plain {
                        fathers_burden::inventory_for(&mut tx, fence, &keys).await?
                    } else if npc == "oteryn:npc.tereban" {
                        fathers_burden::inventory(&mut tx, fence).await?
                    } else if npc == "oteryn:npc.pemaret" {
                        fathers_burden::inventory_for(&mut tx, fence, &[marlin_trophy::FISH.into()])
                            .await?
                    } else {
                        return Ok(Err(CharacterProgressionError::InvalidInput));
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(NpcProgramInventory {
                        npc,
                        fence: fence.item_fence(),
                        snapshot,
                    }))
                })
            })
            .await?
    }
}
