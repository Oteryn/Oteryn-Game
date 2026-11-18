//! Chat message rows and their context menu.
use super::*;

#[derive(Component, Clone)]
pub(super) struct MessageRow {
    generation: u64,
    row: RenderedRow,
}

pub(in crate::bevy_shell::game_ui::chat) fn message_row(
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
            chat_font(f32::from(row.font_px)),
            Pickable::default(),
            TextColor(speech_color(row.line.as_ref())),
            bevy::text::LineHeight::RelativeToFont(1.2),
        ))
        .observe(message_menu)
        .id()
}

pub(super) fn speech_color(line: Option<&oteryn_session::ChatLine>) -> Color {
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

impl RenderedRow {
    fn target(&self) -> Option<&str> {
        use oteryn_session::ChatLine;
        match self.line.as_ref()? {
            ChatLine::Local {
                speaker_name, mode, ..
            } if *mode != oteryn_session::ChatSpeechMode::MonsterSay => Some(speaker_name),
            ChatLine::Private { speaker_name, .. } => {
                Some(self.recipient.as_deref().unwrap_or(speaker_name))
            }
            ChatLine::Room { speaker_name, .. } | ChatLine::Party { speaker_name, .. } => {
                Some(speaker_name)
            }
            ChatLine::Guild(line) => Some(&line.speaker_name),
            ChatLine::PrivateChannel(line) => Some(&line.speaker_name),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum MessageAction {
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
pub(super) struct MessageControl {
    message: MessageRow,
    action: MessageAction,
}

pub(super) fn message_menu(
    event: On<PointerClick>,
    rows: Query<&MessageRow>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    mut commands: Commands,
    (skins, scale): (Res<Skins>, Option<Res<UiScale>>),
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
    let name = message.row.target();
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
    let (menu, layer) = crate::bevy_shell::game_ui::hud_panes::context_menu::spawn(
        &mut commands,
        &look,
        position,
        scale.as_deref(),
        &items,
    );
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
pub(super) fn message_dismiss(
    _event: On<PointerClick>,
    mut commands: Commands,
    mut state: ResMut<ChatState>,
) {
    if let Some(menu) = state.menu.take() {
        commands.entity(menu).despawn();
    }
}

pub(super) fn message_action(
    event: On<bevy::ui_widgets::Activate>,
    controls: Query<&MessageControl>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    mut commands: Commands,
    mut state: ResMut<ChatState>,
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
    let name = control.message.row.target();
    match control.action {
        MessageAction::Copy => {
            commands.trigger(super::CopyText(control.message.row.text.clone()));
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

#[cfg(test)]
mod tests;
