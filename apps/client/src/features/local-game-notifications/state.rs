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
    shared_frontend: std::sync::atomic::AtomicBool,
    #[cfg(all(windows, not(test)))]
    started: std::sync::atomic::AtomicBool,
}
#[derive(Debug, Clone)]
pub(in crate::play) struct State(pub Arc<Owned>);
impl Default for State {
    fn default() -> Self {
        Self(Arc::new(Owned {
            center: Mutex::new(NotificationCenter::default()),
            origin: Instant::now(),
            shared_frontend: std::sync::atomic::AtomicBool::new(false),
            #[cfg(all(windows, not(test)))]
            started: std::sync::atomic::AtomicBool::new(false),
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
        let (text, popup) = match &message.value {
            ServerMessage::StatusText(text) | ServerMessage::Advance(text) => (text, false),
            ServerMessage::Warning(text) => (text, true),
        };
        self.show(&message.server_sequence.to_string(), text, popup);
    }
    pub(in crate::play) fn show(&self, id: &str, text: &str, popup: bool) {
        let now_ms = self.elapsed(Instant::now());
        let inserted = lock(&self.0).show(id, text, popup, now_ms, 8000);
        #[cfg(any(not(windows), test))]
        let _ = inserted;
        #[cfg(all(windows, not(test)))]
        if inserted
            && !self.0.shared_frontend.load(std::sync::atomic::Ordering::Relaxed)
            && !self
                .0
                .started
                .swap(true, std::sync::atomic::Ordering::Relaxed)
            && !super::view::native::open(Arc::downgrade(&self.0))
        {
            self.0
                .started
                .store(false, std::sync::atomic::Ordering::Relaxed);
        }
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
// On-screen server text after the OTClient `game_textmessage` module: each server message kind
// has its own place and colour, lasts `max(50 ms per character, 4 s)` and is replaced by the
// next message of the same kind. Only the kinds the server sends are mapped: there is no server
// look, loot or hotkey message, so the OTClient green places stay unused.
pub use screen::{ScreenMessages, ScreenPlace};
mod screen {
    use oteryn_session::ServerMessage;
    use std::time::{Duration, Instant};

    /// Where on the game screen a message is drawn.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ScreenPlace {
        /// Event advance (OTClient `centerWhite`, `middleCenterLabel`).
        MiddleCenter,
        /// Warning (OTClient `centerRed`, `lowCenterLabel`).
        LowCenter,
        /// Status text (OTClient `status`, `statusLabel` at the bottom of the game window).
        Status,
    }

    impl ScreenPlace {
        pub const ALL: [Self; 3] = [Self::MiddleCenter, Self::LowCenter, Self::Status];

        /// The place and text of a server message kind.
        #[must_use]
        pub fn of(message: &ServerMessage) -> (Self, &str) {
            match message {
                ServerMessage::Advance(text) => (Self::MiddleCenter, text),
                ServerMessage::Warning(text) => (Self::LowCenter, text),
                ServerMessage::StatusText(text) => (Self::Status, text),
            }
        }

        /// OTClient `TextColors`: red `#f55e5e` for warnings, white for the rest.
        #[must_use]
        pub fn rgb(self) -> [u8; 3] {
            match self {
                Self::LowCenter => [0xf5, 0x5e, 0x5e],
                Self::MiddleCenter | Self::Status => [0xff, 0xff, 0xff],
            }
        }

        fn slot(self) -> usize {
            self as usize
        }
    }

    /// How long a message stays on screen.
    #[must_use]
    pub fn visible_for(text: &str) -> Duration {
        Duration::from_millis((text.chars().count() as u64 * 50).max(4_000))
    }

    /// The current text of each place; a new message replaces the old one of its place.
    #[derive(Debug, Clone, Default)]
    pub struct ScreenMessages {
        slots: [Option<(String, Instant)>; 3],
    }

    impl ScreenMessages {
        pub fn show(&mut self, message: &ServerMessage, now: Instant) {
            let (place, text) = ScreenPlace::of(message);
            if text.trim().is_empty() {
                return;
            }
            self.slots[place.slot()] = Some((text.to_owned(), now + visible_for(text)));
        }

        /// The text of `place` while it is still visible at `now`.
        #[must_use]
        pub fn text(&self, place: ScreenPlace, now: Instant) -> Option<&str> {
            self.slots[place.slot()]
                .as_ref()
                .filter(|(_, until)| now < *until)
                .map(|(text, _)| text.as_str())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn server_kinds_map_to_their_places_and_colours() {
            let warning = ServerMessage::Warning("Fire".into());
            let (place, text) = ScreenPlace::of(&warning);
            assert_eq!((place, text), (ScreenPlace::LowCenter, "Fire"));
            assert_eq!(place.rgb(), [0xf5, 0x5e, 0x5e]);
            let (place, _) = ScreenPlace::of(&ServerMessage::Advance("Level".into()));
            assert_eq!((place, place.rgb()), (ScreenPlace::MiddleCenter, [255; 3]));
            let (place, _) = ScreenPlace::of(&ServerMessage::StatusText("Hi".into()));
            assert_eq!((place, place.rgb()), (ScreenPlace::Status, [255; 3]));
        }

        #[test]
        fn duration_is_four_seconds_or_fifty_milliseconds_a_character() {
            assert_eq!(visible_for("short"), Duration::from_millis(4_000));
            assert_eq!(visible_for(&"x".repeat(80)), Duration::from_millis(4_000));
            assert_eq!(visible_for(&"x".repeat(100)), Duration::from_millis(5_000));
            assert_eq!(visible_for(&"é".repeat(100)), Duration::from_millis(5_000));
        }

        #[test]
        fn a_new_message_of_the_same_kind_replaces_the_old_and_others_stay() {
            let t = Instant::now();
            let mut screen = ScreenMessages::default();
            screen.show(&ServerMessage::Warning("one".into()), t);
            screen.show(&ServerMessage::StatusText("status".into()), t);
            screen.show(&ServerMessage::Warning("two".into()), t + Duration::from_secs(1));
            assert_eq!(screen.text(ScreenPlace::LowCenter, t), Some("two"));
            assert_eq!(screen.text(ScreenPlace::Status, t), Some("status"));
            assert_eq!(screen.text(ScreenPlace::MiddleCenter, t), None);
        }

        #[test]
        fn a_message_expires_on_its_own_timer() {
            let t = Instant::now();
            let mut screen = ScreenMessages::default();
            screen.show(&ServerMessage::Advance("x".repeat(100)), t);
            let at = |ms| t + Duration::from_millis(ms);
            assert!(screen.text(ScreenPlace::MiddleCenter, at(4_999)).is_some());
            assert!(screen.text(ScreenPlace::MiddleCenter, at(5_000)).is_none());
            screen.show(&ServerMessage::Advance("  ".into()), at(5_100));
            assert!(screen.text(ScreenPlace::MiddleCenter, at(5_100)).is_none());
        }
    }
}
