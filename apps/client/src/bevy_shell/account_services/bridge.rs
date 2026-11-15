//! Bounded handoff of public account views; credentials remain in the admission worker.
use crate::{account_services, premium};
use oteryn_foundation::CancellationToken;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
    mpsc::{self, Receiver, SyncSender},
};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::bevy_shell) enum Action {
    Premium,
    Refresh,
    Back,
}
pub(in crate::bevy_shell) struct Request {
    pub id: u64,
    pub account_feedback: String,
    pub premium: Option<premium::View>,
    cancel: CancellationToken,
    reply: SyncSender<Action>,
    epoch: u64,
    deadline: Instant,
}
pub(in crate::bevy_shell) struct Services {
    requests: Receiver<Request>,
    pending: Option<Request>,
    epoch: Arc<AtomicU64>,
    account: account_services::Presenter,
    premium: premium::Presenter,
}
impl Services {
    pub fn new() -> Self {
        let (sender, requests) = mpsc::sync_channel(1);
        let epoch = Arc::new(AtomicU64::new(1));
        let sequence = Arc::new(AtomicU64::new(1));
        let callback_epoch = epoch.clone();
        let ask = move |premium, account_feedback, cancel: CancellationToken| {
            let current = callback_epoch.load(Ordering::Acquire);
            let (reply, result) = mpsc::sync_channel(1);
            let deadline = Instant::now() + Duration::from_secs(300);
            let request = Request {
                id: sequence.fetch_add(1, Ordering::Relaxed),
                premium,
                account_feedback,
                cancel: cancel.clone(),
                reply,
                epoch: current,
                deadline,
            };
            if cancel.is_cancelled() || sender.try_send(request).is_err() {
                return Action::Back;
            }
            loop {
                if cancel.is_cancelled()
                    || callback_epoch.load(Ordering::Acquire) != current
                    || Instant::now() >= deadline
                {
                    return Action::Back;
                }
                match result.recv_timeout(Duration::from_millis(50)) {
                    Ok(action) => {
                        return if cancel.is_cancelled()
                            || callback_epoch.load(Ordering::Acquire) != current
                            || Instant::now() >= deadline
                        {
                            Action::Back
                        } else {
                            action
                        };
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(_) => return Action::Back,
                }
            }
        };
        // Both callbacks share the bounded queue and identity sequence.
        let account_ask = ask.clone();
        Self {
            requests,
            pending: None,
            epoch,
            account: Arc::new(move |view, cancel| {
                match account_ask(None, view.feedback.clone(), cancel) {
                    Action::Premium => account_services::Action::Premium,
                    _ => account_services::Action::Back,
                }
            }),
            premium: Arc::new(move |view, cancel| {
                match ask(Some(view.clone()), String::new(), cancel) {
                    Action::Refresh => premium::Action::Refresh,
                    _ => premium::Action::Back,
                }
            }),
        }
    }
    pub fn configure(&self, config: &mut crate::NativeLoginConfig) {
        config.account_services = self.account.clone();
        config.premium = self.premium.clone();
    }
    pub fn pending(&self) -> Option<&Request> {
        self.pending.as_ref().filter(|r| {
            !r.cancel.is_cancelled()
                && r.epoch == self.epoch.load(Ordering::Acquire)
                && Instant::now() < r.deadline
        })
    }
    pub fn poll(&mut self) {
        if self.pending().is_none() {
            self.pending = None;
        }
        while let Ok(request) = self.requests.try_recv() {
            if self.pending.is_none()
                && !request.cancel.is_cancelled()
                && request.epoch == self.epoch.load(Ordering::Acquire)
                && Instant::now() < request.deadline
            {
                self.pending = Some(request);
            }
        }
    }
    pub fn respond(&mut self, id: u64, action: Action) {
        if !self.pending().is_some_and(|r| {
            r.id == id
                && (action == Action::Back
                    || if r.premium.is_some() {
                        action == Action::Refresh
                    } else {
                        action == Action::Premium
                    })
        }) {
            return;
        }
        if let Some(request) = self.pending.take() {
            let _ = request.reply.try_send(action);
        }
    }
    pub fn cancel(&mut self) {
        self.epoch.fetch_add(1, Ordering::AcqRel);
        self.pending = None;
        while self.requests.try_recv().is_ok() {}
    }
}
impl Drop for Services {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn premium_cancellation_and_shutdown_release_waiting_presenter()
    -> Result<(), Box<dyn std::error::Error>> {
        for shutdown in [false, true] {
            let mut services = Services::new();
            let show = services.premium.clone();
            let cancel = CancellationToken::new();
            let worker_cancel = cancel.clone();
            let worker = std::thread::spawn(move || show(&premium::View::default(), worker_cancel));
            let deadline = Instant::now() + Duration::from_secs(3);
            while services.pending().is_none() && Instant::now() < deadline {
                services.poll();
                std::thread::yield_now();
            }
            assert!(services.pending().is_some());
            if shutdown {
                services.cancel();
            } else {
                cancel.cancel();
            }
            services.poll();
            assert!(services.pending().is_none());
            assert!(matches!(
                worker.join().map_err(|_| "worker")?,
                premium::Action::Back
            ));
        }
        Ok(())
    }
}
