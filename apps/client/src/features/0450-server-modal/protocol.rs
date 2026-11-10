//! Session-owned modal transport; the window never owns or blocks the socket.
use crate::play::Stepper;
use oteryn_client_ui::server_modal::{self as ui, Answer, Panel};
use oteryn_session::SessionError;
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
    panel: Panel,
    answers: mpsc::Receiver<Submission>,
    sender: mpsc::Sender<Submission>,
    next_query: Instant,
    shown: Option<u32>,
}
impl Driver {
    pub(in crate::play) fn new(selected: bool) -> Self {
        let (sender, answers) = mpsc::channel(1);
        Self {
            panel: Panel {
                selected,
                ..Panel::default()
            },
            answers,
            sender,
            next_query: Instant::now(),
            shown: None,
        }
    }
    pub(in crate::play) async fn poll<T: Stepper>(
        &mut self,
        session: &mut T,
    ) -> Result<Option<Completion>, SessionError> {
        if !self.panel.selected {
            return Ok(None);
        }
        let mut completed = None;
        if let Ok((answer, acknowledged)) = self.answers.try_recv() {
            let accepted = if let Some(command) = ui::answer(&mut self.panel, answer) {
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
                super::view::open(dialog.clone(), self.sender.clone());
                self.shown = Some(dialog.id);
            }
        } else {
            self.shown = None;
        }
        Ok(completed)
    }
}
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
