//! C08 parity: the decreasing damage list against Crystal `201189f8`
//! `ConditionDamage::generateDamageList` and `ConditionDamage::init`
//! (`src/creatures/combat/condition.cpp`, checked against Canary `47dfd51f`). The oracle below is
//! a test-only restatement of the source arithmetic (int32 median, float division, double fabs),
//! not production code.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::super::{DecisionOccurrenceId, GameplayDecisionRoot};
use super::*;

/// Crystal `generateDamageList(amount, start)`: one tick value per list entry.
fn crystal_damage_list(amount: i32, start: i32) -> Vec<i32> {
    let mut list = Vec::new();
    let mut sum = 0_i32;
    for i in (1..=start).rev() {
        let n = start + 1 - i;
        let med = (n * amount) / start;
        loop {
            sum += i;
            list.push(i);
            let x1 = (1.0_f64 - f64::from((sum as f32 + i as f32) / med as f32)).abs();
            let x2 = (1.0_f64 - f64::from(sum as f32 / med as f32)).abs();
            // C++ `do { ... } while (x1 < x2)`.
            if x1 < x2 {
                continue;
            }
            break;
        }
    }
    list
}

/// Crystal `ConditionDamage::init` start rule for a drawn `amount` and a configured `start`.
fn crystal_start(amount: i32, configured: i32, max_damage: i32) -> i32 {
    if configured > max_damage {
        max_damage
    } else if configured == 0 {
        1.max((f64::from(amount) / 20.0).ceil() as i32)
    } else {
        configured
    }
}

fn expand(segments: &[DamageSegment]) -> Vec<i32> {
    segments
        .iter()
        .flat_map(|s| std::iter::repeat_n(s.amount as i32, s.count as usize))
        .collect()
}

#[test]
fn c08_decreasing_list_equals_crystal_generate_damage_list() {
    for amount in 1..=MAX_DECREASING_AMOUNT {
        let starts: &[u32] = if amount <= 400 {
            &[]
        } else {
            &[1, 2, 3, 5, 7, 10, 13, 20, 32, 50, 64, 100, 125, 128]
        };
        let all: Vec<u32> = (1..=MAX_SEGMENTS as u32).collect();
        for &start in if starts.is_empty() { &all[..] } else { starts } {
            let segments = decreasing(amount, start, 4_000).expect("bounded start");
            assert!(segments.iter().all(|s| s.interval_ms == 4_000));
            assert_eq!(
                expand(&segments),
                crystal_damage_list(amount as i32, start as i32),
                "amount {amount} start {start}"
            );
        }
    }
}

#[test]
fn c08_compiled_default_and_clamped_start_equal_crystal_init() {
    let root = GameplayDecisionRoot::from_bytes([7; 32]);
    let facts = ApplicationFacts {
        now: 0,
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([1; 16]),
    };
    for amount in 1..=MAX_DECREASING_AMOUNT {
        for initial in [None, Some(1), Some(5), Some(amount), Some(amount + 1)] {
            let schedule = DamageSchedule::Decreasing {
                delayed: false,
                minimum: amount,
                maximum: amount,
                initial,
                interval_ms: 4_000,
            };
            let configured = initial.unwrap_or(0) as i32;
            let start = crystal_start(amount as i32, configured, amount as i32);
            if start > MAX_SEGMENTS as i32 {
                assert!(!schedule.validate(), "amount {amount} start {start}");
                continue;
            }
            let cursor = schedule.compile(&facts).expect("valid schedule");
            let expected = crystal_damage_list(amount as i32, start);
            assert_eq!(
                expand(&cursor.segments),
                expected,
                "amount {amount} {initial:?}"
            );
            assert_eq!(cursor.remaining as i32, expected.iter().sum::<i32>());
        }
    }
}
