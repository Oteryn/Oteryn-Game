use super::*;
// Tibia Global console QML: 16 px tab bar plus a 2 px overlap, 96 x 18 tabs, 18 x 18 tab
// scroll buttons; the input row is one 16 px line.
pub(super) const TAB_HEIGHT: f32 = 18.;
pub(super) const TAB_WIDTH: f32 = 96.;
pub(super) const TAB_BUTTON: f32 = 18.;
pub(super) const INPUT_HEIGHT: f32 = 16.;
pub(super) const INPUT_SPACING: f32 = 2.;
pub(super) const CHAT_MODE_WIDTH: f32 = 64.;
use crate::bevy_shell::gameplay_dialogs::{
    art::{self, ArtSlot},
    popup,
};
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::text::TextCursorStyle;
use bevy::ui_widgets::{ScrollArea, TextInput};

pub(super) fn setup(mut commands: Commands, regions: Res<GameRegions>, skins: Res<Skins>) {
    let root = commands
        .spawn((
            ChatRoot,
            TabGroup::new(0),
            Node {
                width: percent(100.),
                height: percent(100.),
                min_height: px(0.),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    commands.entity(regions.chat).add_child(root);
    let tab_row = commands
        .spawn((
            TabRow,
            Node {
                width: percent(100.),
                min_height: px(TAB_HEIGHT),
                height: px(TAB_HEIGHT),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    let tabs = commands
        .spawn((
            Tabs,
            ScrollArea,
            Node {
                flex_grow: 1.,
                min_width: px(0.),
                height: px(TAB_HEIGHT),
                overflow: Overflow::scroll_x(),
                column_gap: px(1.),
                ..default()
            },
        ))
        .id();
    let previous = sized_button(
        &mut commands,
        &skins,
        "<",
        Vec2::splat(TAB_BUTTON),
        Action::PreviousTab,
    );
    let next = sized_button(
        &mut commands,
        &skins,
        ">",
        Vec2::splat(TAB_BUTTON),
        Action::NextTab,
    );
    let channels = sized_button(
        &mut commands,
        &skins,
        "+",
        Vec2::splat(TAB_BUTTON),
        Action::Channels,
    );
    let close = sized_button(
        &mut commands,
        &skins,
        "x",
        Vec2::splat(TAB_BUTTON),
        Action::Close,
    );
    commands
        .entity(tab_row)
        .add_children(&[previous, tabs, next, channels, close]);
    let history = commands
        .spawn((
            HistoryScroll,
            ScrollArea,
            ScrollPosition::default(),
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                min_height: px(0.),
                flex_grow: 1.,
                overflow: Overflow::scroll(),
                padding: UiRect::all(px(2.)),
                ..default()
            },
        ))
        .id();
    let transcript = commands
        .spawn((
            TranscriptRoot,
            Node {
                width: percent(100.),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    let text = message_row(
        &mut commands,
        &skins,
        0,
        &RenderedRow {
            text: String::new(),
            line: None,
            font_px: oteryn_client_ui::chat::ConsoleDisplay::default().font_px,
        },
    );
    commands.entity(transcript).add_child(text);
    commands.entity(history).add_child(transcript);
    let compose = commands
        .spawn((
            ComposeRow,
            Node {
                width: percent(100.),
                height: px(INPUT_HEIGHT),
                min_height: px(INPUT_HEIGHT),
                flex_shrink: 0.,
                column_gap: px(INPUT_SPACING),
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .id();
    // A square Say/Whisper/Yell button as tall as the input; Enter sends, there is no Send.
    let mode = commands
        .spawn((
            source::button(skins.buttons.clone(), INPUT_HEIGHT),
            Control {
                action: Action::Mode,
                generation: 0,
            },
            ModeButton,
        ))
        .insert(fixed(Vec2::splat(INPUT_HEIGHT)))
        .observe(activate)
        .id();
    let icon = art::icon(
        &mut commands,
        ArtSlot::ChatSay,
        skins.font.clone(),
        Vec2::splat(INPUT_HEIGHT - 2.),
    );
    commands.entity(mode).add_child(icon);
    let frame = commands
        .spawn((
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                flex_grow: 1.,
                min_width: px(0.),
                height: px(INPUT_HEIGHT),
                ..default()
            },
        ))
        .id();
    let input = text_input(&mut commands, &skins, Field::Message, 255);
    commands.entity(frame).add_child(input);
    let chat_mode = sized_button(
        &mut commands,
        &skins,
        "Chat On",
        Vec2::new(CHAT_MODE_WIDTH, INPUT_HEIGHT),
        Action::ComposerMode,
    );
    commands.entity(chat_mode).insert(ChatModeButton);
    commands
        .entity(compose)
        .add_children(&[mode, frame, chat_mode]);
    // The read-only mirror takes the right third of the console, as the drop target did; the
    // transcript shrinks to the rest while it is open.
    let mirror = commands
        .spawn((
            MirrorPane,
            ScrollArea,
            ScrollPosition::default(),
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                right: px(0.),
                top: px(TAB_HEIGHT),
                bottom: px(INPUT_HEIGHT),
                width: percent(100. / 3.),
                overflow: Overflow::scroll(),
                padding: UiRect::all(px(2.)),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(20, 20, 20)),
        ))
        .id();
    let mirror_text = commands
        .spawn((
            MirrorText,
            source::text("", source::font(skins.font.clone(), false)),
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(mirror).add_child(mirror_text);
    commands
        .entity(root)
        .add_children(&[tab_row, history, compose, mirror]);
    super::channels::setup(&mut commands, root, &skins);
}

/// A button of exactly `size` logical pixels with a caption.
fn sized_button(
    commands: &mut Commands,
    skins: &Skins,
    caption: &str,
    size: Vec2,
    action: Action,
) -> Entity {
    let entity = button(commands, skins, caption, size.x, action, 0);
    commands.entity(entity).insert(fixed(size));
    entity
}
/// The node of a fixed-size framed button.
pub(super) fn fixed(size: Vec2) -> Node {
    Node {
        width: px(size.x),
        min_width: px(size.x),
        height: px(size.y),
        min_height: px(size.y),
        border: UiRect::all(px(1.)),
        flex_shrink: 0.,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

pub(super) fn button(
    commands: &mut Commands,
    skins: &Skins,
    caption: &str,
    width: f32,
    action: Action,
    generation: u64,
) -> Entity {
    let entity = commands
        .spawn((
            source::button(skins.buttons.clone(), width),
            Control { action, generation },
        ))
        .observe(activate)
        .id();
    let caption = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    entity
}

pub(super) fn text_input(
    commands: &mut Commands,
    skins: &Skins,
    field: Field,
    limit: usize,
) -> Entity {
    commands
        .spawn((
            field,
            crate::bevy_shell::input::ChatComposer,
            TextInput,
            EditableText {
                max_characters: Some(limit),
                ..default()
            },
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(0),
            Node {
                width: percent(100.),
                height: px(INPUT_HEIGHT),
                min_height: px(INPUT_HEIGHT),
                padding: UiRect::axes(px(3.), px(1.)),
                ..default()
            },
        ))
        .id()
}

pub(super) fn activate(
    event: On<bevy::ui_widgets::Activate>,
    controls: Query<&Control>,
    mut fields: Query<(Entity, &Field, &mut EditableText)>,
    mut focus: ResMut<InputFocus>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<ChatState>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if owner.modal_active() || state.generation != Some(owner.generation()) {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if !matches!(control.action, Action::Conversation) {
        state.pending = None;
    }
    match &control.action {
        Action::ComposerMode => {
            play.link.chat_console_mode();
            if play.link.chat_panel().console.can_send() {
                for (entity, field, _) in &mut fields {
                    if *field == Field::Message {
                        focus.set(entity, FocusCause::Navigated);
                        break;
                    }
                }
            } else {
                focus.clear();
            }
        }
        Action::Tab(tab) if control.generation == owner.generation() => {
            commands::select(&play.link, tab)
        }
        Action::Entry(entry) if control.generation == owner.generation() => {
            play.link.chat_open_entry(entry);
            play.link.chat_channel_dialog_cancel();
        }
        Action::Mode => state.mode = next_mode(state.mode),
        Action::PreviousTab => play.link.chat_channel_cycle(false),
        Action::NextTab => play.link.chat_channel_cycle(true),
        Action::Channels => play.link.chat_channel_dialog_open(),
        Action::Close => play.link.chat_close_current(),
        Action::ChannelCancel => play.link.chat_channel_dialog_cancel(),
        Action::Conversation => {
            let mut opened = false;
            for (entity, field, mut text) in &mut fields {
                if *field == Field::Recipient {
                    opened = request(&owner, &mut state, entity, *field, &mut text, true);
                }
            }
            if opened {
                for (entity, field, _) in &mut fields {
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

/// The mode button cycles Say, Whisper and Yell, as in Tibia Global.
pub(super) fn next_mode(mode: Mode) -> Mode {
    match mode {
        Mode::Say => Mode::Whisper,
        Mode::Whisper => Mode::Yell,
        _ => Mode::Say,
    }
}
/// The mode button's icon.
pub(super) fn mode_art(mode: Mode) -> ArtSlot {
    match mode {
        Mode::Whisper => ArtSlot::ChatWhisper,
        Mode::Yell => ArtSlot::ChatYell,
        _ => ArtSlot::ChatSay,
    }
}

#[derive(Clone, PartialEq)]
pub(super) struct RenderedRow {
    text: String,
    line: Option<oteryn_session::ChatLine>,
    /// The console font size in pixels; a change re-renders the transcript.
    font_px: u16,
}
pub(super) fn transcript_rows(panel: &oteryn_client_ui::chat::ChatPanel) -> Vec<RenderedRow> {
    let mut single = panel.clone();
    single.lines.clear();
    single.recipients.clear();
    let font_px = panel.console.display.font_px;
    // The console shows no help header: the shared projection's header line is dropped.
    let header = oteryn_client_ui::tr!("Local Chat — Say / Whisper / Yell / Private\n");
    let summary = oteryn_client_ui::chat::panel_text(&single);
    let mut rows = vec![RenderedRow {
        text: summary.strip_prefix(header).unwrap_or(&summary).to_owned(),
        line: None,
        font_px,
    }];
    single.feedback = None;
    single.console.show_server_messages = false;
    single.console.display.time = false;
    let prefix = oteryn_client_ui::chat::panel_text(&single);
    for (index, line) in panel.lines.iter().enumerate().skip(
        panel
            .lines
            .len()
            .saturating_sub(usize::from(panel.console.presentation.limit)),
    ) {
        single.lines.clear();
        single.lines.push(line.clone());
        single.recipients.clear();
        single
            .recipients
            .push(panel.recipients.get(index).cloned().flatten());
        let rendered = oteryn_client_ui::chat::panel_text(&single);
        if let Some(text) = rendered
            .strip_prefix(&prefix)
            .filter(|text| !text.is_empty())
        {
            rows.push(RenderedRow {
                text: format!(
                    "{}{text}",
                    panel.console.metadata.prefix(panel.console.display, index)
                ),
                line: Some(line.clone()),
                font_px,
            });
        }
    }
    rows
}
#[derive(Component, Clone)]
struct MessageRow {
    generation: u64,
    row: RenderedRow,
}
pub(super) fn message_row(
    commands: &mut Commands,
    skins: &Skins,
    generation: u64,
    row: &RenderedRow,
) -> Entity {
    commands
        .spawn((
            Transcript,
            MessageRow {
                generation,
                row: row.clone(),
            },
            TextInput,
            EditableText::new(&row.text),
            bevy::text::TextReadWriteMode::ReadOnly,
            TextCursorStyle::default(),
            source::text(
                &row.text,
                bevy::text::TextFont {
                    font_size: bevy::text::FontSize::Px(f32::from(row.font_px)),
                    ..source::font(skins.font.clone(), false)
                },
            ),
            Node {
                width: percent(100.),
                flex_shrink: 0.,
                min_height: px(16.),
                ..default()
            },
        ))
        .insert((
            Pickable::default(),
            TextColor(speech_color(row.line.as_ref())),
            bevy::text::LineHeight::RelativeToFont(1.2),
        ))
        .observe(message_menu)
        .id()
}
fn speech_color(line: Option<&oteryn_session::ChatLine>) -> Color {
    use oteryn_session::{ChatLine, ChatRoom, ChatSpeechMode};
    match line {
        Some(ChatLine::Private { .. } | ChatLine::PrivateChannel(_)) => {
            Color::srgb_u8(95, 195, 195)
        }
        Some(ChatLine::Guild(_)) => Color::srgb_u8(255, 165, 0),
        Some(ChatLine::Local {
            mode: ChatSpeechMode::MonsterSay,
            ..
        }) => Color::srgb_u8(255, 154, 0),
        Some(ChatLine::Room {
            room: ChatRoom::Gamemaster,
            ..
        }) => Color::srgb_u8(250, 96, 96),
        Some(ChatLine::Local { .. } | ChatLine::Room { .. } | ChatLine::Party { .. }) => {
            Color::srgb_u8(255, 255, 0)
        }
        _ => source::TEXT_COLOR,
    }
}
fn speaker(line: &oteryn_session::ChatLine) -> Option<&str> {
    use oteryn_session::ChatLine;
    match line {
        ChatLine::Local {
            speaker_name, mode, ..
        } if *mode != oteryn_session::ChatSpeechMode::MonsterSay => Some(speaker_name),
        ChatLine::Private { speaker_name, .. }
        | ChatLine::Room { speaker_name, .. }
        | ChatLine::Party { speaker_name, .. } => Some(speaker_name),
        ChatLine::Guild(line) => Some(&line.speaker_name),
        ChatLine::PrivateChannel(line) => Some(&line.speaker_name),
        _ => None,
    }
}
#[derive(Clone, Copy)]
enum MessageAction {
    Copy,
    Private,
    Ignore,
    Report,
    Lists,
    Save,
    Mirror,
    CloseMirror,
}
#[derive(Component)]
struct MessageControl {
    message: MessageRow,
    action: MessageAction,
}
fn message_menu(
    event: On<PointerClick>,
    rows: Query<&MessageRow>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    mut commands: Commands,
    skins: Res<Skins>,
    mut state: ResMut<ChatState>,
) {
    if event.button != bevy::picking::pointer::PointerButton::Secondary
        || owner.modal_active()
        || !guard.allows(event.entity)
    {
        return;
    }
    let Ok(message) = rows.get(event.entity) else {
        return;
    };
    if message.generation != owner.generation() {
        return;
    }
    if let Some(old) = state.menu.take() {
        commands.entity(old).despawn();
    }
    let position = event.pointer.location().position;
    let name = message.row.line.as_ref().and_then(speaker);
    let mut actions = Vec::new();
    if let Some(name) = name {
        actions.extend([
            (Some(MessageAction::Private), format!("Message to {name}")),
            (Some(MessageAction::Ignore), format!("Ignore {name}")),
            (Some(MessageAction::Report), format!("Report {name}")),
            (None, String::new()),
        ]);
    }
    actions.extend([
        (Some(MessageAction::Copy), "Copy Message".to_string()),
        (None, String::new()),
        (Some(MessageAction::Lists), "Communication Lists".into()),
        (Some(MessageAction::Save), "Save Messages".into()),
    ]);
    actions.push(
        if owner
            .play
            .as_ref()
            .is_some_and(|play| play.link.chat_panel().console.mirror.is_some())
        {
            (
                Some(MessageAction::CloseMirror),
                "Close Mirrored Tab".into(),
            )
        } else {
            (Some(MessageAction::Mirror), "Mirror This Tab".into())
        },
    );
    let items: Vec<_> = actions
        .iter()
        .map(|(action, label)| match action {
            Some(_) => popup::Item::entry(label.as_str()),
            None => popup::Item::Separator,
        })
        .collect();
    let look = popup::Look {
        font: skins.font.clone(),
        frame: skins.frame.clone(),
        checked: Some(skins.checked.clone()),
        unchecked: Some(skins.unchecked.clone()),
    };
    let menu = popup::spawn(&mut commands, &look, position, &items);
    let layer = commands
        .spawn((
            super::super::GameModal,
            GlobalZIndex(200),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                ..default()
            },
        ))
        .add_children(&[menu.scrim, menu.root])
        .id();
    commands.entity(menu.scrim).observe(message_dismiss);
    for (entity, action) in menu
        .entries
        .iter()
        .zip(actions.iter().filter_map(|(action, _)| *action))
    {
        commands
            .entity(*entity)
            .insert(MessageControl {
                message: message.clone(),
                action,
            })
            .observe(message_action);
    }
    state.menu = Some(layer);
}
/// A click outside the message menu closes it.
fn message_dismiss(_event: On<PointerClick>, mut commands: Commands, mut state: ResMut<ChatState>) {
    if let Some(menu) = state.menu.take() {
        commands.entity(menu).despawn();
    }
}
fn message_action(
    event: On<bevy::ui_widgets::Activate>,
    controls: Query<&MessageControl>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    mut commands: Commands,
    mut state: ResMut<ChatState>,
    mut clipboard: ResMut<bevy::clipboard::Clipboard>,
) {
    if owner.modal_active() || !guard.allows(event.entity) {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    let Some(play) = &owner.play else { return };
    if control.message.generation != owner.generation() {
        return;
    }
    if control
        .message
        .row
        .line
        .as_ref()
        .is_some_and(|line| !play.link.chat_panel().lines.contains(line))
    {
        return;
    }
    let name = control.message.row.line.as_ref().and_then(speaker);
    match control.action {
        MessageAction::Copy => {
            let _ = clipboard.set_text(control.message.row.text.clone());
        }
        MessageAction::Private => {
            if let Some(name) = name {
                play.link.chat_open_conversation(name);
            }
        }
        MessageAction::Ignore => {
            if let Some(name) = name {
                play.link.chat_communication(
                    Some(oteryn_client_ui::chat::CommunicationList::Ignore),
                    name,
                    true,
                );
            }
        }
        MessageAction::Report => {
            if let Some(name) = name {
                play.link.rule_violation_open(name);
            }
        }
        MessageAction::Lists => play.link.chat_lists_open(name),
        MessageAction::Save => {
            let _ = play
                .link
                .chat_save_log(std::path::Path::new("chat.log"), true);
        }
        MessageAction::Mirror => {
            let _ = play.link.chat_console_mirror();
        }
        MessageAction::CloseMirror => play.link.chat_console_mirror_close(),
    }
    if let Some(menu) = state.menu.take() {
        commands.entity(menu).despawn();
    }
}
