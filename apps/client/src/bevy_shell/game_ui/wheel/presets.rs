//! Named point builds use the retained local store; importing never applies to the server.
use super::*;
use bevy::{input_focus::tab_navigation::TabIndex, text::EditableText, ui_widgets::TextInput};
use oteryn_client_ui_bevy as source;

#[derive(Component)]
pub(super) struct Field(pub usize);
#[derive(Component)]
pub(super) struct Action {
    pub key: Key,
    pub index: usize,
}
pub(super) fn render(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    panel: &WheelPanel,
    fields: &[String; 2],
    ready: bool,
) {
    for (index, caption) in ["Preset name", "Build code or URL (select and copy / paste)"]
        .into_iter()
        .enumerate()
    {
        widgets::label(commands, skins, parent, caption);
        let entity = commands
            .spawn((
                Field(index),
                TextInput,
                EditableText::new(&fields[index]),
                source::font(skins.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                bevy::text::TextCursorStyle::default(),
                TabIndex(0),
                Node {
                    width: percent(100.),
                    min_height: px(26.),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
            ))
            .id();
        commands.entity(parent).add_child(entity);
    }
    let selected = panel.preset_book.0.contains_key(&panel.preset_fields[0]);
    for (index, caption) in [
        (0, "Save named build"),
        (6, "Rename selected"),
        (2, "Delete selected"),
        (3, "Export code"),
        (4, "Export URL"),
        (5, "Import code / URL"),
    ] {
        let entity = widgets::action_button(
            commands,
            skins,
            parent,
            caption,
            ready && (!matches!(index, 2 | 6) || selected),
        );
        commands
            .entity(entity)
            .insert(Action { key, index })
            .observe(activate);
    }
    widgets::label(
        commands,
        skins,
        parent,
        "Save named build replaces the same name. Loaded points are a proposal; Save applies them. Codes contain points only.",
    );
}
fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let Some((key, expected)) = &state.snapshot else {
        return;
    };
    if !guard.allows(event.entity)
        || action.key != *key
        || key.generation != owner.generation()
        || owner.modal_active()
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    if fields.iter().any(|(_, text)| {
        text.pending_paste.is_some()
            || text
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
    }) {
        return;
    }
    let mut values = expected.preset_fields.clone();
    for (field, text) in &fields {
        values[field.0] = text.value().to_string();
    }
    if action.index == 2 {
        values[0] = expected.preset_fields[0].clone();
    }
    play.link.wheel_preset_bound(expected, action.index, values);
}
