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
    Screenshots,
    Help,
    HelpGuide,
    MinimapTransfer,
    ConsoleFilters,
    ActionBarLayout,
    Language,
}

impl SettingsPage {
    pub const ALL: [Self; 18] = [
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
        Self::Screenshots,
        Self::Help,
        Self::HelpGuide,
        Self::MinimapTransfer,
        Self::ConsoleFilters,
        Self::ActionBarLayout,
        Self::Language,
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
            Self::Screenshots => "Screenshots",
            Self::Help => "Help / settings transfer",
            Self::HelpGuide => "Help / guide and support",
            Self::MinimapTransfer => "Help / minimap transfer",
            Self::ConsoleFilters => "Console",
            Self::ActionBarLayout => "Interface / action bar layout",
            Self::Language => "Interface / language",
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
    pub(super) minimap_transfer: Option<(bool, PathBuf)>,
    pub(super) help_report: Option<super::help::Report>,
    pub(super) screenshot_runtime: super::screenshot::ScreenshotRuntime,
    pub(super) screenshot_requested: bool,
    pub(super) window_actions: std::collections::VecDeque<&'static str>,
    pending_chat_mode: Option<bool>,
    chat_context: Option<bool>,
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

#[path = "settings_actions.rs"]
mod settings_actions;

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
