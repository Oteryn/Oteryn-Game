//! Platform-independent settings window state, input and play-view synchronization.
use super::{PreferenceError, PreferenceInput, PreferenceStore, ReferencePreferences};
use crate::play::PlayView;
use oteryn_input_actions::NormalizedInputEvent;
use std::{
    io,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsPage {
    #[default]
    Gameplay,
    Interface,
    Graphics,
    Sound,
    Help,
}

impl SettingsPage {
    pub const ALL: [Self; 5] = [
        Self::Gameplay,
        Self::Interface,
        Self::Graphics,
        Self::Sound,
        Self::Help,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Gameplay => "Gameplay",
            Self::Interface => "Interface",
            Self::Graphics => "Graphics",
            Self::Sound => "Sound",
            Self::Help => "Help / settings transfer",
        }
    }
}

pub const WINDOW_ACTIONS: &[(&str, &str)] = &[
    ("0199-cyclopedia-character-open", "Cyclopedia"),
    ("0287-actor-spell-catalog-open", "Spellbook"),
    ("local-spell-action-bar-open", "Spell action bar"),
    ("0279-skill-progress-open", "Skills"),
    ("0276-condition-status-open", "Conditions"),
    ("0277-rest-status-open", "Rest and stamina"),
    ("0016-quest-log-open", "Quest log"),
    ("0007-chat-keybindings", "Chat"),
];

pub struct SettingsState {
    window_action: Option<&'static str>,
    battle_edit: u64,
    magical_archive_requested: bool,
    store: Option<PreferenceStore>,
    input: PreferenceInput,
    pub preferences: ReferencePreferences,
    pub feedback: String,
    pub open: bool,
    pub shutdown: bool,
    pub page: SettingsPage,
    last_key: std::collections::BTreeMap<oteryn_input_actions::KeyCode, std::time::Instant>,
}

impl SettingsState {
    pub fn request_magical_archive(&mut self) {
        self.magical_archive_requested = true;
    }
    pub fn take_magical_archive_request(&mut self) -> bool {
        std::mem::take(&mut self.magical_archive_requested)
    }

    pub fn queue_window(&mut self, id: &str) -> bool {
        let Some((id, _)) = WINDOW_ACTIONS.iter().find(|(key, _)| *key == id) else {
            return false;
        };
        self.window_action = Some(id);
        true
    }

    pub fn take_window_events(&mut self) -> Vec<NormalizedInputEvent> {
        self.window_action.take().map_or_else(Vec::new, |id| {
            super::bindings::window_events(&self.preferences, id)
        })
    }

    pub fn export_to(&self, path: &Path) -> io::Result<()> {
        PreferenceStore::export_to(&self.preferences, path)
    }

    pub fn import_from(&mut self, path: &Path) -> io::Result<()> {
        self.replace_saved(PreferenceStore::read_transfer(path)?)
    }

    pub fn reset_defaults(&mut self) -> io::Result<()> {
        self.replace_saved(ReferencePreferences::default())
    }

    pub fn restore_backup(&mut self) -> io::Result<()> {
        let path = self
            .store
            .as_ref()
            .ok_or_else(|| io::Error::other("Preference storage is unavailable"))?
            .backup_path();
        self.import_from(&path)
    }

    fn replace_saved(&mut self, preferences: ReferencePreferences) -> io::Result<()> {
        let store = self
            .store
            .as_mut()
            .ok_or_else(|| io::Error::other("Preference storage is unavailable"))?;
        store.replace_with_backup(preferences.clone())?;
        self.preferences = preferences;
        self.battle_edit = crate::play::feature_0017_attack::view::next_edit();
        Ok(())
    }

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
            window_action: None,
            battle_edit: 0,
            magical_archive_requested: false,
            store,
            preferences,
            feedback,
            open: false,
            shutdown: false,
            input: PreferenceInput::default(),
            page: SettingsPage::default(),
            last_key: Default::default(),
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
        let mut routed = self
            .input
            .route(&self.preferences, events)
            .unwrap_or_default();
        use oteryn_input_actions::{ButtonState, MouseButton};
        let now = std::time::Instant::now();
        routed.retain_mut(|event| {
            match event {
                NormalizedInputEvent::MouseButton { button, .. } if self.preferences.swap_mouse_buttons => {
                    *button = match *button {
                        MouseButton::PRIMARY => MouseButton::SECONDARY,
                        MouseButton::SECONDARY => MouseButton::PRIMARY,
                        other => other,
                    };
                }
                NormalizedInputEvent::Key { code, state: ButtonState::Pressed, repeat, .. } => {
                    if *repeat && self.last_key.get(code).is_some_and(|last| now.duration_since(*last).as_millis() < u128::from(self.preferences.keyboard_delay_ms)) {
                        return false;
                    }
                    self.last_key.insert(*code, now);
                }
                NormalizedInputEvent::Key { code, state: ButtonState::Released, .. } => { self.last_key.remove(code); }
                NormalizedInputEvent::FocusChanged { focused: false } | NormalizedInputEvent::DeviceLost => self.last_key.clear(),
                _ => {}
            }
            true
        });
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
        if self.preferences.battle_lists != preferences.battle_lists {
            self.battle_edit = crate::play::feature_0017_attack::view::next_edit();
        }
        self.preferences = preferences;
        self.feedback = match result {
            Ok(()) => "Applied and saved".into(),
            Err(error) => format!("Applied for this session; could not save: {error}"),
        };
    }

    pub fn sync_battle_lists(&mut self, link: &crate::play::PlayLink) {
        let mut preferences = self.preferences.clone();
        if let Some(edit) = link.sync_battle_lists(&mut preferences.battle_lists, self.battle_edit)
        {
            self.apply(preferences);
            self.battle_edit = edit;
        }
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
