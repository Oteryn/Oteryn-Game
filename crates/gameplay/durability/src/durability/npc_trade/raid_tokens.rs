//! CRAFT1 C8: Jorge's one-stack token take and fresh replica give share NPC-TRADE.
use super::super::item_transfer::ItemWeightLookup;
use super::*;
use oteryn_gameplay_quests::quest::raid_tokens as source;

pub(super) fn validate(request: &NpcTradeRequest, token: &ItemDefinitionFacts) -> Result<()> {
    let offers = source::offers().map_err(|_| NpcTradeError::InvalidInput)?;
    let Some((item, _)) = offers.get(request.cause.offer_index as usize) else {
        return Err(NpcTradeError::InvalidInput);
    };
    super::super::item_transfer::validate_facts(token).map_err(|_| NpcTradeError::InvalidInput)?;
    super::super::item_transfer::validate_facts(&request.item)
        .map_err(|_| NpcTradeError::InvalidInput)?;
    if request.cause.side != NpcTradeSide::Exchange
        || request.cause.npc_key != source::NPC
        || request.cause.catalogue_revision != 1
        || request.quantity != 1
        || request.offer_count != 1
        || request.unit_price != 0
        || token.definition.family != "Item"
        || token.definition.production_key != format!("oteryn:item.tibia.i{}", source::TOKEN)
        || token.container_capacity.is_some()
        || token.weight.is_none()
        || stack_maximum(token.stack)
            .map_err(|_| NpcTradeError::InvalidInput)?
            .0
            < source::COST
        || request.item.definition.family != "Item"
        || request.item.definition.production_key != format!("oteryn:item.tibia.i{item}")
        || request.item.container_capacity.is_some()
        || request.item.weight.is_none()
    {
        return Err(NpcTradeError::InvalidInput);
    }
    Ok(())
}

pub(super) fn bind(previous: Vec<u8>, token: &ItemDefinitionFacts) -> Vec<u8> {
    let mut hash = Sha256::new();
    hash.update(previous);
    hash.update(b"oteryn:jorge-token-exchange:v1\0");
    for field in [
        &token.definition.family,
        &token.definition.production_key,
        &token.definition.revision_ref,
    ] {
        hash.update((field.len() as u64).to_be_bytes());
        hash.update(field.as_bytes());
    }
    hash.update(source::COST.to_be_bytes());
    hash.finalize().to_vec()
}

pub(super) fn plan(
    request: &NpcTradeRequest,
    token: &ItemDefinitionFacts,
    capacity: usize,
    entries: &[Entry],
) -> Result<Plan> {
    let input = entries
        .iter()
        .find(|entry| {
            entry.live
                && !entry.not_default
                && entry.definition == token.definition
                && entry.quantity >= source::COST
        })
        .ok_or(NpcTradeError::InsufficientFunds)?;
    let after = input.quantity - source::COST;
    if entries.len() + usize::from(after != 0) > capacity {
        return Err(NpcTradeError::DoesNotFit);
    }
    let ordinal = entries
        .iter()
        .map(|entry| entry.placement_ordinal)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(NpcTradeError::CapacityExceeded)?;
    let output = request.output_item_instance_ids[2];
    Ok(Plan {
        lines: vec![
            NpcTradeLine {
                kind: NpcTradeLineKind::ItemBurn,
                item_instance_id: input.item_instance_id,
                placement_ordinal: input.placement_ordinal,
                coin: None,
                quantity_before: input.quantity,
                quantity_after: after,
            },
            NpcTradeLine {
                kind: NpcTradeLineKind::ItemMint,
                item_instance_id: output,
                placement_ordinal: ordinal,
                coin: None,
                quantity_before: 0,
                quantity_after: 1,
            },
        ],
        burned_gold_units: 0,
        minted_gold_units: 0,
        bank_debit_gold_units: 0,
        bank_debit: None,
        item_instance_id: output,
    })
}

pub(super) fn audit(message: &mut OneItemNpcTradeV1, token: &ItemDefinitionFacts) {
    for line in &mut message.lines {
        if line.kind == LINE_ITEM_BURN {
            for state in [&mut line.before, &mut line.after].into_iter().flatten() {
                state.definition = Some(OneItemTypedDefinitionRevisionV1 {
                    family: token.definition.family.clone(),
                    production_key: token.definition.production_key.clone(),
                    revision_ref: token.definition.revision_ref.clone(),
                });
            }
        }
    }
}

pub(super) async fn weight(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterItemFence,
    request: &NpcTradeRequest,
    token: &ItemDefinitionFacts,
    weights: Option<&dyn ItemWeightLookup>,
) -> Result<()> {
    let weights = weights.ok_or(NpcTradeError::InvalidInput)?;
    let carried = super::super::item_loop_move::carried_weight(
        tx,
        fence.character_id,
        *fence.runtime_scope.world_id().as_bytes(),
        weights,
    )
    .await
    .map_err(|_| NpcTradeError::CapacityExceeded)?;
    let capacity = super::super::item_transfer::read_derived_transfer_capacity_centi_oz(
        tx,
        fence.character_id,
        weights,
    )
    .await?;
    let removed =
        u64::from(token.weight.ok_or(NpcTradeError::InvalidInput)?) * u64::from(source::COST);
    let added = u64::from(request.item.weight.ok_or(NpcTradeError::InvalidInput)?);
    if carried
        .checked_sub(removed)
        .and_then(|n| n.checked_add(added))
        .is_none_or(|n| n > capacity)
    {
        return Err(NpcTradeError::DoesNotFit);
    }
    Ok(())
}

impl DurabilityRoot {
    /// Jorge's source-bound exchange; the existing NPC cause owns both item lines.
    pub async fn commit_raid_token_exchange(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: NpcTradeRequest,
        token: ItemDefinitionFacts,
        weights: &dyn ItemWeightLookup,
    ) -> Result<NpcTradeOutcome> {
        self.commit_trade(
            authority,
            node,
            fence,
            request,
            TradeCommitOptions {
                exchange: Some(token),
                weights: Some(weights),
                ..TradeCommitOptions::default()
            },
        )
        .await
    }
}
