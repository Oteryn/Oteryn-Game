use super::*;

#[test]
fn near_duration_is_doubled_speed_zero_stands_and_replay_keeps_draws_and_steps() {
    use crate::movement::speed::{
        EngineeringGroundSpeed, creature_speed_fixture::Native, creature_step_duration,
    };
    let mut traces = Vec::new();
    for _ in 0..2 {
        let mut channel = Channel::new();
        let creature = channel.admit(at(114, 100), 0);
        let mut behavior = rat(1);
        behavior.targeting.static_attack_chance_ppm = 0;
        channel
            .table
            .entry_mut(creature)
            .expect("entry")
            .state
            .profile =
            CreatureBehaviourProfile::project(Some(&behavior), &abilities()).expect("profile");
        channel.thinks(0, true);
        let source = Native {
            actor: creature,
            base: 67,
            pin: channel.runtime.content_pin().server_artifact_digest(),
        };
        let entry = channel.table.entry(creature).expect("entry");
        let next = entry.motion.prepared.expect("dance destination").1;
        let duration = creature_step_duration(
            &channel.runtime,
            creature,
            &source,
            &EngineeringGroundSpeed,
            crate::content::LogicalCell {
                x: next.x,
                y: next.y,
                z: 7,
            },
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .expect("native duration");
        assert_eq!(
            entry.motion.due,
            u64::try_from(duration.as_micros() * 2).ok()
        );
        let mut trace = Vec::new();
        for _ in 0..8 {
            let due = channel
                .table
                .entry(creature)
                .expect("entry")
                .motion
                .due
                .expect("dance deadline");
            channel.advance_movement(due, true);
            trace.push(channel.position(creature));
            assert_eq!(
                crate::movement::path_search::chebyshev(
                    channel.position(creature),
                    channel.position(channel.player)
                ),
                1
            );
        }
        traces.push(trace);
        // Reuse the same exact-source fixture at base zero: the next prepared step is refused.
        let source = Native { base: 0, ..source };
        let due = channel
            .table
            .entry(creature)
            .expect("entry")
            .motion
            .due
            .expect("deadline");
        let before = channel.position(creature);
        let navigation = channel.runtime.content_pin().map_revision_digest();
        channel.table.drive_movement(
            &mut channel.runtime,
            &channel.cells,
            &BTreeSet::new(),
            navigation,
            &source,
            &EngineeringGroundSpeed,
            &channel.root,
            due,
            &mut channel.path_budget,
            false,
        );
        assert_eq!(channel.position(creature), before);
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
    assert_eq!(
        traces[0], traces[1],
        "same seed, routes, timer order and AI_DANCE draws"
    );
}

#[test]
fn wander_waits_its_interval_and_repeats_on_the_step_timer_inside_the_radius() {
    for interval_ms in [950, 2000] {
        let mut channel = Channel::new();
        let creature = channel.admit(at(110, 100), 0);
        let mut behavior = rat(1);
        behavior.targeting.can_target = false;
        behavior.movement.wander = Some(crate::content::ProjectV2Wander {
            interval_ms,
            radius_tiles: 4,
        });
        channel
            .table
            .entry_mut(creature)
            .expect("entry")
            .state
            .profile = CreatureBehaviourProfile::project(Some(&behavior), &abilities())
            .expect("wander profile");
        channel.thinks(0, true);
        let due = channel
            .table
            .entry(creature)
            .expect("entry")
            .motion
            .due
            .expect("wander timer");
        assert_eq!(due, interval_ms * 1000);
        channel.advance_movement(due - 1, true);
        assert_eq!(channel.position(creature), at(110, 100));
        channel.advance_movement(due, true);
        let first = channel.position(creature);
        assert_ne!(first, at(110, 100));
        let next = channel
            .table
            .entry(creature)
            .expect("entry")
            .motion
            .due
            .expect("second wander");
        assert_eq!(next, 2 * interval_ms * 1000);
        channel.advance_movement(next, true);
        assert_ne!(channel.position(creature), first);
        assert!(
            crate::movement::path_search::chebyshev(channel.position(creature), at(110, 100)) <= 4
        );
    }
}

#[test]
fn a_local_step_is_bound_to_its_start_and_clock_overflow_never_moves_it() {
    use crate::movement::speed::{EngineeringGroundSpeed, creature_speed_fixture::Native};
    for relocate in [true, false] {
        let mut channel = Channel::new();
        let creature = channel.admit(at(114, 100), 0);
        let mut behavior = rat(1);
        behavior.targeting.static_attack_chance_ppm = 0;
        channel
            .table
            .entry_mut(creature)
            .expect("entry")
            .state
            .profile = CreatureBehaviourProfile::project(Some(&behavior), &abilities())
            .expect("dance profile");
        channel.thinks(0, true);
        let due = channel
            .table
            .entry(creature)
            .expect("entry")
            .motion
            .due
            .expect("local step");
        if relocate {
            let snapshot = channel
                .runtime
                .read_actor_position(creature)
                .expect("before external move");
            channel
                .runtime
                .borrow_movement_position()
                .commit_cardinal(snapshot, at(113, 100))
                .expect("another owner input");
        }
        let before = channel
            .runtime
            .read_actor_position(creature)
            .expect("position before timer");
        let source = Native {
            actor: creature,
            base: 67,
            pin: channel.runtime.content_pin().server_artifact_digest(),
        };
        let navigation = channel.runtime.content_pin().map_revision_digest();
        channel.table.drive_movement(
            &mut channel.runtime,
            &channel.cells,
            &BTreeSet::new(),
            navigation,
            &source,
            &EngineeringGroundSpeed,
            &channel.root,
            if relocate { due } else { u64::MAX },
            &mut channel.path_budget,
            false,
        );
        assert_eq!(
            channel
                .runtime
                .read_actor_position(creature)
                .expect("after timer"),
            before
        );
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
fn base_zero_stands_before_admitting_a_search_or_step_timer() {
    use crate::movement::speed::{EngineeringGroundSpeed, creature_speed_fixture::Native};
    let mut channel = Channel::new();
    let creature = channel.admit(at(110, 100), 0);
    let players = [TargetCandidate {
        actor: channel.player,
        position: channel.position(channel.player),
        eligible: true,
        invisible: false,
        health: 100,
        damage: 0,
    }];
    channel.table.run_due_thinks(
        &mut channel.runtime,
        0,
        &channel.root,
        &channel.revisions,
        &players,
    );
    let source = Native {
        actor: creature,
        base: 0,
        pin: channel.runtime.content_pin().server_artifact_digest(),
    };
    let navigation = channel.runtime.content_pin().map_revision_digest();
    channel.table.drive_movement(
        &mut channel.runtime,
        &channel.cells,
        &BTreeSet::new(),
        navigation,
        &source,
        &EngineeringGroundSpeed,
        &channel.root,
        0,
        &mut channel.path_budget,
        false,
    );
    let motion = &channel.table.entry(creature).expect("entry").motion;
    assert_eq!(
        motion.goal,
        crate::ai_think::movement_goal::MovementGoal::Stand
    );
    assert!(motion.pending.is_none() && motion.due.is_none());
    assert_eq!(channel.path_budget.charged_work(), 0);
    assert_eq!(channel.position(creature), at(110, 100));
}
