//! Al Dee's item-only exchange, on the existing NPC trade writer (CRAFT-1 C8).
use super::*;

pub(super) fn input() -> &'static TypedDefinitionRef {
    static INPUT: std::sync::OnceLock<TypedDefinitionRef> = std::sync::OnceLock::new();
    INPUT.get_or_init(|| TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i3462".into(),
        revision_ref: "definition-r1".into(),
    })
}

pub(super) fn output() -> &'static TypedDefinitionRef {
    static OUTPUT: std::sync::OnceLock<TypedDefinitionRef> = std::sync::OnceLock::new();
    OUTPUT.get_or_init(|| TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i3456".into(),
        revision_ref: "definition-r1".into(),
    })
}

pub(super) fn validate(request: &NpcTradeRequest, sell_all: bool) -> Result<()> {
    if sell_all
        || request.cause.npc_key != "oteryn:npc.al_dee"
        || request.cause.catalogue_revision == 0
        || request.cause.offer_index != 0
        || request.quantity != 1
        || request.unit_price != 0
        || request.offer_count != 1
        || request.item.definition.family != "Item"
        || request.item.definition.production_key != "oteryn:item.tibia.i3456"
        || request.item.definition.revision_ref != "definition-r1"
        || request.item.container_capacity.is_some()
    {
        return Err(NpcTradeError::InvalidInput);
    }
    Ok(())
}

pub(super) fn plan(request: &NpcTradeRequest, capacity: usize, entries: &[Entry]) -> Result<Plan> {
    plan_with_input(request, capacity, entries, input())
}

pub(super) fn plan_with_input(
    request: &NpcTradeRequest,
    capacity: usize,
    entries: &[Entry],
    input: &TypedDefinitionRef,
) -> Result<Plan> {
    let source = entries
        .iter()
        .find(|entry| {
            entry.live && !entry.not_default && entry.quantity == 1 && entry.definition == *input
        })
        .ok_or(NpcTradeError::ItemNotFound)?;
    if entries.len() > capacity {
        return Err(NpcTradeError::DoesNotFit);
    }
    let ordinal = entries
        .iter()
        .map(|entry| entry.placement_ordinal)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(NpcTradeError::CapacityExceeded)?;
    let output = request.output_item_instance_ids[0];
    Ok(Plan {
        lines: vec![
            NpcTradeLine {
                kind: NpcTradeLineKind::ItemBurn,
                item_instance_id: source.item_instance_id,
                placement_ordinal: source.placement_ordinal,
                coin: None,
                quantity_before: 1,
                quantity_after: 0,
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

impl DurabilityRoot {
    /// Fixed Al Dee exchange, using the existing NPC cause, item fence and audit transaction.
    pub async fn exchange_small_axe_for_pick(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: NpcTradeRequest,
        weights: &dyn super::super::item_transfer::ItemWeightLookup,
    ) -> Result<NpcTradeOutcome> {
        if request.cause.side != NpcTradeSide::Exchange || validate(&request, false).is_err() {
            return Err(NpcTradeError::InvalidInput);
        }
        self.commit_trade(
            authority,
            node,
            fence,
            request,
            TradeCommitOptions {
                weights: Some(weights),
                ..TradeCommitOptions::default()
            },
        )
        .await
    }
}

pub(super) async fn fits(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    weights: Option<&dyn super::super::item_transfer::ItemWeightLookup>,
    take: &[&TypedDefinitionRef],
    give: &TypedDefinitionRef,
) -> std::result::Result<bool, DurabilityError> {
    let weights = weights.ok_or(DurabilityError::InvalidStoredState)?;
    let weight = |definition: &TypedDefinitionRef| {
        weights
            .weight_of(definition)
            .map(u64::from)
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    let carried = super::super::item_loop_move::carried_weight(
        tx,
        fence.character_id,
        *fence.runtime_scope.world_id().as_bytes(),
        weights,
    )
    .await
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let capacity = super::super::item_transfer::read_derived_transfer_capacity_centi_oz(
        tx,
        fence.character_id,
        weights,
    )
    .await?;
    let mut taken = 0_u64;
    for definition in take {
        taken = taken
            .checked_add(weight(definition)?)
            .ok_or(DurabilityError::InvalidStoredState)?;
    }
    Ok(carried
        .checked_sub(taken)
        .and_then(|w| w.checked_add(weight(give).ok()?))
        .is_some_and(|w| w <= capacity))
}
