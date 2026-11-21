//! The main-thread owner of the existing runtime, admitted play and preference store.
mod device_sign_out;
mod local_saves;
mod options_draft;

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
    baseline: ReferencePreferences,
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
    retired_minimaps: Vec<crate::play::MinimapSaves>,
    pub settings: SettingsState,
    preferences_path: Option<PathBuf>,
    draft: Option<Draft>,
    device_sign_out: Option<tokio::sync::watch::Receiver<Option<bool>>>,
    scope: Scope,
    pub phase: Phase,
    pub status: String,
    bulletin: crate::bulletin::Browser,
    pub(super) login: super::login::LoginState,
    pub(super) selection: super::selection::SelectionBridge,
    selector: crate::character_selection::CharacterSelector,
    pub(super) services: super::account_services::Services,
    pub(super) dialogs: super::gameplay_dialogs::Dialogs,
    /// chsw: the target Channel's row, shown as the arrival line after the fresh admission.
    arrival: Option<oteryn_client_ui::channel_switch::ChannelRow>,
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
    /// The dock layout is saved per character beside the client preferences file.
    pub(in crate::bevy_shell) fn dock_layout_path(&self) -> Option<PathBuf> {
        let character = self.scope.character?;
        let hex: String = character.iter().map(|byte| format!("{byte:02x}")).collect();
        self.preferences_path
            .as_ref()
            .map(|path| path.with_extension(format!("dock.{hex}.json")))
    }
    #[cfg(test)]
    pub(in crate::bevy_shell) fn enter_character(&mut self, character: Option<[u8; 16]>) {
        self.change_scope(character);
    }
    pub fn new(path: Option<PathBuf>) -> Result<Self, Box<dyn std::error::Error>> {
        let client = ClientBootstrap::new()?.with_shared_frontend();
        let (selection, selector) = super::selection::SelectionBridge::new();
        Ok(Self {
            entry: Some(Reentry::new(client)),
            controls: super::controls::Controls::new()?,
            play: None,
            retired_minimaps: Vec::new(),
            settings: SettingsState::new(path.clone()),
            preferences_path: path,
            draft: None,
            device_sign_out: None,
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
            arrival: None,
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
        self.retire_play();
        self.change_scope(Some(character));
        view.use_shared_frontend();
        view.enable_background_minimap_save();
        link.use_shared_frontend();
        if let Some(row) = self.arrival.take() {
            view.announce_channel_arrival(&row);
        }
        self.play = Some(ActivePlay { view, link });
        self.phase = Phase::Playing;
        self.status = "Connected to Oteryn.".into();
        true
    }
    fn leave(&mut self, message: &str) {
        self.arrival = None;
        self.retire_play();
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
            || self.login.survey.visible()
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
        config.session_options = Some(self.settings.preferences.miscellaneous);
        config.select_character = self.selector.clone();
        self.services.configure(&mut config);
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
        if let Some(entry) = &mut self.entry
            && entry.cancel_entry()
        {
            entry.forget_native_session();
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
        self.poll_device_sign_out();
        self.poll_retired_minimaps();
        let completion = self.settings.poll_background_save();
        if completion.replaced {
            self.rebase_transferred_preferences();
        }
        if let Some(error) = completion.error {
            self.status.clone_from(&error);
            if self.login.visible {
                self.login.form.feedback(error);
            }
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
        self.login
            .form
            .refresh_gate_message(std::time::Instant::now());
        if let Some(result) = self.entry.as_mut().and_then(Reentry::poll) {
            let bulletin = self.entry.as_mut().and_then(Reentry::take_bulletin);
            self.finish_admission(result, bulletin);
            if self.phase == Phase::Closed {
                return;
            }
        }
        let mut end = None;
        let mut switch = None;
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
            if let Some(result) = play.view.poll_background_minimap_save() {
                self.settings.feedback = match result {
                    Ok(()) => tr!("Minimap transfer saved").into(),
                    Err(error) => lfmt!("Minimap transfer failed: {error}", error = error),
                };
            }
            end = play.view.tick(&play.link).err();
            if end.is_some() {
                switch = play.link.channel_switch_target();
            }
        }
        if let Some(target) = switch.filter(|_| end.is_some()) {
            // chsw: the server released the session for a switch; enter the target Channel.
            self.leave(oteryn_client_ui::channel_switch::wait_text());
            self.arrival = target.row;
            if self.entry.as_mut().is_some_and(|entry| {
                entry.set_session_options(self.settings.preferences.miscellaneous);
                entry.request_switch(target.character_id, target.channel_id)
            }) {
                self.phase = Phase::Entering;
                self.status = oteryn_client_ui::channel_switch::wait_text().into();
            } else {
                self.arrival = None;
            }
            return;
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
        if matches!(
            &result,
            Err(crate::GameplayEntryError::ClientUpdateRequired {
                updater_started: true,
                ..
            })
        ) {
            self.close();
            return;
        }
        self.login
            .form
            .finish(result.as_ref().map(|_| ()).map_err(Clone::clone));
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
            Err(error) => {
                self.leave(&error_text(error));
                if !self.login.visible {
                    self.login.show();
                }
            }
        }
    }
    fn finish_play(&mut self, end: PlayEnd) {
        let message = match end {
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
        self.flush_local_saves();
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

#[cfg(test)]
#[test]
fn cancel_login_schedules_remembered_device_revocation() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let config = crate::NativeLoginConfig::from_env(|name| {
        Some(match name {
            "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => "https://localhost:1".into(),
            "OTERYN_OAUTH_CLIENT_ID" => "019a5f4a-1000-7000-8000-000000000001".into(),
            "OTERYN_DEV_ROOT" => "/tmp/reentry-unused-root".into(),
            _ => String::new(),
        })
    })?
    .ok_or("native login config")?;
    let device = Arc::clone(&config.device_sessions);
    let held_session = runtime.block_on(device.hold_session_for_reentry_test());
    let mut owner = HostOwner::new(None)?;
    let entry = owner.entry.as_mut().ok_or("reentry")?;
    assert!(entry.configure(config));
    entry.arm_cancel_fixture();
    owner.phase = Phase::Entering;
    let owners = Arc::strong_count(&device);
    owner.cancel_login();
    assert_eq!(Arc::strong_count(&device), owners + 1);

    drop(held_session);
    owner.close();
    Ok(())
}

#[cfg(test)]
#[test]
fn logout_reaches_the_picker_even_when_quick_login_has_a_valid_selection()
-> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        use crate::character_selection::SelectionAction;
        use crate::native_entry::test_support::{Stack, channel_node::CHANNEL_B};
        use std::sync::Arc;

        let run = || -> Result<(), Box<dyn std::error::Error>> {
            let stack = Stack::start_two_channels()?;
            let folder = tempfile::tempdir()?;
            let mut config = stack.config();
            let preferences = folder.path().join("preferences.json");
            crate::play::MiscellaneousPreferences {
                stay_logged_in: true,
                quick_login: true,
                ..Default::default()
            }.save(&preferences.with_extension("session-options.json"))?;
            config.preference_path = Some(preferences);
            config.device_sessions = Arc::new(crate::device_login::DeviceLogin::quick_login_fixture(
                folder.path().to_path_buf(),
            ));
            std::fs::write(folder.path().join("last-character.json"), serde_json::to_vec(
                &serde_json::json!({ "character": &config.character_id, "channel": CHANNEL_B }),
            )?)?;
            let (calls, observed) = std::sync::mpsc::channel();
            let character = config.character_id.clone();
            config.select_character = Arc::new(move |rows, _, _| {
                let available = rows.iter().any(|row| {
                    row.available && row.character.character_id == character
                        && row.channels.iter().any(|channel| channel.id == CHANNEL_B)
                });
                let _ = calls.send(available);
                SelectionAction::Cancel
            });
            let mut owner = HostOwner::new(None)?;
            assert!(owner.entry.as_mut().ok_or("entry")?.configure(config));
            owner.finish_play(PlayEnd::LoggedOut);
            assert_eq!(owner.phase, Phase::Entering);
            assert!(observed.recv_timeout(std::time::Duration::from_secs(10))?);
            owner.close();
            stack.finish();
            Ok(())
        };
        run().map_err(|error| error.to_string())
    })?.join().map_err(|_| "logout picker scenario panicked")?.map_err(Into::into)
}

#[cfg(test)]
#[path = "owner/options_revoke_tests.rs"]
mod options_revoke_tests;

#[cfg(test)]
#[path = "owner/admission_tests.rs"]
mod admission_tests;

#[cfg(test)]
mod reentry_tests;
