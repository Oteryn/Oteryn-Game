use super::*;

#[test]
fn every_eligibility_input_and_level_boundary_has_a_stable_reason() {
    let valid = Eligibility {
        active: true,
        account_matches: true,
        world_matches: true,
        level: 20,
        junior: false,
        slot_occupied: false,
        banned: false,
        premium_allowed: true,
    };
    assert_eq!(valid.refusal(), 0);
    for (changed, reason) in [
        (
            Eligibility {
                active: false,
                ..valid
            },
            1,
        ),
        (
            Eligibility {
                account_matches: false,
                ..valid
            },
            1,
        ),
        (
            Eligibility {
                world_matches: false,
                ..valid
            },
            1,
        ),
        (
            Eligibility {
                junior: true,
                ..valid
            },
            2,
        ),
        (Eligibility { level: 19, ..valid }, 3),
        (
            Eligibility {
                slot_occupied: true,
                ..valid
            },
            4,
        ),
        (
            Eligibility {
                banned: true,
                ..valid
            },
            5,
        ),
        (
            Eligibility {
                premium_allowed: false,
                ..valid
            },
            6,
        ),
    ] {
        assert_eq!(changed.refusal(), reason);
    }
}

/// S05 parity with Crystal `201189f8` `data/scripts/talkactions/player/buy_house.lua`: a Free
/// account is refused (`isPremium`), as is an account that already holds its house slot
/// (`houseOwnedByAccount`/`MAX_HOUSES_LIMIT`). Recorded divergence: Crystal's level gate is the
/// configured `houseBuyLevel` (`config.lua.dist` 100, 0 disables it); Oteryn refuses below 20.
#[test]
fn s05_crystal_purchase_gates() {
    let valid = Eligibility {
        active: true,
        account_matches: true,
        world_matches: true,
        level: 100,
        junior: false,
        slot_occupied: false,
        banned: false,
        premium_allowed: true,
    };
    assert_eq!(valid.refusal(), 0);
    let free = Eligibility {
        premium_allowed: false,
        ..valid
    };
    assert_eq!(free.refusal(), 6);
    let second = Eligibility {
        slot_occupied: true,
        ..valid
    };
    assert_eq!(second.refusal(), 4);
    for (level, reason) in [(19, 3), (20, 0), (99, 0)] {
        assert_eq!(
            Eligibility { level, ..valid }.refusal(),
            reason,
            "level {level}"
        );
    }
}
