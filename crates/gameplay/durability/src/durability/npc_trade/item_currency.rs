//! Crystal 201189f8's item-priced NPC trades, materialized over the locked shop backpack.
use super::*;

pub(in crate::durability) const STACKS_MAX: usize = 20;
pub(in crate::durability) const UNITS_MAX: u64 = 2_000;

pub struct ItemCurrencyPayment<'a> {
    pub currency: ItemDefinitionFacts,
    pub sell_all: bool,
    pub weights: &'a dyn super::super::item_transfer::ItemWeightLookup,
    pub yasir_position: Option<(u16, u16, i8)>,
}

impl DurabilityRoot {
    /// Settle an authored item price in the same fenced transaction and receipt as a gold trade.
    pub async fn commit_npc_item_currency_trade(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: NpcTradeRequest,
        payment: ItemCurrencyPayment<'_>,
    ) -> Result<NpcTradeOutcome> {
        let ItemCurrencyPayment { currency, sell_all, weights, yasir_position } = payment;
        let definition = &currency.definition;
        let (maximum, stackable) = stack_maximum(currency.stack)
            .map_err(|_| NpcTradeError::InvalidInput)?;
        if !stackable || !(1..=100).contains(&maximum)
            || currency.container_capacity.is_some()
            || definition.family != COIN_DEFINITION_FAMILY
            || is_coin(definition)
            || check_content_key(&definition.production_key).is_err()
            || check_content_key(&definition.revision_ref).is_err()
            || definition.production_key == request.item.definition.production_key
            || !(1..=UNITS_MAX).contains(&request.unit_price)
            || request.cause.side == NpcTradeSide::Exchange
            || request.cause.npc_key == yeo::NPC
            || super::super::house_hireling::npc_identity(&request.cause.npc_key).is_some()
            || yasir_position.is_some_and(|source| {
                request.cause.npc_key != YASIR || !(0..=15).contains(&source.2)
            })
        {
            return Err(NpcTradeError::InvalidInput);
        }
        self.commit_trade(
            authority,
            node,
            fence,
            request,
            TradeCommitOptions {
                sell_all,
                weights: Some(weights),
                yasir_position,
                currency: Some(currency),
                ..TradeCommitOptions::default()
            },
        )
        .await
    }
}

pub(super) fn bind(binding: Vec<u8>, currency: &TypedDefinitionRef) -> Vec<u8> {
    Sha256::new()
        .chain_update(b"oteryn:npc-trade-item-currency:v1\0")
        .chain_update(binding)
        .chain_update(currency.production_key.as_bytes())
        .chain_update([0])
        .chain_update(currency.revision_ref.as_bytes())
        .finalize()
        .to_vec()
}

pub(super) fn currency_line(side: NpcTradeSide, line: &NpcTradeLine) -> bool {
    matches!(
        (side, line.kind),
        (NpcTradeSide::Buy, NpcTradeLineKind::ItemBurn)
            | (NpcTradeSide::Sell, NpcTradeLineKind::ItemMint)
    )
}

fn burn_currency(
    currency: &ItemDefinitionFacts,
    entries: &[Entry],
    mut remaining: u32,
) -> Result<Vec<NpcTradeLine>> {
    let (maximum, _) = stack_maximum(currency.stack).map_err(|_| NpcTradeError::InvalidInput)?;
    let mut burns = Vec::new();
    for entry in entries.iter().filter(|entry| {
        entry.live && !entry.not_default && entry.definition == currency.definition
    }) {
        if !(1..=maximum).contains(&entry.quantity) {
            return Err(NpcTradeError::InvalidInput);
        }
        if burns.len() == STACKS_MAX {
            return Err(NpcTradeError::CapacityExceeded);
        }
        let used = remaining.min(entry.quantity);
        burns.push(NpcTradeLine {
            kind: NpcTradeLineKind::ItemBurn,
            item_instance_id: entry.item_instance_id,
            placement_ordinal: entry.placement_ordinal,
            coin: None,
            quantity_before: entry.quantity,
            quantity_after: entry.quantity - used,
        });
        remaining -= used;
        if remaining == 0 {
            return Ok(burns);
        }
    }
    Err(NpcTradeError::InsufficientFunds)
}

fn price_units(total: u64) -> Result<u32> {
    u32::try_from(total)
        .ok()
        .filter(|&units| (1..=UNITS_MAX).contains(&u64::from(units)))
        .ok_or(NpcTradeError::CapacityExceeded)
}

pub(super) fn plan(
    request: &NpcTradeRequest,
    currency: &ItemDefinitionFacts,
    priced: Option<(u64, u32)>,
    capacity: usize,
    entries: &[Entry],
) -> Result<(Plan, u64, u32)> {
    let input = plan_input(request);
    let (plan, units) = if request.cause.side == NpcTradeSide::Buy {
        let (total, units) = priced.ok_or(NpcTradeError::InvalidInput)?;
        let mut burns = burn_currency(currency, entries, price_units(total)?)?;
        let freed = burns.iter().filter(|line| line.quantity_after == 0).count();
        let mut purchase = rules::plan_buy(&input, 0, units, capacity + freed, entries, false)?;
        if purchase.lines.len() > STACKS_MAX {
            return Err(NpcTradeError::CapacityExceeded);
        }
        burns.append(&mut purchase.lines);
        purchase.lines = burns;
        (purchase, units)
    } else {
        let (mut sale, units, payout) = match priced {
            Some((total, units)) => (
                rules::plan_sell_for(&input, 0, units, capacity, entries)?,
                units,
                price_units(total)?,
            ),
            None => {
                let (plan, units) = rules::plan_sell_all(&input, 0, capacity, entries)?;
                (plan, units, price_units(request.unit_price * u64::from(units))?)
            }
        };
        if sale.lines.len() > STACKS_MAX {
            return Err(NpcTradeError::CapacityExceeded);
        }
        let freed = sale.lines.iter().filter(|line| line.quantity_after == 0).count();
        let payout_input = rules::PlanInput { item: currency, ..input };
        let mut mint = rules::plan_buy(&payout_input, 0, payout, capacity + freed, entries, false)?;
        if mint.lines.len() > STACKS_MAX
            || sale.lines.len() + mint.lines.len()
                > super::super::npc_trade_audit::SELL_ALL_RL01_TOUCHED_ITEM_INSTANCES_MAX as usize
        {
            return Err(NpcTradeError::CapacityExceeded);
        }
        sale.lines.append(&mut mint.lines);
        (sale, units)
    };
    // Currency lines use item custody and carry no gold or bank value.
    Ok((plan.into(), 0, units))
}

/// Check the net weight of both legs, including item payouts on a sale.
pub(super) fn fits(
    (mut carried, capacity): (u64, u64),
    request: &NpcTradeRequest,
    currency: &ItemDefinitionFacts,
    plan: &Plan,
) -> Result<()> {
    for line in &plan.lines {
        let item = if currency_line(request.cause.side, line) { currency } else { &request.item };
        let weight = u64::from(item.weight.ok_or(NpcTradeError::InvalidInput)?);
        carried = if line_is_burn(line.kind) {
            carried.checked_sub(weight * u64::from(line.quantity_before - line.quantity_after))
        } else {
            carried.checked_add(weight * u64::from(line.quantity_after))
        }
        .ok_or(NpcTradeError::CapacityExceeded)?;
    }
    if carried > capacity {
        return Err(NpcTradeError::DoesNotFit);
    }
    Ok(())
}
