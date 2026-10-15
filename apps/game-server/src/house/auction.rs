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
/// HOUSEOWN0-RL-13/-15: bank balance plus held escrow may not exceed this ceiling.
pub(crate) const GOLD_CEILING: u64 = 9_000_000_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuctionRefusal {
    InvalidBids,
    AuctionFull,
    AuctionClosed,
    TooLow,
    BalanceLimit,
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
