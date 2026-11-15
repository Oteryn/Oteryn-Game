//! Hotkey editor over the existing scoped preference draft and dispatcher.
use super::super::{owner::HostOwner, ui::Skins};
use crate::play::{CustomAction, ObjectTarget, ReferencePreferences};
use bevy::{
    prelude::*,
    text::EditableText,
    ui_widgets::{Activate, Button, TextInput},
};
use oteryn_client_ui_bevy as source;

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
}
#[derive(Component)]
struct Field(usize);
#[derive(Component)]
struct Listing;
#[derive(Component)]
struct Caption;
#[derive(Component)]
enum Control {
    Select(String),
    New,
    Mode,
    Auto,
    Save,
    Remove,
}

pub(super) fn install(app: &mut App) {
    app.add_systems(Update, project);
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
    match control {
        Control::Select(key) => {
            let Some(a) = owner
                .preferences_draft()
                .and_then(|p| p.custom_hotkeys.get(key))
            else {
                return;
            };
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
        Control::New => {
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
    mut captions: Query<&mut Text, With<Caption>>,
    mut held: Local<Vec<String>>,
) {
    let Some(p) = owner.preferences_draft() else {
        held.clear();
        return;
    };
    let Some(mut editor) = editors.iter_mut().next() else {
        held.clear();
        return;
    };
    editor.generation = owner.generation();
    for mut text in &mut captions {
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
mod tests {
    use super::*;
    #[test]
    fn hotkeys_add_edit_remove() {
        let mut p = crate::play::ReferencePreferences::default();
        let action = crate::play::CustomAction::Text {
            text: "hello".into(),
            send: false,
        };
        assert!(save(&mut p, None, "F9", action.clone()).is_ok());
        assert!(save(&mut p, Some("F9"), "F10", action.clone()).is_err());
        assert!(p.custom_hotkeys.contains_key("F9"));
        assert!(save(&mut p, Some("F9"), "SUPER+F10", action).is_ok());
        assert!(!p.custom_hotkeys.contains_key("F9"));
        p.custom_hotkeys.remove("SUPER+F10");
        assert!(!p.custom_hotkeys.contains_key("SUPER+F10"));
    }
    #[test]
    fn hotkeys_list() {
        let mut p = crate::play::ReferencePreferences::default();
        p.custom_hotkeys.clear();
        p.custom_hotkeys.insert(
            "F9".into(),
            crate::play::CustomAction::Text {
                text: "hello".into(),
                send: true,
            },
        );
        assert_eq!(rows(&p), vec!["F9 | Text (send automatically) | hello"]);
    }
    #[test]
    fn hotkey_text_macro_auto_send() -> Result<(), Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!(
            "oteryn-hotkeys-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        let mut store = crate::play::PreferenceStore::defaults(path.clone());
        for send in [false, true] {
            let a = action(0, "hello", send, "", "").ok_or("missing text action")?;
            assert_eq!(
                a,
                crate::play::CustomAction::Text {
                    text: "hello".into(),
                    send
                }
            );
            let mut p = crate::play::ReferencePreferences::default();
            save(&mut p, None, "F9", a.clone())?;
            store.apply(p)?;
            let restored = crate::play::PreferenceStore::read_transfer(&path)?;
            assert_eq!(restored.custom_hotkeys.get("F9"), Some(&a));
        }
        std::fs::remove_file(path)?;
        Ok(())
    }
    #[test]
    fn hotkey_use_object_modes() {
        for (mode, target) in [
            (1, crate::play::ObjectTarget::SelfActor),
            (2, crate::play::ObjectTarget::CurrentTarget),
            (3, crate::play::ObjectTarget::Cursor),
        ] {
            assert_eq!(
                action(mode, "", false, "100", "0"),
                Some(crate::play::CustomAction::Object {
                    definition: 100,
                    sub_type: 0,
                    target
                })
            );
        }
    }
}
