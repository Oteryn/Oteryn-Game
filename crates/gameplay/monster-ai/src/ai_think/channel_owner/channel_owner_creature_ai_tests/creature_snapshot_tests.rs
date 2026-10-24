//! A round trip keeps the cell but changes its independent Movement-owner revision.
use super::*;
#[test]
fn retained_step_refuses_actor_or_target_revision_change_even_after_a_round_trip() {
    for move_target in [false, true] {
        let mut channel = Channel::new();
        let creature = channel.admit(at(110, 100), 0);
        channel.thinks(0, true);
        let before = channel.position(creature);
        let due = channel
            .table
            .entry(creature)
            .expect("entry")
            .motion
            .due
            .expect("step");
        let moved = if move_target {
            channel.player
        } else {
            creature
        };
        let snapshot = channel
            .runtime
            .read_actor_position(moved)
            .expect("snapshot");
        let from = snapshot.position();
        let away = MovementLocalPosition {
            x: from.x + 1,
            ..from
        };
        let moved_snapshot = channel
            .runtime
            .borrow_movement_position()
            .commit_cardinal(snapshot, away)
            .expect("away");
        let returned = channel
            .runtime
            .borrow_movement_position()
            .commit_cardinal(moved_snapshot, from)
            .expect("back");
        assert_eq!(snapshot.position(), returned.position());
        assert_eq!(returned.revision(), snapshot.revision() + 2);
        channel.advance_movement(due, true);
        assert_eq!(
            channel.position(creature),
            before,
            "a changed snapshot is not a grant to consume the retained route"
        );
        let motion = &channel.table.entry(creature).expect("entry").motion;
        assert!(motion.due.is_none() && motion.route.is_none());
    }
}
