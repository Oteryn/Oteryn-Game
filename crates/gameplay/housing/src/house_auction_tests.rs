#![allow(clippy::expect_used)]
use super::value::*;
use super::*;

fn bid(seed: u8, maximum: u64, order: u64) -> Bid {
    Bid {
        account: [seed; 16],
        maximum,
        maximum_reached_order: order,
    }
}

#[test]
fn proxy_prices_and_durable_ties() {
    assert_eq!(quote(&[]), Ok(None));
    assert_eq!(
        quote(&[bid(1, 900, 1)]),
        Ok(Some(Quote {
            leader: [1; 16],
            price: 0
        }))
    );
    assert_eq!(
        quote(&[bid(1, 900, 2), bid(2, 700, 1)]),
        Ok(Some(Quote {
            leader: [1; 16],
            price: 701
        }))
    );
    assert_eq!(
        quote(&[bid(1, 900, 2), bid(2, 900, 1)]),
        Ok(Some(Quote {
            leader: [2; 16],
            price: 900
        }))
    );
    assert_eq!(
        quote(&[bid(1, u64::MAX, 1), bid(2, u64::MAX, 2)]),
        Ok(Some(Quote {
            leader: [1; 16],
            price: u64::MAX
        }))
    );
    assert_eq!(
        quote(&[bid(1, 0, 1), bid(2, 0, 2)])
            .expect("valid auction fixture")
            .expect("valid auction fixture")
            .price,
        0
    );
}

#[test]
fn bidder_bound_and_ambiguous_inputs() {
    let mut bids: Vec<_> = (1_u16..=256)
        .map(|n| {
            let mut account = [0; 16];
            account[..2].copy_from_slice(&n.to_be_bytes());
            Bid {
                account,
                maximum: u64::from(n),
                maximum_reached_order: u64::from(n),
            }
        })
        .collect();
    assert!(quote(&bids).is_ok());
    bids.push(bid(2, 900, 257));
    assert_eq!(quote(&bids), Err(AuctionError::Full));
    for invalid in [
        vec![bid(1, 1, 1), bid(1, 2, 2)],
        vec![bid(1, 1, 1), bid(2, 1, 1)],
        vec![bid(1, 1, 0)],
        vec![bid(0, 1, 1)],
    ] {
        assert_eq!(quote(&invalid), Err(AuctionError::InvalidInput));
    }
}

#[test]
fn seven_days_extension_close_and_time_overflow() {
    assert_eq!(bid_deadline(0, None), Ok(OPENING_US));
    assert_eq!(bid_deadline(1, Some(OPENING_US)), Ok(OPENING_US));
    let late = OPENING_US - 1;
    assert_eq!(
        bid_deadline(late, Some(OPENING_US)),
        Ok(late + EXTENSION_US)
    );
    assert_eq!(
        bid_deadline(OPENING_US, Some(OPENING_US)),
        Err(AuctionError::Closed)
    );
    assert_eq!(bid_deadline(i64::MAX, None), Err(AuctionError::Overflow));
    assert_eq!(
        bid_deadline(i64::MAX - 1, Some(i64::MAX)),
        Err(AuctionError::Overflow)
    );
    assert_eq!(bid_deadline(-1, None), Err(AuctionError::InvalidInput));
}

#[test]
fn reserve_raise_lower_repeat_and_release_conserve_value() {
    let first = plan_bid_value(1000, 0, 400, 100, 1000).expect("valid auction fixture");
    assert_eq!((first.balance, first.escrow), (500, 500));
    assert_eq!(first.lines[0].kind, LedgerKind::Reserve);
    let raised = plan_bid_value(500, 500, 500, 100, 1000).expect("valid auction fixture");
    assert_eq!(raised.lines[0].amount, 100);
    let lower = plan_bid_value(400, 600, 300, 100, 1000).expect("valid auction fixture");
    assert_eq!((lower.balance, lower.escrow), (600, 400));
    assert_eq!(lower.lines[0].kind, LedgerKind::Release);
    assert!(
        plan_bid_value(600, 400, 300, 100, 1000)
            .expect("valid auction fixture")
            .lines
            .is_empty()
    );
    assert_eq!(
        plan_release_value(600, 400, 1000)
            .expect("valid auction fixture")
            .balance,
        1000
    );
    assert!(
        plan_release_value(1000, 0, 1000)
            .expect("valid auction fixture")
            .lines
            .is_empty()
    );
    assert_eq!(
        plan_bid_value(3, 0, 3, 1, 1000),
        Err(AuctionError::Unfunded)
    );
    assert_eq!(
        plan_bid_value(1000, 0, u64::MAX, 1, 1000),
        Err(AuctionError::Overflow)
    );
    assert_eq!(
        plan_release_value(1000, 1, 1000),
        Err(AuctionError::Overflow)
    );
}

#[test]
fn winner_burns_only_custody_and_has_at_most_four_value_lines() {
    let p = plan_winner_value(300, 700, 600, 201, 100, 1000).expect("valid auction fixture");
    assert_eq!((p.balance, p.escrow), (699, 0));
    assert_eq!(p.lines.len() + usize::from(p.escrow != 700), 4);
    assert_eq!(
        p.lines.iter().map(|l| l.kind).collect::<Vec<_>>(),
        [LedgerKind::Release, LedgerKind::Price, LedgerKind::Rent]
    );
    for maximum in 0..20 {
        for rent in 0..20 {
            for price in 0..=maximum {
                let escrow = maximum + rent;
                let p = plan_winner_value(1000 - escrow, escrow, maximum, price, rent, 1000)
                    .expect("valid auction fixture");
                assert_eq!(p.balance + price + rent, 1000);
                assert_eq!(p.escrow, 0);
                assert!(p.lines.iter().all(|l| l.amount > 0));
            }
        }
    }
    assert_eq!(
        plan_winner_value(0, 700, 600, 601, 100, 1000),
        Err(AuctionError::InvalidInput)
    );
    assert_eq!(
        plan_winner_value(0, 699, 600, 1, 100, 1000),
        Err(AuctionError::InvalidInput)
    );
}
