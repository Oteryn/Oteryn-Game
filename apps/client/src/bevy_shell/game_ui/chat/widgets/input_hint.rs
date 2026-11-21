//! Chat input placeholder text, tab cycling and focus release.
use super::*;

/// Tab and Shift+Tab in the chat input switch channels, as in the game view, instead of
/// moving the focus.
pub(in crate::bevy_shell::game_ui::chat) fn tab_cycle(
    mut event: On<bevy::input_focus::FocusedInput<bevy::input::keyboard::KeyboardInput>>,
    owner: NonSend<HostOwner>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
) {
    let key = &event.input;
    if key.key_code != KeyCode::Tab || key.state != bevy::input::ButtonState::Pressed || key.repeat
    {
        return;
    }
    event.propagate(false);
    if owner.modal_active() {
        return;
    }
    let back = keys.is_some_and(|keys| keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]));
    if let Some(play) = &owner.play {
        play.link.chat_channel_cycle(!back);
    }
}

/// The selected tab's caption is white and the others grey.
pub(in crate::bevy_shell::game_ui::chat) fn tab_colors(
    controls: Query<(&Control, Has<bevy::ui::Checked>, &Children)>,
    mut captions: Query<&mut TextColor, With<source::ButtonCaption>>,
) {
    for (control, checked, children) in &controls {
        if !matches!(control.action, Action::Tab(_)) {
            continue;
        }
        let color = if checked { ACTIVE_TAB } else { INACTIVE_TAB };
        for child in children {
            if let Ok(mut caption) = captions.get_mut(*child)
                && caption.0 != color
            {
                caption.0 = color;
            }
        }
    }
}

/// Shows the input hint while the message field is empty and unfocused.
pub(in crate::bevy_shell::game_ui::chat) fn placeholder(
    focus: Res<InputFocus>,
    owner: NonSend<HostOwner>,
    fields: Query<(Entity, &Field, &EditableText)>,
    mut hints: Query<(&mut Visibility, &mut Text), With<Placeholder>>,
) {
    use oteryn_client_ui::chat::ComposerMode;
    let mode = owner
        .play
        .as_ref()
        .map_or(ComposerMode::On, |play| play.link.chat_panel().console.mode);
    let hint = if mode == ComposerMode::Auto {
        AUTO_PLACEHOLDER
    } else {
        PLACEHOLDER
    };
    let shown = mode != ComposerMode::Off
        && fields.iter().any(|(entity, field, text)| {
            *field == Field::Message && focus.get() != Some(entity) && text.value() == ""
        });
    let wanted = if shown {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (mut visibility, mut text) in &mut hints {
        if *visibility != wanted {
            *visibility = wanted;
        }
        if text.0 != hint {
            hint.clone_into(&mut text.0);
        }
    }
}

/// Chat Auto: a mouse press outside the input (on the map or any other UI) ends typing.
pub(in crate::bevy_shell::game_ui::chat) fn auto_release(
    owner: NonSend<HostOwner>,
    buttons: Option<Res<ButtonInput<MouseButton>>>,
    mut focus: ResMut<InputFocus>,
    fields: Query<&Field>,
    areas: Query<&bevy::picking::hover::Hovered, With<InputArea>>,
) {
    let Some(play) = &owner.play else {
        return;
    };
    if play.link.chat_panel().console.mode != oteryn_client_ui::chat::ComposerMode::Auto
        || !buttons.is_some_and(|buttons| buttons.get_just_pressed().next().is_some())
        || areas.iter().any(|hovered| hovered.get())
    {
        return;
    }
    if focus
        .get()
        .is_some_and(|entity| fields.get(entity) == Ok(&Field::Message))
    {
        focus.clear();
    }
}
