//! Rookgaard barter shares the craft debit/mint/audit transaction and its receipt.
//! A probe precedes inventory and conversation lookup: consumed notebooks remain replayable.
use super::*;
use oteryn_gameplay_quests::quest::rookgaard_exchanges::exchange;
use oteryn_protocol_oteryn::npc_service::decode_npc_talk_intent;
use serde::{Deserialize, Serialize};

mod sinatuki;

type StoredExchangeReceipt = (Vec<u8>, Vec<u8>, Option<Value>, bool);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcQuestExchange {
    pub npc_key: String,
    pub npc_identity: [u8; 16],
    pub npc_generation: u64,
    pub catalogue_revision: u64,
    pub conversation_revision: u64,
}

impl DurabilityRoot {
    /// One command either consumes a source unit and mints its exact reward, or
    /// writes nothing. Historical receipts never reacquire current write authority.
    // The owner boundary carries permit, recovery, node, fence and content proofs separately.
    #[allow(clippy::too_many_arguments)]
    pub async fn exchange_npc_quest_item<C: ItemLoopContent>(
        &self,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        intent: Option<Vec<u8>>,
        binding: Option<NpcQuestExchange>,
        native: &NativeGameplayState,
        content: &C,
    ) -> Result<Option<(NpcQuestExchange, UseDisposition, bool)>, DurabilityError> {
        if command.game_session_id() != fence.game_session_id {
            return Err(DurabilityError::InvalidStoredState);
        }
        permit.check_scope(fence.runtime_scope)?;
        let recovery = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (native, content);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            super::super::character_authority::assert_recovery_fence(&mut tx, &recovery).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:item-craft:' || encode($1,'hex') || ':' || $2,0))")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
                .execute(&mut **tx).await?;
            let previous: Option<StoredExchangeReceipt> = sqlx::query_as(
                "SELECT intent,uuid_send(character_id),npc_exchange,jsonb_array_length(outputs)>0 FROM game_item_craft_records WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
                .fetch_optional(&mut **tx).await?;
            if let Some((original, character, stored, rewarded)) = previous {
                let Some(stored) = stored else { tx.commit(deadline).await?; return Ok(None); };
                let stored: NpcQuestExchange = serde_json::from_value(stored).map_err(|_| DurabilityError::InvalidStoredState)?;
                let disposition = if character == fence.character_id.as_bytes()
                    && intent.as_ref().is_none_or(|wire| *wire == original)
                    && binding.as_ref().is_none_or(|expected| *expected == stored) {
                    UseDisposition::Committed
                } else { UseDisposition::Rejected };
                tx.commit(deadline).await?;
                return Ok(Some((stored, disposition, rewarded)));
            }
            let (Some(binding), Some(intent)) = (binding.as_ref(), intent.as_ref()) else {
                tx.commit(deadline).await?;
                return Ok(None);
            };
            let Some(rule) = exchange(&binding.npc_key) else { return Err(DurabilityError::InvalidStoredState); };
            let decoded = decode_npc_talk_intent(intent).map_err(|_| DurabilityError::InvalidStoredState)?;
            if decoded.npc_actor.identity != binding.npc_identity || decoded.npc_actor.generation != binding.npc_generation
                || binding.npc_generation == 0 || binding.catalogue_revision == 0 || binding.conversation_revision == 0
                || !rule.confirmed_by(&decoded.text) || context.1.loop_content_digest() != context.0.source_digest() {
                return Ok(Some((binding.clone(), UseDisposition::Rejected, false)));
            }
            if assert_current_equipment_in_transaction(&mut tx, &recovery, &node, &fence, Some(command), context.1.loop_content_digest()).await.is_err() {
                return Ok(Some((binding.clone(), UseDisposition::Rejected, false)));
            }
            let inventory = super::super::item_use::read_use_inventory(&mut tx, &fence, String::new(), String::new())
                .await.map_err(|_| DurabilityError::InvalidStoredState)?;
            let source = inventory.backpack.iter().flat_map(|bag| &bag.entries)
                .chain(inventory.nested.iter().map(|(_, entry)| entry))
                .find(|entry| rule.accepts_input(&entry.item.definition.production_key) && entry.item.quantity > 0);
            let Some(source) = source else { return Ok(Some((binding.clone(), UseDisposition::NothingToUse, false))); };
            let request = ItemCraftRequest {
                command, source: source.item.item_instance_id, tool: None, ingredient: None,
                station: None, intent: intent.clone(), gathering: None, enchanting: None,
            };
            let disposition = write(&mut tx, fence, &request, context.0, context.1, Some(binding)).await?;
            let rewarded = if disposition == UseDisposition::Committed {
                let rewarded: bool = sqlx::query_scalar("SELECT jsonb_array_length(outputs)>0 FROM game_item_craft_records WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                    .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_one(&mut **tx).await?;
                tx.commit(deadline).await?;
                rewarded
            } else { false };
            Ok(Some((binding.clone(), disposition, rewarded)))
        })).await
    }
}

pub(super) async fn conversion(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    binding: &NpcQuestExchange,
    native: &NativeGameplayState,
    definition: &TypedDefinitionRef,
) -> Result<Option<oteryn_gameplay_items::item_craft::CookingConversion>, DurabilityError> {
    if binding.npc_key == "oteryn:npc.sinatuki" {
        return sinatuki::conversion(tx, fence, native, definition).await;
    }
    Ok(leaf_conversion(binding, native, definition))
}

fn leaf_conversion(
    binding: &NpcQuestExchange,
    native: &NativeGameplayState,
    definition: &TypedDefinitionRef,
) -> Option<oteryn_gameplay_items::item_craft::CookingConversion> {
    let rule = exchange(&binding.npc_key)?;
    if definition.family != "Item"
        || definition.production_key != rule.input
        || definition.revision_ref != "definition-r1"
    {
        return None;
    }
    let output = native.item_policy_for_source_id(rule.output)?.record();
    let name = binding.npc_key.strip_prefix("oteryn:npc.")?;
    output
        .admission
        .materializable
        .then(|| oteryn_gameplay_items::item_craft::CookingConversion {
            recipe: format!("oteryn:npc-quest-exchange.{name}"),
            outputs: vec![(
                TypedDefinitionRef {
                    family: output.production_definition.family.clone(),
                    production_key: output.production_definition.production_key.clone(),
                    revision_ref: output.production_definition.revision_ref.clone(),
                },
                1,
            )],
        })
}
