//! Bounded handoff from the existing blocking selector callback to the UI thread.
use crate::character_selection::{CharacterRow, CharacterSelector, SelectionAction};
use oteryn_foundation::CancellationToken;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{self, Receiver, SyncSender},
};
use std::time::{Duration, Instant};

const POLL: Duration = Duration::from_millis(50);
const MAX_ROWS: usize = 64;

pub struct SelectionRequest {
    pub id: u64,
    pub rows: Vec<CharacterRow>,
    pub preselect: String,
    cancellation: CancellationToken,
    reply: SyncSender<SelectionAction>,
    epoch: u64,
    expires_at: Instant,
}

/// Receiver stays on the UI thread. The callback owns no core or mailbox lock.
pub struct SelectionBridge {
    requests: Receiver<SelectionRequest>,
    pending: Option<SelectionRequest>,
    closed: Arc<AtomicBool>,
    epoch: Arc<AtomicU64>,
}
impl SelectionBridge {
    pub fn new() -> (Self, CharacterSelector) {
        Self::with_timeout(Duration::from_secs(300))
    }
    fn with_timeout(timeout: Duration) -> (Self, CharacterSelector) {
        let (sender, requests) = mpsc::sync_channel(1);
        let closed = Arc::new(AtomicBool::new(false));
        let callback_closed = closed.clone();
        let epoch = Arc::new(AtomicU64::new(1));
        let callback_epoch = epoch.clone();
        let sequence = AtomicU64::new(1);
        let callback: CharacterSelector = Arc::new(move |rows, preselect, cancellation| {
            if callback_closed.load(Ordering::Acquire)
                || cancellation.is_cancelled()
                || rows.len() > MAX_ROWS
            {
                return SelectionAction::Cancel;
            }
            let current_epoch = callback_epoch.load(Ordering::Acquire);
            let expires_at = Instant::now()
                .checked_add(timeout)
                .unwrap_or_else(Instant::now);
            let id = sequence.fetch_add(1, Ordering::Relaxed);
            if id == 0 || id == u64::MAX {
                callback_closed.store(true, Ordering::Release);
                return SelectionAction::Cancel;
            }
            let (reply, result) = mpsc::sync_channel(1);
            let request = SelectionRequest {
                id,
                rows: rows.to_vec(),
                preselect: preselect.into(),
                cancellation: cancellation.clone(),
                reply,
                epoch: current_epoch,
                expires_at,
            };
            if sender.try_send(request).is_err() {
                return SelectionAction::Cancel;
            }
            loop {
                if cancellation.is_cancelled()
                    || callback_closed.load(Ordering::Acquire)
                    || callback_epoch.load(Ordering::Acquire) != current_epoch
                    || Instant::now() >= expires_at
                {
                    return SelectionAction::Cancel;
                }
                match result.recv_timeout(POLL) {
                    Ok(action) => {
                        return if cancellation.is_cancelled()
                            || callback_closed.load(Ordering::Acquire)
                            || callback_epoch.load(Ordering::Acquire) != current_epoch
                            || Instant::now() >= expires_at
                        {
                            SelectionAction::Cancel
                        } else {
                            action
                        };
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => return SelectionAction::Cancel,
                }
            }
        });
        (
            Self {
                requests,
                pending: None,
                closed,
                epoch,
            },
            callback,
        )
    }
    pub fn pending(&self) -> Option<&SelectionRequest> {
        self.pending.as_ref().filter(|request| {
            !request.cancellation.is_cancelled()
                && request.epoch == self.epoch.load(Ordering::Acquire)
                && Instant::now() < request.expires_at
        })
    }
    /// Returns whether visible request identity changed; canceled rows never enter the UI.
    pub fn poll(&mut self) -> bool {
        let before = self.pending().map(|r| r.id);
        if self
            .pending
            .as_ref()
            .is_some_and(|r| r.cancellation.is_cancelled() || Instant::now() >= r.expires_at)
        {
            self.pending = None;
        }
        while let Ok(request) = self.requests.try_recv() {
            if request.cancellation.is_cancelled()
                || request.epoch != self.epoch.load(Ordering::Acquire)
                || Instant::now() >= request.expires_at
            {
                continue;
            }
            if self.pending.is_none() {
                self.pending = Some(request);
            }
            // An overlapping invocation is refused by dropping its reply sender.
        }
        before != self.pending().map(|r| r.id)
    }
    pub fn respond(&mut self, id: u64, action: SelectionAction) -> bool {
        let Some(request) = self.pending() else {
            return false;
        };
        if request.id != id || !allowed(request, &action) {
            return false;
        }
        let Some(request) = self.pending.take() else {
            return false;
        };
        request.reply.try_send(action).is_ok()
    }
    pub fn cancel(&mut self) {
        if self.epoch.fetch_add(1, Ordering::AcqRel) == u64::MAX {
            self.closed.store(true, Ordering::Release);
        }
        self.pending = None;
        while self.requests.try_recv().is_ok() {}
    }
}
impl Drop for SelectionBridge {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::Release);
        self.cancel();
    }
}
fn allowed(request: &SelectionRequest, action: &SelectionAction) -> bool {
    match action {
        SelectionAction::Play {
            character_id,
            channel_id,
        } => request.rows.iter().any(|row| {
            row.available
                && &row.character.character_id == character_id
                && channel_id
                    .as_ref()
                    .is_none_or(|id| row.channels.iter().any(|channel| &channel.0 == id))
        }),
        SelectionAction::Cancel | SelectionAction::Refresh | SelectionAction::Create => true,
        SelectionAction::Train { .. }
        | SelectionAction::TrainMany { .. }
        | SelectionAction::AccountServices => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_platform_client::native_login::{CharacterAvailability, OwnerCharacter};
    use std::{thread, time::Instant};
    type TestResult = Result<(), Box<dyn std::error::Error>>;
    fn rows() -> Vec<CharacterRow> {
        vec![CharacterRow {
            character: OwnerCharacter {
                character_id: "character".into(),
                world_id: "world".into(),
                name: "Server name".into(),
                availability: CharacterAvailability::Available,
            },
            world_name: "Server world".into(),
            channels: vec![("channel".into(), "Server channel".into())],
            available: true,
        }]
    }
    fn wait(bridge: &mut SelectionBridge) -> Result<u64, &'static str> {
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            bridge.poll();
            if let Some(request) = bridge.pending() {
                return Ok(request.id);
            }
            thread::yield_now();
        }
        Err("selector request did not reach UI")
    }
    #[test]
    fn expired_callback_returns_cancel_without_an_orphan_picker() -> TestResult {
        let (mut bridge, callback) = SelectionBridge::with_timeout(Duration::ZERO);
        let worker = thread::spawn(move || callback(&rows(), "", CancellationToken::new()));
        assert_eq!(
            worker.join().map_err(|_| "worker")?,
            SelectionAction::Cancel
        );
        bridge.poll();
        assert!(bridge.pending().is_none());
        Ok(())
    }
    #[test]
    fn callback_real_rows_and_channel_choice_return_once() -> TestResult {
        let (mut bridge, callback) = SelectionBridge::new();
        let handle =
            thread::spawn(move || callback(&rows(), "character", CancellationToken::new()));
        let id = wait(&mut bridge)?;
        assert_eq!(
            bridge.pending().ok_or("request")?.rows[0].character.name,
            "Server name"
        );
        let action = SelectionAction::Play {
            character_id: "character".into(),
            channel_id: Some("channel".into()),
        };
        assert!(bridge.respond(id, action.clone()));
        assert!(!bridge.respond(id, action.clone()));
        assert_eq!(
            handle.join().map_err(|_| "selector worker panicked")?,
            action
        );
        Ok(())
    }
    #[test]
    fn foreign_unavailable_and_unmigrated_actions_never_return_play() -> TestResult {
        let (mut bridge, callback) = SelectionBridge::new();
        let handle = thread::spawn(move || callback(&rows(), "", CancellationToken::new()));
        let id = wait(&mut bridge)?;
        for action in [
            SelectionAction::Play {
                character_id: "foreign".into(),
                channel_id: None,
            },
            SelectionAction::Play {
                character_id: "character".into(),
                channel_id: Some("foreign".into()),
            },
            SelectionAction::AccountServices,
        ] {
            assert!(!bridge.respond(id, action));
        }
        bridge.pending.as_mut().ok_or("request")?.rows[0].available = false;
        assert!(!bridge.respond(
            id,
            SelectionAction::Play {
                character_id: "character".into(),
                channel_id: None
            }
        ));
        assert!(bridge.respond(id, SelectionAction::Cancel));
        assert_eq!(
            handle.join().map_err(|_| "worker")?,
            SelectionAction::Cancel
        );
        Ok(())
    }
    #[test]
    fn cancellation_shutdown_and_old_reply_cannot_select_next_request() -> TestResult {
        let (mut bridge, callback) = SelectionBridge::new();
        let token = CancellationToken::new();
        let worker_token = token.clone();
        let first = callback.clone();
        let handle = thread::spawn(move || first(&rows(), "", worker_token));
        let old = wait(&mut bridge)?;
        token.cancel();
        assert_eq!(
            handle.join().map_err(|_| "worker")?,
            SelectionAction::Cancel
        );
        bridge.poll();
        assert!(bridge.pending().is_none());
        let handle = thread::spawn(move || callback(&rows(), "", CancellationToken::new()));
        let fresh = wait(&mut bridge)?;
        assert_ne!(old, fresh);
        assert!(!bridge.respond(old, SelectionAction::Refresh));
        drop(bridge);
        assert_eq!(
            handle.join().map_err(|_| "worker")?,
            SelectionAction::Cancel
        );
        Ok(())
    }
    #[test]
    fn cancel_dismisses_waiting_callback_and_refresh_uses_fresh_request() -> TestResult {
        let (mut bridge, callback) = SelectionBridge::new();
        let first = callback.clone();
        let handle = thread::spawn(move || first(&rows(), "", CancellationToken::new()));
        let old = wait(&mut bridge)?;
        bridge.cancel();
        assert_eq!(
            handle.join().map_err(|_| "worker")?,
            SelectionAction::Cancel
        );
        let handle = thread::spawn(move || callback(&rows(), "", CancellationToken::new()));
        let fresh = wait(&mut bridge)?;
        assert_ne!(old, fresh);
        assert!(bridge.respond(fresh, SelectionAction::Refresh));
        assert_eq!(
            handle.join().map_err(|_| "worker")?,
            SelectionAction::Refresh
        );
        Ok(())
    }
}
