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

use std::sync::{Arc, Mutex};

/// Original complete frame bodies received by one qualification connection, plus its commands.
#[derive(Clone, Default)]
pub struct WireCapture {
    frames: Arc<Mutex<(Vec<Vec<u8>>, Vec<Vec<u8>>)>>,
}

tokio::task_local! {
    static WIRE_CAPTURE: WireCapture;
}

impl WireCapture {
    /// Poll one operation on the captured connection. Other peers stay outside this scope.
    pub async fn observe<T>(&self, operation: impl Future<Output = T>) -> T {
        WIRE_CAPTURE.scope(self.clone(), operation).await
    }

    /// Return received frames and original outbound gameplay commands in their individual orders.
    pub fn take(&self) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
        std::mem::take(&mut *self.frames.lock().expect("wire capture lock"))
    }
}

pub(super) fn record_received_frame(frame: &[u8]) {
    record_command_result(frame);
    let _ = WIRE_CAPTURE.try_with(|capture| {
        capture
            .frames
            .lock()
            .expect("wire capture lock")
            .0
            .push(frame.to_vec());
    });
}

pub(super) fn record_sent_frame(frame: &[u8]) {
    let _ = WIRE_CAPTURE.try_with(|capture| {
        // Only gameplay commands are needed. In particular, never keep bootstrap credentials.
        if wire::decode_wire_envelope(frame)
            .is_ok_and(|e| e.message_type() == wire::MessageType::ClientCommand)
        {
            capture
                .frames
                .lock()
                .expect("wire capture lock")
                .1
                .push(frame.to_vec());
        }
    });
}
