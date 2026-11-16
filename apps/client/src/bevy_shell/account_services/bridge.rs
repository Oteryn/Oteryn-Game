//! Bounded handoff of public account views; credentials remain in the admission worker.
use super::screens;
use crate::{account_services, coin_shop, premium};
use oteryn_foundation::CancellationToken;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
    mpsc::{self, Receiver, SyncSender},
};
use std::time::{Duration, Instant};

/// One selectable list; choosing a row may put its value into a field.
#[derive(Clone, Default)]
pub(in crate::bevy_shell) struct List {
    pub label: &'static str,
    pub rows: Vec<String>,
    pub selected: Option<usize>,
    pub fill: Option<(usize, Vec<String>)>,
}
#[derive(Clone)]
pub(in crate::bevy_shell) struct Field {
    pub label: &'static str,
    pub value: String,
}
#[derive(Clone)]
pub(in crate::bevy_shell) struct Button {
    pub id: &'static str,
    pub label: &'static str,
    pub enabled: bool,
}
/// What one account screen shows. The worker builds it from its controller view.
#[derive(Clone, Default)]
pub(in crate::bevy_shell) struct Screen {
    pub title: &'static str,
    pub text: String,
    pub notice: Option<String>,
    /// Premium text is re-evaluated every frame against the controller's own clock.
    pub premium: Option<premium::View>,
    pub lists: Vec<List>,
    pub fields: Vec<Field>,
    pub buttons: Vec<Button>,
    /// The button Escape presses.
    pub escape: &'static str,
}
impl Screen {
    pub fn text(&self) -> String {
        let mut text = self
            .premium
            .as_ref()
            .map_or_else(|| self.text.clone(), premium::View::status_text);
        if let Some(notice) = &self.notice {
            text.push_str("\n\n");
            text.push_str(notice);
        }
        text
    }
    /// Shows the screen again with the player's choices and typed values kept.
    pub fn keep(&mut self, reply: &Reply, notice: String) {
        for (list, selected) in self.lists.iter_mut().zip(&reply.selected) {
            list.selected = *selected;
        }
        for (field, value) in self.fields.iter_mut().zip(&reply.fields) {
            field.value.clone_from(value);
        }
        self.notice = Some(notice);
    }
}
/// The pressed button with the selections and field values it was pressed with.
pub(in crate::bevy_shell) struct Reply {
    pub button: &'static str,
    pub selected: Vec<Option<usize>>,
    pub fields: Vec<String>,
}
pub(in crate::bevy_shell) struct Request {
    pub id: u64,
    pub screen: Screen,
    cancel: CancellationToken,
    reply: SyncSender<Reply>,
    epoch: u64,
    deadline: Instant,
}
/// Shows a screen and waits for its answer; `None` when cancelled, superseded or expired.
pub(super) type Ask = dyn Fn(Screen, &CancellationToken) -> Option<Reply> + Send + Sync;
pub(in crate::bevy_shell) struct Services {
    requests: Receiver<Request>,
    pending: Option<Request>,
    epoch: Arc<AtomicU64>,
    account: account_services::Presenter,
    coins: coin_shop::Presenter,
    premium: premium::Presenter,
}
impl Services {
    pub fn new() -> Self {
        let (sender, requests) = mpsc::sync_channel(1);
        let epoch = Arc::new(AtomicU64::new(1));
        let sequence = AtomicU64::new(1);
        let callback_epoch = epoch.clone();
        let ask: Arc<Ask> = Arc::new(move |screen, cancel| {
            let current = callback_epoch.load(Ordering::Acquire);
            let (reply, result) = mpsc::sync_channel(1);
            let deadline = Instant::now() + Duration::from_secs(300);
            let request = Request {
                id: sequence.fetch_add(1, Ordering::Relaxed),
                screen,
                cancel: cancel.clone(),
                reply,
                epoch: current,
                deadline,
            };
            if cancel.is_cancelled() || sender.try_send(request).is_err() {
                return None;
            }
            let live = || {
                !cancel.is_cancelled()
                    && callback_epoch.load(Ordering::Acquire) == current
                    && Instant::now() < deadline
            };
            loop {
                if !live() {
                    return None;
                }
                match result.recv_timeout(Duration::from_millis(50)) {
                    Ok(reply) => return live().then_some(reply),
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(_) => return None,
                }
            }
        });
        // Every presenter shares the bounded queue and identity sequence.
        let account = ask.clone();
        let coins = ask.clone();
        Self {
            requests,
            pending: None,
            epoch,
            account: Arc::new(move |view, cancel| screens::account(view, &*account, &cancel)),
            coins: Arc::new(move |view, cancel| screens::coins(view, &*coins, &cancel)),
            premium: Arc::new(move |view, cancel| screens::premium(view, &*ask, &cancel)),
        }
    }
    pub fn configure(&self, config: &mut crate::NativeLoginConfig) {
        config.account_services = self.account.clone();
        config.coin_shop = self.coins.clone();
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
    /// Answers the shown screen with an enabled button; a stale or malformed answer is dropped.
    pub fn respond(&mut self, id: u64, reply: Reply) {
        if !self.pending().is_some_and(|r| {
            let screen = &r.screen;
            r.id == id
                && screen
                    .buttons
                    .iter()
                    .any(|b| b.id == reply.button && b.enabled)
                && reply.selected.len() == screen.lists.len()
                && screen
                    .lists
                    .iter()
                    .zip(&reply.selected)
                    .all(|(list, at)| at.is_none_or(|at| at < list.rows.len()))
                && reply.fields.len() == screen.fields.len()
        }) {
            return;
        }
        if let Some(request) = self.pending.take() {
            let _ = request.reply.try_send(reply);
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
