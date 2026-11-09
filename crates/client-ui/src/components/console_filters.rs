use crate::chat::{ConsoleDisplay, ConsoleTab};
use oteryn_client_runtime::chat::ChatLine;
use std::{collections::VecDeque, fmt::Write};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsoleFilters {
    pub information: bool,
    pub events: bool,
    pub status: bool,
    pub private_messages: bool,
    pub store: bool,
    pub private_tab: bool,
}
impl Default for ConsoleFilters {
    fn default() -> Self {
        Self {
            information: true,
            events: true,
            status: true,
            private_messages: true,
            store: true,
            private_tab: false,
        }
    }
}
impl ConsoleFilters {
    pub fn visible(self, tab: ConsoleTab, line: &ChatLine) -> bool {
        let private = matches!(line, ChatLine::Private { .. } | ChatLine::PrivateChannel(_));
        !private || (self.private_messages && !(self.private_tab && tab == ConsoleTab::All))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeClass {
    Event,
    Status,
    Store,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Notice {
    class: NoticeClass,
    text: String,
    received: u64,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConsoleNotices {
    entries: VecDeque<Notice>,
}
impl ConsoleNotices {
    pub fn push(&mut self, class: NoticeClass, text: &str, received: u64) {
        if self.entries.len() == 100 {
            self.entries.pop_front();
        }
        self.entries.push_back(Notice {
            class,
            text: text.chars().take(1024).collect(),
            received,
        });
    }
    pub fn text(&self, filters: ConsoleFilters, display: ConsoleDisplay) -> String {
        let mut text = String::new();
        for notice in &self.entries {
            let (visible, label) = match notice.class {
                NoticeClass::Event => (filters.events, "Event"),
                NoticeClass::Status => (filters.status, "Status"),
                NoticeClass::Store => (filters.store, "Store"),
            };
            if visible {
                let _ = writeln!(
                    text,
                    "{}[{label}] {}",
                    display.timestamp(notice.received),
                    notice.text
                );
            }
        }
        text
    }
}
