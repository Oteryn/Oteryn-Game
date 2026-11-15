//! Local catalogue search; selection and ownership stay in the appearance owner.
use super::*;
use bevy::{
    input_focus::{FocusCause, InputFocus},
    text::EditableText,
    ui_widgets::TextInput,
};
#[derive(Component)]
pub(super) struct Field;
pub(super) fn matches(key: &str, query: &str) -> bool {
    key.to_lowercase()
        .contains(query.trim().to_lowercase().as_str())
}
pub(super) fn update(
    fields: Query<&EditableText, (With<Field>, Changed<EditableText>)>,
    mut state: ResMut<State>,
) {
    for text in &fields {
        let value = text.value().to_string();
        if state.search != value {
            state.search = value;
            state.dirty = true;
        }
    }
}
pub(super) fn build(
    commands: &mut Commands,
    skin: &Skins,
    root: Entity,
    value: &str,
    searching: bool,
    focus: &mut InputFocus,
) {
    let mut text = EditableText::new(value);
    text.max_characters = Some(128);
    let field = commands
        .spawn((
            Field,
            TextInput,
            text,
            Name::new("outfit.search"),
            source::font(skin.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            bevy::text::TextCursorStyle::default(),
            bevy::input_focus::tab_navigation::TabIndex(0),
            Node {
                width: px(250.),
                min_height: px(25.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(field);
    if searching {
        focus.set(field, FocusCause::Navigated);
    }
}
