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
    Hotkeys,
    BarHotkeys,
    CustomHotkeys,
    Arcs,
    ActorHud,
    Status,
    Console,
    Interface,
    Graphics,
    Sound,
    Mixer,
    Help,
}

impl SettingsPage {
    pub const ALL: [Self; 13] = [
        Self::Gameplay,
        Self::Hotkeys,
        Self::BarHotkeys,
        Self::CustomHotkeys,
        Self::Arcs,
        Self::ActorHud,
        Self::Status,
        Self::Console,
        Self::Interface,
        Self::Graphics,
        Self::Sound,
        Self::Mixer,
        Self::Help,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Gameplay => "Gameplay",
            Self::Hotkeys => "Controls / General Hotkeys",
            Self::BarHotkeys => "Controls / Action Bar Hotkeys",
            Self::Console => "Interface / Console",
            Self::Status => "Interface / Status Panel",
            Self::ActorHud => "Interface / HUD",
            Self::Arcs => "Interface / HUD Arcs",
            Self::CustomHotkeys => "Controls / Custom Hotkey Profiles",
            Self::Interface => "Interface",
            Self::Graphics => "Graphics",
            Self::Sound => "Sound",
            Self::Mixer => "Sound / Mixer",
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
    pending_chat_mode: Option<bool>,
    chat_context: Option<bool>,
    window_action: Option<&'static str>,
    pub(super) help_report_pending: bool,
    base_path: Option<PathBuf>,
    character: Option<[u8; 16]>,
    skills_edit: u64,
    pub(crate) item_hotkeys_edit: u64,
    minimap_edit: u64,
    spellbook_edit: u64,
    archive_edit: u64,
    battle_edit: u64,
    store: Option<PreferenceStore>,
    input: PreferenceInput,
    control_input: super::controls::ControlInput,
    control_epoch: std::time::Instant,
    pub preferences: ReferencePreferences,
    pub feedback: String,
    pub open: bool,
    pub shutdown: bool,
    pub page: SettingsPage,
    last_key: std::collections::BTreeMap<oteryn_input_actions::KeyCode, std::time::Instant>,
}

impl SettingsState {
    pub fn select_page(&mut self, index: usize) -> bool {
        let Some(page) = SettingsPage::ALL.get(index) else {
            return false;
        };
        self.page = *page;
        true
    }

    pub fn sync_local_preferences(&self, link: &crate::play::PlayLink) {
        if self.preferences.validate().is_err() {
            return;
        }
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox
            .skill_progress
            .order
            .clone_from(&self.preferences.skill_order);
        mailbox
            .condition_status
            .order
            .clone_from(&self.preferences.condition_order);
        if mailbox.local_preferences != self.preferences {
            mailbox.local_preferences.clone_from(&self.preferences);
        }
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
        store.replace_with_backup(preferences.clone(), &self.preferences)?;
        self.preferences = preferences;
        self.battle_edit = crate::play::feature_0017_attack::view::next_edit();
        self.skills_edit = self.battle_edit;
        self.item_hotkeys_edit = self.battle_edit;
        self.minimap_edit = self.battle_edit;
        self.spellbook_edit = self.battle_edit;
        self.archive_edit = self.battle_edit;
        Ok(())
    }

    pub fn new(path: Option<PathBuf>) -> Self {
        let base_path = path.clone();
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
            pending_chat_mode: None,
            chat_context: None,
            window_action: None,
            help_report_pending: false,
            control_input: super::controls::ControlInput::default(),
            control_epoch: std::time::Instant::now(),
            base_path,
            character: None,
            skills_edit: 0,
            item_hotkeys_edit: 0,
            minimap_edit: 0,
            spellbook_edit: 0,
            archive_edit: 0,
            battle_edit: 0,
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

    pub fn control_events(
        &mut self,
        events: &[NormalizedInputEvent],
        chat_open: bool,
    ) -> Vec<NormalizedInputEvent> {
        self.control_events_at(events, chat_open, self.control_epoch.elapsed())
    }

    pub fn control_events_at(
        &mut self,
        events: &[NormalizedInputEvent],
        chat_open: bool,
        now: std::time::Duration,
    ) -> Vec<NormalizedInputEvent> {
        self.control_input
            .route(&self.preferences.controls, events, chat_open, now)
    }

    pub fn input(&mut self, events: &[NormalizedInputEvent]) -> (Vec<NormalizedInputEvent>, bool) {
        self.input_in_context(events, self.preferences.hotkey_contexts.chat_on)
    }

    pub fn input_from_play(
        &mut self,
        events: &[NormalizedInputEvent],
        link: &crate::play::PlayLink,
    ) -> (Vec<NormalizedInputEvent>, bool) {
        self.input_in_context(events, link.preference_chat_open())
    }

    pub fn input_in_context(
        &mut self,
        events: &[NormalizedInputEvent],
        chat_on: bool,
    ) -> (Vec<NormalizedInputEvent>, bool) {
        let mut routed = self
            .input
            .route_chat(&self.preferences, events, chat_on)
            .unwrap_or_default();
        use oteryn_input_actions::{ButtonState, MouseButton};
        let now = std::time::Instant::now();
        routed.retain_mut(|event| {
            match event {
                NormalizedInputEvent::MouseButton { button, .. }
                    if self.preferences.swap_mouse_buttons =>
                {
                    *button = match *button {
                        MouseButton::PRIMARY => MouseButton::SECONDARY,
                        MouseButton::SECONDARY => MouseButton::PRIMARY,
                        other => other,
                    };
                }
                NormalizedInputEvent::Key {
                    code,
                    state: ButtonState::Pressed,
                    repeat,
                    ..
                } => {
                    if *repeat
                        && self.last_key.get(code).is_some_and(|last| {
                            now.duration_since(*last).as_millis()
                                < u128::from(self.preferences.keyboard_delay_ms)
                        })
                    {
                        return false;
                    }
                    self.last_key.insert(*code, now);
                }
                NormalizedInputEvent::Key {
                    code,
                    state: ButtonState::Released,
                    ..
                } => {
                    self.last_key.remove(code);
                }
                NormalizedInputEvent::FocusChanged { focused: false }
                | NormalizedInputEvent::DeviceLost => self.last_key.clear(),
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
        if self.preferences.archive_bindings != preferences.archive_bindings {
            self.archive_edit = crate::play::feature_0017_attack::view::next_edit();
        }
        if self.preferences.spellbook_filter != preferences.spellbook_filter {
            self.spellbook_edit = crate::play::feature_0017_attack::view::next_edit();
        }
        if self.preferences.minimap != preferences.minimap {
            self.minimap_edit = crate::play::feature_0017_attack::view::next_edit();
        }
        if self.preferences.skills_layout != preferences.skills_layout {
            self.skills_edit = crate::play::feature_0017_attack::view::next_edit();
        }
        if self.preferences.battle_lists != preferences.battle_lists {
            self.battle_edit = crate::play::feature_0017_attack::view::next_edit();
        }
        if !preferences.chat_context_hotkeys {
            self.pending_chat_mode = None;
            self.chat_context = None;
        }
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
        if view.local.minimap_edit > self.minimap_edit {
            let edit = view.local.minimap_edit;
            let mut preferences = self.preferences.clone();
            preferences.minimap = view.preferences().minimap.clone();
            self.apply(preferences);
            self.minimap_edit = edit;
        }
        view.local.minimap_edit = self.minimap_edit;
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

impl crate::play::PlayLink {
    pub fn preference_chat_open(&self) -> bool {
        crate::play::mailbox::lock(&self.mailbox).chat.open
    }
}

#[path = "panel_sync.rs"]
mod panel_sync;

#[path = "chat_context.rs"]
mod chat_context;

#[path = "character_scope.rs"]
mod character_scope;
