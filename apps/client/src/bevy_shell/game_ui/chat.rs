//! Interactive console projected from the retained chat owner; no local message echoes.
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
        }
    }
}
#[derive(Component)]
struct ChatRoot;
#[derive(Component)]
struct Tabs;
#[derive(Component)]
struct Transcript;
#[derive(Component)]
struct HistoryScroll;
#[derive(Component)]
struct Feedback;
#[derive(Component)]
struct ModeButton;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Field {
    Message,
    Recipient,
}
#[derive(Clone)]
enum Action {
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
        .add_systems(Startup, widgets::setup.after(super::setup))
        .add_systems(Update, (keyboard, project).chain());
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
    mut fields: Query<(Entity, &Field, &mut EditableText)>,
    owner: NonSend<HostOwner>,
    state: Res<ChatState>,
) {
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
                if focused.is_some_and(|entity| fields.get(entity).is_ok()) {
                    focus.clear();
                }
                play.link.chat_channel_dialog_cancel();
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if let Some(entity) = focused {
                    if let Ok((_, field, mut text)) = fields.get_mut(entity) {
                        match field {
                            Field::Message => {
                                if commands::submit(
                                    &play.link,
                                    state.generation.unwrap_or_default(),
                                    owner.generation(),
                                    state.mode,
                                    &text.value().to_string(),
                                ) {
                                    set_text(&mut text, "");
                                }
                            }
                            Field::Recipient => {
                                play.link
                                    .chat_channel_dialog_name(&text.value().to_string());
                                play.link.chat_channel_dialog_confirm();
                            }
                        }
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
    mut fields: Query<(Entity, &Field, &mut EditableText)>,
    mut focus: ResMut<InputFocus>,
    mut texts: Query<&mut Text, With<Transcript>>,
    mut feedback: Query<&mut Text, (With<Feedback>, Without<Transcript>)>,
    mut captions: Query<&mut Text, (Without<Feedback>, Without<Transcript>)>,
    mode_buttons: Query<&Children, With<ModeButton>>,
    tabs: Query<Entity, With<Tabs>>,
    controls: Query<(Entity, &Control)>,
    mut scroll: Query<(&ComputedNode, &mut ScrollPosition), With<HistoryScroll>>,
) {
    let Some(play) = &owner.play else {
        *previous = None;
        if state.generation.take().is_some() {
            for (entity, _, mut text) in &mut fields {
                set_text(&mut text, "");
                if focus.get() == Some(entity) {
                    focus.clear();
                }
            }
            state.value.clear();
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
        state.tabs.clear();
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
        let transcript = oteryn_client_ui::chat::panel_text(&panel);
        for mut text in &mut texts {
            if text.0 != transcript {
                text.0.clone_from(&transcript);
                changed = true;
            }
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
    let message = panel.feedback.as_deref().unwrap_or("");
    for mut text in &mut feedback {
        if text.0 != message {
            text.0 = message.into();
        }
    }
    let mode = if !panel.console.can_send() {
        "Chat off"
    } else {
        match state.mode {
            Mode::Say => "Say",
            Mode::Whisper => "Whisper",
            Mode::Yell => "Yell",
            Mode::Guild => "Guild",
            Mode::Private => "Private",
        }
    };
    for children in &mode_buttons {
        for child in children {
            if let Ok(mut text) = captions.get_mut(*child) {
                if text.0 != mode {
                    text.0 = mode.into();
                }
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
    if state.tabs != current_tabs || state.labels != labels {
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
                commands.entity(parent).add_child(button);
            }
        }
        state.tabs = current_tabs;
        state.labels = labels;
    }
    for (entity, control) in &controls {
        if let Action::Tab(tab) = &control.action {
            let selected = match tab {
                commands::Tab::Fixed(tab) => {
                    panel.console.current.is_none() && panel.console.tab == *tab
                }
                commands::Tab::Channel(tab) => panel.console.current.as_ref() == Some(tab),
            };
            if selected {
                commands.entity(entity).insert(bevy::ui::Checked);
            } else {
                commands.entity(entity).remove::<bevy::ui::Checked>();
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
            let focused = focus.get() == Some(entity) && !disabled;
            if text.value() != state.value.as_str() || state.focus != focused {
                state.value = text.value().to_string();
                state.focus = focused;
                play.link.chat_composer(focused, state.value.clone());
            }
        }
    }
    *previous = Some(panel);
}
