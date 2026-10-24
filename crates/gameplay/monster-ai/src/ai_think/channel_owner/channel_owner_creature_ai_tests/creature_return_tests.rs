use super::*;

#[test]
fn walk_back_reaches_spawn_and_idles_while_a_failed_route_waits_for_wake() {
    for walkable in [true, false] {
        let mut channel = Channel::new();
        let creature = channel.admit(at(90, 100), 0);
        let spawn = at(86, 100);
        channel
            .table
            .entry_mut(creature)
            .expect("entry")
            .state
            .spawn = Some(spawn);
        let reports = channel.thinks(0, walkable);
        assert!(!reports[0].1.idle, "the walk-back goal retains its think");
        for _ in 0..8 {
            let Some(due) = channel.table.entry(creature).expect("entry").motion.due else {
                break;
            };
            channel.advance_movement(due, walkable);
        }
        assert_eq!(
            channel.position(creature),
            if walkable { spawn } else { at(90, 100) }
        );
        assert_eq!(channel.table.due(creature), None);
        assert!(
            channel
                .table
                .entry(creature)
                .expect("entry")
                .motion
                .due
                .is_none()
        );
    }
}

#[test]
fn leash_relocates_once_or_idles_in_place_if_the_spawn_is_occupied() {
    for occupied in [false, true] {
        let mut channel = Channel::new();
        let creature = channel.admit(at(50, 100), 0);
        let spawn = if occupied {
            channel.position(channel.player)
        } else {
            at(105, 100)
        };
        channel
            .table
            .entry_mut(creature)
            .expect("entry")
            .state
            .spawn = Some(spawn);
        let before = channel
            .runtime
            .read_actor_position(creature)
            .expect("before");
        channel.thinks(0, true);
        let after = channel
            .runtime
            .read_actor_position(creature)
            .expect("after");
        assert_eq!(
            after.position(),
            if occupied { before.position() } else { spawn }
        );
        assert_eq!(after.revision(), before.revision() + u64::from(!occupied));
        assert_eq!(channel.table.due(creature), None);
        assert!(
            channel
                .table
                .entry(creature)
                .expect("entry")
                .motion
                .due
                .is_none()
        );
    }
}
