//! Bounded general game-event presentations shared by banner and popup surfaces.
use std::collections::VecDeque;
#[derive(Debug, Clone, PartialEq, Eq)]
struct EventNotice {
    id: String,
    text: String,
    popup: bool,
    expires_at_ms: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationCenter {
    pub banners_enabled: bool,
    pub popups_enabled: bool,
    entries: VecDeque<EventNotice>,
}
impl Default for NotificationCenter {
    fn default() -> Self {
        Self {
            banners_enabled: true,
            popups_enabled: true,
            entries: VecDeque::new(),
        }
    }
}
impl NotificationCenter {
    /// Time is supplied by the UI clock, making expiry independent of server clock skew.
    pub fn show(
        &mut self,
        id: &str,
        text: &str,
        popup: bool,
        now_ms: u64,
        lifetime_ms: u32,
    ) -> bool {
        if id.is_empty()
            || id.len() > 128
            || id.contains('\0')
            || text.trim().is_empty()
            || text.len() > 4096
            || text.contains('\0')
            || !(100..=60_000).contains(&lifetime_ms)
        {
            return false;
        }
        self.expire(now_ms);
        self.entries.retain(|entry| entry.id != id);
        if self.entries.len() == 32 {
            self.entries.pop_front();
        }
        self.entries.push_back(EventNotice {
            id: id.into(),
            text: text.into(),
            popup,
            expires_at_ms: now_ms.saturating_add(u64::from(lifetime_ms)),
        });
        true
    }
    pub fn expire(&mut self, now_ms: u64) {
        self.entries.retain(|entry| entry.expires_at_ms > now_ms);
    }
    pub fn dismiss(&mut self, id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.id != id);
        before != self.entries.len()
    }
    pub fn text(&self) -> String {
        let mut text = String::new();
        for entry in &self.entries {
            if (entry.popup && self.popups_enabled) || (!entry.popup && self.banners_enabled) {
                text.push_str(if entry.popup { "Popup: " } else { "Banner: " });
                text.push_str(&entry.text);
                text.push_str("\r\n");
            }
        }
        text
    }
}
