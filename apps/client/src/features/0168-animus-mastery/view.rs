use crate::play::{PlayLink, mailbox::lock};

impl PlayLink {
    pub fn animus_panel(&self) -> oteryn_client_ui::animus_mastery::AnimusPanel {
        lock(&self.mailbox).animus.clone()
    }
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

/// A single-use presentation snapshot tied to the mailbox that supplied it.
#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub struct AnimusBinding {
    session: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    pub panel: oteryn_client_ui::animus_mastery::AnimusPanel,
    consumed: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
#[cfg(feature = "bevy-shell")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AnimusControl {
    Toggle,
    Close,
    Request(oteryn_client_ui::animus_mastery::Intent),
}
#[cfg(feature = "bevy-shell")]
impl AnimusBinding {
    pub fn consumed(&self) -> bool {
        self.consumed.load(std::sync::atomic::Ordering::Relaxed)
    }
    fn consume(&self) {
        self.consumed
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn same_session(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session)
    }
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub fn animus_binding(&self) -> AnimusBinding {
        AnimusBinding {
            session: std::sync::Arc::downgrade(&self.mailbox),
            panel: lock(&self.mailbox).animus.clone(),
            consumed: Default::default(),
        }
    }

    /// Admission, snapshot validation, queueing and pending state share one owner lock.
    pub fn animus_dispatch_bound(
        &self,
        expected: &AnimusBinding,
        control: AnimusControl,
        admitted: bool,
    ) -> bool {
        use oteryn_client_ui::animus_mastery::{Intent, request};
        let mut mailbox = lock(&self.mailbox);
        if !admitted
            || expected.consumed()
            || !expected
                .session
                .ptr_eq(&std::sync::Arc::downgrade(&self.mailbox))
            || mailbox.animus != expected.panel
            || mailbox.lens.active
            || mailbox.quest_shutdown
            || mailbox.inventory_shutdown
            || self.commands.is_closed()
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
        {
            return false;
        }
        let mut next = mailbox.animus.clone();
        let intent = match control {
            AnimusControl::Close | AnimusControl::Toggle if next.open => {
                next.open = false;
                mailbox.animus = next;
                expected.consume();
                return true;
            }
            AnimusControl::Toggle => {
                next.open = true;
                // Reopening an in-flight or unavailable panel must not invent a query.
                if !next.selected || next.pending.is_some() {
                    mailbox.animus = next;
                    expected.consume();
                    return true;
                }
                Intent::Refresh
            }
            AnimusControl::Request(intent) if next.open => intent,
            AnimusControl::Close | AnimusControl::Request(_) => return false,
        };
        let Some(payload) = request(&mut next, intent) else {
            return false;
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::Animus(payload))
            .is_err()
        {
            return false;
        }
        mailbox.animus = next;
        expected.consume();
        true
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_animus_selected(&self) {
        self.animus_initialize(true);
    }
    pub(crate) fn bevy_animus_reply(&self, result: &oteryn_session::GatedCommandResult) {
        self.animus_result(result);
    }
    pub(crate) fn bevy_animus_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
#[path = "tests/bound.rs"]
mod bound_tests;
