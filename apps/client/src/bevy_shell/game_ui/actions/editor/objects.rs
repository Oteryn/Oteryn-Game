//! The visible object picker: carried objects fill the draft's object fields, so the object
//! actions (and a multiaction draft) can take an object while the editor fences item drops.
use super::*;
use crate::bevy_shell::game_ui::{inventory::carried_items, modal::ModalGuard};

#[derive(Component)]
pub(super) struct ObjectsRoot;
/// One carried object: its definition and subtype.
type Object = (std::num::NonZeroU32, u32);
/// The editor selection and carried objects the picker last rendered.
type Rendered = ((usize, u64), Vec<Object>);
#[derive(Component, Clone, Copy)]
struct Choice {
    slot: usize,
    generation: u64,
    definition: std::num::NonZeroU32,
    sub_type: u32,
}

/// The carried objects, one entry per definition and subtype, in carried order.
fn objects(link: &crate::play::PlayLink) -> Vec<Object> {
    let mut objects = Vec::new();
    for item in carried_items(link) {
        let object = (item.item_definition_ref, item.sub_type);
        if !objects.contains(&object) {
            objects.push(object);
        }
    }
    objects
}

fn label(definition: std::num::NonZeroU32, sub_type: u32) -> String {
    if sub_type == 0 {
        format!("Object {definition}")
    } else {
        format!("Object {definition} ({sub_type})")
    }
}

pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    state: Res<EditorState>,
    roots: Query<Entity, With<ObjectsRoot>>,
    mut rendered: Local<Option<Rendered>>,
) {
    let Some(selected) = state.selected else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    let key = (selected, objects(&play.link));
    if rendered.as_ref() == Some(&key) {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn_children();
        for &(definition, sub_type) in &key.1 {
            let button = commands
                .spawn((
                    source::button(skins.buttons.clone(), 150.),
                    Choice {
                        slot: selected.0,
                        generation: selected.1,
                        definition,
                        sub_type,
                    },
                ))
                .observe(choose)
                .id();
            let text = commands
                .spawn(source::button_caption(
                    &label(definition, sub_type),
                    skins.font.clone(),
                ))
                .id();
            commands.entity(button).add_child(text);
            commands.entity(root).add_child(button);
        }
        if key.1.is_empty() {
            let text = commands
                .spawn(source::text(
                    "No carried object.",
                    source::font(skins.font.clone(), false),
                ))
                .id();
            commands.entity(root).add_child(text);
        }
    }
    *rendered = Some(key);
}

fn choose(
    event: On<Activate>,
    guard: ModalGuard,
    choices: Query<&Choice>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<EditorState>,
    mut fields: Query<(&Field, &mut EditableText)>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(choice) = choices.get(event.entity) else {
        return;
    };
    if owner.modal_active()
        || owner.generation() != choice.generation
        || state.selected != Some((choice.slot, choice.generation))
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if !objects(&play.link).contains(&(choice.definition, choice.sub_type)) {
        state.message = tr!("That object is no longer carried; choose it again.").into();
        return;
    }
    for (field, mut text) in &mut fields {
        let value = match field.0 {
            1 => choice.definition.to_string(),
            4 => choice.sub_type.to_string(),
            _ => continue,
        };
        text.pending_edits.clear();
        text.pending_paste = None;
        text.editor_mut().set_text(&value);
    }
    state.message = format!(
        "Selected {}. Press an object action to use it.",
        label(choice.definition, choice.sub_type)
    );
}
