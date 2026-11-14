use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::imbue::ImbuePanel;

impl PlayLink {
    #[must_use]
    pub fn imbue_panel(&self) -> ImbuePanel {
        lock(&self.mailbox).imbue.clone()
    }

    pub fn open_imbue(&self) {
        self.imbue_request(oteryn_client_ui::imbue::Intent::Refresh);
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).imbue.open = true;
        }
    }
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

impl PlayLink {
    pub fn open_imbue_tracker(&self) {
        if lock(&self.mailbox).imbue.tracker_open {
            return;
        }
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            use crate::play::feature_0115_mail::view::native::{Form, open};
            open(
                self,
                Form {
                    title: "Imbuement time tracker — Ctrl+Alt+Shift+I",
                    labels: &["Maximum minutes (0 = all)"],
                    buttons: &["Apply time filter", "Refresh equipment"],
                    snapshot: |m| {
                        (
                            m.imbue.tracker_open,
                            m.imbue.equipment.pending.is_some(),
                            vec![m.imbue.tracker_minutes.clone()],
                            oteryn_client_ui::imbue::tracker_text(&m.imbue),
                        )
                    },
                    set_open: |m, value| m.imbue.tracker_open = value,
                    submit: |m, commands, action, values| {
                        if let Some(value) = values.first() {
                            super::input::tracker::set_filter(m, value);
                        }
                        if action == 1 {
                            super::input::tracker::enqueue(m, commands, true);
                        }
                    },
                },
            );
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).imbue.tracker_open = true;
        }
        self.imbue_tracker_refresh();
        #[cfg(windows)]
        super::input::tracker::poll(
            std::sync::Arc::downgrade(&self.mailbox),
            self.commands.clone(),
        );
    }
}

#[cfg(feature = "bevy-shell")]
#[derive(Clone, Debug)]
pub struct ImbueSnapshot {
    pub panel: ImbuePanel,
    pub items: oteryn_client_runtime::depot::Items,
    containers: Option<oteryn_client_runtime::container_tree::ContainerViews>,
    session: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
}
#[cfg(feature = "bevy-shell")]
impl PartialEq for ImbueSnapshot {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Weak::ptr_eq(&self.session, &other.session)
            && self.panel == other.panel
            && self.items == other.items
            && self.containers == other.containers
    }
}
#[cfg(feature = "bevy-shell")]
impl Eq for ImbueSnapshot {}
#[cfg(feature = "bevy-shell")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImbueControl {
    Item(oteryn_client_ui::imbue::ItemHandle),
    Slot(u8),
    Offer(usize),
    Refresh,
    Details,
    Apply,
    Clear,
    TrackerRefresh,
    Close,
    CloseTracker,
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub fn imbue_snapshot(&self) -> ImbueSnapshot {
        let m = lock(&self.mailbox);
        ImbueSnapshot {
            panel: m.imbue.clone(),
            items: m.depot.items.clone(),
            containers: m.containers.views.clone(),
            session: std::sync::Arc::downgrade(&self.mailbox),
        }
    }

    /// Compare and dispatch in one owner critical section; no predicted item state.
    pub fn imbue_dispatch_bound(&self, expected: &ImbueSnapshot, control: ImbueControl) -> bool {
        use ImbueControl as C;
        use oteryn_client_ui::imbue::{Intent, request};
        let mut m = lock(&self.mailbox);
        if !std::sync::Weak::ptr_eq(&expected.session, &std::sync::Arc::downgrade(&self.mailbox))
            || m.containers.views != expected.containers
            || self.commands.is_closed()
            || m.imbue_shutdown
            || m.world_input_blocked()
            || m.server_modal.is_active()
            || m.imbue != expected.panel
            || m.depot.items != expected.items
            || !m.imbue.selected
            || m.imbue.pending
            || m.imbue.details_pending.is_some()
        {
            return false;
        }
        let tracker = matches!(control, C::TrackerRefresh | C::CloseTracker);
        if (tracker && !m.imbue.tracker_open) || (!tracker && !m.imbue.open) {
            return false;
        }
        if !matches!(
            control,
            C::Item(_) | C::Close | C::CloseTracker | C::TrackerRefresh
        ) && m.imbue.item.is_none_or(|item| {
            !super::input::carried_items(&m)
                .iter()
                .any(|entry| entry.handle == item)
        }) {
            return false;
        }
        let before = m.imbue.clone();
        let inspection = m.discovery.inspection.clone();
        let accepted = (|| match control {
            C::Item(item) => {
                if !super::input::carried_items(&m)
                    .iter()
                    .any(|entry| entry.handle == item)
                {
                    return false;
                }
                m.imbue.select_item(item);
                let Some(command) = request(&mut m.imbue, Intent::Refresh) else {
                    return false;
                };
                self.commands
                    .send(crate::combat_input::PlayCommand::Imbue(command))
                    .is_ok()
            }
            C::Slot(slot) => m.imbue.select_slot(slot),
            C::Offer(offer) => m.imbue.select_offer(offer),
            C::Close => {
                m.imbue.open = false;
                true
            }
            C::CloseTracker => {
                m.imbue.tracker_open = false;
                true
            }
            C::TrackerRefresh => super::input::tracker::enqueue(&mut m, &self.commands, true),
            C::Details => {
                let Some(command) = oteryn_client_ui::imbue::details::request(&mut m.imbue) else {
                    return false;
                };
                self.commands
                    .send(crate::combat_input::PlayCommand::Feature(command))
                    .is_ok()
            }
            C::Refresh | C::Apply | C::Clear => {
                let intent = match control {
                    C::Apply => Intent::Apply,
                    C::Clear => Intent::Clear,
                    _ => Intent::Refresh,
                };
                if m.imbue.refresh_required && intent != Intent::Refresh {
                    return false;
                }
                let Some(command) = request(&mut m.imbue, intent) else {
                    return false;
                };
                self.commands
                    .send(crate::combat_input::PlayCommand::Imbue(command))
                    .is_ok()
            }
        })();
        if !accepted {
            m.imbue = before;
            m.discovery.inspection = inspection;
        }
        accepted
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
#[path = "tests/bound.rs"]
mod bound_tests;

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_imbue_fixture(&self, panel: ImbuePanel) {
        let mut m = lock(&self.0);
        let item = panel.item;
        m.imbue = panel;
        m.imbue_shutdown = false;
        if let Some(handle) = item {
            m.depot.items.inventory.entries = vec![oteryn_client_runtime::depot::ItemEntry {
                handle,
                item_definition_ref: std::num::NonZeroU32::MIN,
                count: std::num::NonZeroU32::MIN,
                sub_type: 0,
            }];
        }
    }
    pub(crate) fn bevy_imbue_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}
