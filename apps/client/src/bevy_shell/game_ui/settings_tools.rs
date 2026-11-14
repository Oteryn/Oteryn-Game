//! Portable presentation of the retained settings owners. All execution is on the owner thread.
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::{ReferencePreferences, SettingsPage, SettingsState};
use bevy::{
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui_widgets::{Activate, TextInput},
};
use oteryn_client_ui::localization::Language;
use oteryn_client_ui_bevy as source;
use std::path::PathBuf;
mod capture;
mod widgets;
use widgets::{label, project};
#[cfg(test)]
mod tests;

#[derive(Clone, PartialEq, Eq)]
struct Snapshot {
    preferences: ReferencePreferences,
    feedback: String,
}
impl Snapshot {
    fn read(settings: &SettingsState) -> Self {
        Self {
            preferences: settings.preferences.clone(),
            feedback: settings.feedback.clone(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Import,
    Export,
    Backup,
    Defaults,
    MinimapImport,
    MinimapExport,
    SaveLanguage,
    SaveScreenshot,
    Capture,
    Report,
    BugReport,
    PlayerReport,
    Close,
}
impl Action {
    fn destructive(self) -> bool {
        matches!(
            self,
            Self::Import | Self::Backup | Self::Defaults | Self::MinimapImport
        )
    }
}
struct Admission {
    generation: u64,
    revision: u64,
}
impl Admission {
    fn new(generation: u64) -> Self {
        Self {
            generation,
            revision: 0,
        }
    }
    fn admit(&mut self, generation: u64, revision: u64, blocked: bool, close: bool) -> bool {
        if generation != self.generation || (!close && (blocked || revision != self.revision)) {
            return false;
        }
        self.revision = self.revision.saturating_add(1);
        true
    }
}
#[derive(Clone, Default, PartialEq, Eq)]
struct Draft {
    path: String,
    folder: String,
    backlog: String,
    language: usize,
    topic: usize,
    legacy_topic: Option<usize>,
    flags: [bool; 6],
}
impl Draft {
    fn read(p: &ReferencePreferences) -> Self {
        let s = p.screenshots.effective();
        Self {
            folder: s.folder,
            backlog: s.backlog_frames.to_string(),
            language: Language::ALL
                .iter()
                .position(|l| *l == p.language())
                .unwrap_or_default(),
            flags: [
                s.game_only,
                s.on_death,
                s.on_level,
                s.on_skill,
                s.on_achievement,
                p.screenshots.backlog_seconds > 0,
            ],
            ..default()
        }
    }
}
#[derive(Resource, Default)]
struct State {
    bound: Option<Admission>,
    snapshot: Option<Snapshot>,
    root: Option<Entity>,
    page: Option<SettingsPage>,
    draft: Draft,
    confirm: Option<(Action, Draft)>,
    entries: Vec<PathBuf>,
    dirty: bool,
}
#[derive(Component)]
struct Field(bool);
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    revision: u64,
    kind: Kind,
}
#[derive(Clone)]
enum Kind {
    Action(Action),
    Confirm,
    Page(SettingsPage),
    Language(usize),
    Topic(usize),
    LegacyTopic(usize),
    Flag(usize),
    Browse,
    Path(PathBuf),
}
const PAGES: [SettingsPage; 5] = [
    SettingsPage::Help,
    SettingsPage::MinimapTransfer,
    SettingsPage::HelpGuide,
    SettingsPage::Language,
    SettingsPage::Screenshots,
];
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
    app.add_systems(
        PostUpdate,
        invalidate_confirmation.after(bevy::text::EditableTextSystems),
    );
    capture::install(app);
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if owner.modal_active() {
        return;
    }
    owner.open_graphics();
    if owner.preferences_draft().is_some() {
        owner.settings.page = SettingsPage::Help;
    }
}
fn visible(owner: &HostOwner) -> bool {
    owner.settings.open
        && owner.preferences_draft().is_some()
        && PAGES.contains(&owner.settings.page)
}
fn close(owner: &mut HostOwner, state: &mut State, generation: u64) -> bool {
    if generation != owner.generation() || !visible(owner) {
        return false;
    }
    let Some(bound) = &mut state.bound else {
        return false;
    };
    if !bound.admit(generation, bound.revision, false, true) {
        return false;
    }
    owner.cancel();
    state.confirm = None;
    state.dirty = true;
    true
}
fn keyboard(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<State>,
) {
    for key in keys.read() {
        if key.key_code == KeyCode::Escape
            && key.state == bevy::input::ButtonState::Pressed
            && !key.repeat
            && let Some(generation) = state.bound.as_ref().map(|b| b.generation)
        {
            close(&mut owner, &mut state, generation);
        }
    }
}
fn draft_fields(state: &State, fields: &Query<(&Field, &EditableText)>) -> Draft {
    let mut draft = state.draft.clone();
    for (field, text) in fields {
        if field.0 {
            draft.folder = text.editor.text().to_string();
        } else if state.page == Some(SettingsPage::Screenshots) {
            draft.backlog = text.editor.text().to_string();
        } else {
            draft.path = text.editor.text().to_string();
        }
    }
    draft
}
fn invalidate_confirmation(mut state: ResMut<State>, fields: Query<(&Field, &EditableText)>) {
    if state.confirm.is_some() {
        let current = draft_fields(&state, &fields);
        if current != state.draft {
            state.draft = current;
            state.confirm = None;
            state.dirty = true;
        }
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(&Field, &EditableText)>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if matches!(control.kind, Kind::Action(Action::Close)) {
        close(&mut owner, &mut state, control.generation);
        return;
    }
    let blocked = owner.settings.screenshot_pending()
        || !guard.allows(event.entity)
        || owner
            .play
            .as_ref()
            .is_some_and(|p| p.link.server_modal_active());
    if state.page != Some(owner.settings.page)
        || !visible(&owner)
        || state.snapshot.as_ref() != Some(&Snapshot::read(&owner.settings))
    {
        return;
    }
    let Some(bound) = &mut state.bound else {
        return;
    };
    if !bound.admit(
        owner.generation(),
        control.revision,
        blocked || control.generation != owner.generation(),
        false,
    ) {
        return;
    }
    let draft = draft_fields(&state, &fields);
    if draft != state.draft {
        state.confirm = None;
    }
    state.draft = draft;
    state.dirty = true;
    match &control.kind {
        Kind::Action(a) if a.destructive() => state.confirm = Some((*a, state.draft.clone())),
        Kind::Action(a) => execute(&mut owner, *a, &state.draft),
        Kind::Confirm => {
            if let Some((a, frozen)) = state.confirm.take()
                && frozen == state.draft
            {
                execute(&mut owner, a, &frozen);
            }
        }
        Kind::Page(page) => {
            owner.settings.page = *page;
            state.confirm = None;
            state.entries.clear();
        }
        Kind::Language(index) => {
            state.draft.language = *index;
            state.confirm = None;
        }
        Kind::LegacyTopic(index) => {
            state.draft.legacy_topic = Some(*index);
            state.confirm = None;
        }
        Kind::Topic(index) => {
            state.draft.legacy_topic = None;
            state.draft.topic = *index;
            state.confirm = None;
        }
        Kind::Flag(index) => {
            state.draft.flags[*index] = !state.draft.flags[*index];
            state.confirm = None;
        }
        Kind::Path(path) => {
            state.draft.path = path.display().to_string();
            state.entries.clear();
            state.confirm = None;
        }
        Kind::Browse => {
            state.confirm = None;
            let path = if state.draft.path.is_empty() {
                std::env::current_dir()
            } else {
                Ok(PathBuf::from(&state.draft.path))
            };
            match path.and_then(|p| {
                std::fs::read_dir(if p.is_dir() {
                    p
                } else {
                    p.parent().unwrap_or(&p).to_path_buf()
                })
            }) {
                Ok(entries) => {
                    state.entries = entries.filter_map(Result::ok).map(|e| e.path()).collect();
                    state.entries.sort();
                }
                Err(error) => owner.settings.feedback = error.to_string(),
            }
        }
    }
}
fn execute(owner: &mut HostOwner, action: Action, draft: &Draft) {
    let path = PathBuf::from(&draft.path);
    let result = match action {
        Action::Import => owner.settings.import_from(&path),
        Action::Export => owner.settings.export_to(&path),
        Action::Backup => owner.settings.restore_backup(),
        Action::Defaults => owner.settings.reset_defaults(),
        Action::MinimapImport | Action::MinimapExport => {
            if let Some(play) = &mut owner.play {
                owner
                    .settings
                    .request_minimap_transfer(action == Action::MinimapImport, path);
                owner
                    .settings
                    .apply_minimap_transfer(&mut play.view)
                    .map(|_| ())
            } else {
                Err(std::io::Error::other("Join a world before transferring."))
            }
        }
        Action::SaveLanguage => {
            if let Some(language) = Language::ALL.get(draft.language) {
                let mut p = owner.settings.preferences.clone();
                p.set_language(*language);
                owner.settings.apply(p);
                let accepted = owner.settings.preferences.clone();
                owner.edit_preferences(|p| {
                    p.language_code = accepted.language_code;
                    p.english = accepted.english;
                });
            }
            return;
        }
        Action::SaveScreenshot | Action::Capture => {
            if action == Action::Capture && owner.settings.screenshot_pending() {
                return;
            }
            let Ok(frames) = draft.backlog.parse::<u8>() else {
                owner.settings.feedback = "Enter 0 to 10 backlog frames".into();
                return;
            };
            if frames > 10 {
                owner.settings.feedback = "Enter 0 to 10 backlog frames".into();
                return;
            }
            let mut p = owner.settings.preferences.clone();
            let s = &mut p.screenshots;
            s.folder.clone_from(&draft.folder);
            if draft.flags[5] {
                s.backlog_seconds = frames;
                s.backlog_frames = 0;
            } else {
                s.backlog_frames = frames;
                if frames == 0 {
                    s.backlog_seconds = 0;
                }
            }
            [
                s.game_only,
                s.on_death,
                s.on_level,
                s.on_skill,
                s.on_achievement,
            ] = [
                draft.flags[0],
                draft.flags[1],
                draft.flags[2],
                draft.flags[3],
                draft.flags[4],
            ];
            p.screenshots = p
                .screenshots
                .reconciled(&owner.settings.preferences.screenshots);
            owner.settings.apply(p);
            let accepted = owner.settings.preferences.screenshots.clone();
            owner.edit_preferences(|p| p.screenshots = accepted);
            if action == Action::Capture {
                let generation = owner.generation();
                owner.settings.request_screenshot_bound(generation);
            }
            return;
        }
        Action::BugReport | Action::PlayerReport => {
            if let Some(play) = &owner.play {
                owner
                    .settings
                    .request_typed_help_report(if action == Action::BugReport {
                        crate::play::help::Report::Bug
                    } else {
                        crate::play::help::Report::Player
                    });
                owner.settings.open_typed_help_report(&play.link);
            }
            return;
        }
        Action::Report => {
            owner.settings.request_help_report();
            owner
                .settings
                .open_help_report(owner.play.as_ref().map(|p| &p.link));
            return;
        }
        Action::Close => {
            owner.cancel();
            return;
        }
    };
    owner.settings.feedback = match result {
        Ok(()) => "Completed".into(),
        Err(error) => error.to_string(),
    };
    // Refresh the existing transient options draft from the owner after a local replacement.
    if matches!(action, Action::Import | Action::Backup | Action::Defaults) {
        let accepted = owner.settings.preferences.clone();
        owner.edit_preferences(|p| *p = accepted);
    }
}
/// Options navigation delegates to this same window, retaining the existing open chord.
pub(super) fn launcher(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    generation: u64,
    page: SettingsPage,
) {
    let child = commands
        .spawn(source::button(skins.buttons.clone(), 240.))
        .observe(
            move |event: On<Activate>, mut owner: NonSendMut<HostOwner>, guard: ModalGuard| {
                if generation == owner.generation()
                    && guard.allows(event.entity)
                    && owner.preferences_draft().is_some()
                {
                    owner.settings.page = page;
                }
            },
        )
        .id();
    label(commands, skins, child, page.label());
    commands.entity(parent).add_child(child);
}
