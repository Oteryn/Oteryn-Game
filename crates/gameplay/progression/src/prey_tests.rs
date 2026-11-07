use super::*;

// Each row corresponds to a row in HUNT-1's Reference parity table.
// https://github.com/opentibiabr/canary/blob/52550d5e3c0ffe565f4bd1014f4e5575abf1059b/src/io/ioprey.cpp
#[test]
fn prey_reference_bonus_grades() {
    let rows = [
        (BonusType::Damage, [7, 9, 11, 13, 15, 17, 19, 21, 23, 25]),
        (
            BonusType::Reduction,
            [12, 14, 16, 18, 20, 22, 24, 26, 28, 30],
        ),
        (
            BonusType::Experience,
            [13, 16, 19, 22, 25, 28, 31, 34, 37, 40],
        ),
        (BonusType::Loot, [13, 16, 19, 22, 25, 28, 31, 34, 37, 40]),
    ];
    for (kind, values) in rows {
        for (grade, expected) in (1..=10).zip(values) {
            assert_eq!(
                PreyBonus::try_from((kind, grade)),
                Ok(PreyBonus {
                    kind,
                    grade,
                    percent: expected,
                })
            );
        }
        for grade in [0, 11, u8::MAX] {
            assert_eq!(
                PreyBonus::try_from((kind, grade)),
                Err(PreyRefusal::InvalidBonus)
            );
        }
    }
}

#[test]
fn prey_reference_slot_unlocks() {
    // PREY0 §5.1 narrows Canary: no config/free-third-slot bypass and native Premium.
    let rows = [
        (1, false, false, Ok(UnlockedSlot(1))),
        (2, false, false, Err(PreyRefusal::SlotLocked)),
        (2, true, false, Ok(UnlockedSlot(2))),
        (2, false, true, Ok(UnlockedSlot(2))),
        (3, true, false, Err(PreyRefusal::SlotLocked)),
        (3, false, true, Ok(UnlockedSlot(3))),
    ];
    for (slot, current, permanent, expected) in rows {
        let facts = SlotAccess {
            slot,
            has_vocation: true,
            premium: PremiumGate::from_fence_read(current),
            permanent,
        };
        assert_eq!(UnlockedSlot::try_from(facts), expected);
        assert_eq!(
            UnlockedSlot::try_from(SlotAccess {
                has_vocation: false,
                ..facts
            }),
            Err(PreyRefusal::NotEligible)
        );
    }
    for slot in [0, 4, u8::MAX] {
        assert_eq!(
            UnlockedSlot::try_from(SlotAccess {
                slot,
                has_vocation: false,
                premium: PremiumGate::CLOSED,
                permanent: true
            }),
            Err(PreyRefusal::InvalidSlot)
        );
    }
}

#[test]
fn prey_reference_reroll_timing_and_gold() {
    let rows = [
        (99, 100, 1, 200, 100),
        (99, 100, 100, 20_000, 100),
        (100, 100, 100, 0, 72_100),
        (101, 100, 100, 0, 72_101),
        (0, 1, u32::MAX, 858_993_459_000, 1),
    ];
    for (now, free_reroll_at, level, gold, next) in rows {
        assert_eq!(
            RerollQuote::try_from(ListReroll {
                now,
                free_reroll_at,
                level
            }),
            Ok(RerollQuote {
                gold,
                free_reroll_at: next
            })
        );
    }
    assert_eq!(
        RerollQuote::try_from(ListReroll {
            now: u64::MAX,
            free_reroll_at: 0,
            level: 1
        }),
        Err(PreyRefusal::InvalidTime)
    );
    let last = i64::MAX.unsigned_abs();
    assert_eq!(
        RerollQuote::try_from(ListReroll {
            now: last - FREE_REROLL_SECONDS,
            free_reroll_at: 0,
            level: 1,
        }),
        Ok(RerollQuote {
            gold: 0,
            free_reroll_at: last
        })
    );
    for (now, free_reroll_at) in [(last, 0), (0, last + 1)] {
        assert_eq!(
            RerollQuote::try_from(ListReroll {
                now,
                free_reroll_at,
                level: 1
            }),
            Err(PreyRefusal::InvalidTime)
        );
    }
    assert_eq!(
        RerollQuote::try_from(ListReroll {
            now: 0,
            free_reroll_at: 0,
            level: 0
        }),
        Err(PreyRefusal::InvalidLevel)
    );
}

#[test]
fn prey_reference_wildcard_costs_and_option_checks() {
    let rows = [
        (WildcardSpend::BonusReroll, 1),
        (WildcardSpend::SelectAny, 5),
        (WildcardSpend::AutoReroll, 1),
        (WildcardSpend::Lock, 5),
    ];
    for (spend, cost) in rows {
        // Exhaustive bounded conservation/underflow checks, without a new test dependency.
        for before in 0..=WILDCARD_CAP {
            let result = WildcardDebit::try_from((before, spend));
            if before < cost {
                assert_eq!(result, Err(PreyRefusal::NotEnoughWildcards));
            } else {
                assert_eq!(
                    result,
                    Ok(WildcardDebit {
                        before,
                        cost,
                        after: before - cost
                    })
                );
            }
        }
        assert_eq!(
            WildcardDebit::try_from((53, spend)),
            Err(PreyRefusal::InvalidBalance)
        );
    }
    for (option, required) in [
        (PreyOption::None, 0),
        (PreyOption::AutoReroll, 1),
        (PreyOption::Lock, 5),
    ] {
        for balance in 0..=WILDCARD_CAP {
            assert_eq!(
                PreyOption::try_from((option, balance)),
                if balance < required {
                    Err(PreyRefusal::NotEnoughWildcards)
                } else {
                    Ok(option)
                }
            );
        }
        assert_eq!(
            PreyOption::try_from((option, 53)),
            Err(PreyRefusal::InvalidBalance)
        );
    }
}

#[test]
fn prey_reference_storage_bounds_and_closed_codes() {
    assert_eq!(
        (
            SLOT_COUNT,
            LIST_SIZE,
            CREATURE_KEY_BYTES,
            HUNTING_SECONDS,
            WILDCARD_CAP
        ),
        (3, 9, 256, 7_200, 52)
    );
    for (code, kind) in [
        ("DAMAGE", BonusType::Damage),
        ("REDUCTION", BonusType::Reduction),
        ("XP", BonusType::Experience),
        ("LOOT", BonusType::Loot),
    ] {
        assert_eq!(BonusType::try_from(code), Ok(kind));
    }
    for (code, option) in [
        ("NONE", PreyOption::None),
        ("AUTO_REROLL", PreyOption::AutoReroll),
        ("LOCK", PreyOption::Lock),
    ] {
        assert_eq!(PreyOption::try_from(code), Ok(option));
    }
    assert_eq!(
        BonusType::try_from("UNKNOWN"),
        Err(PreyRefusal::InvalidBonus)
    );
    assert_eq!(
        PreyOption::try_from("UNKNOWN"),
        Err(PreyRefusal::InvalidOption)
    );
}
