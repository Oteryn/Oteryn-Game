//! Coin denominations and the deterministic fee payment plan (GOLD-FEE-1a; decision
//! `CHARACTER-GOLD-FEE-BOUNDARY-V1` §4.2, owner decisions D175/D176).
//!
//! Pure: the plan is a function of the backpack's durable coin stacks and the fee only, never of
//! client order or ItemInstanceId. Worth is in gold units; arithmetic is exact and checked.

/// Every coin is an ItemInstance of this definition family.
pub const COIN_DEFINITION_FAMILY: &str = "Item";
/// D176: the stack maximum of each coin definition (within `GAMEITEM01-STACK-QUANTITY-MAX`).
pub const COIN_STACK_MAXIMUM: u32 = 100;
/// §4.2 step 5: a plan needing more inputs is rejected (`CAPACITY_EXCEEDED`).
pub const FEE_INPUTS_MAX: usize = 20;
/// §4.2 step 4 and the §2 interpretation: at most one platinum and one gold change stack.
pub const FEE_CHANGE_OUTPUTS_MAX: usize = 2;
/// `GAMEITEM01-CONTAINER-ENTRIES-MAX`: direct entries of the main backpack.
pub const BACKPACK_ENTRIES_MAX: usize = 20;

/// The closed worth table. Any other definition key is not a coin; changing the table needs an
/// amendment of the decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Coin {
    Gold,
    Platinum,
    Crystal,
}

impl Coin {
    pub const ALL: [Self; 3] = [Self::Gold, Self::Platinum, Self::Crystal];

    #[must_use]
    pub const fn production_key(self) -> &'static str {
        match self {
            Self::Gold => "oteryn:item.tibia.i3031",
            Self::Platinum => "oteryn:item.tibia.i3035",
            Self::Crystal => "oteryn:item.tibia.i3043",
        }
    }

    /// Worth of one coin in gold units.
    #[must_use]
    pub const fn worth(self) -> u64 {
        match self {
            Self::Gold => 1,
            Self::Platinum => 100,
            Self::Crystal => 10_000,
        }
    }

    #[must_use]
    pub fn from_production_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|coin| coin.production_key() == key)
    }
}

/// One eligible input: a live coin stack in a direct backpack entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoinStack {
    pub coin: Coin,
    pub quantity: u32,
    /// Placement ordinal of its backpack entry; display order is highest first.
    pub placement_ordinal: u64,
}

/// One BURN line: `burned` units of `stacks[input]`; the whole stack when equal to its quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeeBurnLine {
    pub input: usize,
    pub burned: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeePlan {
    /// In burn order: worth ascending, then display order.
    pub lines: Vec<FeeBurnLine>,
    /// Burned worth minus the fee, in gold units; `< worth` of the last input.
    pub change: u64,
    /// `floor(change / 100)`, at most 99.
    pub change_platinum: u32,
    /// `change mod 100`, at most 99.
    pub change_gold: u32,
}

impl FeePlan {
    /// Change stacks to mint: platinum first, then gold, each only when positive.
    #[must_use]
    pub fn change_outputs(&self) -> usize {
        usize::from(self.change_platinum > 0) + usize::from(self.change_gold > 0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeePlanError {
    /// A zero fee, an out-of-range stack, a repeated ordinal or too many entries.
    InvalidInput,
    InsufficientFunds,
    /// More than [`FEE_INPUTS_MAX`] inputs would be burned.
    CapacityExceeded,
    /// Fewer free backpack entries after the burn than change outputs.
    ChangeDoesNotFit,
}

/// §4.2: plan paying `fee` gold units from `stacks`. `entries` is the backpack's live direct
/// entry count, coins and other items together, before the burn (B3 bounds it by
/// [`BACKPACK_ENTRIES_MAX`]; the plan does not rely on that bound).
pub fn plan_fee(fee: u64, stacks: &[CoinStack], entries: usize) -> Result<FeePlan, FeePlanError> {
    plan_fee_within(fee, stacks, entries, BACKPACK_ENTRIES_MAX)
}

/// [`plan_fee`] for a backpack declaring `capacity` direct entries (at most
/// [`BACKPACK_ENTRIES_MAX`]): the change must fit that capacity after the burn.
pub fn plan_fee_within(
    fee: u64,
    stacks: &[CoinStack],
    entries: usize,
    capacity: usize,
) -> Result<FeePlan, FeePlanError> {
    if fee == 0 || stacks.len() > entries || !(1..=BACKPACK_ENTRIES_MAX).contains(&capacity) {
        return Err(FeePlanError::InvalidInput);
    }
    let mut order: Vec<usize> = (0..stacks.len()).collect();
    order.sort_by(|a, b| {
        let (a, b) = (&stacks[*a], &stacks[*b]);
        a.coin
            .worth()
            .cmp(&b.coin.worth())
            .then(b.placement_ordinal.cmp(&a.placement_ordinal))
    });
    let mut total: u64 = 0;
    for (position, index) in order.iter().enumerate() {
        let stack = &stacks[*index];
        if !(1..=COIN_STACK_MAXIMUM).contains(&stack.quantity)
            || order[position + 1..]
                .iter()
                .any(|other| stacks[*other].placement_ordinal == stack.placement_ordinal)
        {
            return Err(FeePlanError::InvalidInput);
        }
        total = u64::from(stack.quantity)
            .checked_mul(stack.coin.worth())
            .and_then(|worth| total.checked_add(worth))
            .ok_or(FeePlanError::InvalidInput)?;
    }
    if total < fee {
        return Err(FeePlanError::InsufficientFunds);
    }

    let mut lines = Vec::new();
    let mut remaining = fee;
    let mut change = 0;
    for index in order {
        if remaining == 0 {
            break;
        }
        let stack = &stacks[index];
        let worth = stack.coin.worth();
        let burned = u64::from(stack.quantity).min(remaining.div_ceil(worth));
        let value = burned * worth;
        if value >= remaining {
            change = value - remaining;
            remaining = 0;
        } else {
            remaining -= value;
        }
        lines.push(FeeBurnLine {
            input: index,
            burned: u32::try_from(burned).map_err(|_| FeePlanError::InvalidInput)?,
        });
    }
    if lines.len() > FEE_INPUTS_MAX {
        return Err(FeePlanError::CapacityExceeded);
    }
    let plan = FeePlan {
        lines,
        change,
        change_platinum: u32::try_from(change / 100).map_err(|_| FeePlanError::InvalidInput)?,
        change_gold: u32::try_from(change % 100).map_err(|_| FeePlanError::InvalidInput)?,
    };
    let whole = plan
        .lines
        .iter()
        .filter(|line| line.burned == stacks[line.input].quantity)
        .count();
    if plan.change_outputs() > capacity.saturating_sub(entries) + whole {
        return Err(FeePlanError::ChangeDoesNotFit);
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn stack(coin: Coin, quantity: u32, placement_ordinal: u64) -> CoinStack {
        CoinStack {
            coin,
            quantity,
            placement_ordinal,
        }
    }

    fn conserved(fee: u64, stacks: &[CoinStack], plan: &FeePlan) {
        let burned: u64 = plan
            .lines
            .iter()
            .map(|line| u64::from(line.burned) * stacks[line.input].coin.worth())
            .sum();
        assert_eq!(burned - plan.change, fee);
        assert_eq!(
            plan.change,
            u64::from(plan.change_platinum) * 100 + u64::from(plan.change_gold)
        );
        let partial = plan
            .lines
            .iter()
            .filter(|line| line.burned < stacks[line.input].quantity)
            .count();
        assert!(partial <= 1);
        if partial == 1 {
            let last = plan.lines.last().unwrap();
            assert!(last.burned < stacks[last.input].quantity);
        }
    }

    #[test]
    fn worth_table_is_closed() {
        assert_eq!(
            Coin::from_production_key("oteryn:item.tibia.i3035"),
            Some(Coin::Platinum)
        );
        assert_eq!(Coin::from_production_key("oteryn:item.tibia.i3032"), None);
        assert_eq!(Coin::Crystal.worth(), 100 * Coin::Platinum.worth());
        assert_eq!(Coin::Platinum.worth(), 100 * Coin::Gold.worth());
    }

    #[test]
    fn gold_burns_in_display_order_and_only_the_last_stack_partly() {
        let stacks = [
            stack(Coin::Gold, 40, 1),
            stack(Coin::Gold, 30, 5),
            stack(Coin::Gold, 50, 3),
        ];
        let plan = plan_fee(100, &stacks, 3).unwrap();
        // Highest ordinal first: 30 (ordinal 5), 50 (ordinal 3), then 20 of 40 (ordinal 1).
        assert_eq!(
            plan.lines,
            vec![
                FeeBurnLine {
                    input: 1,
                    burned: 30
                },
                FeeBurnLine {
                    input: 2,
                    burned: 50
                },
                FeeBurnLine {
                    input: 0,
                    burned: 20
                },
            ]
        );
        assert_eq!(plan.change, 0);
        conserved(100, &stacks, &plan);
    }

    #[test]
    fn lower_worth_burns_first_and_change_is_platinum_plus_gold() {
        // The decision §8 example: 2,350 from one crystal coin mints 76 platinum and 50 gold.
        let stacks = [stack(Coin::Crystal, 1, 1)];
        let plan = plan_fee(2_350, &stacks, 1).unwrap();
        assert_eq!((plan.change_platinum, plan.change_gold), (76, 50));
        assert_eq!(plan.change_outputs(), 2);
        conserved(2_350, &stacks, &plan);

        let stacks = [
            stack(Coin::Crystal, 2, 9),
            stack(Coin::Gold, 30, 1),
            stack(Coin::Platinum, 3, 2),
        ];
        let plan = plan_fee(10_450, &stacks, 3).unwrap();
        // All gold, all platinum, then one crystal: 30 + 300 + 10,000 - 10,450 = 9,880 change.
        assert_eq!(
            plan.lines.iter().map(|line| line.input).collect::<Vec<_>>(),
            vec![1, 2, 0]
        );
        assert_eq!((plan.change_platinum, plan.change_gold), (98, 80));
        conserved(10_450, &stacks, &plan);
    }

    #[test]
    fn exact_payment_mints_no_change() {
        let stacks = [stack(Coin::Platinum, 5, 1), stack(Coin::Gold, 50, 2)];
        let plan = plan_fee(350, &stacks, 2).unwrap();
        assert_eq!(plan.change, 0);
        assert_eq!(plan.change_outputs(), 0);
        conserved(350, &stacks, &plan);
    }

    #[test]
    fn insufficient_funds_and_invalid_input_reject() {
        let stacks = [stack(Coin::Gold, 99, 1)];
        assert_eq!(
            plan_fee(100, &stacks, 1),
            Err(FeePlanError::InsufficientFunds)
        );
        assert_eq!(plan_fee(0, &stacks, 1), Err(FeePlanError::InvalidInput));
        assert_eq!(
            plan_fee(1, &[stack(Coin::Gold, 101, 1)], 1),
            Err(FeePlanError::InvalidInput)
        );
        assert_eq!(
            plan_fee(1, &[stack(Coin::Gold, 0, 1)], 1),
            Err(FeePlanError::InvalidInput)
        );
        assert_eq!(
            plan_fee(1, &[stack(Coin::Gold, 1, 1), stack(Coin::Gold, 1, 1)], 2),
            Err(FeePlanError::InvalidInput)
        );
        assert_eq!(plan_fee(1, &stacks, 0), Err(FeePlanError::InvalidInput));
    }

    #[test]
    fn twenty_inputs_are_admitted_and_twenty_one_are_rejected() {
        let stacks: Vec<CoinStack> = (1..=20).map(|n| stack(Coin::Gold, 100, n)).collect();
        let plan = plan_fee(2_000, &stacks, 20).unwrap();
        assert_eq!(plan.lines.len(), FEE_INPUTS_MAX);
        conserved(2_000, &stacks, &plan);
        // B3 makes a 21st entry unreachable; the plan still rejects it on its own.
        let stacks: Vec<CoinStack> = (1..=21).map(|n| stack(Coin::Gold, 100, n)).collect();
        assert_eq!(
            plan_fee(2_001, &stacks, 21),
            Err(FeePlanError::CapacityExceeded)
        );
    }

    #[test]
    fn change_must_fit_after_the_burn() {
        // One crystal among 20 entries: burning it whole frees one entry, two outputs need two.
        let mut stacks = vec![stack(Coin::Crystal, 1, 20)];
        assert_eq!(
            plan_fee(2_350, &stacks, 20),
            Err(FeePlanError::ChangeDoesNotFit)
        );
        // 19 entries: one free plus the burned crystal's entry.
        assert!(plan_fee(2_350, &stacks, 19).is_ok());
        // A partly burned crystal frees nothing; one output needs one free entry.
        stacks[0].quantity = 2;
        assert_eq!(
            plan_fee(9_900, &stacks, 20),
            Err(FeePlanError::ChangeDoesNotFit)
        );
        assert_eq!(plan_fee(9_900, &stacks, 19).unwrap().change_outputs(), 1);
        // A backpack declaring 8 entries holds 8 at most: full, a partial burn frees nothing.
        assert_eq!(
            plan_fee_within(9_900, &stacks, 8, 8),
            Err(FeePlanError::ChangeDoesNotFit)
        );
        assert_eq!(
            plan_fee_within(9_900, &stacks, 7, 8)
                .unwrap()
                .change_outputs(),
            1
        );
        assert_eq!(
            plan_fee_within(9_900, &stacks, 1, 0),
            Err(FeePlanError::InvalidInput)
        );
    }

    #[test]
    fn the_largest_reachable_fee_is_exact() {
        let stacks: Vec<CoinStack> = (1..=20).map(|n| stack(Coin::Crystal, 100, n)).collect();
        let plan = plan_fee(20_000_000, &stacks, 20).unwrap();
        assert_eq!(plan.change, 0);
        assert_eq!(
            plan_fee(20_000_001, &stacks, 20),
            Err(FeePlanError::InsufficientFunds)
        );
    }
}
