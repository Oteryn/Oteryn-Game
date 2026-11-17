//! The main-thread owner of the existing runtime, admitted play and preference store.
use crate::{
    ClientBootstrap,
    native_login::error_text,
    play::{PlayEnd, PlayLink, PlayView, ReferencePreferences, SettingsState},
    reentry::Reentry,
};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Offline,
    Entering,
    Playing,
    Closed,
}

/// Public UI values only. No login configuration or session material belongs here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct GraphicsDraft {
    pub fullscreen: bool,
    pub vsync: bool,
    pub integer_scaling: bool,
}
impl From<&ReferencePreferences> for GraphicsDraft {
    fn from(p: &ReferencePreferences) -> Self {
        Self {
            fullscreen: p.graphics.fullscreen,
            vsync: p.graphics.vsync,
            integer_scaling: p.graphics.integer_scaling,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Scope {
    character: Option<[u8; 16]>,
    generation: u64,
}
struct Draft {
    scope: Scope,
    values: ReferencePreferences,
}
pub(super) struct ActivePlay {
    pub view: PlayView,
    pub link: PlayLink,
}

/// Inserted with `insert_non_send_resource`: Reentry is intentionally not Sync.
/// Dropping this owner first drops active play, then consumes its one runtime exactly once.
pub(super) struct HostOwner {
    entry: Option<Reentry>,
    pub(super) controls: super::controls::Controls,
    pub play: Option<ActivePlay>,
    pub settings: SettingsState,
    preferences_path: Option<PathBuf>,
    draft: Option<Draft>,
    scope: Scope,
    pub phase: Phase,
    pub status: String,
    bulletin: crate::bulletin::Browser,
    pub(super) login: super::login::LoginState,
    pub(super) selection: super::selection::SelectionBridge,
    selector: crate::character_selection::CharacterSelector,
    pub(super) services: super::account_services::Services,
    pub(super) dialogs: super::gameplay_dialogs::Dialogs,
    /// Counts ShowOptionsHotkeys requests; the options window turns to its hotkey page on a new one.
    pub(super) hotkey_options: u64,
}
impl HostOwner {
    /// Window layouts are saved beside the client preferences file.
    pub(in crate::bevy_shell) fn window_layouts_path(&self) -> Option<PathBuf> {
        self.preferences_path
            .as_ref()
            .map(|path| path.with_extension("windows.json"))
    }
    pub fn new(path: Option<PathBuf>) -> Result<Self, Box<dyn std::error::Error>> {
        let client = ClientBootstrap::new()?.with_shared_frontend();
        let (selection, selector) = super::selection::SelectionBridge::new();
        Ok(Self {
            entry: Some(Reentry::new(client)),
            controls: super::controls::Controls::new()?,
            play: None,
            settings: SettingsState::new(path.clone()),
            preferences_path: path,
            draft: None,
            scope: Scope {
                character: None,
                generation: 0,
            },
            phase: Phase::Offline,
            status: offline_message().into(),
            bulletin: Default::default(),
            login: Default::default(),
            selection,
            selector,
            services: super::account_services::Services::new(),
            dialogs: Default::default(),
            hotkey_options: 0,
        })
    }
    pub fn accepted(&self) -> GraphicsDraft {
        (&self.settings.preferences).into()
    }
    pub fn draft(&self) -> Option<GraphicsDraft> {
        self.draft.as_ref().map(|d| (&d.values).into())
    }
    pub fn generation(&self) -> u64 {
        self.scope.generation
    }
    pub fn open_graphics(&mut self) {
        if self.phase == Phase::Closed {
            return;
        }
        if self.modal_active() {
            return;
        }
        self.reset_input();
        self.settings.open = true;
        self.draft = Some(Draft {
            scope: self.scope,
            values: self.settings.preferences.clone(),
        });
    }
    /// Global's ShowOptionsHotkeys: the options window, opened on its hotkey page.
    pub fn show_hotkey_options(&mut self) {
        self.open_graphics();
        if self.draft.is_some() {
            self.hotkey_options += 1;
        }
    }
    pub fn edit(&mut self, edit: impl FnOnce(&mut GraphicsDraft)) {
        let Some(mut graphics) = self.preferences_draft().map(GraphicsDraft::from) else {
            return;
        };
        edit(&mut graphics);
        self.edit_preferences(|values| {
            values.graphics.fullscreen = graphics.fullscreen;
            values.graphics.vsync = graphics.vsync;
            values.graphics.integer_scaling = graphics.integer_scaling;
        });
    }
    pub fn preferences_draft(&self) -> Option<&ReferencePreferences> {
        self.draft.as_ref().map(|draft| &draft.values)
    }
    pub fn edit_preferences(&mut self, edit: impl FnOnce(&mut ReferencePreferences)) {
        if let Some(draft) = &mut self.draft {
            edit(&mut draft.values);
        }
    }
    pub fn cancel(&mut self) {
        self.reset_input();
        self.draft = None;
        self.settings.open = false;
    }
    pub fn apply(&mut self, close: bool) {
        let Some(draft) = self.draft.take() else {
            return;
        };
        if draft.scope != self.scope || self.phase == Phase::Closed {
            self.settings.open = false;
            return;
        }
        // The retained owner alone validates, saves and installs preferences. Invalid edits
        // remain in the transient draft; a disk failure still accepts the live values.
        let before = self.settings.preferences.miscellaneous;
        self.settings.apply(draft.values.clone());
        if self.settings.preferences != draft.values {
            self.draft = Some(draft);
            return;
        }
        let after = self.settings.preferences.miscellaneous;
        if before.stay_logged_in && !after.stay_logged_in
            || before.remember_device && !after.remember_device
        {
            self.forget_device();
        }
        if close {
            self.settings.open = false;
        } else {
            self.open_graphics();
        }
    }
    fn change_scope(&mut self, character: Option<[u8; 16]>) {
        self.cancel();
        self.selection.cancel();
        self.services.cancel();
        self.dialogs.clear();
        self.scope = Scope {
            character,
            generation: self.scope.generation.saturating_add(1),
        };
        self.settings.select_character(character);
        if character.is_none() {
            self.selection.clear_known_worlds();
        }
    }
    fn install_play(&mut self, mut view: PlayView, link: PlayLink) -> bool {
        let Some(character) = view.preference_character() else {
            // Never substitute a login hint, display name or invented identity.
            drop((view, link));
            self.leave("Admission did not provide a character identity.");
            return false;
        };
        drop(self.play.take());
        self.change_scope(Some(character));
        view.use_shared_frontend();
        link.use_shared_frontend();
        self.play = Some(ActivePlay { view, link });
        self.phase = Phase::Playing;
        self.status = "Connected to Oteryn.".into();
        true
    }
    fn leave(&mut self, message: &str) {
        drop(self.play.take());
        self.change_scope(None);
        self.phase = Phase::Offline;
        self.status = format!("Offline — {message}");
        if let Some(entry) = &mut self.entry {
            entry.leave_play();
        }
        self.login.form.offline();
        self.login.form.feedback(self.status.clone());
    }
    pub fn configure_login(
        &mut self,
        path: Option<PathBuf>,
        lookup: impl Fn(&str) -> Option<String>,
    ) {
        self.login.load(path, lookup);
    }
    pub fn modal_active(&self) -> bool {
        self.draft.is_some()
            || self.login.visible
            || self.login.bulletin.visible
            || self.selection.pending().is_some()
            || self.services.pending().is_some()
            || self.dialogs.active()
            || self
                .play
                .as_ref()
                .is_some_and(|play| play.link.server_modal_active())
    }
    pub fn request_login(&mut self) {
        if self.phase != Phase::Offline {
            return;
        }
        self.cancel();
        self.login.show();
    }
    pub fn submit_login(&mut self) {
        self.submit_login_with_assets(|| std::env::var_os(crate::map::ASSET_DIR_ENV));
    }
    pub(super) fn submit_login_with_assets(
        &mut self,
        lookup: impl FnOnce() -> Option<std::ffi::OsString>,
    ) {
        if self.phase != Phase::Offline || !self.login.visible {
            return;
        }
        let mut config = match self.login.submit() {
            Ok(config) => config,
            Err(_) => return,
        };
        if let Err(message) = super::live_assets::preflight(lookup()) {
            self.login
                .form
                .finish(Err(crate::GameplayEntryError::InvalidConfiguration));
            self.login.form.offline();
            self.login.form.feedback(message);
            return;
        }
        config.preference_path.clone_from(&self.preferences_path);
        config.select_character = self.selector.clone();
        self.services.configure(&mut config);
        // Pre-login purchases are a separate presentation batch.
        config.coin_shop = std::sync::Arc::new(|_, _| crate::coin_shop::Action::Back);
        self.selection.cancel();
        self.services.cancel();
        if self
            .entry
            .as_mut()
            .is_some_and(|entry| entry.configure(config) && entry.request())
        {
            self.cancel();
            self.phase = Phase::Entering;
            self.status = "Signing in — waiting for native admission.".into();
        } else {
            self.login
                .form
                .finish(Err(crate::GameplayEntryError::Rejected(
                    oteryn_platform_client::native_login::PublicClass::SessionUnavailable,
                )));
        }
    }
    pub fn cancel_login(&mut self) {
        self.selection.cancel();
        self.services.cancel();
        if let Some(entry) = &mut self.entry {
            entry.cancel_entry();
        }
    }
    /// The login window's explicit sign-out, as the Win32 options offered it: turns
    /// remember-device off, cancels any entry and forgets the remembered device on the
    /// Platform the connection settings name. A normal game logout keeps the device.
    pub fn sign_out_device(&mut self) {
        if self.play.is_some() {
            return;
        }
        let mut preferences = self.settings.preferences.clone();
        preferences.miscellaneous.remember_device = false;
        self.settings.apply(preferences);
        self.cancel_login();
        // Before the first sign-in of this process the entry has no Platform yet.
        let configured = self.login.form.settings.config().is_ok_and(|config| {
            self.entry
                .as_mut()
                .is_some_and(|entry| entry.configure(config))
        });
        if let Some(entry) = &mut self.entry {
            entry.native_sign_out();
        }
        self.login.form.feedback(if configured {
            "Signed out; this device is no longer remembered.".into()
        } else {
            "Remember-device is off. Correct the connection settings to also forget this device \
             on the Platform."
                .into()
        });
    }
    /// Turning stay-logged-in or remember-device off in the options forgets this device on the
    /// Platform and, as the Win32 page did, logs the character in play out.
    fn forget_device(&mut self) {
        if self.phase == Phase::Offline
            && let Ok(config) = self.login.form.settings.config()
            && let Some(entry) = &mut self.entry
        {
            entry.configure(config);
        }
        if let Some(entry) = &mut self.entry {
            entry.native_sign_out();
        }
        if let Some(play) = &mut self.play {
            play.view.logout();
        }
    }
    /// Opens a Platform page (support tickets, cookie policy) from the login window with the
    /// Platform the connection settings name.
    pub fn open_web_link(&mut self, link: crate::web_links::WebLink) {
        if self.phase == Phase::Offline
            && let Ok(config) = self.login.form.settings.config()
            && let Some(entry) = &mut self.entry
        {
            entry.configure(config);
        }
        if !self
            .entry
            .as_ref()
            .is_some_and(|entry| entry.open_web_link(link))
        {
            self.login.form.feedback(
                "Correct the connection settings to open Platform pages in the browser.".into(),
            );
        }
    }
    pub fn poll(&mut self, viewport: [u32; 2]) {
        if self.phase == Phase::Closed {
            return;
        }
        if self.entry.as_ref().is_some_and(Reentry::is_entering) {
            self.phase = Phase::Entering;
        }
        if let Some(message) = self
            .entry
            .as_mut()
            .and_then(|e| e.poll_wait(std::time::Instant::now()))
        {
            self.status.clone_from(&message);
            self.login.form.feedback(message);
        }
        if let Some(result) = self.entry.as_mut().and_then(Reentry::poll) {
            let bulletin = self.entry.as_mut().and_then(Reentry::take_bulletin);
            self.finish_admission(result, bulletin);
        }
        let mut end = None;
        if let Some(play) = &mut self.play {
            // The same owner projects accepted graphics, zoom, bindings and chat context.
            self.settings.sync_skills_layout(&play.link);
            self.settings.sync_item_hotkeys(&play.link);
            self.settings.sync_bar_actions(&play.link);
            self.settings.sync_spellbook_filter(&play.link);
            self.settings.sync_stash_sort(&play.link);
            self.settings.sync_archive_bindings(&play.link);
            self.settings.refresh_custom_choices(&play.link);
            self.settings.sync_presentation(&play.link);
            self.settings.sync_battle_lists(&play.link);
            self.settings.sync_chat_context(&play.link);
            if let Err(error) = self.settings.restore(&mut play.view, viewport) {
                self.settings.feedback = error.to_string();
            }
            self.settings.sync_local_preferences(&play.link);
            end = play.view.tick(&play.link).err();
        }
        if let Some(end) = end {
            self.finish_play(end);
        }
    }
    fn finish_admission(
        &mut self,
        result: Result<(PlayView, PlayLink), crate::GameplayEntryError>,
        bulletin: Option<String>,
    ) {
        self.login
            .form
            .finish(result.as_ref().map(|_| ()).map_err(|error| *error));
        if result.is_ok() {
            self.login.visible = false;
        }
        if let Some(document) = &bulletin
            && let Err(error) = self.login.bulletin.receive(document)
        {
            self.login.form.feedback(error);
        }
        match result {
            Ok((view, link)) => {
                if self.install_play(view, link)
                    && let Some(document) = bulletin
                {
                    match self.bulletin.replace(&document) {
                        Ok(()) => {
                            self.status.push_str("\n\n");
                            self.status.push_str(&self.bulletin.news());
                        }
                        Err(error) => self
                            .status
                            .push_str(&format!("\nBulletin unavailable: {error}")),
                    }
                }
            }
            Err(error) => self.leave(error_text(error)),
        }
    }
    fn finish_play(&mut self, end: PlayEnd) {
        let message = match end {
            PlayEnd::LoggedOut => "Logged out. Choose a character after signing in again.",
            PlayEnd::Ended(class) => error_text(crate::GameplayEntryError::Rejected(class)),
        };
        self.leave(message);
        if matches!(end, PlayEnd::LoggedOut)
            && self
                .entry
                .as_mut()
                .is_some_and(|entry| entry.is_entering() || entry.request())
        {
            self.phase = Phase::Entering;
        }
    }
    fn reset_input(&mut self) {
        self.route_input(
            &[oteryn_input_actions::NormalizedInputEvent::DeviceLost],
            true,
        );
    }
    pub fn close(&mut self) {
        if self.phase == Phase::Closed {
            return;
        }
        self.cancel();
        self.selection.cancel();
        self.services.cancel();
        self.selection.clear_known_worlds();
        self.dialogs.clear();
        self.login.visible = false;
        self.login.bulletin.visible = false;
        drop(self.play.take());
        if let Some(mut entry) = self.entry.take() {
            entry.cancel_entry();
            entry.shutdown();
        }
        self.phase = Phase::Closed;
    }
}
impl Drop for HostOwner {
    fn drop(&mut self) {
        self.close();
    }
}
fn offline_message() -> &'static str {
    "Offline — no admitted character or world. Use Sign in for connection settings and character selection."
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "live_assets/tests.rs"]
mod live_asset_tests;

#[cfg(test)]
mod selection_return_tests {
    use super::*;
    #[test]
    fn confirmed_logout_restarts_admission_but_disconnect_does_not()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut owner = HostOwner::new(None)?;
        owner.finish_play(PlayEnd::Ended(
            oteryn_platform_client::native_login::PublicClass::SessionUnavailable,
        ));
        assert_eq!(owner.phase, Phase::Offline);
        owner.finish_play(PlayEnd::LoggedOut);
        assert_eq!(owner.phase, Phase::Entering);
        assert!(owner.entry.as_ref().is_some_and(Reentry::is_entering));
        owner.close();
        Ok(())
    }
}
