//! The final source cut follows input AND autonomous-owner quiescence, not a
//! cancellation snapshot. Conditions/channels drive every wait; no guessed sleep.
#![allow(
    clippy::expect_used,
    reason = "bounded native-owner test fixtures and thread joins"
)]
use super::QuiescedKillOwners;
use crate::foundation::{
    ChannelId, ChannelRuntimeV1, CreatureDeathOccurrenceKey, ExactActorRef, MovementLocalPosition,
    NodeId, WorldId,
};
use crate::gameplay_transport::input_gate::InputGate;
use crate::gameplay_transport::public_kill_statistics::CaptureQueue;
use std::future::{Future, poll_fn};
use std::pin::pin;
use std::task::Poll;

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}
fn killed_actors() -> (
    ChannelRuntimeV1,
    [(ExactActorRef, CreatureDeathOccurrenceKey); 2],
) {
    let world = WorldId::decode(&id(1)).expect("World");
    let content =
        oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin::from_activation(
            world,
            1,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
            (100, 100, 7),
        );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&id(2)).expect("Channel"),
        NodeId::decode(&id(3)).expect("Node"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        content,
    )
    .expect("native runtime");
    let mut captured = Vec::new();
    for x in [101, 102] {
        let actor = runtime
            .admit_pinned_test_creature(MovementLocalPosition {
                x,
                y: 100,
                floor: 7,
            })
            .expect("physical actor");
        runtime
            .commit_monster_lab_damage(actor, b"pubproj-shutdown", 20)
            .expect("lethal owner turn");
        let mut owner = runtime.borrow_combat_death();
        let receipt = owner
            .committed_lethal_receipt(actor)
            .expect("committed physical receipt");
        let key = receipt.statistics_death_key().expect("sealed key");
        owner
            .project_committed_lethal(receipt)
            .expect("retained projection");
        captured.push((actor, key));
    }
    let captures: [_; 2] = captured.try_into().expect("two actor captures");
    (runtime, captures)
}
fn run(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(test)
        .expect("native owner test thread")
        .join()
        .expect("native owner test result");
}
#[test]
fn final_kill_cut_covers_an_event_captured_in_the_same_millisecond() {
    run(|| {
        let (runtime, actors) = killed_actors();
        let queue = CaptureQueue::default();
        let now = 1790000000000;
        queue.capture(
            &runtime,
            actors[0].0,
            actors[0].1,
            "oteryn:creature.rat",
            false,
            now,
        );
        // This fixture has no concurrent producer; the finalized owner turn is
        // therefore the same quiesced state that serving proves after its joins.
        let proof = QuiescedKillOwners(());
        let batch = queue.final_batch(now, &proof).expect("final batch");
        assert!(!batch.more);
        assert_eq!(batch.events.len(), 1);
        assert!(
            batch
                .events
                .iter()
                .all(|event| event.occurred_ms() <= batch.cut_ms)
        );
        assert_eq!(
            batch.cut_ms, now,
            "ended_ms must not precede the included lethal event"
        );
    });
}
#[test]
fn final_kill_snapshot_waits_for_input_permits_and_autonomous_owner_join() {
    run(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(async {
                let (runtime, actors) = killed_actors();
                let queue = CaptureQueue::default();
                let gate = InputGate::default();
                let input_started = tokio::sync::Notify::new();
                let release_input = tokio::sync::Notify::new();
                let owner_started = tokio::sync::Notify::new();
                let release_owner = tokio::sync::Notify::new();
                let owners_joined = tokio::sync::Notify::new();
                let now = 1790000000000;
                let player = async {
                    let permit = gate.enter().await.expect("in-flight command permit");
                    input_started.notify_one();
                    release_input.notified().await;
                    queue.capture(
                        &runtime,
                        actors[0].0,
                        actors[0].1,
                        "oteryn:creature.rat",
                        false,
                        now,
                    );
                    drop(permit);
                };
                let autonomous = async {
                    owner_started.notify_one();
                    release_owner.notified().await;
                    queue.capture(
                        &runtime,
                        actors[1].0,
                        actors[1].1,
                        "oteryn:creature.dragon",
                        true,
                        now,
                    );
                };
                let producers = async {
                    tokio::join!(player, autonomous);
                    owners_joined.notify_one();
                };
                let finalize = async {
                    input_started.notified().await;
                    owner_started.notified().await;
                    assert!(
                        queue
                            .batch(now, true)
                            .expect("optimistic snapshot")
                            .events
                            .is_empty(),
                        "the former cancellation-time snapshot can miss both producers"
                    );
                    let mut quiet = pin!(gate.quiesce());
                    poll_fn(|context| {
                        let pending = quiet.as_mut().poll(context).is_pending();
                        assert!(pending, "quiescence must wait for the command's permit");
                        release_input.notify_one();
                        Poll::Ready(())
                    })
                    .await;
                    quiet.await;
                    assert!(
                        gate.enter().await.is_none(),
                        "new commands cannot append after the gate closes"
                    );
                    assert_eq!(
                        queue
                            .batch(now, true)
                            .expect("input-only snapshot")
                            .events
                            .len(),
                        1,
                        "input quiescence alone does not join autonomous producers"
                    );
                    release_owner.notify_one();
                    owners_joined.notified().await;
                    let proof = QuiescedKillOwners(());
                    let final_batch = queue
                        .final_batch(now, &proof)
                        .expect("joined-owner final snapshot");
                    assert_eq!(final_batch.events.len(), 2);
                    assert!(!final_batch.more);
                    assert!(
                        final_batch
                            .events
                            .iter()
                            .all(|event| event.occurred_ms() <= final_batch.cut_ms)
                    );
                    queue.acknowledge(&final_batch);
                    assert!(
                        queue
                            .final_batch(now, &proof)
                            .expect("all final facts acknowledged")
                            .events
                            .is_empty()
                    );
                };
                tokio::join!(producers, finalize);
            });
    });
}
