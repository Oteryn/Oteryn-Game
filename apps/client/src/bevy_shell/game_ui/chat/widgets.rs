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
    let text = transcript_row(&mut commands, &skins, 0, "", source::TEXT_COLOR);
    commands.entity(transcript).add_child(text);
    commands.entity(history).add_child(transcript);
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
    if !matches!(control.action, Action::Send | Action::Conversation) {
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
        Action::Send => {
            for (entity, field, mut text) in &mut fields {
                if *field == Field::Message
                    && submission::request(&owner, &mut state, entity, *field, &mut text, true)
                {
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
            let mut opened = false;
            for (entity, field, mut text) in &mut fields {
                if *field == Field::Recipient {
                    opened =
                        submission::request(&owner, &mut state, entity, *field, &mut text, true);
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

pub(super) fn transcript_row(
    commands: &mut Commands,
    skins: &Skins,
    index: usize,
    value: &str,
    color: Color,
) -> Entity {
    commands
        .spawn((
            Transcript,
            TranscriptRow(index),
            TextInput,
            EditableText::new(value),
            bevy::text::TextReadWriteMode::ReadOnly,
            TextCursorStyle::default(),
            source::text(value, source::font(skins.font.clone(), false)),
            Node {
                width: percent(100.),
                flex_shrink: 0.,
                min_height: px(16.),
                ..default()
            },
        ))
        .insert((Pickable::default(), TextColor(color)))
        .id()
}
// Keep the owner's filters, source indices and metadata; speech types determine row colours.
pub(super) fn transcript_rows(panel: &oteryn_client_ui::chat::ChatPanel) -> Vec<(String, Color)> {
    use oteryn_client_ui::chat::{ChannelTab, ConsoleTab};
    use oteryn_session::{ChatLine, ChatSpeechMode, GuildRoomState};
    let mut header = panel.clone();
    header.lines.clear();
    header.recipients.clear();
    let mut rows = vec![(
        oteryn_client_ui::chat::panel_text(&header)
            .trim_end()
            .to_owned(),
        source::TEXT_COLOR,
    )];
    let channel = panel.console.current.as_ref();
    if channel.is_some_and(ChannelTab::receive_only) {
        return rows;
    }
    let tab = if channel.is_some() {
        ConsoleTab::Private
    } else {
        panel.console.tab
    };
    if !panel.console.shows_messages(tab) {
        return rows;
    }
    for (index, line) in panel.lines.iter().enumerate().skip(
        panel
            .lines
            .len()
            .saturating_sub(usize::from(panel.console.presentation.limit)),
    ) {
        if !panel.console.presentation.shows(line)
            || !panel.console.filters.visible(tab, line)
            || !panel.console.communication.visible(line)
        {
            continue;
        }
        let recipient = panel.recipients.get(index).and_then(|name| name.as_deref());
        let visible = match (channel, line) {
            (_, ChatLine::Dropped) => true,
            (Some(ChannelTab::Room(room)), ChatLine::Room { room: actual, .. }) => room == actual,
            (Some(ChannelTab::Private(id)), ChatLine::PrivateChannel(line)) => {
                *id == line.channel_id
            }
            (Some(ChannelTab::Conversation(name)), ChatLine::Private { .. }) => {
                oteryn_client_ui::chat_private_echo::in_conversation(name, line, recipient)
            }
            (Some(_), _) => false,
            (None, _) => matches!(
                (tab, line),
                (ConsoleTab::All, _)
                    | (ConsoleTab::Local, ChatLine::Local { .. })
                    | (
                        ConsoleTab::Private,
                        ChatLine::Private { .. } | ChatLine::PrivateChannel(_)
                    )
                    | (ConsoleTab::Guild, ChatLine::Guild(_))
            ),
        };
        if !visible {
            continue;
        }
        let labels = panel.console.presentation.mode_labels;
        let name = |speaker: &str| panel.console.metadata.name(panel.console.display, speaker);
        let yellow = Color::srgb_u8(255, 255, 0);
        let cyan = Color::srgb_u8(95, 195, 195);
        let orange = Color::srgb_u8(255, 165, 0);
        let (body, color) = match line {
            ChatLine::Local {
                speaker_name,
                mode,
                text,
                ..
            } => {
                let speaker = if *mode == ChatSpeechMode::MonsterSay {
                    speaker_name.clone()
                } else {
                    name(speaker_name)
                };
                (
                    if labels {
                        format!("[{mode:?}] {speaker}: {text}")
                    } else {
                        format!("{speaker}: {text}")
                    },
                    if *mode == ChatSpeechMode::MonsterSay {
                        orange
                    } else {
                        yellow
                    },
                )
            }
            ChatLine::Private { speaker_name, text } => {
                let body = if let Some(recipient) = recipient {
                    oteryn_client_ui::chat_private_echo::echo_text(recipient, text)
                } else {
                    format!(
                        "{}{}: {text}",
                        if labels { "[Private] " } else { "" },
                        name(speaker_name)
                    )
                };
                (body, cyan)
            }
            ChatLine::Guild(line) => {
                if !matches!(&panel.guild, GuildRoomState::Member { guild_id, .. } if *guild_id == line.guild_id)
                {
                    continue;
                }
                (
                    format!(
                        "{}{}: {}",
                        if labels { "[Guild] " } else { "" },
                        name(&line.speaker_name),
                        line.text
                    ),
                    orange,
                )
            }
            ChatLine::Party { speaker_name, text } => (
                format!(
                    "{}{}: {text}",
                    if labels { "[Party] " } else { "" },
                    name(speaker_name)
                ),
                Color::srgb_u8(0, 255, 0),
            ),
            ChatLine::Room {
                room,
                speaker_name,
                text,
            } => (
                format!(
                    "{}{}: {text}",
                    if labels {
                        format!("[{room:?}] ")
                    } else {
                        String::new()
                    },
                    name(speaker_name)
                ),
                yellow,
            ),
            ChatLine::PrivateChannel(line) => (
                format!(
                    "{}{}: {}",
                    if labels { "[Private channel] " } else { "" },
                    name(&line.speaker_name),
                    line.text
                ),
                cyan,
            ),
            ChatLine::Dropped => ("[Some chat lines were dropped]".into(), source::TEXT_COLOR),
        };
        rows.push((
            format!(
                "{}{body}",
                panel.console.metadata.prefix(panel.console.display, index)
            ),
            color,
        ));
    }
    rows
}
