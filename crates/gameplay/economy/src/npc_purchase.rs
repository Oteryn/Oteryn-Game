//! Crystal NPC purchases: complete stacks, twenty-slot shopping bags, and atomic overflow.

pub const SHOPPING_BAG_ITEM_ID: u32 = 21411;
pub const SHOPPING_BAG_SLOTS: u32 = 20;
pub const SHOPPING_BAG_PRICE: u64 = 20;
pub const TILE_LIMIT: u32 = 30;
pub const MAX_PURCHASE_UNITS: u32 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurchaseRefusal {
    InvalidInput,
    InsufficientFunds,
    Capacity,
    Room,
}
impl PurchaseRefusal {
    pub const fn message(self) -> &'static str {
        match self {
            Self::Capacity => "You do not have enough capacity.",
            Self::Room => "There is not enough room.",
            Self::InsufficientFunds => "You do not have enough money.",
            Self::InvalidInput => "Sorry, not possible.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PurchaseInput {
    pub amount: u32,
    pub stack_size: u32,
    pub unit_price: u64,
    pub in_backpacks: bool,
    pub ignore_cap: bool,
    pub free_slots: u32,
    pub tile_items: u32,
    pub free_capacity: u64,
    pub item_weight: u64,
    pub bag_weight: u64,
    pub funds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchasePlan {
    /// Quantities of the bought stacks, in creation order.
    pub stacks: Vec<u32>,
    pub bags: u32,
    /// Number of top-level outputs carried.
    pub carried: u32,
    /// A later, smaller stack or bag may fit even when an earlier one overflowed.
    pub carried_roots: Vec<bool>,
    pub total: u64,
}

pub fn plan_purchase(input: PurchaseInput) -> Result<PurchasePlan, PurchaseRefusal> {
    use PurchaseRefusal as Error;
    if input.amount == 0 || input.amount > MAX_PURCHASE_UNITS || input.stack_size == 0 {
        return Err(Error::InvalidInput);
    }
    let slots = input.amount.div_ceil(input.stack_size);
    let bags = if input.in_backpacks {
        slots.div_ceil(SHOPPING_BAG_SLOTS)
    } else {
        0
    };
    let total = input
        .unit_price
        .checked_mul(u64::from(input.amount))
        .and_then(|price| price.checked_add(u64::from(bags) * SHOPPING_BAG_PRICE))
        .ok_or(Error::InvalidInput)?;
    if total > input.funds {
        return Err(Error::InsufficientFunds);
    }
    let roots = if bags > 0 { bags } else { slots };
    let mut stacks = Vec::new();
    let mut remaining = input.amount;
    while remaining > 0 {
        let count = remaining.min(input.stack_size);
        stacks.push(count);
        remaining -= count;
    }
    let mut free_capacity = input.free_capacity;
    let mut carried = 0;
    let mut carried_roots = Vec::new();
    for root in 0..roots {
        let start = if bags > 0 {
            root * SHOPPING_BAG_SLOTS
        } else {
            root
        };
        let end = if bags > 0 {
            (start + SHOPPING_BAG_SLOTS).min(slots)
        } else {
            start + 1
        };
        let units: u64 = stacks[start as usize..end as usize]
            .iter()
            .map(|n| u64::from(*n))
            .sum();
        let weight = units
            .checked_mul(input.item_weight)
            .and_then(|weight| weight.checked_add(if bags > 0 { input.bag_weight } else { 0 }))
            .ok_or(Error::InvalidInput)?;
        if carried >= input.free_slots {
            if !input.ignore_cap {
                return Err(Error::Room);
            }
            carried_roots.push(false);
            continue;
        }
        if weight > free_capacity {
            if !input.ignore_cap {
                return Err(Error::Capacity);
            }
            carried_roots.push(false);
            continue;
        }
        free_capacity -= weight;
        carried += 1;
        carried_roots.push(true);
    }
    let overflow = roots - carried;
    if overflow > 0
        && input
            .tile_items
            .checked_add(overflow)
            .is_none_or(|n| n > TILE_LIMIT)
    {
        return Err(Error::Room);
    }
    Ok(PurchasePlan {
        stacks,
        bags,
        carried,
        carried_roots,
        total,
    })
}

pub fn purchase_message(amount: u32, name: &str, bags: u32, total: u64) -> String {
    let coins = if total == 1 {
        "gold coin"
    } else {
        "gold coins"
    };
    if bags == 0 {
        format!("Bought {amount}x {name} for {total} {coins}.")
    } else {
        let bag_name = if bags == 1 {
            "shopping bag"
        } else {
            "shopping bags"
        };
        format!("Bought {amount}x {name} and {bags} {bag_name} for {total} {coins}.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> PurchaseInput {
        PurchaseInput {
            amount: 45,
            stack_size: 1,
            unit_price: 2,
            in_backpacks: true,
            ignore_cap: false,
            free_slots: 20,
            tile_items: 0,
            free_capacity: 10000,
            item_weight: 1,
            bag_weight: 1,
            funds: 150,
        }
    }
    #[test]
    fn nonstackable_bags_twenty_twenty_five() {
        let plan = plan_purchase(input());
        assert_eq!(
            plan,
            Ok(PurchasePlan {
                stacks: vec![1; 45],
                bags: 3,
                carried: 3,
                carried_roots: vec![true; 3],
                total: 150
            })
        );
    }
    #[test]
    fn stackable_250_uses_one_bag() {
        let plan = plan_purchase(PurchaseInput {
            amount: 250,
            stack_size: 100,
            funds: 520,
            ..input()
        });
        assert_eq!(
            plan,
            Ok(PurchasePlan {
                stacks: vec![100, 100, 50],
                bags: 1,
                carried: 1,
                carried_roots: vec![true],
                total: 520
            })
        );
    }
    #[test]
    fn cannot_pay_for_items_without_bags() {
        assert_eq!(
            plan_purchase(PurchaseInput {
                funds: 90,
                ..input()
            }),
            Err(PurchaseRefusal::InsufficientFunds)
        );
    }
    #[test]
    fn ignore_cap_drops_overflow_and_enforces_tile_limit() {
        let value = PurchaseInput {
            ignore_cap: true,
            free_slots: 1,
            tile_items: 28,
            ..input()
        };
        assert_eq!(plan_purchase(value).map(|p| p.carried), Ok(1));
        assert_eq!(
            plan_purchase(PurchaseInput {
                tile_items: 29,
                ..value
            }),
            Err(PurchaseRefusal::Room)
        );
        assert_eq!(
            plan_purchase(PurchaseInput {
                free_capacity: 0,
                tile_items: 27,
                ..value
            })
            .map(|p| p.carried),
            Ok(0)
        );
    }
    #[test]
    fn without_ignore_cap_capacity_and_room_refuse() {
        assert_eq!(
            plan_purchase(PurchaseInput {
                free_capacity: 0,
                ..input()
            }),
            Err(PurchaseRefusal::Capacity)
        );
        assert_eq!(
            plan_purchase(PurchaseInput {
                free_slots: 0,
                ..input()
            }),
            Err(PurchaseRefusal::Room)
        );
    }
    #[test]
    fn overflow_still_carries_a_later_smaller_stack_or_bag() {
        let stacks = PurchaseInput {
            amount: 250,
            stack_size: 100,
            in_backpacks: false,
            ignore_cap: true,
            free_capacity: 75,
            funds: 500,
            ..input()
        };
        assert_eq!(
            plan_purchase(stacks).map(|plan| plan.carried_roots),
            Ok(vec![false, false, true])
        );
        assert_eq!(
            plan_purchase(PurchaseInput {
                ignore_cap: true,
                free_capacity: 10,
                ..input()
            })
            .map(|plan| plan.carried_roots),
            Ok(vec![false, false, true])
        );
    }
    #[test]
    fn donor_messages() {
        assert_eq!(
            purchase_message(45, "sword", 3, 150),
            "Bought 45x sword and 3 shopping bags for 150 gold coins."
        );
        assert_eq!(
            purchase_message(250, "arrow", 1, 520),
            "Bought 250x arrow and 1 shopping bag for 520 gold coins."
        );
        assert_eq!(
            purchase_message(1, "arrow", 0, 1),
            "Bought 1x arrow for 1 gold coin."
        );
    }
}
