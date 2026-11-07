use super::*;

fn pair() -> PairFacts {
    PairFacts {
        same_character: false,
        same_account: false,
        same_channel: true,
        positions: [(0, 0, 0), (2, 2, 0)],
        mutually_visible: true,
        line_of_sight: true,
    }
}
fn leaf() -> LeafFacts {
    LeafFacts {
        direct_backpack_entry: true,
        is_main_backpack: false,
        has_contents: false,
        pickupable: true,
        tradeable: TradeFlag::Unknown,
        quantity: 100,
    }
}
fn destination() -> Destination {
    Destination {
        entries: 19,
        entry_limit: 20,
        highest_ordinal: 500,
        carried_weight: 100,
        capacity: 200,
        incoming_unit_weight: Some(1),
        incoming_quantity: 100,
    }
}

// https://github.com/opentibiabr/canary/blob/52550d5e3c0ffe565f4bd1014f4e5575abf1059b/src/game/game.cpp
// playerRequestTrade, internalStartTrade, playerAcceptTrade (read 2026-10-23).
// Same revision src/creatures/players/player.cpp: checkTradeState, onCreatureMove,
// onRemoveCreature (item/movement/logout cancellation).
// Each row corresponds to exactly one task-card Reference parity row. Canary is
// emulator evidence, not proof of CipSoft internals. The accepted leaf-only scope
// intentionally rejects container trees even though Reference allows <=100 items.
#[test]
fn player_trade_reference_parity_table() {
    let rows: &[(&str, fn())] = &[
        ("range_2", || {
            assert_eq!(pair().check(), Ok(()));
            assert_eq!(
                PairFacts {
                    positions: [(0, 0, 0), (3, 0, 0)],
                    ..pair()
                }
                .check(),
                Err(Refusal::TooFar)
            );
        }),
        ("same_floor", || {
            assert_eq!(
                PairFacts {
                    positions: [(1, 1, 0), (1, 1, -1)],
                    ..pair()
                }
                .check(),
                Err(Refusal::TooFar)
            )
        }),
        ("sight_line", || {
            assert_eq!(
                PairFacts {
                    line_of_sight: false,
                    ..pair()
                }
                .check(),
                Err(Refusal::Rejected)
            )
        }),
        ("no_self", || {
            assert_eq!(
                PairFacts {
                    same_character: true,
                    ..pair()
                }
                .check(),
                Err(Refusal::Rejected)
            )
        }),
        ("pickupable", || {
            assert_eq!(
                LeafFacts {
                    pickupable: false,
                    ..leaf()
                }
                .check(),
                Err(Refusal::NotTradeable)
            )
        }),
        ("one_root_100_items", || {
            assert_eq!(REFERENCE_ITEM_LIMIT, 100);
            assert_eq!(leaf().check(), Ok(()));
            assert_eq!(
                LeafFacts {
                    has_contents: true,
                    ..leaf()
                }
                .check(),
                Err(Refusal::NotTradeable)
            );
        }),
        ("counter_offer_both_accept", || {
            let mut trade = Trade::offered(1, 0);
            assert_eq!(trade.accept(0, &3, 4, 1), Err(Refusal::Stale));
            assert_eq!(trade.counter_offer(2, 3, 2), Ok(()));
            assert_eq!(trade.accept(0, &3, 4, 3), Ok(Accept::First));
            assert_eq!(trade.phase(), Phase::Ready);
            assert_eq!(trade.accept(1, &3, 5, 4), Ok(Accept::Transfer));
            assert_eq!(trade.phase(), Phase::Transferring);
        }),
        ("room_before_removal_no_merge", || {
            assert_eq!(destination().next_entry(), Ok(501));
            assert_eq!(
                Destination {
                    entries: 20,
                    ..destination()
                }
                .next_entry(),
                Err(Refusal::NoRoom)
            );
        }),
        ("incoming_capacity", || {
            assert_eq!(destination().next_entry(), Ok(501));
            assert_eq!(
                Destination {
                    capacity: 199,
                    ..destination()
                }
                .next_entry(),
                Err(Refusal::Overweight)
            );
        }),
        ("item_change_cancels", || {
            let mut trade = Trade::offered(1, 0);
            assert_eq!(
                trade.invalidate(Invalidation::ItemOrBackpackChanged, 1),
                Ok(())
            );
            assert_eq!(trade.phase(), Phase::Closed(CloseReason::Stale));
        }),
        ("distance_cancels", || {
            let mut trade = Trade::offered(1, 0);
            assert_eq!(trade.invalidate(Invalidation::Distance, 1), Ok(()));
            assert_eq!(trade.phase(), Phase::Closed(CloseReason::Cancelled));
        }),
        ("logout_cancels", || {
            let mut trade = Trade::offered(1, 0);
            assert_eq!(trade.invalidate(Invalidation::Logout, 1), Ok(()));
            assert_eq!(trade.phase(), Phase::Closed(CloseReason::Cancelled));
        }),
    ];
    for (name, check) in rows {
        check();
        assert!(!name.is_empty());
    }
}

#[test]
fn player_trade_owner_policy_and_invalidation() {
    for facts in [
        PairFacts {
            same_account: true,
            ..pair()
        },
        PairFacts {
            same_channel: false,
            ..pair()
        },
        PairFacts {
            mutually_visible: false,
            ..pair()
        },
    ] {
        assert_eq!(facts.check(), Err(Refusal::Rejected));
    }
    for flag in [TradeFlag::Unknown, TradeFlag::Known(true)] {
        assert_eq!(
            LeafFacts {
                tradeable: flag,
                ..leaf()
            }
            .check(),
            Ok(())
        );
    }
    for flag in [TradeFlag::Known(false), TradeFlag::Ambiguous] {
        assert_eq!(
            LeafFacts {
                tradeable: flag,
                ..leaf()
            }
            .check(),
            Err(Refusal::NotTradeable)
        );
    }
    for item in [
        LeafFacts {
            is_main_backpack: true,
            ..leaf()
        },
        LeafFacts {
            direct_backpack_entry: false,
            ..leaf()
        },
        LeafFacts {
            quantity: LEAF_QUANTITY_MAX + 1,
            ..leaf()
        },
        LeafFacts {
            quantity: 0,
            ..leaf()
        },
    ] {
        assert_eq!(item.check(), Err(Refusal::NotTradeable));
    }
    for cause in [
        Invalidation::ItemOrBackpackChanged,
        Invalidation::Distance,
        Invalidation::Logout,
        Invalidation::Reconnect,
        Invalidation::ChannelTransfer,
        Invalidation::Cancel,
    ] {
        let mut trade = Trade::offered(1, 0);
        assert_eq!(trade.invalidate(cause, 1), Ok(()));
        assert!(matches!(trade.phase(), Phase::Closed(_)));
    }
}

#[test]
fn player_trade_idle_clock_stale_accept_and_transfer_retention() {
    let mut trade = Trade::offered(1, u64::MAX - IDLE_TIMEOUT_MS);
    assert_eq!(trade.observe(u64::MAX - 1), Ok(()));
    assert_eq!(trade.phase(), Phase::Offered);
    assert_eq!(trade.observe(u64::MAX - 2), Err(Refusal::ClockReversed));
    assert_eq!(trade.observe(u64::MAX), Ok(()));
    assert_eq!(trade.phase(), Phase::Closed(CloseReason::Cancelled));
    let mut trade = Trade::offered(1, 0);
    assert_eq!(trade.counter_offer(2, 3, 1), Ok(()));
    assert_eq!(trade.accept(0, &99, 4, 2), Err(Refusal::Stale));
    assert_eq!(trade.phase(), Phase::Ready);
    assert_eq!(trade.accept(0, &3, 4, 3), Ok(Accept::First));
    assert_eq!(trade.accept(0, &3, 999, 4), Ok(Accept::Repeated));
    // The repeated command does not replace the first acceptance or extend its idle clock.
    assert_eq!(trade.accept(1, &3, 5, 5), Ok(Accept::Transfer));
    for cause in [
        Invalidation::Cancel,
        Invalidation::Reconnect,
        Invalidation::Logout,
        Invalidation::Distance,
    ] {
        assert_eq!(
            trade.invalidate(cause, u64::MAX),
            Err(Refusal::Transferring)
        );
    }
    assert_eq!(trade.phase(), Phase::Transferring);
}

#[test]
fn player_trade_arithmetic_bounds() {
    for capacity in [0, 1, u64::MAX - 1, u64::MAX] {
        for carried_weight in [0, 1, u64::MAX - 1, u64::MAX] {
            for unit in [0, 1, u32::MAX] {
                for quantity in [1, 100, u32::MAX] {
                    let d = Destination {
                        capacity,
                        carried_weight,
                        incoming_unit_weight: Some(unit),
                        incoming_quantity: quantity,
                        ..destination()
                    };
                    let fits = u128::from(carried_weight) + u128::from(unit) * u128::from(quantity)
                        <= u128::from(capacity);
                    assert_eq!(d.next_entry().is_ok(), fits);
                }
            }
        }
    }
    assert_eq!(
        Destination {
            entry_limit: 100,
            ..destination()
        }
        .next_entry(),
        Ok(501)
    );
    assert_eq!(
        Destination {
            entry_limit: 100,
            entries: 20,
            ..destination()
        }
        .next_entry(),
        Err(Refusal::NoRoom)
    );
    assert_eq!(
        Destination {
            highest_ordinal: u64::MAX,
            ..destination()
        }
        .next_entry(),
        Err(Refusal::NoRoom)
    );
    assert_eq!(
        Destination {
            incoming_unit_weight: None,
            ..destination()
        }
        .next_entry(),
        Err(Refusal::UnknownWeight)
    );
}
