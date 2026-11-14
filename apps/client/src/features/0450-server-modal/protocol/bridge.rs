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
impl SharedModal {
    pub(in crate::play) fn is_active(&self) -> bool {
        self.panel.active.is_some()
    }
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
        lock(&self.mailbox).server_modal.is_active()
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

/// Keeps the real driver's mailbox projection alive for frontend caller tests.
#[cfg(all(test, feature = "bevy-shell"))]
#[must_use]
pub(crate) struct BevyServerModalFixture {
    _driver: Driver,
    input_peer: Option<super::tests::AmountPeer>,
}
#[cfg(all(test, feature = "bevy-shell"))]
impl PlayLink {
    pub(crate) fn bevy_server_modal_fixture(&self) -> BevyServerModalFixture {
        self.use_shared_frontend();
        let mut driver = Driver::new(true).with_outbox(&Outbox(Arc::clone(&self.mailbox)));
        driver.panel.active = Some(oteryn_client_ui::server_modal::Dialog {
            id: 1,
            title: "Test server dialog".into(),
            message: "Frontend modal gating fixture".into(),
            choices: Vec::new(),
            buttons: vec![(1, "Close".into())],
            cancel_button: Some(1),
            image: None,
        });
        driver.publish(false);
        BevyServerModalFixture {
            _driver: driver,
            input_peer: None,
        }
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl PlayLink {
    pub(crate) fn bevy_server_modal_input_fixture(
        &self,
    ) -> Result<BevyServerModalFixture, Box<dyn std::error::Error>> {
        self.use_shared_frontend();
        let mut driver = Driver::new(true)
            .with_input(true)
            .with_outbox(&Outbox(Arc::clone(&self.mailbox)));
        let mut peer = super::tests::AmountPeer::default();
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(driver.poll(&mut peer))?;
        Ok(BevyServerModalFixture {
            _driver: driver,
            input_peer: Some(peer),
        })
    }
}
#[cfg(all(test, feature = "bevy-shell"))]
impl BevyServerModalFixture {
    pub(crate) fn with_choices(&mut self) {
        if let Some(dialog) = &mut self._driver.panel.active {
            dialog.choices = vec![(7, "First".into()), (9, "Second".into())];
        }
        self._driver.publish(false);
    }
    pub(crate) fn accepted_input(
        &mut self,
    ) -> Result<
        Option<oteryn_client_runtime::server_modal::input::InputAnswer>,
        Box<dyn std::error::Error>,
    > {
        let peer = self.input_peer.as_mut().ok_or("input fixture peer")?;
        let completion = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(self._driver.poll(peer))?;
        if completion.is_none() {
            return Ok(None);
        }
        let bytes = peer.answer.take().ok_or("accepted driver answer")?;
        Ok(Some(
            oteryn_client_runtime::server_modal::input::InputAnswer::decode(&bytes)
                .map_err(|_| "input answer decode")?,
        ))
    }
}
