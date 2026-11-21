use super::*;

#[test]
fn retained_movement_cutoff_keeps_the_original_timer_and_uses_current_cadence() {
    use crate::movement::speed::{EngineeringGroundSpeed, creature_speed_fixture::Native};
    let mut channel = Channel::new();
    let creature = channel.admit(at(110, 100), 0);
    channel.thinks(0, true);
    let motion = &channel.table.entry(creature).expect("entry").motion;
    let due = motion.due.expect("actual scheduled movement");
    let prepared = motion.prepared;
    let cursor = motion.cursor;
    let before = channel.position(creature);
    let execution_now = due.checked_add(2 * SECOND).expect("execution clock");
    channel
        .runtime
        .note_rotten_movement_time(execution_now)
        .expect("current physical movement clock");
    let source = Native {
        actor: creature,
        base: 67,
        pin: channel.runtime.content_pin().server_artifact_digest(),
    };
    channel.table.fire_steps_through(
        &mut channel.runtime,
        &channel.cells,
        &BTreeSet::new(),
        None,
        &source,
        &EngineeringGroundSpeed,
        &channel.root,
        due.checked_sub(1).expect("earlier retained cutoff"),
        execution_now,
    );
    assert_eq!(channel.position(creature), before);
    let motion = &channel.table.entry(creature).expect("entry").motion;
    assert_eq!(motion.due, Some(due));
    assert_eq!(motion.prepared, prepared);
    assert_eq!(motion.cursor, cursor);
    assert_eq!(
        channel
            .table
            .step_timers
            .as_ref()
            .expect("lane")
            .pending_len(),
        1
    );

    channel.table.fire_steps_through(
        &mut channel.runtime,
        &channel.cells,
        &BTreeSet::new(),
        None,
        &source,
        &EngineeringGroundSpeed,
        &channel.root,
        due,
        execution_now,
    );
    assert_ne!(channel.position(creature), before);
    assert_eq!(
        channel
            .runtime
            .committed_movement_occurrence(creature)
            .expect("actual actor")
            .expect("real translation")
            .observed_us(),
        execution_now
    );
    assert!(
        channel
            .table
            .entry(creature)
            .expect("entry")
            .motion
            .due
            .is_some_and(|next| next > execution_now),
        "the next cadence is calculated from current execution, never the old cutoff"
    );
}

#[test]
fn creature_steps_on_its_own_deadline_and_refusal_waits_for_the_next_think() {
    let mut channel = Channel::new();
    let creature = channel.admit(at(110, 100), 0);
    let before = channel.position(creature);
    channel.thinks(0, true);
    assert_eq!(channel.position(creature), before, "a think never steps");
    let due = channel
        .table
        .entry(creature)
        .expect("entry")
        .motion
        .due
        .expect("step timer");
    channel.advance_movement(due - 1, true);
    assert_eq!(channel.position(creature), before);
    channel.advance_movement(due, true);
    assert_ne!(channel.position(creature), before);
    let after = channel.position(creature);
    let next = channel
        .table
        .entry(creature)
        .expect("entry")
        .motion
        .due
        .expect("next step");
    channel.advance_movement(next, false);
    assert_eq!(channel.position(creature), after);
    let entry = channel.table.entry(creature).expect("entry");
    assert!(entry.motion.route.is_none() && entry.motion.due.is_none());
    channel.advance_movement(next + 50_000, true);
    assert_eq!(
        channel.position(creature),
        after,
        "no immediate retry after refusal"
    );
}

#[test]
fn timer_lane_has_one_deadline_and_idle_cancels_it() {
    let mut channel = Channel::new();
    let creature = channel.admit(at(110, 100), 0);
    channel.thinks(0, true);
    let lane = channel.table.step_timers.as_ref().expect("step lane");
    assert_eq!(lane.pending_len(), 1);
    channel.advance_movement(1, true);
    assert_eq!(
        channel
            .table
            .step_timers
            .as_ref()
            .expect("lane")
            .pending_len(),
        1
    );
    // No spawn point: a distant player leaves this creature with the stand goal.
    for x in 116..=127 {
        channel.walk(at(x, 100), 1);
    }
    channel.thinks(1_000_000, true);
    assert_eq!(channel.table.due(creature), None);
    assert_eq!(
        channel
            .table
            .step_timers
            .as_ref()
            .expect("lane")
            .pending_len(),
        0
    );
}

#[test]
fn repeated_owner_inputs_do_not_request_twice_and_a_third_ai_timer_is_refused() {
    use crate::ai_think::creature_steps::StepFamily;
    use crate::foundation::owner_timer::{OwnerTimerError, SemanticTimeMicros};
    let mut channel = Channel::new();
    let creature = channel.admit(at(110, 100), 0);
    channel.thinks(0, true);
    let before = channel.path_budget.charged_work();
    channel.advance_movement(0, true);
    assert_eq!(
        channel.path_budget.charged_work(),
        before,
        "one request per think"
    );
    assert_eq!(channel.table.pending(), 1);
    let stamp = channel.runtime.issue_owner_work().expect("owner stamp");
    let fence = channel.runtime.owner_fence().expect("current owner");
    assert_eq!(
        channel
            .table
            .step_timers
            .as_mut()
            .expect("step lane")
            .schedule(
                fence,
                stamp,
                StepFamily,
                channel.player,
                Some(creature),
                SemanticTimeMicros::from_micros(1)
            ),
        Err(OwnerTimerError::PendingLimitReached)
    );
}

#[test]
fn a_changed_bundle_revision_drops_paths_and_deadlines_before_a_step() {
    use crate::movement::speed::{EngineeringGroundSpeed, creature_speed_fixture::Native};
    let mut channel = Channel::new();
    let creature = channel.admit(at(110, 100), 0);
    channel.thinks(0, true);
    assert!(
        channel
            .table
            .entry(creature)
            .expect("entry")
            .motion
            .route
            .is_some()
    );
    let source = Native {
        actor: creature,
        base: 67,
        pin: channel.runtime.content_pin().server_artifact_digest(),
    };
    let mut changed = channel.runtime.content_pin().map_revision_digest();
    changed[0] ^= 1;
    assert_eq!(
        channel.table.drive_movement(
            &mut channel.runtime,
            &channel.cells,
            &BTreeSet::new(),
            changed,
            &source,
            &EngineeringGroundSpeed,
            &channel.root,
            1,
            &mut channel.path_budget,
            false
        ),
        None
    );
    let entry = channel.table.entry(creature).expect("entry");
    assert!(
        entry.motion.route.is_none()
            && entry.motion.due.is_none()
            && entry.motion.pending.is_none()
    );
    assert!(channel.table.step_timers.is_none());
    assert_eq!(channel.position(creature), at(110, 100));
}
