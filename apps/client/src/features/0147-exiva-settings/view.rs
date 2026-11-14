use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn exiva_panel(&self) -> oteryn_client_ui::exiva_settings::ExivaPanel {
        lock(&self.mailbox).exiva.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_exiva_initialize(&self, selected: bool) {
        self.exiva_initialize(selected);
    }
    pub(crate) fn bevy_exiva_response(&self, result: &oteryn_session::GatedCommandResult) {
        self.exiva_result(result);
    }
    pub(crate) fn bevy_exiva_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
#[path = "tests/bound.rs"]
mod bound_tests;

/// A server-owned projection tied to the exact session mailbox, even before HostOwner polls.
#[cfg(feature = "bevy-shell")]
#[derive(Clone, Debug)]
pub(crate) struct ExivaSnapshot {
    pub panel: oteryn_client_ui::exiva_settings::ExivaPanel,
    pub live: bool,
    // Cloned controls share one consent; an identical server reply cannot revive it.
    submitted: std::sync::Arc<std::sync::atomic::AtomicBool>,
    session: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
}
#[cfg(feature = "bevy-shell")]
impl PartialEq for ExivaSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session) && self.live == other.live && self.panel == other.panel
    }
}
#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub(crate) enum ExivaControl {
    Open,
    Close,
    Refresh,
    Toggle(u8),
    AddName(String),
    Remove([u8; 16]),
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn exiva_snapshot(&self) -> ExivaSnapshot {
        let mailbox = lock(&self.mailbox);
        ExivaSnapshot {
            panel: mailbox.exiva.clone(),
            live: !mailbox.quest_shutdown && !self.commands.is_closed(),
            submitted: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            session: std::sync::Arc::downgrade(&self.mailbox),
        }
    }
    /// Admission, target validation, pending latch and queue submission share the owner lock.
    /// A rejected request leaves even local error/pending state unchanged.
    pub(crate) fn exiva_dispatch_bound(
        &self,
        expected: &ExivaSnapshot,
        control: ExivaControl,
        confirmed: bool,
    ) -> bool {
        if !expected
            .session
            .ptr_eq(&std::sync::Arc::downgrade(&self.mailbox))
        {
            return false;
        }
        let mut mailbox = lock(&self.mailbox);
        if expected
            .submitted
            .load(std::sync::atomic::Ordering::Relaxed)
            || !expected.live
            || mailbox.quest_shutdown
            || self.commands.is_closed()
            || mailbox.server_modal.is_active()
            || mailbox.exiva != expected.panel
        {
            return false;
        }
        let panel = &mut mailbox.exiva;
        if matches!(control, ExivaControl::Close) {
            if !panel.open {
                return false;
            }
            panel.open = false;
            expected
                .submitted
                .store(true, std::sync::atomic::Ordering::Relaxed);
            return true;
        }
        let opening = matches!(control, ExivaControl::Open);
        if opening == panel.open || panel.pending {
            return false;
        }
        // Opening an unavailable feature still presents the owner's unavailable message.
        if opening && !panel.selected {
            panel.open = true;
            expected
                .submitted
                .store(true, std::sync::atomic::Ordering::Relaxed);
            return true;
        }
        let payload = match control {
            ExivaControl::Open | ExivaControl::Refresh => vec![5],
            ExivaControl::Toggle(bit) => {
                let Some(list) = &panel.list else {
                    return false;
                };
                if bit >= 5 {
                    return false;
                }
                vec![0, list.modes ^ (1 << bit)]
            }
            ExivaControl::AddName(name) => {
                if panel.list.is_none() {
                    return false;
                }
                let mut payload = vec![6];
                payload.extend(name.as_bytes());
                payload
            }
            ExivaControl::Remove(id) => {
                if !confirmed
                    || !panel
                        .list
                        .as_ref()
                        .is_some_and(|list| list.players.iter().any(|(held, _)| *held == id))
                {
                    return false;
                }
                let mut payload = vec![2];
                payload.extend(id);
                payload
            }
            ExivaControl::Close => return false,
        };
        let mut next = panel.clone();
        let Some(payload) = oteryn_client_ui::exiva_settings::request(&mut next, payload) else {
            return false;
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::Exiva(payload))
            .is_err()
        {
            return false;
        }
        if opening {
            next.open = true;
        }
        *panel = next;
        expected
            .submitted
            .store(true, std::sync::atomic::Ordering::Relaxed);
        true
    }
}
#[cfg(feature = "bevy-shell")]
impl ExivaSnapshot {
    pub(crate) fn same_session(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session)
    }
}
