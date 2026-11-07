//! Source callbacks may move actors or lose ownership between reservation and attack selection.
use super::*;
use crate::ai_think::summon_plan::{MonsterThinkReader, ReservedNativeThinkBatch};

struct NoSummonReads;
impl MonsterThinkReader for NoSummonReads {
    async fn summon_facts(
        &mut self,
        _: &ChannelRuntimeV1,
        _: crate::ai_think::ThinkOccurrence,
        _: Option<ExactActorRef>,
        _: Option<&[MovementLocalPosition]>,
    ) -> Option<crate::ai_think::profile_schedule::MonsterSummonFacts> {
        panic!("the source fixture has no summon schedule")
    }
}
fn finish(
    channel: &mut Channel,
    batch: ReservedNativeThinkBatch,
) -> Vec<(ExactActorRef, crate::ai_think::ThinkReport)> {
    let players = [TargetCandidate {
        actor: channel.player,
        position: channel.position(channel.player),
        eligible: true,
        invisible: false,
        health: 100,
        damage: 0,
    }];
    let mut reader = NoSummonReads;
    let future = channel.table.run_reserved_current_thinks(
        &mut channel.runtime,
        batch,
        &channel.root,
        &channel.revisions,
        &players,
        &mut reader,
        |_, _, _, _, _| {},
    );
    let mut future = std::pin::pin!(future);
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    match std::future::Future::poll(future.as_mut(), &mut context) {
        std::task::Poll::Ready(reports) => reports,
        std::task::Poll::Pending => panic!("a fixture without summon reads cannot await IO"),
    }
}
#[test]
fn srx3_reserved_native_think_rechecks_position_and_publishes_one_deadline() {
    let mut channel = Channel::new();
    let actor = channel.admit(at(110, 100), 0);
    let batch = channel
        .table
        .reserve_current_thinks(&mut channel.runtime, 0)
        .expect("reserve");
    let position = channel
        .runtime
        .read_actor_position(actor)
        .expect("position");
    channel
        .runtime
        .borrow_movement_position()
        .commit_cardinal(position, at(111, 100))
        .expect("source move");
    let reports = finish(&mut channel, batch);
    assert_eq!(reports.len(), 1);
    assert_eq!((reports[0].0, reports[0].1.sequence), (actor, 1));
    assert_eq!(channel.position(actor), at(111, 100));
    assert_eq!(
        channel
            .table
            .entries
            .get(&actor.placement_identity())
            .expect("entry")
            .due_us,
        Some(SECOND)
    );
    assert!(
        channel
            .table
            .reserve_current_thinks(&mut channel.runtime, 0)
            .expect("current owner")
            .occurrences()
            .is_empty()
    );
}
#[test]
fn srx3_reserved_native_think_cannot_publish_after_owner_retirement() {
    let mut channel = Channel::new();
    let actor = channel.admit(at(110, 100), 0);
    let batch = channel
        .table
        .reserve_current_thinks(&mut channel.runtime, 0)
        .expect("reserve");
    channel.runtime.retire_owner_cycle();
    assert!(finish(&mut channel, batch).is_empty());
    assert_eq!(
        channel
            .table
            .entries
            .get(&actor.placement_identity())
            .expect("entry")
            .sequence,
        0
    );
    assert!(
        channel
            .table
            .reserve_current_thinks(&mut channel.runtime, 0)
            .is_none()
    );
}

#[test]
fn srx3_reserved_native_think_pauses_when_idle_and_resumes_one_sequence_on_player_wake() {
    let mut channel = Channel::new();
    let actor = channel.admit(at(100, 100), 0);
    let initial = channel
        .table
        .reserve_current_thinks(&mut channel.runtime, 0)
        .expect("initial reserve");
    assert_eq!(initial.occurrences().len(), 1);
    let reports = finish(&mut channel, initial);
    assert_eq!(reports.len(), 1);
    assert!(reports[0].1.idle);
    assert_eq!(channel.table.due(actor), None);
    // Idle time is not a sequence of invented callback occurrences.
    for now in [SECOND, 34 * SECOND, 3600 * SECOND] {
        let batch = channel
            .table
            .reserve_current_thinks(&mut channel.runtime, now)
            .expect("current reserve");
        assert!(batch.occurrences().is_empty());
    }
    // Wake from an actual current player position, not an unbound callback flag.
    for x in (108..115).rev() {
        let current = channel
            .runtime
            .read_actor_position(channel.player)
            .expect("current player");
        channel
            .runtime
            .borrow_movement_position()
            .commit_cardinal(current, at(x, 100))
            .expect("player enters view");
    }
    let now = 3600 * SECOND;
    assert_eq!(
        channel
            .table
            .wake_for_players(&channel.runtime, &[channel.player], now),
        [actor]
    );
    let resumed = channel
        .table
        .reserve_current_thinks(&mut channel.runtime, now)
        .expect("resumed reserve");
    assert_eq!(
        resumed.occurrences(),
        &[crate::ai_think::ThinkOccurrence { actor, sequence: 2 }]
    );
    let reports = finish(&mut channel, resumed);
    assert_eq!(reports[0].1.sequence, 2);
    assert!(!reports[0].1.idle);
    assert_eq!(channel.table.due(actor), Some(now + SECOND));
}
