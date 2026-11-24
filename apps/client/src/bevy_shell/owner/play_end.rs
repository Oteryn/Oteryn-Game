use super::*;

impl HostOwner {
    fn show_training_confirmation(&self) -> bool {
        self.settings
            .preferences
            .game_window
            .offline_training_messages
            && self
                .play
                .as_ref()
                .is_some_and(|play| play.link.offline_training_confirmed())
    }
    pub(super) fn finish_play(&mut self, end: PlayEnd) {
        let message = match end {
            PlayEnd::LoggedOut if self.show_training_confirmation() => {
                "Offline training confirmed. Logged out.".into()
            }
            PlayEnd::LoggedOut => "Logged out. Choose a character after signing in again.".into(),
            PlayEnd::Ended(class) => error_text(crate::GameplayEntryError::Rejected(class)),
        };
        self.leave(&message);
        if matches!(end, PlayEnd::LoggedOut)
            && self.entry.as_mut().is_some_and(|entry| {
                entry.set_session_options(self.settings.preferences.miscellaneous);
                entry.is_entering() || entry.request_picker()
            })
        {
            self.phase = Phase::Entering;
        }
    }
}

#[cfg(test)]
pub(crate) fn offline_training_logout_status(
    link: crate::play::PlayLink,
) -> Result<String, String> {
    let mut owner = HostOwner::new(None).map_err(|e| e.to_string())?;
    owner
        .settings
        .preferences
        .game_window
        .offline_training_messages = true;
    owner.play = Some(ActivePlay {
        link,
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])
            .map_err(|e| e.to_string())?,
    });
    owner.poll([800, 600]);
    Ok(std::mem::take(&mut owner.status))
}
