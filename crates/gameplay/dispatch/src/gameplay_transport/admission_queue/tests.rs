use super::*;
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1};
use crate::foundation::{ChannelId, GameSessionId, NodeId, WorldId};
use std::time::{Duration, Instant};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn id(last: u8) -> [u8; 16] {
    [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, last]
}
fn session(last: u8) -> Result<GameSessionId, String> {
    GameSessionId::decode(&id(last)).map_err(|error| format!("{error:?}"))
}
fn runtime(channel: u8) -> Result<ChannelRuntimeV1, String> {
    let world = WorldId::decode(&id(1)).map_err(|error| format!("{error:?}"))?;
    ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&id(channel)).map_err(|error| format!("{error:?}"))?,
        NodeId::decode(&id(9)).map_err(|error| format!("{error:?}"))?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        ChannelContentPin::test(world),
    )
    .map_err(|error| format!("{error:?}"))
}
fn position(
    result: Result<
        crate::foundation::PlayerActorReservation,
        super::super::connection::AdmissionRefusal,
    >,
) -> Option<u32> {
    match result {
        Err(super::super::connection::AdmissionRefusal::Queued(queue)) => Some(queue.position),
        _ => None,
    }
}
#[test]
fn node_capacity_queue_preserves_fifo_when_a_real_runtime_slot_is_freed() -> TestResult {
    let mut runtime = runtime(2)?;
    let held1 = runtime
        .reserve_fresh_session(session(11)?)
        .map_err(|error| format!("{error:?}"))?;
    let held2 = runtime
        .reserve_fresh_session(session(12)?)
        .map_err(|error| format!("{error:?}"))?;
    let now = Instant::now();
    let mut queue = AdmissionWaitQueue::default();
    assert_eq!(
        position(queue.reserve(&mut runtime, session(21)?, [1; 32], now)),
        Some(1)
    );
    assert_eq!(
        position(queue.reserve(&mut runtime, session(22)?, [2; 32], now)),
        Some(2)
    );
    runtime
        .rollback_definitely_uncommitted(held1)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        position(queue.reserve(&mut runtime, session(22)?, [2; 32], now)),
        Some(2)
    );
    let first = queue
        .reserve(&mut runtime, session(21)?, [1; 32], now)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        position(queue.reserve(&mut runtime, session(22)?, [2; 32], now)),
        Some(1)
    );
    runtime
        .rollback_definitely_uncommitted(first)
        .map_err(|error| format!("{error:?}"))?;
    let second = queue
        .reserve(&mut runtime, session(22)?, [2; 32], now)
        .map_err(|error| format!("{error:?}"))?;
    runtime
        .rollback_definitely_uncommitted(second)
        .map_err(|error| format!("{error:?}"))?;
    runtime
        .rollback_definitely_uncommitted(held2)
        .map_err(|error| format!("{error:?}"))?;
    Ok(())
}
#[test]
fn an_expired_head_does_not_block_a_live_waiter_or_another_channel() -> TestResult {
    let mut runtime = runtime(2)?;
    let held1 = runtime
        .reserve_fresh_session(session(11)?)
        .map_err(|error| format!("{error:?}"))?;
    let held2 = runtime
        .reserve_fresh_session(session(12)?)
        .map_err(|error| format!("{error:?}"))?;
    let mut queue = AdmissionWaitQueue::default();
    let now = Instant::now();
    assert_eq!(
        position(queue.reserve(&mut runtime, session(21)?, [1; 32], now)),
        Some(1)
    );
    assert_eq!(
        position(queue.reserve(&mut runtime, session(22)?, [2; 32], now)),
        Some(2)
    );
    let mut other = self::runtime(3)?;
    let separate = queue
        .reserve(&mut other, session(31)?, [1; 32], now)
        .map_err(|error| format!("{error:?}"))?;
    other
        .rollback_definitely_uncommitted(separate)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        position(queue.reserve(
            &mut runtime,
            session(22)?,
            [2; 32],
            now + Duration::from_secs(4)
        )),
        Some(2)
    );
    runtime
        .rollback_definitely_uncommitted(held1)
        .map_err(|error| format!("{error:?}"))?;
    let live = queue
        .reserve(
            &mut runtime,
            session(22)?,
            [2; 32],
            now + Duration::from_secs(7),
        )
        .map_err(|error| format!("{error:?}"))?;
    runtime
        .rollback_definitely_uncommitted(live)
        .map_err(|error| format!("{error:?}"))?;
    runtime
        .rollback_definitely_uncommitted(held2)
        .map_err(|error| format!("{error:?}"))?;
    Ok(())
}

#[test]
fn full_wait_list_is_bounded_and_existing_waiters_keep_their_positions() -> TestResult {
    let mut runtime = runtime(2)?;
    let first = runtime
        .reserve_fresh_session(session(11)?)
        .map_err(|error| format!("{error:?}"))?;
    let second = runtime
        .reserve_fresh_session(session(12)?)
        .map_err(|error| format!("{error:?}"))?;
    let mut queue = AdmissionWaitQueue::default();
    let now = Instant::now();
    for index in 0..MAX_QUEUE_POSITION {
        let mut key = [0; 32];
        key[..4].copy_from_slice(&index.to_le_bytes());
        assert_eq!(
            position(queue.reserve(&mut runtime, session(21)?, key, now)),
            Some(index + 1)
        );
    }
    assert_eq!(queue.entries.len(), MAX_QUEUE_POSITION as usize);
    assert!(matches!(
        queue.reserve(&mut runtime, session(22)?, [255; 32], now),
        Err(AdmissionRefusal::Classified(
            crate::foundation::FoundationProtocolError::AdmissionCapacityExceeded
        ))
    ));
    assert_eq!(
        position(queue.reserve(&mut runtime, session(21)?, [0; 32], now)),
        Some(1)
    );
    runtime
        .rollback_definitely_uncommitted(first)
        .map_err(|error| format!("{error:?}"))?;
    runtime
        .rollback_definitely_uncommitted(second)
        .map_err(|error| format!("{error:?}"))?;
    Ok(())
}
