//! Crystal 201189f8, data-global/npc/sinatuki.lua:54-75.
use super::*;

pub const NPC: &str = "oteryn:npc.sinatuki";
pub const SHARD: &str = "oteryn:item.tibia.i7290";
const FISH: [&str; 2] = ["oteryn:item.tibia.i7158", "oteryn:item.tibia.i7159"];
pub const REWARD: u32 = 5;
// The fixed NPC exchange contract allows three input burn lines.
const MAX_INPUT_STACKS: usize = 3;

fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    }
}

pub(super) fn validate(request: &NpcTradeRequest, sell_all: bool) -> Result<()> {
    if sell_all
        || request.fluid_subtype.is_some()
        || request.full_charges.is_some()
        || request.cause.catalogue_revision == 0
        || request.cause.offer_index != 0
        || request.quantity != 1
        || request.unit_price != 0
        || request.offer_count != REWARD
        || request.item.definition != definition(SHARD)
        || request.item.container_capacity.is_some()
    {
        return Err(NpcTradeError::InvalidInput);
    }
    Ok(())
}

pub(super) async fn permitted(
    tx: &mut Type2Transaction<'_>,
    character: CharacterId,
) -> std::result::Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT game_sinatuki_exchange_permitted(encode($1,'hex')::uuid)")
        .bind(character.as_bytes().as_slice())
        .fetch_one(&mut ***tx)
        .await
}

pub(super) fn plan(
    request: &NpcTradeRequest,
    capacity: usize,
    entries: &[Entry],
) -> Result<(Plan, TypedDefinitionRef, u32)> {
    // The donor tries rainbow trout first, including its 99-fish fallback.
    let (fish, count, stacks) = FISH
        .into_iter()
        .map(definition)
        .find_map(|fish| {
            let mut stacks = entries
                .iter()
                .filter(|e| eligible(e, &fish))
                .collect::<Vec<_>>();
            stacks.sort_by_key(|entry| std::cmp::Reverse(entry.quantity));
            stacks.truncate(MAX_INPUT_STACKS);
            let available: u32 = stacks
                .iter()
                .map(|entry| entry.quantity.min(100))
                .sum();
            (available >= 99).then_some((fish, available.min(100), stacks))
        })
        .ok_or(NpcTradeError::ItemNotFound)?;
    let mut remaining = count;
    let mut freed = 0;
    let mut lines = Vec::new();
    for entry in stacks {
        let taken = remaining.min(entry.quantity);
        let after = entry.quantity - taken;
        freed += usize::from(after == 0);
        lines.push(NpcTradeLine {
            kind: NpcTradeLineKind::ItemBurn,
            item_instance_id: entry.item_instance_id,
            placement_ordinal: entry.placement_ordinal,
            coin: None,
            quantity_before: entry.quantity,
            quantity_after: after,
        });
        remaining -= taken;
        if remaining == 0 {
            break;
        }
    }
    if entries.len() - freed >= capacity {
        return Err(NpcTradeError::DoesNotFit);
    }
    let ordinal = entries
        .iter()
        .map(|e| e.placement_ordinal)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(NpcTradeError::CapacityExceeded)?;
    let output = request.output_item_instance_ids[0];
    lines.push(NpcTradeLine {
        kind: NpcTradeLineKind::ItemMint,
        item_instance_id: output,
        placement_ordinal: ordinal,
        coin: None,
        quantity_before: 0,
        quantity_after: REWARD,
    });
    Ok((
        Plan {
            lines,
            burned_gold_units: 0,
            minted_gold_units: 0,
            bank_debit_gold_units: 0,
            bank_debit: None,
            item_instance_id: output,
        },
        fish,
        count,
    ))
}

fn eligible(entry: &Entry, fish: &TypedDefinitionRef) -> bool {
    entry.live && !entry.not_default && entry.quantity > 0 && entry.definition == *fish
}

impl DurabilityRoot {
    /// Exchange 99 or 100 fish of one species for five shards.
    pub async fn exchange_sinatuki_fish(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: NpcTradeRequest,
        weights: &dyn super::super::item_transfer::ItemWeightLookup,
    ) -> Result<NpcTradeOutcome> {
        if request.cause.side != NpcTradeSide::Exchange || request.cause.npc_key != NPC {
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

pub(in crate::durability) fn audit(
    value: &OneItemNpcTradeV1,
) -> std::result::Result<(), AuditError> {
    let cause = value.cause.as_ref().ok_or(AuditError::InvalidInput)?;
    let (give, burns) = value.lines.split_last().ok_or(AuditError::InvalidInput)?;
    let output = give.after.as_ref().ok_or(AuditError::InvalidInput)?;
    let matches = |state: &OneItemStateV1, key: &str| {
        state.definition.as_ref().is_some_and(|d| {
            d.family == "Item" && d.production_key == key && d.revision_ref == "definition-r1"
        })
    };
    let mut count = 0_u32;
    let mut fish = None;
    for line in burns {
        let before = line.before.as_ref().ok_or(AuditError::InvalidInput)?;
        let after = line.after.as_ref().ok_or(AuditError::InvalidInput)?;
        let key = FISH
            .into_iter()
            .find(|key| matches(before, key))
            .ok_or(AuditError::InvalidInput)?;
        if line.kind != LINE_ITEM_BURN || fish.is_some_and(|previous| previous != key) {
            return Err(AuditError::InvalidInput);
        }
        fish = Some(key);
        count = count
            .checked_add(
                before
                    .quantity
                    .checked_sub(after.quantity)
                    .ok_or(AuditError::InvalidInput)?,
            )
            .ok_or(AuditError::InvalidInput)?;
    }
    if cause.catalogue_revision == 0
        || cause.offer_index != 0
        || value.quantity != 1
        || value.unit_price != 0
        || value.sell_all
        || value.bank_debit.is_some()
        || value.house_key.is_some()
        || value.hireling_package.is_some()
        || value.house_furniture_kit.is_some()
        || value.fluid_subtype.is_some()
        || value.full_charges.is_some()
        || burns.len() > MAX_INPUT_STACKS
        || !matches!(count, 99 | 100)
        || give.kind != LINE_ITEM_MINT
        || output.quantity != REWARD
        || !matches(output, SHARD)
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}
