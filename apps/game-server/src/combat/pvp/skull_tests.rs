use super::*;

const NOW: u64 = 100 * DAY_MS;

fn none() -> SkullState {
    SkullState {
        skull: PvpSkull::None,
        until_ms: None,
    }
}

fn rows(mut total_milli: u16, committed_at_ms: u64) -> Vec<SkullPoint> {
    let mut rows = Vec::new();
    while total_milli > 0 {
        let points_milli = total_milli.min(1_000);
        rows.push(SkullPoint {
            committed_at_ms,
            points_milli,
        });
        total_milli -= points_milli;
    }
    rows
}

#[test]
fn each_window_reaches_red_and_black_at_the_inclusive_milli_threshold()
-> Result<(), SkullEvaluationError> {
    // Older rows isolate the weekly/monthly thresholds from the shorter windows.
    for (age, red, black) in [
        (0, 3_000, 6_000),
        (DAY_MS, 5_000, 10_000),
        (7 * DAY_MS, 10_000, 20_000),
    ] {
        for (total, expected) in [
            (red - 1, PvpSkull::None),
            (red, PvpSkull::Red),
            (red + 1, PvpSkull::Red),
            (black - 1, PvpSkull::Red),
            (black, PvpSkull::Black),
            (black + 1, PvpSkull::Black),
        ] {
            let result = evaluate_skull(NOW, none(), &rows(total - 1, NOW - age), 1)?;
            assert_eq!(result.state.skull, expected, "age={age}, milli={total}");
            let expected_until = match expected {
                PvpSkull::None => None,
                PvpSkull::Red => Some(NOW + 30 * DAY_MS),
                PvpSkull::Black => Some(NOW + 45 * DAY_MS),
            };
            assert_eq!(result.state.until_ms, expected_until);
        }
    }
    Ok(())
}

#[test]
fn rolling_windows_include_now_and_exclude_the_exact_age_boundary()
-> Result<(), SkullEvaluationError> {
    // Endpoint convention is a detached technical assumption awaiting architect acceptance.
    let history = [
        (0, 100),
        (DAY_MS - 1, 200),
        (DAY_MS, 300),
        (7 * DAY_MS - 1, 400),
        (7 * DAY_MS, 500),
        (30 * DAY_MS - 1, 600),
        (30 * DAY_MS, 700),
        (45 * DAY_MS, 800),
    ]
    .map(|(age, points_milli)| SkullPoint {
        committed_at_ms: NOW - age,
        points_milli,
    });
    let result = evaluate_skull(NOW, none(), &history, 17)?;
    assert_eq!(result.points_milli, [317, 1_017, 2_117]);
    assert_eq!(result.state, none());
    Ok(())
}

#[test]
fn the_new_increment_is_counted_once_in_every_window() -> Result<(), SkullEvaluationError> {
    let result = evaluate_skull(NOW, none(), &[], 500)?;
    assert_eq!(result.points_milli, [500; 3]);
    assert_eq!(result.state, none());
    let result = evaluate_skull(NOW, none(), &rows(2_000, NOW), 1_000)?;
    assert_eq!(result.points_milli, [3_000; 3]);
    assert_eq!(result.state.skull, PvpSkull::Red);
    Ok(())
}

#[test]
fn new_points_refresh_an_active_red_or_black_even_below_all_thresholds()
-> Result<(), SkullEvaluationError> {
    for (skull, days) in [(PvpSkull::Red, 30), (PvpSkull::Black, 45)] {
        let current = SkullState {
            skull,
            until_ms: Some(NOW + 1),
        };
        let result = evaluate_skull(NOW, current, &[], 1)?;
        assert_eq!(
            result.state,
            SkullState {
                skull,
                until_ms: Some(NOW + days * DAY_MS)
            }
        );
    }
    Ok(())
}

#[test]
fn an_active_black_never_downgrades_to_red_and_uses_the_black_duration()
-> Result<(), SkullEvaluationError> {
    let current = SkullState {
        skull: PvpSkull::Black,
        until_ms: Some(NOW + DAY_MS),
    };
    let result = evaluate_skull(NOW, current, &rows(2_999, NOW), 1)?;
    assert_eq!(result.state.skull, PvpSkull::Black);
    assert_eq!(result.state.until_ms, Some(NOW + 45 * DAY_MS));
    Ok(())
}

#[test]
fn an_active_red_upgrades_to_black() -> Result<(), SkullEvaluationError> {
    let current = SkullState {
        skull: PvpSkull::Red,
        until_ms: Some(NOW + DAY_MS),
    };
    let result = evaluate_skull(NOW, current, &rows(5_999, NOW), 1)?;
    assert_eq!(result.state.skull, PvpSkull::Black);
    assert_eq!(result.state.until_ms, Some(NOW + 45 * DAY_MS));
    Ok(())
}

#[test]
fn expiry_is_effective_at_the_deadline_but_not_one_millisecond_before()
-> Result<(), SkullEvaluationError> {
    for skull in [PvpSkull::Red, PvpSkull::Black] {
        for (until, expected) in [
            (NOW - 1, PvpSkull::None),
            (NOW, PvpSkull::None),
            (NOW + 1, skull),
        ] {
            let current = SkullState {
                skull,
                until_ms: Some(until),
            };
            assert_eq!(evaluate_skull(NOW, current, &[], 1)?.state.skull, expected);
        }
    }
    Ok(())
}

#[test]
fn an_expired_black_does_not_prevent_a_new_red_evaluation() -> Result<(), SkullEvaluationError> {
    let current = SkullState {
        skull: PvpSkull::Black,
        until_ms: Some(NOW),
    };
    let result = evaluate_skull(NOW, current, &rows(2_999, NOW), 1)?;
    assert_eq!(result.state.skull, PvpSkull::Red);
    assert_eq!(result.state.until_ms, Some(NOW + 30 * DAY_MS));
    Ok(())
}

#[test]
fn zero_or_oversized_new_points_cannot_refresh_a_skull() {
    let current = SkullState {
        skull: PvpSkull::Black,
        until_ms: Some(NOW + 1),
    };
    for milli in [0, 1_001, u16::MAX] {
        assert_eq!(
            evaluate_skull(NOW, current, &[], milli),
            Err(SkullEvaluationError::InvalidPoints)
        );
    }
}

#[test]
fn malformed_state_or_rows_and_future_rows_are_refused_independently() {
    for current in [
        SkullState {
            skull: PvpSkull::None,
            until_ms: Some(NOW + 1),
        },
        SkullState {
            skull: PvpSkull::Red,
            until_ms: None,
        },
        SkullState {
            skull: PvpSkull::Black,
            until_ms: None,
        },
    ] {
        assert_eq!(
            evaluate_skull(NOW, current, &[], 1),
            Err(SkullEvaluationError::InvalidState)
        );
    }
    for points_milli in [0, 1_001, u16::MAX] {
        let history = [SkullPoint {
            committed_at_ms: NOW,
            points_milli,
        }];
        assert_eq!(
            evaluate_skull(NOW, none(), &history, 1),
            Err(SkullEvaluationError::InvalidPoints)
        );
    }
    let history = [SkullPoint {
        committed_at_ms: NOW + 1,
        points_milli: 1,
    }];
    assert_eq!(
        evaluate_skull(NOW, none(), &history, 1),
        Err(SkullEvaluationError::FuturePoint)
    );
}

#[test]
fn timestamp_zero_does_not_underflow_and_deadline_overflow_refuses()
-> Result<(), SkullEvaluationError> {
    let history = [SkullPoint {
        committed_at_ms: 0,
        points_milli: 1_000,
    }];
    assert_eq!(
        evaluate_skull(0, none(), &history, 1_000)?.points_milli,
        [2_000; 3]
    );
    let current = SkullState {
        skull: PvpSkull::Red,
        until_ms: Some(u64::MAX),
    };
    assert_eq!(
        evaluate_skull(u64::MAX - 1, current, &[], 1),
        Err(SkullEvaluationError::DeadlineOverflow)
    );
    assert_eq!(
        evaluate_skull(u64::MAX, none(), &rows(2_999, u64::MAX), 1),
        Err(SkullEvaluationError::DeadlineOverflow)
    );
    assert_eq!(evaluate_skull(u64::MAX, none(), &[], 1)?.state, none());
    Ok(())
}

#[test]
fn identical_and_reordered_inputs_are_deterministic_without_mutation()
-> Result<(), SkullEvaluationError> {
    let mut history = rows(4_000, NOW - DAY_MS);
    history.extend(rows(9_000, NOW - 7 * DAY_MS));
    history.extend(rows(1_500, NOW));
    let original = history.clone();
    let first = evaluate_skull(NOW, none(), &history, 500)?;
    assert_eq!(history, original);
    assert_eq!(evaluate_skull(NOW, none(), &history, 500)?, first);
    history.reverse();
    assert_eq!(evaluate_skull(NOW, none(), &history, 500)?, first);
    Ok(())
}
