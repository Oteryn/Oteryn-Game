//! Source callbacks may move actors or lose ownership between reservation and attack selection.
use super::*;
use crate::ai_think::summon_plan::{MonsterThinkReader, ReservedNativeThinkBatch};

struct NoSummonReads {
    called: bool,
}
impl MonsterThinkReader for NoSummonReads {
    async fn summon_facts(
        &mut self,
        _: &ChannelRuntimeV1,
        _: crate::ai_think::ThinkOccurrence,
        _: Option<ExactActorRef>,
        _: Option<&[MovementLocalPosition]>,
    ) -> Option<crate::ai_think::profile_schedule::MonsterSummonFacts> {
        self.called = true;
        None
    }
}
fn finish(
    channel: &mut Channel,
    batch: ReservedNativeThinkBatch,
) -> Result<Vec<(ExactActorRef, crate::ai_think::ThinkReport)>, String> {
    let players = [TargetCandidate {
        actor: channel.player,
        position: channel.position(channel.player),
        eligible: true,
        invisible: false,
        health: 100,
        damage: 0,
    }];
    let mut reader = NoSummonReads { called: false };
    let polled = {
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
        std::future::Future::poll(future.as_mut(), &mut context)
    };
    assert!(!reader.called, "the source fixture has no summon schedule");
    match polled {
        std::task::Poll::Ready(reports) => Ok(reports),
        std::task::Poll::Pending => Err("a fixture without summon reads cannot await IO".into()),
    }
}
#[test]
fn srx3_reserved_native_think_rechecks_position_and_publishes_one_deadline() -> Result<(), String> {
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
    let reports = finish(&mut channel, batch)?;
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
    Ok(())
}
#[test]
fn srx3_reserved_native_think_cannot_publish_after_owner_retirement() -> Result<(), String> {
    let mut channel = Channel::new();
    let actor = channel.admit(at(110, 100), 0);
    let batch = channel
        .table
        .reserve_current_thinks(&mut channel.runtime, 0)
        .expect("reserve");
    channel.runtime.retire_owner_cycle();
    assert!(finish(&mut channel, batch)?.is_empty());
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
    Ok(())
}

#[test]
fn srx3_reserved_native_think_pauses_when_idle_and_resumes_one_sequence_on_player_wake()
-> Result<(), String> {
    let mut channel = Channel::new();
    let actor = channel.admit(at(100, 100), 0);
    let initial = channel
        .table
        .reserve_current_thinks(&mut channel.runtime, 0)
        .expect("initial reserve");
    assert_eq!(initial.occurrences().len(), 1);
    let reports = finish(&mut channel, initial)?;
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
    let reports = finish(&mut channel, resumed)?;
    assert_eq!(reports[0].1.sequence, 2);
    assert!(!reports[0].1.idle);
    assert_eq!(channel.table.due(actor), Some(now + SECOND));
    Ok(())
}

fn finish_borrowed(
    channel: &mut Channel,
    batch: &ReservedNativeThinkBatch,
) -> Result<Vec<(ExactActorRef, crate::ai_think::ThinkReport)>, String> {
    let players = [TargetCandidate {
        actor: channel.player,
        position: channel.position(channel.player),
        eligible: true,
        invisible: false,
        health: 100,
        damage: 0,
    }];
    let mut reader = NoSummonReads { called: false };
    let result = {
        let future = channel.table.run_reserved_current_thinks_borrowed(
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
        std::future::Future::poll(future.as_mut(), &mut context)
    };
    assert!(
        !reader.called,
        "same no-summon source fixture performs no IO"
    );
    match result {
        std::task::Poll::Ready(reports) => Ok(reports),
        std::task::Poll::Pending => Err("unexpected source fixture IO".into()),
    }
}

#[test]
fn srx3_borrowed_original_native_batch_preserves_clock_and_refuses_reconsume() -> Result<(), String>
{
    let mut channel = Channel::new();
    let actor = channel.admit(at(110, 100), 0);
    let batch = std::sync::Arc::new(
        channel
            .table
            .reserve_current_thinks(&mut channel.runtime, 0)
            .ok_or("original native reservation")?,
    );
    let retained = std::sync::Arc::clone(&batch);
    assert_eq!(retained.observed_at_us(), 0);
    assert!(retained.current_for(&channel.runtime));
    let first = finish_borrowed(&mut channel, &batch)?;
    assert_eq!(first.len(), 1);
    assert_eq!((first[0].0, first[0].1.sequence), (actor, 1));
    assert!(
        finish_borrowed(&mut channel, &retained)?.is_empty(),
        "same original before-sequence cannot consume twice"
    );
    assert_eq!(
        channel
            .table
            .entries
            .get(&actor.placement_identity())
            .ok_or("actual native entry")?
            .due_us,
        Some(SECOND)
    );
    channel.runtime.retire_owner_cycle();
    assert!(!retained.current_for(&channel.runtime));
    assert!(
        finish_borrowed(&mut channel, &retained)?.is_empty(),
        "retained Arc is not fresh authority after owner retirement"
    );
    assert_eq!(retained.observed_at_us(), 0);
    Ok(())
}

#[test]
fn srx3_cancelled_original_batch_skips_retired_actor_and_resumes_survivor() -> Result<(), String> {
    use std::future::Future;
    let mut channel = Channel::new();
    let retiring = channel.admit(at(110, 100), 0);
    let survivor = channel.admit(at(111, 100), 0);
    let batch = std::sync::Arc::new(
        channel
            .table
            .reserve_current_thinks(&mut channel.runtime, 0)
            .ok_or("original two-actor reservation")?,
    );
    let original = batch.occurrences().to_vec();
    assert_eq!(original.len(), 2);
    let retained = std::sync::Arc::clone(&batch);
    let mut cancelled = Box::pin(async move {
        // Only a temporary continuation handle is cancelled; the canonical
        // original batch remains retained by its owner.
        let _lease = retained;
        std::future::pending::<()>().await;
    });
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    assert!(matches!(
        cancelled.as_mut().poll(&mut context),
        std::task::Poll::Pending
    ));
    assert_eq!(std::sync::Arc::strong_count(&batch), 2);
    drop(cancelled);
    assert_eq!(std::sync::Arc::strong_count(&batch), 1);
    channel
        .runtime
        .remove_test_actor(retiring)
        .map_err(|error| format!("canonical creature retirement: {error:?}"))?;
    let reports = finish_borrowed(&mut channel, &batch)?;
    assert_eq!(reports.len(), 1);
    assert_eq!((reports[0].0, reports[0].1.sequence), (survivor, 1));
    assert_eq!(
        batch.occurrences(),
        original.as_slice(),
        "retirement cannot rewrite original members or reserve a fresh sequence"
    );
    assert_eq!(batch.observed_at_us(), 0);
    assert_eq!(
        channel
            .table
            .entries
            .get(&survivor.placement_identity())
            .ok_or("surviving original entry")?
            .due_us,
        Some(SECOND)
    );
    assert!(
        finish_borrowed(&mut channel, &batch)?.is_empty(),
        "same original survivor cannot consume twice"
    );
    Ok(())
}
