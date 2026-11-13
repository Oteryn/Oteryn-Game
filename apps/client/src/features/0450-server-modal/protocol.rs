//! Session-owned modal transport; the window never owns or blocks the socket.
use crate::play::Stepper;
use oteryn_client_ui::server_modal::{self as ui, Answer, Panel};
use oteryn_client_ui::server_modal_input::Entry;
use oteryn_session::SessionError;
use std::sync::{Arc, Mutex, PoisonError};
use tokio::{
    sync::{mpsc, oneshot},
    time::{Duration, Instant},
};
pub(super) type Submission = (Answer, oneshot::Sender<bool>);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::play) enum Completion {
    Answered(Answer),
    LoggedOut,
}
pub(in crate::play) struct Driver {
    shared: Option<crate::play::mailbox::SharedMailbox>,
    panel: Panel,
    answers: mpsc::Receiver<Submission>,
    sender: mpsc::Sender<Submission>,
    next_query: Instant,
    shown: Option<u32>,
    /// What the dialog window collected next to the answer (capability 490).
    entry: Arc<Mutex<Entry>>,
}
impl Driver {
    pub(in crate::play) fn new(selected: bool) -> Self {
        let (sender, answers) = mpsc::channel(1);
        Self {
            shared: None,
            panel: Panel {
                selected,
                ..Panel::default()
            },
            answers,
            sender,
            next_query: Instant::now(),
            shown: None,
            entry: Arc::default(),
        }
    }
    /// Selects capability 490: dialogs may ask for a value and answers carry it.
    #[must_use]
    pub(in crate::play) fn with_input(mut self, selected: bool) -> Self {
        self.panel.input = selected;
        self
    }
    pub(in crate::play) async fn poll<T: Stepper>(
        &mut self,
        session: &mut T,
    ) -> Result<Option<Completion>, SessionError> {
        if !self.panel.selected {
            return Ok(None);
        }
        let mut completed = None;
        let mut answered = false;
        if let Ok((answer, acknowledged)) = self.answers.try_recv() {
            answered = true;
            let entry =
                std::mem::take(&mut *self.entry.lock().unwrap_or_else(PoisonError::into_inner));
            let accepted = if let Some(command) = ui::answer_with(&mut self.panel, answer, &entry) {
                let result = session.feature(&command).await?;
                ui::apply_result(&mut self.panel, &result)
            } else {
                false
            };
            completed = accepted.then_some(if self.panel.logged_out {
                Completion::LoggedOut
            } else {
                Completion::Answered(answer)
            });
            let _ = acknowledged.send(accepted);
        } else if Instant::now() >= self.next_query {
            self.next_query = Instant::now() + Duration::from_secs(5);
            if let Some(command) = ui::query(&mut self.panel) {
                let result = session.feature(&command).await?;
                ui::apply_result(&mut self.panel, &result);
            }
        }
        if let Some(dialog) = &self.panel.active {
            if self.shown != Some(dialog.id) {
                if !self.shared_frontend() {
                    super::view::open(
                        dialog.clone(),
                        self.panel.active_input.clone(),
                        self.panel.input,
                        (self.sender.clone(), self.entry.clone()),
                    );
                }
                self.shown = Some(dialog.id);
            }
        } else {
            self.shown = None;
        }
        self.publish(answered);
        Ok(completed)
    }
}
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "protocol/bridge_tests.rs"]
mod bridge_tests;

#[path = "protocol/bridge.rs"]
mod bridge;
pub use bridge::ServerModalPanel;
pub(in crate::play) use bridge::SharedModal;

#[cfg(test)]
pub(crate) use bridge::BevyServerModalFixture;
