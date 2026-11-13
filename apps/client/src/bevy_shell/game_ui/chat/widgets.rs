use super::*;
// TibiaStyle.qml:302,307,309: source chat limit and tab geometry.
pub(super) const TAB_HEIGHT: f32 = 16.;
pub(super) const TAB_WIDTH: f32 = 96.;
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
    let tabs = commands
        .spawn((
            Tabs,
            ScrollArea,
            Node {
                width: percent(100.),
                min_height: px(TAB_HEIGHT),
                height: px(TAB_HEIGHT),
                flex_shrink: 0.,
                overflow: Overflow::scroll_x(),
                column_gap: px(1.),
                ..default()
            },
        ))
        .id();
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
    let text = commands
        .spawn((
            Transcript,
            source::text("", source::font(skins.font.clone(), false)),
        ))
        .id();
    commands.entity(history).add_child(text);
    let compose = commands
        .spawn(Node {
            width: percent(100.),
            height: px(23.),
            min_height: px(23.),
            flex_shrink: 0.,
            column_gap: px(2.),
            align_items: AlignItems::Center,
            ..default()
        })
        .id();
    let mode = button(&mut commands, &skins, "Say", 55., Action::Mode, 0);
    commands.entity(mode).insert(ModeButton);
    let frame = commands
        .spawn((
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                flex_grow: 1.,
                min_width: px(0.),
                height: px(23.),
                ..default()
            },
        ))
        .id();
    let input = text_input(&mut commands, &skins, Field::Message, 255);
    commands.entity(frame).add_child(input);
    let send = button(&mut commands, &skins, "Send", 44., Action::Send, 0);
    let channels = button(&mut commands, &skins, "+", 22., Action::Channels, 0);
    let close = button(&mut commands, &skins, "×", 22., Action::Close, 0);
    let chat_mode = button(
        &mut commands,
        &skins,
        "Chat On",
        70.,
        Action::ComposerMode,
        0,
    );
    commands.entity(chat_mode).insert(ChatModeButton);
    commands
        .entity(compose)
        .add_children(&[mode, frame, send, channels, close, chat_mode]);
    commands
        .entity(root)
        .add_children(&[tabs, history, compose]);
    super::channels::setup(&mut commands, root, &skins);
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
                height: px(23.),
                min_height: px(23.),
                padding: UiRect::all(px(3.)),
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
    match &control.action {
        Action::ComposerMode => {} // Deliberately unwired until the native caller test is observed red.
        Action::Tab(tab) if control.generation == owner.generation() => {
            commands::select(&play.link, tab)
        }
        Action::Entry(entry) if control.generation == owner.generation() => {
            play.link.chat_open_entry(entry);
            play.link.chat_channel_dialog_cancel();
        }
        Action::Send => {
            for (entity, field, mut text) in &mut fields {
                if *field == Field::Message
                    && commands::submit(
                        &play.link,
                        owner.generation(),
                        owner.generation(),
                        state.mode,
                        &text.value().to_string(),
                    )
                {
                    set_text(&mut text, "");
                    focus.set(entity, FocusCause::Navigated);
                }
            }
        }
        Action::Mode => {
            state.mode = match state.mode {
                Mode::Say => Mode::Whisper,
                Mode::Whisper => Mode::Yell,
                Mode::Yell => Mode::Guild,
                _ => Mode::Say,
            };
        }
        Action::Channels => play.link.chat_channel_dialog_open(),
        Action::Close => play.link.chat_close_current(),
        Action::ChannelCancel => play.link.chat_channel_dialog_cancel(),
        Action::Conversation => {
            for (_, field, text) in &mut fields {
                if *field == Field::Recipient {
                    play.link
                        .chat_channel_dialog_name(&text.value().to_string());
                }
            }
            if play.link.chat_channel_dialog_confirm() {
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
