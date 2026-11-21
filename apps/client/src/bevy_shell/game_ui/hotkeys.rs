//! Hotkey editor over the existing scoped preference draft and dispatcher.
use super::super::{owner::HostOwner, ui::Skins};
use crate::play::{CustomAction, ObjectTarget, ReferencePreferences};
use bevy::{
    prelude::*,
    text::EditableText,
    ui_widgets::{Activate, Button, TextInput},
};
use oteryn_client_runtime::depot::ItemEntry;
use oteryn_client_ui_bevy as source;
mod capture;
mod object_picker;
use capture::capture;
use object_picker::project as objects;

fn save(
    p: &mut ReferencePreferences,
    old: Option<&str>,
    key: &str,
    action: CustomAction,
) -> Result<(), String> {
    let mut candidate = p.clone();
    if let Some(old) = old {
        candidate.custom_hotkeys.remove(old);
    }
    candidate
        .assign_custom_hotkey(key, action)
        .map_err(|error| error.to_string())?;
    *p = candidate;
    Ok(())
}
fn action(
    mode: usize,
    text: &str,
    send: bool,
    definition: &str,
    subtype: &str,
) -> Option<CustomAction> {
    if mode == 0 {
        return Some(CustomAction::Text {
            text: text.into(),
            send,
        });
    }
    Some(CustomAction::Object {
        definition: definition.parse().ok()?,
        sub_type: subtype.parse().ok()?,
        target: match mode {
            1 => ObjectTarget::SelfActor,
            2 => ObjectTarget::CurrentTarget,
            3 => ObjectTarget::Cursor,
            _ => ObjectTarget::Use,
        },
    })
}
fn rows(p: &ReferencePreferences) -> Vec<String> {
    p.custom_hotkeys
        .iter()
        .map(|(key, action)| match action {
            CustomAction::Text { text, send } => format!(
                "{key} | Text{} | {text}",
                if *send {
                    " (send automatically)"
                } else {
                    " (draft)"
                }
            ),
            CustomAction::Object {
                definition,
                sub_type,
                target,
            } => format!("{key} | Use {target:?} | {definition}:{sub_type}"),
            other => format!("{key} | {other:?} |"),
        })
        .collect()
}
#[derive(Component)]
struct Editor {
    generation: u64,
    selected: Option<String>,
    mode: usize,
    send: bool,
    feedback: String,
    capturing: bool,
    picking: bool,
}
#[derive(Component)]
struct Field(usize);
#[derive(Component)]
struct Listing;
#[derive(Component)]
struct Caption;
#[derive(Component)]
struct Objects;
#[derive(Component)]
enum Control {
    Select(String),
    New,
    Capture,
    Pick,
    Object(ItemEntry),
    Mode,
    Auto,
    Save,
    Remove,
}

pub(super) fn install(app: &mut App) {
    app.add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (capture, project, objects).chain());
}
pub(super) fn build(commands: &mut Commands, parent: Entity, skins: &Skins) {
    let root = commands
        .spawn((
            Editor {
                generation: 0,
                selected: None,
                mode: 0,
                send: false,
                feedback: String::new(),
                capturing: false,
                picking: false,
            },
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                width: percent(100.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(root);
    label(commands, root, skins, "Hotkeys — key | action | text");
    let listing = commands
        .spawn((
            Listing,
            bevy::ui_widgets::ScrollArea,
            Node {
                height: px(125.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(listing);
    let objects = commands
        .spawn((
            Objects,
            Node {
                display: Display::None,
                height: px(100.),
                overflow: Overflow::scroll_y(),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            bevy::ui_widgets::ScrollArea,
        ))
        .id();
    commands.entity(root).add_child(objects);
    for (index, title) in [
        "Key chord (for example F9)",
        "Text macro",
        "Object definition id",
        "Object subtype",
    ]
    .iter()
    .enumerate()
    {
        label(commands, root, skins, title);
        let field = commands
            .spawn((
                Field(index),
                TextInput,
                EditableText::new(if index == 3 { "0" } else { "" }),
                source::font(skins.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                bevy::text::TextCursorStyle::default(),
                bevy::input_focus::tab_navigation::TabIndex(0),
                Node {
                    width: percent(100.),
                    height: px(24.),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
            ))
            .id();
        commands.entity(root).add_child(field);
    }
    let caption = label(commands, root, skins, "");
    commands.entity(caption).insert(Caption);
    let bar = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(root).add_child(bar);
    for (text, control) in [
        ("New", Control::New),
        ("Capture key", Control::Capture),
        ("Choose carried object", Control::Pick),
        ("Action", Control::Mode),
        ("Send automatically", Control::Auto),
        ("Add / Save", Control::Save),
        ("Remove", Control::Remove),
    ] {
        button(commands, bar, skins, text, control);
    }
}
fn label(commands: &mut Commands, parent: Entity, skins: &Skins, text: &str) -> Entity {
    let id = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(id);
    id
}
fn button(commands: &mut Commands, parent: Entity, skins: &Skins, text: &str, control: Control) {
    let id = commands
        .spawn((
            Button,
            control,
            Node {
                min_height: px(22.),
                padding: UiRect::all(px(3.)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.22, 0.22, 0.22)),
            bevy::input_focus::tab_navigation::TabIndex(0),
        ))
        .observe(activate)
        .id();
    label(commands, id, skins, text);
    commands.entity(parent).add_child(id);
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    mut editors: Query<&mut Editor>,
    mut fields: Query<(&Field, &mut EditableText)>,
    mut owner: NonSendMut<HostOwner>,
    guard: super::modal::ModalGuard,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
) {
    if !guard.allows(event.entity) || owner.preferences_draft().is_none() {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    let Some(mut editor) = editors.iter_mut().next() else {
        return;
    };
    if editor.generation != owner.generation() {
        return;
    }
    if fields.iter().any(|(_, text)| {
        text.pending_paste.is_some()
            || text
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
    }) {
        editor.feedback = "Text is still updating; try again".into();
        return;
    }
    let mut values = [String::new(), String::new(), String::new(), String::new()];
    for (field, text) in &fields {
        values[field.0] = text.value().to_string();
    }
    if !matches!(control, Control::Capture) {
        editor.capturing = false;
    }
    match control {
        Control::Select(key) => {
            let Some(a) = owner
                .preferences_draft()
                .and_then(|p| p.custom_hotkeys.get(key))
            else {
                return;
            };
            editor.capturing = false;
            editor.picking = false;
            editor.feedback.clear();
            editor.selected = Some(key.clone());
            values[0] = key.clone();
            match a {
                CustomAction::Text { text, send } => {
                    editor.mode = 0;
                    editor.send = *send;
                    values[1] = text.clone();
                }
                CustomAction::Object {
                    definition,
                    sub_type,
                    target,
                } => {
                    editor.mode = match target {
                        ObjectTarget::SelfActor => 1,
                        ObjectTarget::CurrentTarget => 2,
                        ObjectTarget::Cursor => 3,
                        _ => 4,
                    };
                    values[2] = definition.to_string();
                    values[3] = sub_type.to_string();
                }
                _ => {
                    editor.feedback =
                        "Choose a text or object action to replace this binding".into();
                }
            }
        }
        Control::Capture => {
            editor.capturing = !editor.capturing;
            editor.picking = false;
            focus.clear();
            editor.feedback = if editor.capturing {
                "Press a key with optional modifiers; click Capture key again to cancel"
            } else {
                "Key capture cancelled"
            }
            .into();
        }
        Control::Pick => {
            editor.picking = !editor.picking;
            editor.capturing = false;
            editor.feedback =
                "Choose a carried object (definition : subtype × count). Artwork unavailable."
                    .into();
        }
        Control::Object(item) => {
            if !editor.picking {
                return;
            }
            if !owner
                .play
                .as_ref()
                .is_some_and(|play| object_picker::carried(&play.link).contains(item))
            {
                editor.feedback = "Object is no longer carried; choose again".into();
                return;
            }
            editor.picking = false;
            if editor.mode == 0 {
                editor.mode = 4;
            }
            values[2] = item.item_definition_ref.to_string();
            values[3] = item.sub_type.to_string();
            editor.feedback = "Object selected; Add / Save to assign".into();
        }
        Control::New => {
            editor.capturing = false;
            editor.picking = false;
            editor.feedback.clear();
            editor.selected = None;
            values = [String::new(), String::new(), String::new(), "0".into()];
        }
        Control::Mode => editor.mode = (editor.mode + 1) % 5,
        Control::Auto => editor.send = !editor.send,
        Control::Remove => {
            if let Some(key) = editor.selected.take() {
                owner.edit_preferences(|p| {
                    p.custom_hotkeys.remove(&key);
                });
            }
        }
        Control::Save => {
            let Some(a) = action(editor.mode, &values[1], editor.send, &values[2], &values[3])
            else {
                editor.feedback = "Enter valid object ids".into();
                return;
            };
            let mut result = Ok(());
            owner.edit_preferences(|p| {
                result = save(p, editor.selected.as_deref(), &values[0], a);
            });
            editor.feedback = if result.is_ok() {
                editor.selected = Some(values[0].trim().to_ascii_uppercase());
                "Binding updated; Apply to persist"
            } else {
                "Invalid or conflicting binding"
            }
            .into();
        }
    }
    for (field, mut text) in &mut fields {
        text.pending_edits.clear();
        text.pending_paste = None;
        text.editor_mut().set_text(&values[field.0]);
    }
}
// The retained editor, list and caption have separate ECS roles.
#[allow(clippy::too_many_arguments)]
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut editors: Query<&mut Editor>,
    lists: Query<Entity, With<Listing>>,
    mut captions: Query<(&mut Text, &mut TextColor), With<Caption>>,
    mut held: Local<Vec<String>>,
) {
    let Some(p) = owner.preferences_draft() else {
        for mut editor in &mut editors {
            editor.capturing = false;
            editor.picking = false;
        }
        held.clear();
        return;
    };
    let Some(mut editor) = editors.iter_mut().next() else {
        held.clear();
        return;
    };
    if editor.generation != owner.generation() {
        editor.capturing = false;
        editor.picking = false;
    }
    editor.generation = owner.generation();
    for (mut text, mut color) in &mut captions {
        color.0 = if editor.feedback.contains("conflict") {
            Color::srgb(1., 0.3, 0.3)
        } else {
            source::TEXT_COLOR
        };
        text.0 = format!(
            "Action: {} | Send automatically: {} | {}",
            [
                "Text",
                "Use on self",
                "Use on target",
                "Use with crosshair",
                "Use object"
            ][editor.mode],
            editor.send,
            editor.feedback
        );
    }
    let current = rows(p);
    if current != *held || lists.iter().any(|_| held.is_empty()) {
        for list in &lists {
            commands.entity(list).despawn_children();
            for ((key, _), row) in p.custom_hotkeys.iter().zip(&current) {
                button(
                    &mut commands,
                    list,
                    &skins,
                    row,
                    Control::Select(key.clone()),
                );
            }
        }
        *held = current;
    }
}

#[cfg(test)]
#[path = "hotkeys/tests.rs"]
mod tests;
