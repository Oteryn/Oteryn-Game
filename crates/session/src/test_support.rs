//! Passive capture for qualification clients.
pub use oteryn_protocol_oteryn as wire;
use std::{cell::RefCell, future::Future};

tokio::task_local! {
    static COMMAND_RESULTS: RefCell<Vec<Vec<u8>>>;
}

/// Record incoming command-result envelopes while polling this operation.
pub async fn capture_command_results<T>(operation: impl Future<Output = T>) -> (T, Vec<Vec<u8>>) {
    COMMAND_RESULTS
        .scope(RefCell::new(Vec::new()), async {
            let result = operation.await;
            (result, COMMAND_RESULTS.with(|frames| frames.take()))
        })
        .await
}

pub(super) fn record_command_result(frame: &[u8]) {
    let _ = COMMAND_RESULTS.try_with(|frames| {
        if wire::decode_wire_envelope(frame)
            .is_ok_and(|frame| frame.message_type() == wire::MessageType::CommandResult)
        {
            frames.borrow_mut().push(frame.to_vec());
        }
    });
}

/// Return the original observer snapshot and its baseline sequence.
pub fn visibility_snapshot<S>(session: &super::Session<S>) -> Option<(u64, &super::GatedSnapshot)> {
    Some((
        session.replay.world_sequence,
        session.replay.world_snapshot.as_deref()?,
    ))
}
