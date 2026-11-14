use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::hireling::HirelingPanel;

impl PlayLink {
    #[must_use]
    pub fn hireling_panel(&self) -> HirelingPanel {
        lock(&self.mailbox).hireling.clone()
    }

    #[cfg(windows)]
    pub fn open_hireling_panel(&self) {
        native::open_window(self);
    }
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn hireling_services_test_initialize(&self) {
        self.hireling_initialize(true);
    }
    pub(crate) fn hireling_services_test_response(&self, active: bool) {
        self.hireling_result(&oteryn_client_ui::hireling::HirelingResponse {
            status: oteryn_client_ui::hireling::CommandStatus::Accepted,
            active: Some(active),
        });
    }
    pub(crate) fn hireling_services_test_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}

#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub(crate) struct HirelingServicesSnapshot {
    pub panel: HirelingPanel,
    session: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
}
#[cfg(feature = "bevy-shell")]
impl PartialEq for HirelingServicesSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session) && self.panel == other.panel
    }
}
#[cfg(feature = "bevy-shell")]
#[derive(Clone, Copy)]
pub(crate) enum HirelingServicesAction {
    Toggle,
    Close,
    Prepare(oteryn_client_ui::hireling::Intent),
    Request {
        intent: oteryn_client_ui::hireling::Intent,
        confirmed: bool,
    },
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn hireling_services_snapshot(&self) -> HirelingServicesSnapshot {
        HirelingServicesSnapshot {
            panel: self.hireling_panel(),
            session: std::sync::Arc::downgrade(&self.mailbox),
        }
    }

    /// Validate the displayed session and target and enqueue at most one existing command
    /// under the owner lock. Refused submissions leave the retained panel untouched.
    pub(crate) fn hireling_services_dispatch_bound(
        &self,
        expected: &HirelingServicesSnapshot,
        action: HirelingServicesAction,
    ) -> bool {
        use oteryn_client_ui::hireling::{Intent, request};
        let mut mailbox = lock(&self.mailbox);
        if !expected
            .session
            .ptr_eq(&std::sync::Arc::downgrade(&self.mailbox))
            || mailbox.hireling != expected.panel
            || mailbox.hireling_shutdown
            || mailbox.inventory_shutdown
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
            || self.commands.is_closed()
        {
            return false;
        }
        let mut prepared = mailbox.hireling.clone();
        let intent = match action {
            HirelingServicesAction::Close => {
                if !prepared.open {
                    return false;
                }
                mailbox.hireling.open = false;
                return true;
            }
            HirelingServicesAction::Toggle => {
                prepared.open = !prepared.open;
                if !prepared.open || !prepared.selected || prepared.pending.is_some() {
                    mailbox.hireling = prepared;
                    return true;
                }
                Intent::Refresh
            }
            HirelingServicesAction::Prepare(intent)
            | HirelingServicesAction::Request { intent, .. } => {
                if !prepared.open
                    || !prepared.selected
                    || prepared.pending.is_some()
                    || intent == Intent::Summon && prepared.target.is_none()
                {
                    return false;
                }
                if matches!(action, HirelingServicesAction::Prepare(_)) {
                    return true;
                }
                if intent != Intent::Refresh
                    && !matches!(
                        action,
                        HirelingServicesAction::Request {
                            confirmed: true,
                            ..
                        }
                    )
                {
                    return false;
                }
                intent
            }
        };
        let Some(command) = request(&mut prepared, intent) else {
            return false;
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::Hireling(command))
            .is_err()
        {
            return false;
        }
        mailbox.hireling = prepared;
        true
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
#[path = "tests/bound.rs"]
mod bound_tests;
