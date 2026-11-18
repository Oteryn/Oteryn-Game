//! A local marker editor bound to the exact viewed minimap coordinate.
use super::*;
use bevy::{
    input_focus::InputFocus,
    text::EditableText,
    ui_widgets::{Activate, TextInput},
};
type MarkerKey = (u64, (i32, i32, i16));
#[derive(Resource, Default)]
pub(super) struct Editor {
    at: Option<MarkerKey>,
    root: Option<Entity>,
    pending: Option<Entity>,
    color: u8,
}
#[derive(Component)]
struct Field;
#[derive(Component)]
struct Control {
    key: MarkerKey,
    action: Action,
}
#[derive(Clone, Copy)]
enum Action {
    Save,
    Remove,
    Cancel,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<Editor>()
        .add_systems(Update, project)
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}
pub(super) fn click(
    event: On<PointerClick>,
    nodes: Query<(&MiniImage, &RelativeCursorPosition)>,
    owner: NonSend<HostOwner>,
    guard: super::super::modal::ModalGuard,
    mut editor: ResMut<Editor>,
) {
    if event.button != bevy::picking::pointer::PointerButton::Secondary
        || owner.modal_active()
        || !guard.allows(event.entity)
    {
        return;
    }
    let Ok((node, cursor)) = nodes.get(event.entity) else {
        return;
    };
    let Some(point) = image_position(cursor) else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    if node.0 != owner.generation() || play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let (x, y, floor, zoom, _) = play.view.minimap_camera();
    let (Some(x), Some(y)) = (
        x.checked_add(((point.x * 31.) as i32 - 15) * zoom),
        y.checked_add(((point.y * 25.) as i32 - 12) * zoom),
    ) else {
        return;
    };
    if !(0..=65535).contains(&x) || !(0..=65535).contains(&y) {
        return;
    }
    editor.open((owner.generation(), (x, y, floor)));
}
impl Editor {
    fn open(&mut self, key: MarkerKey) {
        self.at = Some(key);
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut editor: ResMut<Editor>,
    mut focus: ResMut<InputFocus>,
    mut seen: Local<Option<MarkerKey>>,
) {
    if editor.at.is_some_and(|key| key.0 != owner.generation()) || owner.play.is_none() {
        editor.at = None;
    }
    if let Some(at) = owner
        .play
        .as_ref()
        .and_then(|play| play.view.take_minimap_marker_edit())
    {
        editor.open((owner.generation(), at));
    }
    if *seen == editor.at {
        return;
    }
    if let Some(root) = editor.root.take() {
        commands.entity(root).despawn();
        focus.clear();
    }
    *seen = editor.at;
    let Some(key) = editor.at else {
        return;
    };
    let root = commands
        .spawn((
            super::super::GameModal,
            GlobalZIndex(180),
            Node {
                position_type: PositionType::Absolute,
                left: percent(35.),
                top: percent(30.),
                width: px(320.),
                padding: px(12.).all(),
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
        ))
        .id();
    let caption = commands
        .spawn(source::button_caption(
            format!("Map mark: {}, {}, floor {}", key.1.0, key.1.1, key.1.2),
            skin.font.clone(),
        ))
        .id();
    commands.entity(root).add_child(caption);
    let mut label = String::new();
    let mut existing = false;
    editor.color = 0;
    if let Some(play) = &owner.play {
        play.view.satellite_map(|_, markers| {
            if let Some(mark) = markers.iter().find(|m| (m.x, m.y, m.floor) == key.1) {
                existing = true;
                label.clone_from(&mark.label);
                editor.color = mark.color;
            }
        });
        if !existing && let Some(mark) = play.view.remembered_minimap_mark(key.1) {
            existing = true;
            label.clone_from(&mark.label);
        }
    }
    if label.is_empty() {
        label = "Mark".into();
    }
    let mut text = EditableText::new(label);
    text.max_characters = Some(128);
    let field = commands
        .spawn((
            Field,
            TextInput,
            text,
            source::font(skin.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            bevy::text::TextCursorStyle::default(),
            bevy::input_focus::tab_navigation::TabIndex(0),
            Node {
                min_height: px(25.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(field);
    focus.set(field, bevy::input_focus::FocusCause::Navigated);
    for (action, label) in [
        (Action::Save, "Save mark"),
        (Action::Remove, "Remove mark"),
        (Action::Cancel, "Cancel"),
    ] {
        if matches!(action, Action::Remove) && !existing {
            continue;
        }
        let button = commands
            .spawn((
                source::button(skin.buttons.clone(), 110.),
                Control { key, action },
            ))
            .observe(activate)
            .id();
        let text = commands
            .spawn(source::button_caption(label, skin.font.clone()))
            .id();
        commands.entity(button).add_child(text);
        commands.entity(root).add_child(button);
    }
    editor.root = Some(root);
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: super::super::modal::ModalGuard,
    mut editor: ResMut<Editor>,
    mut owner: NonSendMut<HostOwner>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity) || editor.at != Some(control.key) {
        return;
    }
    match control.action {
        Action::Save => editor.pending = Some(event.entity),
        Action::Cancel => {
            editor.at = None;
            editor.pending = None;
        }
        Action::Remove => {
            let (generation, at) = control.key;
            if owner.generation() != generation || owner.modal_active() {
                return;
            }
            let owner = &mut *owner;
            let Some(play) = &mut owner.play else { return };
            if play.view.gameplay_input_blocked(&play.link) {
                return;
            }
            let world_mark = play.view.satellite_map(|_, marks| {
                marks.iter().any(|mark| (mark.x, mark.y, mark.floor) == at)
            });
            if world_mark {
                if play.view.remove_minimap_marker(at).is_err() {
                    return;
                }
            } else if let Some(mark) = play.view.remembered_minimap_mark(at) {
                let mut preferences = owner.settings.preferences.clone();
                if preferences.minimap.binding == play.view.preferences().minimap.binding
                    && let Some(index) = preferences
                        .minimap
                        .marks
                        .iter()
                        .position(|saved| saved == mark)
                {
                    preferences.minimap.remove_mark(index);
                    owner.settings.apply(preferences);
                    if let Err(error) = owner.settings.capture(&mut play.view) {
                        owner.status = error.to_string();
                    }
                }
            }
            editor.at = None;
            editor.pending = None;
        }
    }
}
fn finish(
    mut owner: NonSendMut<HostOwner>,
    mut editor: ResMut<Editor>,
    fields: Query<&EditableText, With<Field>>,
    guard: super::super::modal::ModalGuard,
) {
    let (Some(entity), Some((generation, at))) = (editor.pending, editor.at) else {
        return;
    };
    if generation != owner.generation() || owner.modal_active() || !guard.allows(entity) {
        editor.pending = None;
        return;
    }
    let Ok(text) = fields.single() else {
        editor.pending = None;
        return;
    };
    if text.pending_paste.is_some()
        || text
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
    {
        return;
    }
    editor.pending = None;
    let Some(play) = &mut owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    if play
        .view
        .edit_minimap_marker(at, text.value().to_string(), editor.color)
        .is_ok()
    {
        editor.at = None;
    }
}
