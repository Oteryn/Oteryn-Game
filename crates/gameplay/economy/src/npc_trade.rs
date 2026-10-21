//! Pure NPC trade prices, denomination outputs and inventory plans.

use oteryn_gameplay_domain::domain::currency::{
    BACKPACK_ENTRIES_MAX, COIN_DEFINITION_FAMILY, COIN_STACK_MAXIMUM, Coin, CoinStack,
    FeePlanError, plan_fee_within,
};
use oteryn_gameplay_domain::domain::item_definition::{ItemDefinitionFacts, TypedDefinitionRef};

/// The largest BUY price: what 20 stacks of 100 crystal coins hold (NPC-0 §5.3).
pub const NPC_TRADE_BUY_GOLD_UNITS_MAX: u64 = 20_000_000 + 999_999_999_999;
/// The largest SELL value: 100 crystal, 99 platinum and 99 gold coins (NPC-0 §5.4).
pub const NPC_TRADE_SELL_GOLD_UNITS_MAX: u64 = 1_009_999;
/// Units of one trade: at most one stack (D82).
pub const NPC_TRADE_QUANTITY_MAX: u32 = 100;

/// The coins a SELL of `value` gold units mints: crystal, platinum, gold, each only when
/// positive (NPC-0 §5.4). `None` when a stack would exceed 100.
pub fn sell_coins(value: u64) -> Option<Vec<(Coin, u32)>> {
    let crystal = value / Coin::Crystal.worth();
    if crystal > u64::from(COIN_STACK_MAXIMUM) {
        return None;
    }
    Some(
        [
            (Coin::Crystal, crystal),
            (
                Coin::Platinum,
                value % Coin::Crystal.worth() / Coin::Platinum.worth(),
            ),
            (Coin::Gold, value % Coin::Platinum.worth()),
        ]
        .into_iter()
        .filter(|(_, quantity)| *quantity > 0)
        .map(|(coin, quantity)| (coin, u32::try_from(quantity).unwrap_or(u32::MAX)))
        .collect(),
    )
}

/// The change a BUY mints: `change / 100` platinum, then `change % 100` gold, each only when
/// positive (D175).
pub fn buy_change_coins(change: u64) -> Vec<(Coin, u32)> {
    [(Coin::Platinum, change / 100), (Coin::Gold, change % 100)]
        .into_iter()
        .filter(|(_, quantity)| *quantity > 0)
        .map(|(coin, quantity)| (coin, u32::try_from(quantity).unwrap_or(u32::MAX)))
        .collect()
}

/// The three coin definitions at their current compatible revisions: every live coin stack a BUY
/// pays from must be at its coin's revision, and change and proceeds are minted at them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcTradeCoins {
    pub gold: TypedDefinitionRef,
    pub platinum: TypedDefinitionRef,
    pub crystal: TypedDefinitionRef,
}

impl NpcTradeCoins {
    pub fn of(&self, coin: Coin) -> &TypedDefinitionRef {
        match coin {
            Coin::Gold => &self.gold,
            Coin::Platinum => &self.platinum,
            Coin::Crystal => &self.crystal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcTradeLineKind {
    CoinBurn,
    CoinMint,
    ItemBurn,
    ItemMint,
}

/// One line: a BURN keeps its entry while units are left; a MINT is a fresh stack in a new entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcTradeLine {
    pub kind: NpcTradeLineKind,
    pub item_instance_id: [u8; 16],
    pub placement_ordinal: u64,
    /// The coin of a coin line.
    pub coin: Option<Coin>,
    /// 0 for a MINT.
    pub quantity_before: u32,
    /// 0 for a whole BURN.
    pub quantity_after: u32,
}

/// One locked direct entry of the backpack, highest ordinal first.
#[derive(Clone)]
pub struct Entry {
    pub item_instance_id: [u8; 16],
    pub placement_ordinal: u64,
    pub definition: TypedDefinitionRef,
    pub quantity: u32,
    pub live: bool,
    /// Holds entries of its own or a timed state.
    pub not_default: bool,
}

/// The plain Content facts and fixed output identities needed by a trade plan.
pub struct PlanInput<'a> {
    pub item: &'a ItemDefinitionFacts,
    pub coins: &'a NpcTradeCoins,
    pub output_item_instance_ids: [[u8; 16]; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanError {
    InvalidInput,
    InvalidState,
    InsufficientFunds,
    CapacityExceeded,
    DoesNotFit,
    ItemNotFound,
}

/// The trade materialized from the locked before-state, before audit encoding.
pub struct Plan {
    pub lines: Vec<NpcTradeLine>,
    pub burned_gold_units: u64,
    pub minted_gold_units: u64,
    pub bank_debit_gold_units: u64,
    /// The bought or sold item.
    pub item_instance_id: [u8; 16],
}

/// The units one trade moves (§5.3/§5.4): a stackable offer `quantity x count` within the stack
/// maximum; a non-stackable offer exactly one item.
pub fn item_units(
    quantity: u32,
    offer_count: u32,
    maximum: u32,
    stackable: bool,
) -> Result<u32, PlanError> {
    let units = quantity
        .checked_mul(offer_count)
        .ok_or(PlanError::InvalidInput)?;
    if (!stackable && units != 1) || !(1..=maximum).contains(&units) {
        return Err(PlanError::InvalidInput);
    }
    Ok(units)
}

/// Checked price and quantity limits for a BUY or SELL.
pub fn trade_total(buy: bool, quantity: u32, unit_price: u64) -> Result<u64, PlanError> {
    let total = unit_price
        .checked_mul(u64::from(quantity))
        .ok_or(PlanError::InvalidInput)?;
    let total_max = if buy {
        NPC_TRADE_BUY_GOLD_UNITS_MAX
    } else {
        NPC_TRADE_SELL_GOLD_UNITS_MAX
    };
    if !(1..=NPC_TRADE_QUANTITY_MAX).contains(&quantity) || total > total_max {
        return Err(PlanError::InvalidInput);
    }
    Ok(total)
}

/// Lines of a BUY: the fee plan's BURN lines, the change MINT and the item MINT.
pub fn plan_buy(
    request: &PlanInput<'_>,
    total: u64,
    units: u32,
    capacity: usize,
    entries: &[Entry],
    bank_allowed: bool,
) -> Result<Plan, PlanError> {
    let mut inputs = Vec::new();
    // §5.3: a price of 0 pays from no coin, so no coin stack is an input or refused.
    for entry in entries.iter().filter(|entry| entry.live && total != 0) {
        let Some(coin) = Coin::from_production_key(&entry.definition.production_key) else {
            continue;
        };
        if entry.definition.family != COIN_DEFINITION_FAMILY {
            continue;
        }
        // An input is at the compatible revision; a stack at another one is not skipped (the
        // database plan guard counts every coin stack).
        if entry.definition.revision_ref != request.coins.of(coin).revision_ref {
            return Err(PlanError::InvalidInput);
        }
        // A burned coin has no contents and its default state; a coin stack that is not
        // default is refused rather than skipped (the database plan guard counts it).
        if entry.not_default {
            return Err(PlanError::InvalidInput);
        }
        inputs.push((
            CoinStack {
                coin,
                quantity: entry.quantity,
                placement_ordinal: entry.placement_ordinal,
            },
            entry.item_instance_id,
        ));
    }
    let stacks: Vec<CoinStack> = inputs.iter().map(|(stack, _)| *stack).collect();
    let eligible = stacks
        .iter()
        .try_fold(0_u64, |total, stack| {
            total.checked_add(u64::from(stack.quantity) * stack.coin.worth())
        })
        .ok_or(PlanError::InvalidState)?;
    let bank_debit_gold_units = match total.checked_sub(eligible) {
        None | Some(0) => 0,
        Some(_) if !bank_allowed => return Err(PlanError::InsufficientFunds),
        Some(rest) => rest,
    };
    let coin_part = total - bank_debit_gold_units;
    // §5.3: a price of 0 burns nothing and mints no change.
    let (burns, change) = if coin_part == 0 {
        (Vec::new(), 0)
    } else {
        match plan_fee_within(coin_part, &stacks, entries.len(), BACKPACK_ENTRIES_MAX) {
            Ok(plan) => (plan.lines, plan.change),
            Err(FeePlanError::InsufficientFunds) => return Err(PlanError::InsufficientFunds),
            Err(FeePlanError::CapacityExceeded) => return Err(PlanError::CapacityExceeded),
            Err(FeePlanError::ChangeDoesNotFit) => return Err(PlanError::DoesNotFit),
            Err(FeePlanError::InvalidInput) => {
                return Err(PlanError::InvalidState);
            }
        }
    };
    let mut lines: Vec<NpcTradeLine> = burns
        .iter()
        .map(|line| {
            let (stack, item) = &inputs[line.input];
            NpcTradeLine {
                kind: NpcTradeLineKind::CoinBurn,
                item_instance_id: *item,
                placement_ordinal: stack.placement_ordinal,
                coin: Some(stack.coin),
                quantity_before: stack.quantity,
                quantity_after: stack.quantity - line.burned,
            }
        })
        .collect();
    let whole = lines.iter().filter(|line| line.quantity_after == 0).count();
    let change_coins = buy_change_coins(change);
    // §5.3: the change and the item each take a new entry after the burn.
    if entries.len() - whole + change_coins.len() + 1 > capacity {
        return Err(PlanError::DoesNotFit);
    }
    let [platinum, gold, item] = request.output_item_instance_ids;
    let mut ordinal = first_mint_ordinal(entries)?;
    for (coin, quantity) in change_coins {
        lines.push(NpcTradeLine {
            kind: NpcTradeLineKind::CoinMint,
            item_instance_id: if coin == Coin::Platinum {
                platinum
            } else {
                gold
            },
            placement_ordinal: ordinal,
            coin: Some(coin),
            quantity_before: 0,
            quantity_after: quantity,
        });
        ordinal = ordinal.checked_add(1).ok_or(PlanError::DoesNotFit)?;
    }
    lines.push(NpcTradeLine {
        kind: NpcTradeLineKind::ItemMint,
        item_instance_id: item,
        placement_ordinal: ordinal,
        coin: None,
        quantity_before: 0,
        quantity_after: units,
    });
    Ok(Plan {
        lines,
        burned_gold_units: coin_part + change,
        bank_debit_gold_units,
        minted_gold_units: change,
        item_instance_id: item,
    })
}

/// Lines of a SELL: the item BURN and the coin MINT of the price.
pub fn plan_sell(
    request: &PlanInput<'_>,
    total: u64,
    units: u32,
    capacity: usize,
    entries: &[Entry],
) -> Result<Plan, PlanError> {
    let sold = entries
        .iter()
        .find(|entry| {
            entry.live
                && entry.definition == request.item.definition
                && entry.quantity >= units
                && !entry.not_default
        })
        .ok_or(PlanError::ItemNotFound)?;
    let coins = sell_coins(total).ok_or(PlanError::InvalidInput)?;
    let whole = sold.quantity == units;
    if entries.len() - usize::from(whole) + coins.len() > capacity {
        return Err(PlanError::DoesNotFit);
    }
    let mut lines = vec![NpcTradeLine {
        kind: NpcTradeLineKind::ItemBurn,
        item_instance_id: sold.item_instance_id,
        placement_ordinal: sold.placement_ordinal,
        coin: None,
        quantity_before: sold.quantity,
        quantity_after: sold.quantity - units,
    }];
    let [crystal, platinum, gold] = request.output_item_instance_ids;
    let mut ordinal = first_mint_ordinal(entries)?;
    for (coin, quantity) in coins {
        lines.push(NpcTradeLine {
            kind: NpcTradeLineKind::CoinMint,
            item_instance_id: match coin {
                Coin::Crystal => crystal,
                Coin::Platinum => platinum,
                Coin::Gold => gold,
            },
            placement_ordinal: ordinal,
            coin: Some(coin),
            quantity_before: 0,
            quantity_after: quantity,
        });
        ordinal = ordinal.checked_add(1).ok_or(PlanError::DoesNotFit)?;
    }
    Ok(Plan {
        bank_debit_gold_units: 0,
        lines,
        burned_gold_units: 0,
        minted_gold_units: total,
        item_instance_id: sold.item_instance_id,
    })
}

/// After the highest ordinal the backpack held before the trade (entries are highest first).
fn first_mint_ordinal(entries: &[Entry]) -> Result<u64, PlanError> {
    entries
        .first()
        .map_or(0, |entry| entry.placement_ordinal)
        .checked_add(1)
        .ok_or(PlanError::DoesNotFit)
}

#[cfg(test)]
#[path = "npc_trade_tests.rs"]
mod tests;
