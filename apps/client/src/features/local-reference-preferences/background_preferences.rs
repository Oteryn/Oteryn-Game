use super::settings_actions::PreferenceSaveCompletion;
use super::*;

impl SettingsState {
    pub(crate) fn use_background_preferences(&mut self) {
        self.background_preferences = true;
    }

    pub(crate) fn preference_transfer_pending(&self) -> bool {
        self.store.as_ref().is_some_and(PreferenceStore::replacing)
    }

    #[cfg(bevy_shell)]
    pub(crate) fn preference_save_pending(&self) -> bool {
        self.store.as_ref().is_some_and(PreferenceStore::saving) || self.session_save.is_some()
    }

    #[cfg(bevy_shell)]
    pub(crate) fn poll_background_save(&mut self) -> PreferenceSaveCompletion {
        let mut completion = PreferenceSaveCompletion::default();
        let mut failure = None;
        let mut saved = false;
        if let Some(results) = self
            .store
            .as_mut()
            .and_then(PreferenceStore::poll_background_save)
        {
            if let Some(mut restored) = results
                .iter()
                .rev()
                .find_map(|result| result.as_ref().ok()?.as_ref())
                .cloned()
            {
                restored.miscellaneous = self.preferences.miscellaneous;
                completion.replaced = true;
                self.accept_replaced(restored);
            }
            failure = results.into_iter().find_map(Result::err);
            saved = self.session_save.is_none() && self.feedback == "Saving preferences…";
        }
        self.outgoing_stores.retain_mut(|store| {
            if let Some(results) = store.poll_background_save() {
                let error = results.into_iter().find_map(Result::err);
                failure = failure.take().or(error);
            }
            store.saving()
        });
        if !self.store.as_ref().is_some_and(PreferenceStore::saving)
            && let Some(result) = self
                .session_save
                .as_mut()
                .and_then(bevy_tasks::futures::check_ready)
        {
            self.session_save = None;
            self.session_queued = 0;
            match result {
                Err(error) => failure = failure.or_else(|| Some(io::Error::other(error))),
                Ok(()) if self.feedback == "Saving preferences…" => saved = true,
                Ok(()) => {}
            }
        }
        if saved {
            self.feedback = tr!("Applied and saved").into();
        }
        if let Some(error) = failure {
            let message = lfmt!(
                "Applied for this session; could not save: {error}",
                error = error
            );
            self.feedback.clone_from(&message);
            completion.error = Some(message);
        }
        completion
    }

    pub(crate) fn local_preference_writes_pending(&self) -> bool {
        self.preference_save_pending() || self.outgoing_stores.iter().any(PreferenceStore::saving)
    }
}
