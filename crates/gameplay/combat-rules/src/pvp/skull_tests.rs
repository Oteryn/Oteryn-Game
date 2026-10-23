use super::skull::*;

// Reuse the production streaming accumulator while keeping table-driven row cases concise.
fn evaluate_skull(
    now_us: i64,
    stored: SkullState,
    history: &[UnjustifiedPoint],
    increment: u16,
) -> Result<SkullState, InvalidSkullInput> {
    let mut totals = SkullHistory::new(now_us)?;
    for point in history {
        totals.include(*point)?;
    }
    super::skull::evaluate_skull(now_us, stored, &totals, increment)
}

const NOW: i64 = 100 * DAY_US;
const NONE: SkullState = SkullState {
    skull: PvpSkull::None,
    until_us: None,
};

fn points(count: usize, age: i64) -> Vec<UnjustifiedPoint> {
    vec![
        UnjustifiedPoint {
            committed_at_us: NOW - age,
            points_milli: 1_000
        };
        count
    ]
}

#[test]
fn pvp_skull_rolls_each_window_at_its_exclusive_microsecond_endpoint() {
    for (index, window) in FRAG_WINDOWS_US.into_iter().enumerate() {
        let count = usize::try_from(RED_THRESHOLDS_MILLI[index] / 1_000 - 1).unwrap_or(0);
        let inside = evaluate_skull(NOW, NONE, &points(count, window - 1), 1_000);
        assert_eq!(
            inside,
            Ok(SkullState {
                skull: PvpSkull::Red,
                until_us: Some(NOW + RED_DURATION_US)
            })
        );
        assert_eq!(
            evaluate_skull(NOW, NONE, &points(count, window), 1_000),
            Ok(NONE)
        );
    }
}

#[test]
fn pvp_skull_preserves_higher_state_and_later_deadline_and_refreshes_on_points() {
    let red = SkullState {
        skull: PvpSkull::Red,
        until_us: Some(NOW + 1),
    };
    assert_eq!(
        evaluate_skull(NOW, red, &[], 1),
        Ok(SkullState {
            until_us: Some(NOW + RED_DURATION_US),
            ..red
        })
    );
    let black = SkullState {
        skull: PvpSkull::Black,
        until_us: Some(NOW + BLACK_DURATION_US + 1),
    };
    assert_eq!(evaluate_skull(NOW, black, &points(2, 0), 1_000), Ok(black));
    assert_eq!(
        evaluate_skull(
            NOW,
            SkullState {
                until_us: Some(NOW),
                ..black
            },
            &[],
            1_000
        ),
        Ok(NONE)
    );
    assert_eq!(
        evaluate_skull(NOW, red, &points(5, 0), 1_000),
        Ok(SkullState {
            skull: PvpSkull::Black,
            until_us: Some(NOW + BLACK_DURATION_US)
        })
    );
}

#[test]
fn pvp_skull_refuses_invalid_points_state_and_overflow() {
    for increment in [0, 1_001, u16::MAX] {
        assert!(evaluate_skull(NOW, NONE, &[], increment).is_err());
    }
    for point in [
        UnjustifiedPoint {
            committed_at_us: NOW + 1,
            points_milli: 1,
        },
        UnjustifiedPoint {
            committed_at_us: -1,
            points_milli: 1,
        },
        UnjustifiedPoint {
            committed_at_us: NOW,
            points_milli: 0,
        },
        UnjustifiedPoint {
            committed_at_us: NOW,
            points_milli: 1_001,
        },
    ] {
        assert!(evaluate_skull(NOW, NONE, &[point], 1).is_err());
    }
    for state in [
        SkullState {
            skull: PvpSkull::None,
            until_us: Some(NOW),
        },
        SkullState {
            skull: PvpSkull::Red,
            until_us: None,
        },
        SkullState {
            skull: PvpSkull::Red,
            until_us: Some(-1),
        },
    ] {
        assert!(evaluate_skull(NOW, state, &[], 1).is_err());
    }
    assert!(evaluate_skull(-1, NONE, &[], 1).is_err());
    let black = SkullState {
        skull: PvpSkull::Black,
        until_us: Some(i64::MAX),
    };
    assert!(evaluate_skull(i64::MAX - 1, black, &[], 1).is_err());
}

#[test]
fn pvp_damage_credit_counts_all_damage_contributors_and_floors() {
    assert_eq!(contributor_points(PvpContributorKind::Damage, 5), 1_000);
    assert_eq!(contributor_points(PvpContributorKind::Damage, 6), 833);
    assert_eq!(contributor_points(PvpContributorKind::Damage, 80), 62);
    assert_eq!(contributor_points(PvpContributorKind::Assist, 80), 500);
}

#[test]
fn pvp_skull_ruleset_pins_constants_and_value_provenance() -> Result<(), serde_json::Error> {
    let row: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../rulesets/pvp/skulls/skull-policy.json"
    ))?;
    assert_eq!(row["pvp_type"], "OPTIONAL");
    assert_eq!(
        row["red_thresholds_milli"],
        serde_json::json!(RED_THRESHOLDS_MILLI)
    );
    assert_eq!(
        row["black_thresholds_milli"],
        serde_json::json!(BLACK_THRESHOLDS_MILLI)
    );
    assert_eq!(row["frag_windows_us"], serde_json::json!(FRAG_WINDOWS_US));
    for (key, value) in [
        ("red_duration_us", RED_DURATION_US),
        ("black_duration_us", BLACK_DURATION_US),
        ("revenge_duration_us", REVENGE_DURATION_US),
        ("kill_block_us", KILL_BLOCK_US),
        ("ledger_window_us", LEDGER_WINDOW_US),
        ("point_retention_us", POINT_RETENTION_US),
    ] {
        assert_eq!(row[key], value, "{key}");
    }
    assert_eq!(row["max_contributors"], MAX_CONTRIBUTORS);
    assert_eq!(row["assist_points_milli"], ASSIST_POINTS_MILLI);
    assert_eq!(row["damage_points_milli"], DAMAGE_POINTS_MILLI);
    assert_eq!(row["full_damage_contributors"], FULL_DAMAGE_CONTRIBUTORS);
    assert_eq!(row["provenance"]["durations"], "OFFICIAL_MANUAL");
    assert_eq!(
        row["provenance"]["thresholds_and_split"],
        "OTS_HYPOTHESIS_ONLY"
    );
    assert_eq!(
        row["provenance"]["assist_retention_cap"],
        "OTERYN_DECISION:PARITY_PENDING"
    );
    Ok(())
}
