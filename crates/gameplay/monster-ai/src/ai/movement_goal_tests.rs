#![allow(clippy::expect_used)]
use super::*;

#[test]
fn near_counter_saturates_decays_and_never_doubles_flee() {
    let actor = crate::ai_think::think_tests::movement_fixtures::actor();
    let at = MovementLocalPosition {
        x: 0,
        y: 0,
        floor: 7,
    };
    let mut near = NearCounter::default();
    let close = MovementGoal::Chase { actor, at };
    let far = MovementGoal::KeepDistance {
        actor,
        at: MovementLocalPosition { x: 9, ..at },
        distance: 4,
    };
    assert!(near.evaluate(at, close));
    assert!(near.evaluate(at, close));
    assert!(near.evaluate(at, close));
    assert!(near.evaluate(at, far));
    assert!(!near.evaluate(at, far));
    assert!(!near.evaluate(at, MovementGoal::Flee { actor, at }));
}

#[test]
fn dance_extremes_keep_adjacency_and_flee_stops_at_eleven() {
    let actor = crate::ai_think::think_tests::movement_fixtures::actor();
    let at = MovementLocalPosition {
        x: 0,
        y: 0,
        floor: 7,
    };
    let goal = MovementGoal::Chase {
        actor,
        at: MovementLocalPosition { x: 1, ..at },
    };
    assert_eq!(goal.local_step(at, 1_000_000, 999_999, |_| true), None);
    let step = goal.local_step(at, 0, 0, |_| true).expect("dance");
    assert_eq!(
        chebyshev(
            destination(at, step).expect("step"),
            goal.target().expect("target").1
        ),
        1
    );
    let goal = MovementGoal::Flee {
        actor,
        at: MovementLocalPosition { x: 11, ..at },
    };
    assert_eq!(goal.local_step(at, 0, 0, |_| true), None);
}

#[test]
fn keep_distance_four_flee_corner_and_wander_radius_are_local_goal_rules() {
    let actor = crate::ai_think::think_tests::movement_fixtures::actor();
    let origin = MovementLocalPosition {
        x: 0,
        y: 0,
        floor: 7,
    };
    let target = MovementLocalPosition { x: 8, ..origin };
    let goal = MovementGoal::KeepDistance {
        actor,
        at: target,
        distance: 4,
    };
    let mut at = origin;
    for _ in 0..8 {
        if let Some(step) = goal.local_step(at, 0, 0, |_| true) {
            at = destination(at, step).expect("step");
        }
    }
    assert_eq!(chebyshev(at, target), 4);
    let flee = MovementGoal::Flee {
        actor,
        at: MovementLocalPosition { x: 2, ..origin },
    };
    assert_eq!(
        flee.local_step(origin, 0, 0, |_| true),
        Some(CardinalStep::West)
    );
    let step = flee
        .local_step(origin, 0, 0, |next| next.x == 0)
        .expect("cornered side step");
    assert_eq!(
        chebyshev(
            destination(origin, step).expect("side step"),
            flee.target().expect("target").1
        ),
        2
    );
    at = origin;
    let wander = MovementGoal::Wander {
        anchor: origin,
        radius: 1,
        interval_ms: 2000,
    };
    for draw in 0..64 {
        if let Some(step) = wander.local_step(at, 0, draw, |_| true) {
            at = destination(at, step).expect("wander");
        }
        assert!(chebyshev(at, origin) <= 1);
    }
}
