//! Per-connection notification ownership. Native surfaces hold only a weak reference.
use oteryn_client_ui::notifications::NotificationCenter;
use oteryn_session::{AppliedDelta, ServerMessage};
use std::{
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::Instant,
};
#[derive(Debug)]
pub(in crate::play) struct Owned {
    pub center: Mutex<NotificationCenter>,
    pub origin: Instant,
    pub screen_dismissals: std::sync::atomic::AtomicU64,
    shared_frontend: std::sync::atomic::AtomicBool,
}
#[derive(Debug, Clone)]
pub(in crate::play) struct State(pub Arc<Owned>);
impl Default for State {
    fn default() -> Self {
        Self(Arc::new(Owned {
            center: Mutex::new(NotificationCenter::default()),
            origin: Instant::now(),
            screen_dismissals: std::sync::atomic::AtomicU64::new(0),
            shared_frontend: std::sync::atomic::AtomicBool::new(false),
        }))
    }
}
pub(in crate::play) fn lock(owned: &Owned) -> MutexGuard<'_, NotificationCenter> {
    owned.center.lock().unwrap_or_else(PoisonError::into_inner)
}
impl State {
    pub fn use_shared_frontend(&self) {
        self.0.shared_frontend.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn observe(&self, message: &AppliedDelta<ServerMessage>) {
        // The shared frontend projects these receipts into its timed screen areas.
        if self.0.shared_frontend.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        let (text, popup) = match &message.value {
            ServerMessage::StatusText(text) | ServerMessage::Advance(text) => (text, false),
            ServerMessage::Warning(text) => (text, true),
        };
        self.show(&message.server_sequence.to_string(), text, popup);
    }
    pub(in crate::play) fn show(&self, id: &str, text: &str, popup: bool) {
        let now_ms = self.elapsed(Instant::now());
        let inserted = lock(&self.0).show(id, text, popup, now_ms, 8000);
        let _ = inserted;
    }
    pub fn elapsed(&self, now: Instant) -> u64 {
        u64::try_from(now.saturating_duration_since(self.0.origin).as_millis()).unwrap_or(u64::MAX)
    }
    pub fn text_at(&self, popup: bool, now: Instant) -> String {
        let mut center = lock(&self.0);
        center.expire(self.elapsed(now));
        center.surface_text(popup)
    }
}
