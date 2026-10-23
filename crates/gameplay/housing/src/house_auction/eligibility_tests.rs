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
