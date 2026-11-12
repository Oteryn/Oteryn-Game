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
    /// Loot results; they have their own tab and fold into the others while it is closed.
    Loot,
}
/// Which notices a console view lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeScope {
    /// Every notice, except loot while the Loot tab is open.
    Combined {
        loot_tab_open: bool,
    },
    LootOnly,
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
        self.scoped_text(
            filters,
            display,
            NoticeScope::Combined {
                loot_tab_open: false,
            },
        )
    }
    pub fn scoped_text(
        &self,
        filters: ConsoleFilters,
        display: ConsoleDisplay,
        scope: NoticeScope,
    ) -> String {
        let mut text = String::new();
        for notice in &self.entries {
            let in_scope = match scope {
                NoticeScope::Combined { loot_tab_open } => {
                    !(loot_tab_open && notice.class == NoticeClass::Loot)
                }
                NoticeScope::LootOnly => notice.class == NoticeClass::Loot,
            };
            let (visible, label) = match notice.class {
                NoticeClass::Event => (filters.events, "Event"),
                NoticeClass::Status => (filters.status, "Status"),
                NoticeClass::Store => (filters.store, "Store"),
                NoticeClass::Loot => (filters.information, "Loot"),
            };
            let visible = visible && in_scope;
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
