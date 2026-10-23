use super::*;
use crate::movement::speed::{EngineeringGroundSpeed, creature_speed_fixture::Native};

#[test]
fn sixty_five_creature_requests_wait_in_deadline_then_exact_actor_order() {
    for delay_first in [false, true] {
        let mut channel = Channel::with_capacity(66);
        let mut actors = Vec::new();
        for _ in 0..65 {
            let actor = channel.admit(at(64, 64), 0);
            channel.table.entry_mut(actor).expect("entry").state.spawn = Some(at(114, 114));
            actors.push(actor);
        }
        actors.sort_by_key(|actor| (actor.actor_local_id(), actor.actor_local_generation()));
        let blocking = BTreeSet::from([crate::content::LogicalCell {
            x: 114,
            y: 114,
            z: 7,
        }]);
        let sources: Vec<_> = actors
            .iter()
            .map(|actor| Native {
                actor: *actor,
                base: 67,
                pin: channel.runtime.content_pin().server_artifact_digest(),
            })
            .collect();
        let navigation = channel.runtime.content_pin().map_revision_digest();
        assert!(channel.path_budget.admits(0, false, false));
        for _ in 0..64 {
            channel.path_budget.charge(false, 1024);
        }
        channel.table.run_due_thinks(
            &mut channel.runtime,
            0,
            &channel.root,
            &channel.revisions,
            &[],
        );
        channel.table.drive_movement(
            &mut channel.runtime,
            &channel.cells,
            &blocking,
            navigation,
            &sources,
            &EngineeringGroundSpeed,
            &channel.root,
            0,
            &mut channel.path_budget,
            false,
        );
        assert!(
            actors
                .iter()
                .all(|actor| channel.table.entry(*actor).expect("entry").motion.pending == Some(0))
        );
        if delay_first {
            channel
                .table
                .entry_mut(actors[0])
                .expect("first actor")
                .motion
                .pending = Some(1);
        }
        channel.table.drive_movement(
            &mut channel.runtime,
            &channel.cells,
            &blocking,
            navigation,
            &sources,
            &EngineeringGroundSpeed,
            &channel.root,
            50_000,
            &mut channel.path_budget,
            false,
        );
        let waiting: Vec<_> = actors
            .iter()
            .filter(|actor| {
                channel
                    .table
                    .entry(**actor)
                    .expect("entry")
                    .motion
                    .pending
                    .is_some()
            })
            .copied()
            .collect();
        assert_eq!(waiting, [actors[if delay_first { 0 } else { 64 }]]);
        assert_eq!(channel.path_budget.charged_work(), 65_536);
        channel.table.drive_movement(
            &mut channel.runtime,
            &channel.cells,
            &blocking,
            navigation,
            &sources,
            &EngineeringGroundSpeed,
            &channel.root,
            100_000,
            &mut channel.path_budget,
            false,
        );
        assert!(actors.iter().all(|actor| {
            channel
                .table
                .entry(*actor)
                .expect("entry")
                .motion
                .pending
                .is_none()
        }));
        assert_eq!(channel.path_budget.charged_work(), 1024);
    }
}

#[test]
fn a_failed_chase_search_researches_targets_on_the_next_think() {
    let mut channel = Channel::new();
    let creature = channel.admit(at(110, 100), 0);
    channel.thinks(0, false);
    assert!(channel.table.state(creature).expect("state").unreachable);
    let session = GameSessionId::decode(&uuid(5)).expect("second session");
    let reservation = channel
        .runtime
        .reserve_fresh_session(session)
        .expect("reserve");
    let other = channel
        .runtime
        .commit_fresh_session(reservation)
        .expect("second player");
    channel
        .runtime
        .initialize_pinned_test_position(other, at(112, 100))
        .expect("nearer candidate");
    let players: Vec<_> = [channel.player, other]
        .into_iter()
        .map(|actor| TargetCandidate {
            actor,
            position: channel.position(actor),
            eligible: true,
            invisible: false,
            health: 100,
            damage: 0,
        })
        .collect();
    channel.advance_movement(SECOND, true);
    let reports = channel.table.run_due_thinks(
        &mut channel.runtime,
        SECOND,
        &channel.root,
        &channel.revisions,
        &players,
    );
    assert_eq!(reports[0].1.target, Some(other));
    assert!(!channel.table.state(creature).expect("state").unreachable);
    channel.advance_movement(SECOND, true);
    assert!(
        channel
            .table
            .entry(creature)
            .expect("entry")
            .motion
            .route
            .is_some()
    );
}
