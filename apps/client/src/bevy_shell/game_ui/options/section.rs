//! Shared pieces of the option pages whose editors are lists, text fields and buttons.
use super::*;
use bevy::{text::EditableText, ui_widgets::TextInput};

/// A wrapped page caption.
pub(super) fn label(commands: &mut Commands, parent: Entity, skins: &Skins, text: &str) {
    let entity = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                width: percent(100.),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
}

/// A section button; the caller inserts its role and observer.
pub(super) fn action(
    commands: &mut Commands,
    parent: Entity,
    skins: &Skins,
    caption: &str,
    width: f32,
    id: &str,
    selected: bool,
) -> Entity {
    let entity = commands
        .spawn((
            source::button(skins.buttons.clone(), width),
            Name::new(id.to_owned()),
        ))
        .id();
    if selected {
        commands.entity(entity).insert(bevy::ui::Checked);
    }
    let text = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(text);
    commands.entity(parent).add_child(entity);
    entity
}

/// A wrapping row of section buttons.
pub(super) fn bar(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            width: percent(100.),
            flex_shrink: 0.,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            row_gap: px(2.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}

/// A one-line text field.
pub(super) fn field(
    commands: &mut Commands,
    parent: Entity,
    skins: &Skins,
    id: &str,
    value: &str,
) -> Entity {
    let entity = commands
        .spawn((
            TextInput,
            EditableText::new(value),
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            bevy::text::TextCursorStyle::default(),
            bevy::input_focus::tab_navigation::TabIndex(0),
            Name::new(id.to_owned()),
            Node {
                width: percent(100.),
                height: px(24.),
                flex_shrink: 0.,
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        ))
        .id();
    commands.entity(parent).add_child(entity);
    entity
}

/// True when a section button of `page` may edit the current draft.
pub(super) fn editable(
    owner: &HostOwner,
    state: &OptionsState,
    guard: &ModalGuard<'_, '_>,
    entity: Entity,
    page: Page,
) -> bool {
    guard.allows(entity)
        && state.page == page
        && owner.preferences_draft().is_some()
        && state.generation == Some(owner.generation())
        && !top_server_dialog(owner)
}

/// Applies a fallible edit to a copy of the draft; the draft changes only when it succeeds.
pub(super) fn try_edit(
    owner: &mut HostOwner,
    state: &mut OptionsState,
    edit: impl FnOnce(&mut crate::play::ReferencePreferences) -> Result<(), String>,
    done: &str,
) {
    let Some(mut candidate) = owner.preferences_draft().cloned() else {
        return;
    };
    match edit(&mut candidate) {
        Ok(()) => {
            owner.edit_preferences(|draft| *draft = candidate);
            owner.settings.feedback = done.into();
        }
        Err(error) => owner.settings.feedback = error,
    }
    state.revision = state.revision.wrapping_add(1);
}
