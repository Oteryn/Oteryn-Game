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
            if let Some(document) = self.entry.as_mut().and_then(Reentry::take_bulletin) {
                login.bulletin(document);
            }
            login.finish(result.as_ref().map(|_| ()).map_err(|error| *error));
        }
        match result {
            Ok((mut view, link)) => {
                self.settings.restore(&mut view, &link);
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
    pub(super) fn leave_play(&mut self, message: &str) {
        if let Some(renderer) = &mut self.renderer {
            renderer.cancel_capture();
        }
        self.settings.reset_screenshots();
        self.audio.stop();
        self.play = None;
        if let Some(entry) = &mut self.entry {
            entry.leave_play();
        }
        if let Some(login) = &self.login {
            login.offline(message.to_owned());
        }
        lprintln!("Oteryn: {message}", message = message);
        if let Some(window) = &self.window {
            window.set_title(&lfmt!(
                "Oteryn — {message} — Enter: log in again",
                message = message
            ));
        }
        if self.rebuild_offline().is_err() {
            self.scene = None;
        }
    }

    pub(super) fn request_entry(&mut self) {
        if let Some(login) = &self.login {
            login.show(tr!("Review connection settings, then choose Save and sign in.").into());
        }
    }
    /// Reauthenticate after the consumed ticket, then show account character selection.
    pub(super) fn return_to_character_selection(&mut self) {
        self.leave_play(tr!("Choose a character after browser sign-in."));
        if let Some(login) = &self.login {
            login.finish(Ok(()));
        }
        if let Some(entry) = &mut self.entry {
            entry.request();
        }
    }
}
