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
    scale::{Module, Scaled, ScaledFont},
};
/// The chat module's scale tag for a node of `width` x `height` at 100% (0 = layout's).
pub(super) fn chat_size(width: f32, height: f32) -> Scaled {
    Scaled {
        module: Module::Chat,
        size: Vec2::new(width, height),
    }
}
/// The chat module's scale tag for a text of `size` pixels at 100%.
pub(super) fn chat_font(size: f32) -> ScaledFont {
    ScaledFont {
        module: Module::Chat,
        size,
    }
}
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::text::TextCursorStyle;
use bevy::ui_widgets::{ScrollArea, TextInput};

pub(super) fn setup(mut commands: Commands, regions: Res<GameRegions>, skins: Res<Skins>) {
    let root = commands
        .spawn((
            ChatRoot,
            crate::bevy_shell::gameplay_dialogs::scale::ModuleRoot(Module::Chat),
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
            chat_size(0., TAB_HEIGHT),
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
            chat_size(0., TAB_HEIGHT),
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
            chat_size(0., INPUT_HEIGHT),
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
    commands
        .entity(icon)
        .insert(chat_size(INPUT_HEIGHT - 2., INPUT_HEIGHT - 2.));
    commands.entity(mode).add_child(icon);
    let frame = commands
        .spawn((
            InputArea,
            source::frame(skins.inset.clone(), [1.; 4]),
            chat_size(0., INPUT_HEIGHT),
            Node {
                flex_grow: 1.,
                min_width: px(0.),
                height: px(INPUT_HEIGHT),
                ..default()
            },
        ))
        .id();
    let input = text_input(&mut commands, &skins, Field::Message, 255);
    commands.entity(input).observe(tab_cycle);
    let hint = commands
        .spawn((
            Placeholder,
            source::text(PLACEHOLDER, source::font(skins.font.clone(), false)),
            chat_font(11.),
            Node {
                position_type: PositionType::Absolute,
                left: px(3.),
                top: px(1.),
                ..default()
            },
        ))
        .insert((TextColor(INACTIVE_TAB), Pickable::IGNORE))
        .id();
    commands.entity(frame).add_children(&[input, hint]);
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

/// The empty input's hint, shown until the player focuses it or types.
pub(super) const PLACEHOLDER: &str = "Type here to chat";
/// The hint of Chat Auto, where keys reach the chat only after a click into the input.
pub(super) const AUTO_PLACEHOLDER: &str = "Click here to chat";
pub(super) const ACTIVE_TAB: Color = Color::WHITE;
pub(super) const INACTIVE_TAB: Color = Color::srgb(0.56, 0.56, 0.56);
#[derive(Component)]
pub(super) struct Placeholder;

mod input_hint;
pub(super) use input_hint::{auto_release, placeholder, tab_colors, tab_cycle};

/// The input frame of the chat; in Chat Auto a press anywhere else gives the keyboard back
/// to the game.
#[derive(Component)]
#[require(bevy::picking::hover::Hovered)]
pub(super) struct InputArea;

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
/// The node of a fixed-size framed button, scaled with the chat module.
pub(super) fn fixed(size: Vec2) -> (Node, Scaled) {
    (
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
        },
        chat_size(size.x, size.y),
    )
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
        .spawn((
            source::button_caption(caption, skins.font.clone()),
            chat_font(8.),
        ))
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
            chat_font(11.),
            chat_size(0., INPUT_HEIGHT),
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
mod messages;
pub(super) use messages::message_row;
