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
    /// Deferred sidebar toggles belong only to the session that received the hotkey.
    #[cfg(bevy_shell)]
    pub(super) skills_toggles:
        Vec<std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>>,
    pending_chat_mode: Option<bool>,
    /// The play session (by its mailbox) whose chat mode was restored, and the mode last seen.
    chat_mode_seen: Option<(usize, oteryn_client_ui::chat::ComposerMode)>,
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
    /// A page the options window turns to when it next runs (Ctrl+K: the hotkey page).
    page_request: Option<SettingsPage>,
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

#[path = "session_options.rs"]
mod session_options;
#[path = "settings_actions.rs"]
mod settings_actions;

impl crate::play::PlayLink {
    /// A visible console owns the text context only while its composer is enabled.
    /// Docked chat can remain visible with chat Off; closed native panes remain inactive.
    pub fn preference_chat_open(&self) -> bool {
        let mailbox = crate::play::mailbox::lock(&self.mailbox);
        mailbox.chat.open && mailbox.chat.console.can_send()
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
