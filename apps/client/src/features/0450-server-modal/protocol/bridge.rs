//! Session mailbox projection; the driver remains the sole reducer and transport owner.
use super::{Answer, Driver, Entry, Panel, Submission};
use crate::play::{Outbox, PlayLink, mailbox::lock};
use std::sync::{Arc, Mutex, PoisonError};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerModalPanel {
    /// Changes whenever the active server descriptor changes, including closure.
    pub revision: u64,
    pub panel: Panel,
    pub queued: bool,
}

#[derive(Debug, Default)]
pub(in crate::play) struct SharedModal {
    revision: u64,
    panel: Panel,
    sender: Option<mpsc::Sender<Submission>>,
    entry: Option<Arc<Mutex<Entry>>>,
    queued: bool,
}
impl Driver {
    pub(in crate::play) fn with_outbox(mut self, outbox: &Outbox) -> Self {
        self.shared = Some(Arc::clone(&outbox.0));
        self
    }
    pub(super) fn shared_frontend(&self) -> bool {
        self.shared
            .as_ref()
            .is_some_and(|shared| lock(shared).shared_frontend)
    }
    pub(super) fn publish(&self, answered: bool) {
        let Some(shared) = &self.shared else {
            return;
        };
        let mut mailbox = lock(shared);
        let state = &mut mailbox.server_modal;
        if state.panel.active != self.panel.active
            || state.panel.active_input != self.panel.active_input
        {
            let Some(next) = state.revision.checked_add(1) else {
                state.sender = None;
                state.entry = None;
                return;
            };
            state.revision = next;
            state.queued = false;
        }
        state.panel.clone_from(&self.panel);
        if answered {
            state.queued = false;
        }
        if self.panel.active.is_some() {
            state.sender = Some(self.sender.clone());
            state.entry = Some(Arc::clone(&self.entry));
        } else {
            state.sender = None;
            state.entry = None;
        }
    }
}
impl Drop for Driver {
    fn drop(&mut self) {
        if let Some(shared) = &self.shared {
            let mut mailbox = lock(shared);
            let state = &mut mailbox.server_modal;
            state.panel = Panel::default();
            state.sender = None;
            state.entry = None;
            state.queued = false;
        }
    }
}
impl PlayLink {
    #[must_use]
    pub fn server_modal_active(&self) -> bool {
        lock(&self.mailbox).server_modal.panel.active.is_some()
    }

    #[must_use]
    pub fn server_modal_panel(&self) -> Option<ServerModalPanel> {
        let mailbox = lock(&self.mailbox);
        let state = &mailbox.server_modal;
        state.panel.active.as_ref()?;
        Some(ServerModalPanel {
            revision: state.revision,
            panel: state.panel.clone(),
            queued: state.queued,
        })
    }
    /// Validates against the current descriptor and queues at most one answer without blocking.
    pub fn server_modal_submit(&self, revision: u64, answer: Answer, entry: Entry) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !mailbox.shared_frontend {
            return false;
        }
        let state = &mut mailbox.server_modal;
        if state.revision != revision || state.queued {
            return false;
        }
        let mut panel = state.panel.clone();
        if oteryn_client_ui::server_modal::answer_with(&mut panel, answer, &entry).is_none() {
            return false;
        }
        let (Some(sender), Some(collected)) = (&state.sender, &state.entry) else {
            return false;
        };
        if sender.is_closed() {
            return false;
        }
        // The queued latch stays set until the session driver has consumed this exact entry.
        let mut collected = collected.lock().unwrap_or_else(PoisonError::into_inner);
        let previous = std::mem::replace(&mut *collected, entry);
        let (acknowledged, _) = oneshot::channel();
        if sender.try_send((answer, acknowledged)).is_err() {
            *collected = previous;
            return false;
        }
        state.queued = true;
        true
    }
}
