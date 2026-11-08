//! Native login lifecycle adapter for the Windows shell.
use super::*;

impl Application {
    pub(super) fn poll_entry(&mut self) {
        if self
            .login
            .as_ref()
            .is_some_and(LoginWindow::cancel_requested)
            && let Some(entry) = &mut self.entry
        {
            entry.cancel_entry();
        }
        if let Some(message) = self
            .entry
            .as_mut()
            .and_then(|entry| entry.poll_wait(std::time::Instant::now()))
            && let Some(login) = &self.login
        {
            login.progress(message);
        }
        if let Some(config) = self.login.as_ref().and_then(LoginWindow::poll) {
            let started = self
                .entry
                .as_mut()
                .is_some_and(|entry| entry.configure(config) && entry.request());
            if !started && let Some(login) = &self.login {
                login.finish(Err(GameplayEntryError::Rejected(
                    PublicClass::SessionUnavailable,
                )));
            }
        }
        let Some(result) = self.entry.as_mut().and_then(Reentry::poll) else {
            return;
        };
        if let Some(login) = &self.login {
            login.finish(result.as_ref().map(|_| ()).map_err(|error| *error));
        }
        match result {
            Ok((mut view, link)) => {
                self.settings.restore(&mut view);
                self.scene = None;
                self.play = Some(Play {
                    view,
                    link,
                    status: String::new(),
                });
            }
            Err(error) => self.leave_play(error_text(error)),
        }
    }
}
