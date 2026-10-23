//! HOUSE-OWN-0 §4: pricing a qualified bid set is calculation, never settlement authority.

use std::collections::BTreeSet;

pub mod eligibility;
pub mod value;

pub const BIDDERS_MAX: usize = 256;
pub const OPENING_US: i64 = 7 * 24 * 60 * 60 * 1_000_000;
pub const EXTENSION_US: i64 = 15 * 60 * 1_000_000;
pub const FIRST_RENT_US: i64 = 30 * 24 * 60 * 60 * 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuctionError {
    InvalidInput,
    Full,
    Closed,
    Overflow,
    Unfunded,
}

/// `maximum_reached_order` is allocated while holding the property lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bid {
    pub account: [u8; 16],
    pub maximum: u64,
    pub maximum_reached_order: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quote {
    pub leader: [u8; 16],
    pub price: u64,
}

/// The caller excludes currently ineligible bidders before supplying this set.
pub fn quote(bids: &[Bid]) -> Result<Option<Quote>, AuctionError> {
    if bids.len() > BIDDERS_MAX {
        return Err(AuctionError::Full);
    }
    let mut accounts = BTreeSet::new();
    let mut orders = BTreeSet::new();
    let mut ordered = Vec::with_capacity(bids.len());
    for bid in bids {
        if bid.account == [0; 16]
            || bid.maximum_reached_order == 0
            || !accounts.insert(bid.account)
            || !orders.insert(bid.maximum_reached_order)
        {
            return Err(AuctionError::InvalidInput);
        }
        ordered.push(bid);
    }
    ordered.sort_unstable_by_key(|bid| (std::cmp::Reverse(bid.maximum), bid.maximum_reached_order));
    Ok(ordered.first().map(|leader| Quote {
        leader: leader.account,
        price: ordered.get(1).map_or(0, |second| {
            second.maximum.saturating_add(1).min(leader.maximum)
        }),
    }))
}

/// The PostgreSQL clock is read after acquiring the property lock.
pub fn bid_deadline(now_us: i64, current: Option<i64>) -> Result<i64, AuctionError> {
    if now_us < 0 {
        return Err(AuctionError::InvalidInput);
    }
    match current {
        None => now_us.checked_add(OPENING_US).ok_or(AuctionError::Overflow),
        Some(end) if now_us >= end => Err(AuctionError::Closed),
        Some(end) => Ok(end.max(
            now_us
                .checked_add(EXTENSION_US)
                .ok_or(AuctionError::Overflow)?,
        )),
    }
}

#[cfg(test)]
#[path = "house_auction_tests.rs"]
mod tests;
