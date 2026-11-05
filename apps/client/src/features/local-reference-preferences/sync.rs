//! Platform-independent settings window state, input and play-view synchronization.
use super::{PreferenceError, PreferenceInput, PreferenceStore, ReferencePreferences};
use crate::play::PlayView;
use oteryn_input_actions::NormalizedInputEvent;
use std::path::PathBuf;

pub struct SettingsState {
    store: Option<PreferenceStore>,
    input: PreferenceInput,
    pub preferences: ReferencePreferences,
    pub feedback: String,
    pub open: bool,
    pub shutdown: bool,
}

impl SettingsState {
    pub fn new(path: Option<PathBuf>) -> Self {
        let (store, feedback) = match path {
            Some(path) => match PreferenceStore::load(path.clone()) {
                Ok(store) => (Some(store), String::new()),
                Err(error) => (
                    Some(PreferenceStore::defaults(path)),
                    format!("Could not load preferences: {error}"),
                ),
            },
            None => (
                None,
                "LOCALAPPDATA is unavailable; preferences cannot be saved".into(),
            ),
        };
        let preferences = store
            .as_ref()
            .map_or_else(ReferencePreferences::default, |s| s.preferences().clone());
        Self {
            store,
            preferences,
            feedback,
            open: false,
            shutdown: false,
            input: PreferenceInput::default(),
        }
    }

    pub fn request_open(&mut self) -> bool {
        if self.open || self.shutdown {
            return false;
        }
        self.open = true;
        true
    }

    pub fn input(&mut self, events: &[NormalizedInputEvent]) -> (Vec<NormalizedInputEvent>, bool) {
        let routed = self
            .input
            .route(&self.preferences, events)
            .unwrap_or_default();
        if oteryn_client_ui::reference_preferences::open_key(&routed) {
            (Vec::new(), self.request_open())
        } else {
            (routed, false)
        }
    }

    pub fn apply(&mut self, preferences: ReferencePreferences) {
        if let Err(error) = preferences.validate() {
            self.feedback = error.to_string();
            return;
        }
        let result = self
            .store
            .as_mut()
            .ok_or_else(|| "Preference storage is unavailable".to_owned())
            .and_then(|store| store.apply(preferences.clone()).map_err(|e| e.to_string()));
        // A disk error must not disable local controls for this process.
        self.preferences = preferences;
        self.feedback = match result {
            Ok(()) => "Applied and saved".into(),
            Err(error) => format!("Applied for this session; could not save: {error}"),
        };
    }

    pub fn restore(
        &mut self,
        view: &mut PlayView,
        viewport: [u32; 2],
    ) -> Result<(), PreferenceError> {
        view.set_world_viewport(viewport)?;
        if view.preferences() != &self.preferences {
            view.set_preferences(self.preferences.clone())?;
        }
        Ok(())
    }

    pub fn capture(&mut self, view: &mut PlayView) -> Result<(), PreferenceError> {
        if view.preferences().audio != self.preferences.audio {
            let mut preferences = self.preferences.clone();
            preferences.audio = view.preferences().audio;
            self.apply(preferences);
        }
        // Merge any binding/zoom edit made while the shortcut was being handled.
        if view.preferences() != &self.preferences {
            view.set_preferences(self.preferences.clone())?;
        }
        Ok(())
    }
}
