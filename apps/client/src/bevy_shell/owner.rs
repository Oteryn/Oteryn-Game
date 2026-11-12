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
    values: GraphicsDraft,
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
    pub(super) login: super::login::LoginState,
    pub(super) selection: super::selection::SelectionBridge,
    selector: crate::character_selection::CharacterSelector,
    pub(super) dialogs: super::gameplay_dialogs::Dialogs,
}
impl HostOwner {
    pub fn new(path: Option<PathBuf>) -> Result<Self, Box<dyn std::error::Error>> {
        let client = ClientBootstrap::new()?;
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
            login: Default::default(),
            selection,
            selector,
            dialogs: Default::default(),
        })
    }
    pub fn accepted(&self) -> GraphicsDraft {
        (&self.settings.preferences).into()
    }
    pub fn draft(&self) -> Option<GraphicsDraft> {
        self.draft.as_ref().map(|d| d.values)
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
            values: self.accepted(),
        });
    }
    pub fn edit(&mut self, edit: impl FnOnce(&mut GraphicsDraft)) {
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
        let mut preferences = self.settings.preferences.clone();
        preferences.graphics.fullscreen = draft.values.fullscreen;
        preferences.graphics.vsync = draft.values.vsync;
        preferences.graphics.integer_scaling = draft.values.integer_scaling;
        // apply validates internally; even a failed save accepts live settings with feedback.
        self.settings.apply(preferences);
        if close {
            self.settings.open = false;
        } else {
            self.open_graphics();
        }
    }
    fn change_scope(&mut self, character: Option<[u8; 16]>) {
        self.cancel();
        self.selection.cancel();
        self.dialogs.clear();
        self.scope = Scope {
            character,
            generation: self.scope.generation.saturating_add(1),
        };
        self.settings.select_character(character);
    }
    fn install_play(&mut self, view: PlayView, link: PlayLink) -> bool {
        let Some(character) = view.preference_character() else {
            // Never substitute a login hint, display name or invented identity.
            drop((view, link));
            self.leave("Admission did not provide a character identity.");
            return false;
        };
        drop(self.play.take());
        self.change_scope(Some(character));
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
            || self.selection.pending().is_some()
            || self.dialogs.active()
    }
    pub fn request_login(&mut self) {
        if self.phase != Phase::Offline {
            return;
        }
        self.cancel();
        self.login.show();
    }
    pub fn submit_login(&mut self) {
        if self.phase != Phase::Offline || !self.login.visible {
            return;
        }
        let mut config = match self.login.submit() {
            Ok(config) => config,
            Err(_) => return,
        };
        config.preference_path.clone_from(&self.preferences_path);
        config.select_character = self.selector.clone();
        // These account presenters have not migrated; the picker exposes no such actions.
        config.account_services = std::sync::Arc::new(|_, _| crate::account_services::Action::Back);
        config.coin_shop = std::sync::Arc::new(|_, _| crate::coin_shop::Action::Back);
        config.premium = std::sync::Arc::new(|_, _| crate::premium::Action::Back);
        self.selection.cancel();
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
        if let Some(entry) = &mut self.entry {
            entry.cancel_entry();
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
            self.login
                .form
                .finish(result.as_ref().map(|_| ()).map_err(|error| *error));
            if result.is_ok() {
                self.login.visible = false;
            }
            // Bulletin remains owned by Reentry; rich bulletin presentation is a later module.
            let _ = self.entry.as_mut().and_then(Reentry::take_bulletin);
            match result {
                Ok((view, link)) => {
                    self.install_play(view, link);
                }
                Err(error) => self.leave(error_text(error)),
            }
        }
        let mut end = None;
        if let Some(play) = &mut self.play {
            // The same owner projects accepted graphics, zoom, bindings and chat context.
            self.settings.sync_chat_context(&play.link);
            if let Err(error) = self.settings.restore(&mut play.view, viewport) {
                self.settings.feedback = error.to_string();
            }
            self.settings.sync_local_preferences(&play.link);
            end = play.view.tick(&play.link).err();
        }
        if let Some(end) = end {
            let message = match end {
                PlayEnd::LoggedOut => "Logged out. Choose a character after signing in again.",
                PlayEnd::Ended(class) => error_text(crate::GameplayEntryError::Rejected(class)),
            };
            self.leave(message);
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
        self.dialogs.clear();
        self.login.visible = false;
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
