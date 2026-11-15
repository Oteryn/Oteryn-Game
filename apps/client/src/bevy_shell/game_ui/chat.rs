//! Interactive console projected from the retained chat owner; no local message echoes.
use super::modal::ModalGuard;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameRegions,
};
use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    text::EditableText,
};
use oteryn_client_ui::chat::{ChannelEntry, ConsoleTab, Mode};
use oteryn_client_ui_bevy as source;

mod channels;
mod commands;
mod submission;
#[cfg(test)]
mod tests;
mod widgets;

#[derive(Resource)]
struct ChatState {
    generation: Option<u64>,
    mode: Mode,
    tabs: Vec<commands::Tab>,
    labels: Vec<String>,
    focus: bool,
    value: String,
    pending: Option<submission::Pending>,
    history: SentHistory,
    history_pending: Option<(u64, Option<String>)>,
    rows: Vec<widgets::RenderedRow>,
    row_entities: Vec<Entity>,
    menu: Option<Entity>,
}
impl Default for ChatState {
    fn default() -> Self {
        Self {
            generation: None,
            mode: Mode::Say,
            tabs: Vec::new(),
            labels: Vec::new(),
            focus: false,
            value: String::new(),
            pending: None,
            history: default(),
            history_pending: None,
            rows: Vec::new(),
            row_entities: Vec::new(),
            menu: None,
        }
    }
}
#[derive(Default)]
struct SentHistory {
    lines: std::collections::VecDeque<String>,
    cursor: Option<usize>,
    draft: String,
}
impl SentHistory {
    fn sent(&mut self, value: String) {
        if self.lines.back() != Some(&value) {
            if self.lines.len() == 100 {
                self.lines.pop_front();
            }
            self.lines.push_back(value);
        }
        self.cursor = None;
        self.draft.clear();
    }
    fn navigate(&mut self, older: bool, text: &mut EditableText) {
        if self.lines.is_empty() {
            return;
        }
        if older {
            let index = match self.cursor {
                Some(index) => index.saturating_sub(1),
                None => {
                    self.draft = text.value().to_string();
                    self.lines.len() - 1
                }
            };
            self.cursor = Some(index);
            if let Some(value) = self.lines.get(index) {
                set_text(text, value);
            }
        } else if let Some(index) = self.cursor {
            if let Some(value) = self.lines.get(index + 1) {
                self.cursor = Some(index + 1);
                set_text(text, value);
            } else {
                self.cursor = None;
                set_text(text, &self.draft);
            }
        }
    }
}
fn request(
    owner: &HostOwner,
    state: &mut ChatState,
    entity: Entity,
    field: Field,
    text: &mut EditableText,
    focus: bool,
) -> bool {
    let value = text.value().to_string();
    let sent = submission::request(owner, state, entity, field, text, focus);
    if field == Field::Message {
        if sent {
            state.history.sent(value);
        } else if state.pending.is_some() {
            state.history_pending = Some((owner.generation(), None));
        }
    }
    sent
}
fn capture_history(mut state: ResMut<ChatState>, fields: Query<(&Field, &EditableText)>) {
    if state.pending.is_none() {
        return;
    }
    if let Some((_, value)) = &mut state.history_pending {
        for (field, text) in &fields {
            if *field == Field::Message && text.pending_paste.is_none() {
                *value = Some(text.value().to_string());
            }
        }
    }
}
fn finish_history(
    owner: NonSend<HostOwner>,
    mut state: ResMut<ChatState>,
    fields: Query<(&Field, &EditableText)>,
) {
    if state.pending.is_some() {
        return;
    }
    if let Some((generation, Some(value))) = state.history_pending.take()
        && generation == owner.generation()
        && state.generation == Some(generation)
        && !value.is_empty()
        && fields
            .iter()
            .any(|(field, text)| *field == Field::Message && text.value() == "")
    {
        state.history.sent(value);
    }
}
fn install_history(app: &mut App) {
    app.add_systems(
        PostUpdate,
        capture_history
            .after(bevy::text::EditableTextSystems)
            .before(submission::finish),
    );
    app.add_systems(PostUpdate, finish_history.after(submission::finish));
}
#[derive(Component)]
struct ChatRoot;
#[derive(Component)]
struct Tabs;
#[derive(Component)]
struct Transcript;
#[derive(Component)]
struct TranscriptRoot;
#[derive(Component)]
struct HistoryScroll;
#[derive(Component)]
struct ModeButton;
#[derive(Component)]
struct ChatModeButton;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Field {
    Message,
    Recipient,
}
#[derive(Clone)]
enum Action {
    ComposerMode,
    Send,
    Mode,
    Channels,
    Close,
    Conversation,
    ChannelCancel,
    Tab(commands::Tab),
    Entry(ChannelEntry),
}
#[derive(Component)]
struct Control {
    action: Action,
    generation: u64,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<ChatState>()
        .init_resource::<bevy::clipboard::Clipboard>()
        .add_systems(Startup, widgets::setup.after(super::setup))
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(
            PostUpdate,
            submission::finish.after(bevy::text::EditableTextSystems),
        );
    install_history(app);
    channels::install(app);
}
fn set_text(text: &mut EditableText, value: &str) {
    text.pending_edits.clear();
    text.pending_paste = None;
    text.editor_mut().set_text(value);
    text.queue_edit(bevy::text::TextEdit::TextEnd(false));
}

fn keyboard(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    mut focus: ResMut<InputFocus>,
    mut fields: Query<(Entity, &Field, &mut EditableText), Without<Transcript>>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<ChatState>,
    guard: ModalGuard,
    (pressed, mut commands): (Option<Res<ButtonInput<KeyCode>>>, Commands),
) {
    if let Some(menu) = state.menu {
        if !owner.modal_active() && guard.allows(menu) {
            if keys.read().any(|key| {
                key.key_code == KeyCode::Escape
                    && key.state == bevy::input::ButtonState::Pressed
                    && !key.repeat
            }) {
                commands.entity(menu).despawn();
                state.menu = None;
            }
        } else {
            keys.clear();
        }
        return;
    }
    if !guard.allows_focus(focus.get()) {
        keys.clear();
        return;
    }
    if owner.modal_active() || state.generation != Some(owner.generation()) {
        keys.clear();
        return;
    }
    let Some(play) = &owner.play else {
        keys.clear();
        return;
    };
    for key in keys.read() {
        if key.state != bevy::input::ButtonState::Pressed || key.repeat {
            continue;
        }
        let focused = focus.get();
        match key.key_code {
            KeyCode::Escape => {
                state.pending = None;
                state.history_pending = None;
                if focused.is_some_and(|entity| fields.get(entity).is_ok()) {
                    focus.clear();
                }
                play.link.chat_channel_dialog_cancel();
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if let Some(entity) = focused {
                    if let Ok((_, field, mut text)) = fields.get_mut(entity) {
                        request(&owner, &mut state, entity, *field, &mut text, false);
                    }
                } else if play.link.chat_panel().console.can_send() {
                    for (entity, field, _) in &fields {
                        if *field == Field::Message {
                            focus.set(entity, FocusCause::Navigated);
                            break;
                        }
                    }
                }
            }
            KeyCode::ArrowUp | KeyCode::ArrowDown
                if pressed.as_ref().is_some_and(|keys| {
                    keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight)
                }) =>
            {
                if state.pending.is_none()
                    && let Some(entity) = focused
                    && let Ok((_, Field::Message, mut text)) = fields.get_mut(entity)
                    && text.pending_paste.is_none()
                    && !text
                        .pending_edits
                        .iter()
                        .any(bevy::text::TextEdit::is_destructive)
                {
                    state
                        .history
                        .navigate(key.key_code == KeyCode::ArrowUp, &mut text);
                }
            }
            _ => {}
        }
    }
}

// Queries describe independent retained console widgets, not separate chat owners.
#[allow(clippy::too_many_arguments)]
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<ChatState>,
    mut previous: Local<Option<oteryn_client_ui::chat::ChatPanel>>,
    mut fields: Query<(Entity, &Field, &mut EditableText), Without<Transcript>>,
    mut focus: ResMut<InputFocus>,
    transcripts: Query<Entity, With<TranscriptRoot>>,
    mut captions: Query<&mut Text, Without<Transcript>>,
    mode_buttons: Query<&Children, With<ModeButton>>,
    chat_mode_buttons: Query<&Children, With<ChatModeButton>>,
    guard: ModalGuard,
    tabs: Query<Entity, With<Tabs>>,
    controls: Query<(Entity, &Control)>,
    mut scroll: Query<(&ComputedNode, &mut ScrollPosition), With<HistoryScroll>>,
) {
    let Some(play) = &owner.play else {
        *previous = None;
        if let Some(menu) = state.menu.take() {
            commands.entity(menu).despawn();
        }
        if state.generation.take().is_some() {
            for (entity, _, mut text) in &mut fields {
                set_text(&mut text, "");
                if focus.get() == Some(entity) {
                    focus.clear();
                }
            }
            state.value.clear();
            state.history = default();
            state.history_pending = None;
            state.focus = false;
        }
        return;
    };
    if state.generation != Some(owner.generation()) {
        *previous = None;
        state.generation = Some(owner.generation());
        state.mode = Mode::Say;
        state.focus = false;
        state.value.clear();
        state.history = default();
        state.history_pending = None;
        state.tabs.clear();
        state.rows.clear();
        if let Some(menu) = state.menu.take() {
            commands.entity(menu).despawn();
        }
        for (entity, _, mut text) in &mut fields {
            set_text(&mut text, "");
            if focus.get() == Some(entity) {
                focus.clear();
            }
        }
        play.link.open_chat();
    }
    let panel = play.link.chat_panel();
    let mut changed = false;
    // Cache only the last projection; all edits and commands still belong to PlayLink.
    if previous.as_ref() != Some(&panel) {
        let rows = widgets::transcript_rows(&panel);
        if state.rows != rows {
            if focus
                .get()
                .is_some_and(|entity| state.row_entities.contains(&entity))
            {
                focus.clear();
            }
            state.row_entities.clear();
            for parent in &transcripts {
                commands.entity(parent).despawn_children();
                for row in &rows {
                    let entity =
                        widgets::message_row(&mut commands, &skins, owner.generation(), row);
                    commands.entity(parent).add_child(entity);
                    state.row_entities.push(entity);
                }
            }
            state.rows = rows;
            changed = true;
        }
    }
    // Follow new messages only when the user was already at the end of the log.
    if changed {
        for (node, mut position) in &mut scroll {
            let max = ((node.content_size().y - node.size().y) * node.inverse_scale_factor).max(0.);
            if position.y >= max - 3. {
                position.y = 1_000_000.;
            }
        }
    }
    let mode = match state.mode {
        Mode::Say => "Say",
        Mode::Whisper => "Whisper",
        Mode::Yell => "Yell",
        Mode::Guild => "Guild",
        Mode::Private => "Private",
    };
    let composer_mode = match panel.console.mode {
        oteryn_client_ui::chat::ComposerMode::On => "Chat On",
        oteryn_client_ui::chat::ComposerMode::Off => "Chat Off",
        oteryn_client_ui::chat::ComposerMode::Temporary => "Chat Once",
    };
    for children in &chat_mode_buttons {
        for child in children {
            if let Ok(mut text) = captions.get_mut(*child)
                && text.0 != composer_mode
            {
                text.0 = composer_mode.into();
            }
        }
    }
    for children in &mode_buttons {
        for child in children {
            if let Ok(mut text) = captions.get_mut(*child)
                && text.0 != mode
            {
                text.0 = mode.into();
            }
        }
    }
    let mut current_tabs = vec![
        commands::Tab::Fixed(ConsoleTab::All),
        commands::Tab::Fixed(ConsoleTab::Local),
        commands::Tab::Fixed(ConsoleTab::Private),
        commands::Tab::Fixed(ConsoleTab::Guild),
    ];
    current_tabs.extend(
        panel
            .console
            .channels
            .iter()
            .cloned()
            .map(commands::Tab::Channel),
    );
    let labels = panel.console.tab_labels();
    let selected = |tab: &commands::Tab| match tab {
        commands::Tab::Fixed(tab) => panel.console.current.is_none() && panel.console.tab == *tab,
        commands::Tab::Channel(tab) => panel.console.current.as_ref() == Some(tab),
    };
    let rebuilding = state.tabs != current_tabs || state.labels != labels;
    if rebuilding {
        for parent in &tabs {
            commands.entity(parent).despawn_children();
            for (tab, label) in current_tabs.iter().zip(&labels) {
                let width = widgets::TAB_WIDTH;
                let button = widgets::button(
                    &mut commands,
                    &skins,
                    label,
                    width,
                    Action::Tab(tab.clone()),
                    owner.generation(),
                );
                commands.entity(button).insert(Node {
                    width: px(width),
                    height: px(widgets::TAB_HEIGHT),
                    min_height: px(widgets::TAB_HEIGHT),
                    border: UiRect::all(px(1.)),
                    flex_shrink: 0.,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                });
                if selected(tab) {
                    commands.entity(button).insert(bevy::ui::Checked);
                }
                commands.entity(parent).add_child(button);
            }
        }
        state.tabs = current_tabs;
        state.labels = labels;
    }
    if !rebuilding {
        // Old tab entities are already queued for deletion when the channel list changes.
        for (entity, control) in &controls {
            if let Action::Tab(tab) = &control.action {
                if selected(tab) {
                    commands.entity(entity).insert(bevy::ui::Checked);
                } else {
                    commands.entity(entity).remove::<bevy::ui::Checked>();
                }
            }
        }
    }
    for (entity, field, text) in &mut fields {
        let disabled = owner.modal_active()
            || (*field == Field::Message && !panel.console.can_send())
            || (*field == Field::Recipient && !panel.console.dialog.open);
        if disabled {
            commands
                .entity(entity)
                .insert(bevy::ui::InteractionDisabled);
            if focus.get() == Some(entity) {
                focus.clear();
            }
        } else {
            commands
                .entity(entity)
                .remove::<bevy::ui::InteractionDisabled>();
        }
        if *field == Field::Message {
            let focused = focus.get() == Some(entity) && !disabled && guard.allows(entity);
            if text.value() != state.value.as_str() || state.focus != focused {
                state.value = text.value().to_string();
                state.focus = focused;
            }
            // Poll unchanged input too so a rate-limited typing start can retry.
            play.link.chat_composer(focused, state.value.clone());
        }
    }
    *previous = Some(panel);
}
