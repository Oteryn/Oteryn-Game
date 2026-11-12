//! Platform-independent settings window state, input and play-view synchronization.
use super::{PreferenceError, PreferenceInput, PreferenceStore, ReferencePreferences};
use crate::play::PlayView;
use oteryn_input_actions::NormalizedInputEvent;
use std::{
    io,
    path::{Path, PathBuf},
};

#[path = "pages.rs"]
mod pages;
pub use pages::{SettingsPage, WINDOW_ACTIONS};

pub struct SettingsState {
    pub(super) native_sign_out: bool,
    pub(super) web_link: Option<crate::web_links::WebLink>,
    pub(super) minimap_transfer: Option<(bool, PathBuf)>,
    pub(super) help_report: Option<super::help::Report>,
    pub(super) screenshot_runtime: super::screenshot::ScreenshotRuntime,
    pub(super) screenshot_requested: bool,
    pub(super) window_actions: std::collections::VecDeque<&'static str>,
    pending_chat_mode: Option<bool>,
    chat_context: Option<bool>,
    pub custom_choices: Vec<(String, super::CustomAction)>,
    pub(super) custom_choices_refreshed: Option<std::time::Instant>,
    pub(super) help_report_pending: bool,
    base_path: Option<PathBuf>,
    character: Option<[u8; 16]>,
    skills_edit: u64,
    pub(crate) item_hotkeys_edit: u64,
    pub(crate) bar_actions_edit: u64,
    minimap_edit: u64,
    hud_edit: u64,
    spellbook_edit: u64,
    stash_sort_edit: u64,
    archive_edit: u64,
    hud_toggle: Option<crate::play::HudPanel>,
    battle_edit: u64,
    magical_archive_requested: bool,
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

#[cfg(test)]
#[path = "tests/e2e_n05.rs"]
mod e2e_n05;
