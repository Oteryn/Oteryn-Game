//! Detached HOUSE-1a arithmetic (HOUSE-OWN-0 §4; SOCIAL-MAP-PACKETS-1 §1.3, §1.9).
//!
//! These pure values grant no ownership or authority. The future writer must read the complete
//! auction under the property lock, check current Character/Account/World binding, slot, ban,
//! Premium and bank/escrow facts, and apply the session/recovery fences and ledger atomically.
//! Settlement must first exclude every currently ineligible bid. No table or runtime is assumed.
//! Gold uses the bank request type, `u64`; no coin payment or ongoing rent collection is modeled.

use crate::domain::AccountId;

/// HOUSEOWN0-RL-05: maximum number of bidders in one auction.
pub(crate) const BIDDERS_MAX: usize = 256;
/// HOUSEOWN0-RL-02: seven days from the first admitted bid.
pub(crate) const AUCTION_LENGTH_MS: i64 = 7 * 24 * 60 * 60 * 1_000;
/// HOUSEOWN0-RL-04: each admitted bid in the final fifteen minutes extends the close.
pub(crate) const ANTI_SNIPING_MS: i64 = 15 * 60 * 1_000;
/// D1375: retain the native Bank balance ceiling for all held-value headroom.
pub(crate) const GOLD_CEILING: u64 = crate::durability::bank_audit::BANK_BALANCE_MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuctionRefusal {
    InvalidBids,
    AuctionFull,
    AuctionClosed,
    TooLow,
    BalanceLimit,
    InsufficientBalance,
    TimeOverflow,
}

/// One held bid from a single auction's complete, caller-qualified set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Bid {
    pub account: AccountId,
    pub maximum_gold: u64,
    /// Durable total order in which this bid reached its current maximum, assigned by the
    /// serialized writer, not a client timestamp. A changed maximum requires its own order.
    pub maximum_reached_order: u64,
}

/// Public proxy result: it never exposes a private maximum or authorizes settlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProxyQuote {
    pub leader: AccountId,
    pub price_gold: u64,
}

/// HOUSE-OWN-0 §4: descending maxima, then the bid that first reached its current maximum.
/// Invalid/ambiguous order or duplicate Account input refuses rather than inventing a tie rule.
pub(crate) fn proxy_quote(bids: &[Bid]) -> Result<Option<ProxyQuote>, AuctionRefusal> {
    if bids.len() > BIDDERS_MAX {
        return Err(AuctionRefusal::AuctionFull);
    }
    for (index, bid) in bids.iter().enumerate() {
        if bid.maximum_gold > GOLD_CEILING
            || bids[..index].iter().any(|other| {
                other.account == bid.account
                    || other.maximum_reached_order == bid.maximum_reached_order
            })
        {
            return Err(AuctionRefusal::InvalidBids);
        }
    }
    let mut order = bids.iter().collect::<Vec<_>>();
    order.sort_unstable_by(|left, right| {
        right
            .maximum_gold
            .cmp(&left.maximum_gold)
            .then(left.maximum_reached_order.cmp(&right.maximum_reached_order))
    });
    let Some(leader) = order.first() else {
        return Ok(None);
    };
    // Each maximum is bounded below u64::MAX, so the one-gold increment is representable.
    let price_gold = order.get(1).map_or(0, |second| {
        (second.maximum_gold + 1).min(leader.maximum_gold)
    });
    Ok(Some(ProxyQuote {
        leader: leader.account,
        price_gold,
    }))
}

/// Arithmetic only: reserve/release deltas for a bid, raise or lower. The writer must preserve
/// the auction's snapshotted first rent and independently validate its existing held escrow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BidAmounts {
    pub escrow_gold: u64,
    pub reserve_gold: u64,
    pub release_gold: u64,
}

pub(crate) fn bid_amounts(
    previous_maximum: Option<u64>,
    requested_maximum: u64,
    current_price: u64,
    first_rent: u64,
) -> Result<BidAmounts, AuctionRefusal> {
    if current_price > GOLD_CEILING {
        return Err(AuctionRefusal::InvalidBids);
    }
    // §4 sets a floor on lowering an existing maximum. Other bid admission rules stay with
    // the writer; arithmetic does not invent a minimum for new bids or raises.
    if previous_maximum
        .is_some_and(|previous| requested_maximum < previous && requested_maximum < current_price)
    {
        return Err(AuctionRefusal::TooLow);
    }
    let escrow_gold = checked_gold_sum(requested_maximum, first_rent)?;
    let previous = previous_maximum
        .map(|maximum| checked_gold_sum(maximum, first_rent))
        .transpose()?
        .unwrap_or(0);
    Ok(BidAmounts {
        escrow_gold,
        reserve_gold: escrow_gold.saturating_sub(previous),
        release_gold: previous.saturating_sub(escrow_gold),
    })
}

/// The native Bank entry kinds allocated by D1375. The first rent uses the same
/// kind as ongoing rent; its funding source remains the already-debited bid escrow.
pub(crate) const HOUSE_BID_RESERVE: u16 = 7;
pub(crate) const HOUSE_BID_RELEASE: u16 = 8;
pub(crate) const HOUSE_PRICE: u16 = 9;
pub(crate) const HOUSE_RENT: u16 = 10;

/// One House-owned Bank ledger participant. A price or first-rent entry records
/// the burn from escrow with an unchanged Bank balance. The House operation's
/// deferred proof must bind that entry to the actual captured escrow decrease.
/// Ongoing rent instead consumes the Bank balance and has no escrow participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HouseLedgerAmount {
    pub kind: u16,
    pub amount: u64,
    pub balance_before: u64,
    pub balance_after: u64,
}

/// Value arithmetic for one account's already captured bid row. These values are
/// not permission to write a balance or change a bid. The native writer supplies
/// the current row under the canonical lock order and the shared headroom check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BidValuePlan {
    pub escrow_before: u64,
    pub escrow_after: u64,
    pub balance_before: u64,
    pub balance_after: u64,
    pub price_burn: u64,
    pub rent_burn: u64,
    /// Actual nonzero ledger entries, in their required Bank-chain order.
    pub entries: Vec<HouseLedgerAmount>,
}

impl BidValuePlan {
    /// Each changed escrow is one value line and each nonzero ledger entry is
    /// another. Zero-price/zero-rent cases still retain the House operation, but
    /// manufacture neither a zero-amount Bank entry nor a zero-value audit line.
    pub(crate) fn value_lines(&self) -> usize {
        self.entries.len() + usize::from(self.escrow_before != self.escrow_after)
    }

    fn checked(self) -> Result<Self, AuctionRefusal> {
        for amount in [
            self.escrow_before,
            self.escrow_after,
            self.balance_before,
            self.balance_after,
            self.price_burn,
            self.rent_burn,
        ] {
            if amount > GOLD_CEILING {
                return Err(AuctionRefusal::BalanceLimit);
            }
        }
        // The domain cap makes these sums representable, but keep the calculation
        // widened so conservation never depends on a future cap's machine width.
        let before = u128::from(self.balance_before) + u128::from(self.escrow_before);
        let after = u128::from(self.balance_after)
            + u128::from(self.escrow_after)
            + u128::from(self.price_burn)
            + u128::from(self.rent_burn);
        if before != after {
            return Err(AuctionRefusal::InvalidBids);
        }
        Ok(self)
    }
}

/// Reserve, raise, lower or re-submit the same maximum against the actual locked
/// account balance. A lowering pays back only the difference. The unchanged-first-
/// rent rule is checked by the caller against the retained auction snapshot.
pub(crate) fn plan_bid_value(
    balance: u64,
    previous_maximum: Option<u64>,
    requested_maximum: u64,
    current_price: u64,
    first_rent: u64,
) -> Result<BidValuePlan, AuctionRefusal> {
    let amounts = bid_amounts(
        previous_maximum,
        requested_maximum,
        current_price,
        first_rent,
    )?;
    let escrow_before = previous_maximum
        .map(|maximum| checked_gold_sum(maximum, first_rent))
        .transpose()?
        .unwrap_or(0);
    let balance_after = if amounts.reserve_gold > 0 {
        balance
            .checked_sub(amounts.reserve_gold)
            .ok_or(AuctionRefusal::InsufficientBalance)?
    } else {
        checked_gold_sum(balance, amounts.release_gold)?
    };
    let entry = if amounts.reserve_gold > 0 {
        Some(HouseLedgerAmount {
            kind: HOUSE_BID_RESERVE,
            amount: amounts.reserve_gold,
            balance_before: balance,
            balance_after,
        })
    } else if amounts.release_gold > 0 {
        Some(HouseLedgerAmount {
            kind: HOUSE_BID_RELEASE,
            amount: amounts.release_gold,
            balance_before: balance,
            balance_after,
        })
    } else {
        None
    };
    BidValuePlan {
        escrow_before,
        escrow_after: amounts.escrow_gold,
        balance_before: balance,
        balance_after,
        price_burn: 0,
        rent_burn: 0,
        entries: entry.into_iter().collect(),
    }
    .checked()
}

/// Settlement of the winning, currently eligible bid. `price` must be the proxy
/// quote recomputed over the complete valid set under lock. This arithmetic does
/// not accept or manufacture a Premium/eligibility/current-catalogue assertion.
pub(crate) fn plan_winner_value(
    balance: u64,
    maximum: u64,
    first_rent: u64,
    price: u64,
) -> Result<BidValuePlan, AuctionRefusal> {
    let escrow_before = checked_gold_sum(maximum, first_rent)?;
    if price > maximum {
        return Err(AuctionRefusal::InvalidBids);
    }
    let returned = maximum - price;
    let balance_after = checked_gold_sum(balance, returned)?;
    let mut entries = Vec::with_capacity(3);
    if returned > 0 {
        entries.push(HouseLedgerAmount {
            kind: HOUSE_BID_RELEASE,
            amount: returned,
            balance_before: balance,
            balance_after,
        });
    }
    for (kind, amount) in [(HOUSE_PRICE, price), (HOUSE_RENT, first_rent)] {
        if amount > 0 {
            entries.push(HouseLedgerAmount {
                kind,
                amount,
                balance_before: balance_after,
                balance_after,
            });
        }
    }
    BidValuePlan {
        escrow_before,
        escrow_after: 0,
        balance_before: balance,
        balance_after,
        price_burn: price,
        rent_burn: first_rent,
        entries,
    }
    .checked()
}

/// Release one non-winning or excluded HELD bid. The maximum and first rent both
/// return to the captured bidder account. No currently active Character, Premium
/// state, new recipient or owner decision may redirect or refuse this return.
pub(crate) fn plan_release_value(
    balance: u64,
    maximum: u64,
    first_rent: u64,
) -> Result<BidValuePlan, AuctionRefusal> {
    let escrow_before = checked_gold_sum(maximum, first_rent)?;
    let balance_after = checked_gold_sum(balance, escrow_before)?;
    let entries = if escrow_before == 0 {
        Vec::new()
    } else {
        vec![HouseLedgerAmount {
            kind: HOUSE_BID_RELEASE,
            amount: escrow_before,
            balance_before: balance,
            balance_after,
        }]
    };
    BidValuePlan {
        escrow_before,
        escrow_after: 0,
        balance_before: balance,
        balance_after,
        price_burn: 0,
        rent_burn: 0,
        entries,
    }
    .checked()
}

/// The same House rent ledger kind for later periods. Its actual native consumer
/// belongs to HOUSE-1b and must retain the canonical house/period operation. This
/// calculation takes no bid escrow and burns the charged amount from the balance.
pub(crate) fn plan_ongoing_rent_value(
    balance: u64,
    rent: u64,
) -> Result<BidValuePlan, AuctionRefusal> {
    if balance > GOLD_CEILING || rent > GOLD_CEILING {
        return Err(AuctionRefusal::BalanceLimit);
    }
    let balance_after = balance
        .checked_sub(rent)
        .ok_or(AuctionRefusal::InsufficientBalance)?;
    let entries = if rent == 0 {
        Vec::new()
    } else {
        vec![HouseLedgerAmount {
            kind: HOUSE_RENT,
            amount: rent,
            balance_before: balance,
            balance_after,
        }]
    };
    BidValuePlan {
        escrow_before: 0,
        escrow_after: 0,
        balance_before: balance,
        balance_after,
        price_burn: 0,
        rent_burn: rent,
        entries,
    }
    .checked()
}

/// SOCIAL-MAP §1.9: arithmetic for the future shared bank headroom guard. This accepts current
/// inputs only as caller-supplied facts; it neither reads nor reserves a balance or escrow.
pub(crate) fn credit_fits(
    balance: u64,
    held_escrow: u64,
    credit: u64,
) -> Result<(), AuctionRefusal> {
    checked_gold_sum(checked_gold_sum(balance, held_escrow)?, credit).map(|_| ())
}

fn checked_gold_sum(left: u64, right: u64) -> Result<u64, AuctionRefusal> {
    left.checked_add(right)
        .filter(|sum| *sum <= GOLD_CEILING)
        .ok_or(AuctionRefusal::BalanceLimit)
}

/// The future writer supplies clock_timestamp() read after locking the property row.
pub(crate) fn first_bid_deadline(now_unix_ms: i64) -> Result<i64, AuctionRefusal> {
    now_unix_ms
        .checked_add(AUCTION_LENGTH_MS)
        .ok_or(AuctionRefusal::TimeOverflow)
}

/// A bid is admitted strictly before the current close; equality is already closed. This must
/// run only for a bid the writer admits: a refused bid cannot extend an auction.
pub(crate) fn admitted_bid_deadline(
    now_unix_ms: i64,
    ends_at_unix_ms: i64,
) -> Result<i64, AuctionRefusal> {
    if now_unix_ms >= ends_at_unix_ms {
        return Err(AuctionRefusal::AuctionClosed);
    }
    let extended = now_unix_ms
        .checked_add(ANTI_SNIPING_MS)
        .ok_or(AuctionRefusal::TimeOverflow)?;
    Ok(ends_at_unix_ms.max(extended))
}

#[cfg(test)]
#[path = "auction_tests.rs"]
mod tests;
