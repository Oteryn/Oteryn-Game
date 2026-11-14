use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::cyclopedia_tracker::CyclopediaTrackerPanel;
impl PlayLink {
    #[must_use]
    pub fn cyclopedia_tracker_panel(&self) -> CyclopediaTrackerPanel {
        lock(&self.mailbox).cyclopedia_tracker.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

/// A view is tied to the mailbox that supplied it, even across identical session snapshots.
#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub(crate) struct TrackerBinding {
    pub panel: CyclopediaTrackerPanel,
    session: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
}

#[cfg(feature = "bevy-shell")]
pub(crate) enum TrackerViewAction {
    Toggle,
    Close,
    Boss,
    Widget(bool),
    Filter {
        boss: bool,
        search: String,
        sort: oteryn_client_ui::cyclopedia_tracker::TrackerSort,
        hide_completed: bool,
    },
    Request {
        intent: oteryn_client_ui::cyclopedia_tracker::Intent,
        boss: bool,
        creature_key: String,
        confirmed: bool,
    },
}

#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn cyclopedia_trackers_binding(&self) -> TrackerBinding {
        TrackerBinding {
            panel: self.cyclopedia_tracker_panel(),
            session: std::sync::Arc::downgrade(&self.mailbox),
        }
    }

    /// Validate and enqueue in one owner critical section. Never edits confirmed progress.
    pub(crate) fn cyclopedia_trackers_dispatch_bound(
        &self,
        expected: &TrackerBinding,
        action: TrackerViewAction,
        modal_blocked: bool,
    ) -> bool {
        use oteryn_client_ui::cyclopedia_tracker::{Intent, request};
        let mut mailbox = lock(&self.mailbox);
        if !expected
            .session
            .ptr_eq(&std::sync::Arc::downgrade(&self.mailbox))
            || mailbox.custom_hotkeys_ended
            || self.commands.is_closed()
            || modal_blocked
            || mailbox.server_modal.is_active()
            || mailbox.cyclopedia_tracker != expected.panel
        {
            return false;
        }
        let panel = &mut mailbox.cyclopedia_tracker;
        if matches!(action, TrackerViewAction::Toggle) {
            panel.open = !panel.open;
            return true;
        }
        if !panel.open {
            return false;
        }
        if matches!(action, TrackerViewAction::Close) {
            panel.open = false;
            return true;
        }
        if !panel.selected || panel.pending {
            return false;
        }
        match action {
            TrackerViewAction::Boss => panel.boss = !panel.boss,
            TrackerViewAction::Widget(boss) => {
                let mut next = panel.clone();
                let open = if boss {
                    &mut next.bosstiary_widget_open
                } else {
                    &mut next.bestiary_widget_open
                };
                *open = !*open;
                if *open {
                    next.boss = boss;
                    let Some(command) = request(&mut next, Intent::Read) else {
                        return false;
                    };
                    if self
                        .commands
                        .send(crate::combat_input::PlayCommand::Feature(command))
                        .is_err()
                    {
                        return false;
                    }
                }
                *panel = next;
            }
            TrackerViewAction::Filter {
                boss,
                search,
                sort,
                hide_completed,
            } => {
                if search.chars().count() > 128 {
                    return false;
                }
                if boss {
                    panel.boss_search = search;
                    panel.boss_sort = sort;
                    panel.boss_hide_completed = hide_completed;
                } else {
                    panel.search = search;
                    panel.sort = sort;
                    panel.hide_completed = hide_completed;
                }
            }
            TrackerViewAction::Request {
                intent,
                boss,
                creature_key,
                confirmed,
            } => {
                if intent == Intent::Remove && !confirmed {
                    return false;
                }
                // Prepare on a clone so invalid input and a disconnected queue change nothing.
                let mut next = panel.clone();
                next.creature_key = creature_key;
                next.boss = boss;
                let Some(command) = request(&mut next, intent) else {
                    return false;
                };
                if self
                    .commands
                    .send(crate::combat_input::PlayCommand::Feature(command))
                    .is_err()
                {
                    return false;
                }
                *panel = next;
            }
            TrackerViewAction::Toggle | TrackerViewAction::Close => return false,
        }
        true
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_cyclopedia_trackers_initialize(&self) {
        self.cyclopedia_tracker_initialize(true);
    }
    pub(crate) fn bevy_cyclopedia_trackers_result(
        &self,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        self.cyclopedia_tracker_result(result);
    }
    pub(crate) fn bevy_cyclopedia_trackers_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
#[path = "tests/bound.rs"]
mod bound_tests;
