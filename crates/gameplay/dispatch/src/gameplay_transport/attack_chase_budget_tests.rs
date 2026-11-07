//! Resumed drains reuse the existing two-chaser dispatch harness, including its real A* routes.
use super::*;

fn routes(
    order: [usize; 2],
    suspend: bool,
    commit: bool,
) -> (
    [Option<crate::movement::CardinalStep>; 2],
    Option<MovementLocalPosition>,
) {
    let (mut owner, chasers) = two_chasers();
    let room = &owner.room;
    let none = std::collections::BTreeSet::new();
    for index in order {
        let (actor, session) = chasers[index];
        assert_eq!(
            owner
                .attack
                .chase_decision(&owner.runtime, room, actor, session, at(T0), |cell| {
                    chase_path_tile(room.movement_cells(), &none, cell)
                }),
            None
        );
    }
    let now = T0 + crate::combat_attack::CHASE_WINDOW_MICROS;
    if suspend {
        assert!(owner.attack.path_budget.admits(now, true, true));
        owner.attack.path_budget.declare_creatures(64);
        for _ in 0..15 {
            owner.attack.path_budget.charge(true, 1024);
        }
    }
    let mut steps = [None; 2];
    for phase in 0..2 {
        if phase == 1 {
            owner.attack.path_budget.declare_creatures(0);
        }
        for index in order {
            if steps[index].is_some() {
                continue;
            }
            let (actor, session) = chasers[index];
            steps[index] = owner.attack.chase_decision(
                &owner.runtime,
                room,
                actor,
                session,
                at(now),
                |cell| chase_path_tile(room.movement_cells(), &none, cell),
            );
            if let Some(step) = steps[index].filter(|_| commit) {
                let mut movement = owner.runtime.borrow_movement_position();
                let before = movement.read(actor).expect("chaser");
                let mut next = before.position();
                let (dx, dy) = crate::movement::path_search::delta(step);
                next.x += dx;
                next.y += dy;
                movement
                    .commit_cardinal(before, next)
                    .expect("Movement commits chosen step");
            }
        }
        if suspend && phase == 0 {
            assert!(steps[0].is_some() && steps[1].is_none());
        }
    }
    let (actor, session) = chasers[1];
    let following = owner.attack.entries
        [owner.attack.index(actor, session).expect("second chaser")]
    .chase
    .next_cell();
    (steps, following)
}

#[test]
fn resumed_drains_keep_planned_occupancy_in_both_caller_orders_before_and_after_commit() {
    for commit in [false, true] {
        let uninterrupted = routes([0, 1], false, commit);
        assert!(uninterrupted.0.iter().all(Option::is_some));
        assert!(
            uninterrupted.1.is_some(),
            "also observe the following waypoint"
        );
        for order in [[0, 1], [1, 0]] {
            assert_eq!(routes(order, false, commit), uninterrupted);
            assert_eq!(routes(order, true, commit), uninterrupted);
        }
    }
}
