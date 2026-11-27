//! A local marker editor bound to the exact viewed minimap coordinate.
use super::*;
use bevy::{
    input_focus::InputFocus,
    text::EditableText,
    ui_widgets::{Activate, TextInput},
};
pub(super) type MarkerKey = (u64, (i32, i32, i16), u64);
#[derive(Resource, Default)]
pub(super) struct Editor {
    at: Option<MarkerKey>,
    root: Option<(MarkerKey, Entity)>,
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
    Icon(u8),
    Save,
    Remove,
    Cancel,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<Editor>()
        .add_systems(Update, project)
        .add_systems(
            PostUpdate,
            finish
                .after(bevy::text::EditableTextSystems)
                .before(bevy::ui::UiSystems::Stack),
        );
}
impl Editor {
    pub(super) fn open(&mut self, key: MarkerKey) {
        self.at = Some(key);
    }

    fn despawn(&mut self, commands: &mut Commands, focus: &mut InputFocus) {
        if let Some((_, root)) = self.root.take() {
            commands.entity(root).despawn();
            focus.clear();
        }
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut editor: ResMut<Editor>,
    mut focus: ResMut<InputFocus>,
) {
    let binding = owner
        .play
        .as_ref()
        .map(|play| play.view.minimap_binding_generation());
    if editor
        .at
        .is_some_and(|key| key.0 != owner.generation() || Some(key.2) != binding)
    {
        editor.at = None;
        editor.pending = None;
    }
    if let Some(play) = &owner.play
        && let Some(at) = play.view.take_minimap_marker_edit()
    {
        editor.open((
            owner.generation(),
            at,
            play.view.minimap_binding_generation(),
        ));
    }
    if editor.root.map(|(key, _)| key) == editor.at {
        return;
    }
    editor.despawn(&mut commands, &mut focus);
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
    if !existing {
        label = "Mark".into();
    }
    let icons = commands
        .spawn(Node {
            column_gap: px(6.),
            ..default()
        })
        .id();
    commands.entity(root).add_child(icons);
    use crate::play::feature_local_minimap::view::MARKER_COLORS;
    for (icon, (palette, name)) in MARKER_COLORS
        .into_iter()
        .zip(["Red", "Blue", "Green", "Yellow"])
        .enumerate()
    {
        let icon = icon as u8;
        let choice = commands
            .spawn((
                Name::new(format!("{name} mark icon")),
                bevy::ui_widgets::Button,
                Node {
                    width: px(26.),
                    height: px(26.),
                    padding: px(3.).all(),
                    border: px(2.).all(),
                    ..default()
                },
                icon_border(icon == editor.color),
                Control {
                    key,
                    action: Action::Icon(icon),
                },
            ))
            .observe(activate)
            .id();
        let swatch = commands
            .spawn((
                Node {
                    flex_grow: 1.,
                    ..default()
                },
                BackgroundColor(palette_color(palette)),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(choice).add_child(swatch);
        commands.entity(icons).add_child(choice);
    }
    let description = commands
        .spawn(source::button_caption("Description", skin.font.clone()))
        .id();
    commands.entity(root).add_child(description);
    let mut text = EditableText::new(label);
    text.max_characters = Some(128);
    let field = commands
        .spawn((
            Field,
            Name::new("Mark description"),
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
    for (action, name, label) in [
        (Action::Save, "Save mark", "Save mark"),
        (Action::Remove, "Remove mark", "Remove mark"),
        (Action::Cancel, "Cancel mark", "Cancel"),
    ] {
        if matches!(action, Action::Remove) && !existing {
            continue;
        }
        let button = commands
            .spawn((
                source::button(skin.buttons.clone(), 110.),
                Name::new(name),
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
    editor.root = Some((key, root));
}
fn icon_border(selected: bool) -> BorderColor {
    BorderColor::all(if selected { Color::WHITE } else { Color::BLACK })
}

fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    mut borders: Query<(&Control, &mut BorderColor)>,
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
        Action::Icon(selected) => {
            editor.color = selected;
            for (choice, mut border) in &mut borders {
                if choice.key == control.key
                    && let Action::Icon(icon) = choice.action
                {
                    *border = icon_border(icon == selected);
                }
            }
        }
        Action::Save => editor.pending = Some(event.entity),
        Action::Cancel => {
            editor.at = None;
            editor.pending = None;
        }
        Action::Remove => {
            let (generation, at, binding) = control.key;
            if owner.generation() != generation || owner.modal_active() {
                return;
            }
            let Some(play) = &owner.play else { return };
            if play.view.minimap_binding_generation() != binding
                || play.view.gameplay_input_blocked(&play.link)
                || !remove_mark(&mut owner, at)
            {
                return;
            }
            editor.at = None;
            editor.pending = None;
        }
    }
}
fn finish(
    mut commands: Commands,
    mut owner: NonSendMut<HostOwner>,
    mut editor: ResMut<Editor>,
    mut focus: ResMut<InputFocus>,
    fields: Query<&EditableText, With<Field>>,
    guard: super::super::modal::ModalGuard,
) {
    let (Some(entity), Some((generation, at, binding))) = (editor.pending, editor.at) else {
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
    let owner = &mut *owner;
    let Some(play) = &mut owner.play else {
        return;
    };
    if play.view.minimap_binding_generation() != binding
        || play.view.gameplay_input_blocked(&play.link)
    {
        return;
    }
    let label = text.value().to_string();
    if !has_world_mark(&play.view, at)
        && let Some(index) = remembered_index(&play.view, &owner.settings, at)
    {
        if let Err(error) = play.view.require_minimap_preferences_writer() {
            owner.status = error.to_string();
            return;
        }
        let mut preferences = owner.settings.preferences.clone();
        preferences.minimap.marks[index].label = label;
        if !preferences.minimap.valid() {
            owner.status = "Invalid marker: use a label of up to 64 bytes and a valid floor".into();
            return;
        }
        owner
            .settings
            .apply_minimap_marks_in_background(preferences);
        match play
            .view
            .set_preferences(owner.settings.preferences.clone())
        {
            Ok(()) => editor.at = None,
            Err(error) => owner.status = error.to_string(),
        }
    } else if play
        .view
        .edit_minimap_marker(at, label, editor.color)
        .is_ok()
    {
        editor.at = None;
    }
    if editor.at.is_none() {
        editor.despawn(&mut commands, &mut focus);
    }
}

pub(super) fn remove_mark(owner: &mut HostOwner, at: (i32, i32, i16)) -> bool {
    let Some(play) = &mut owner.play else {
        return false;
    };
    if has_world_mark(&play.view, at) {
        if play.view.remove_minimap_marker(at).is_err() {
            return false;
        }
    } else if let Some(index) = remembered_index(&play.view, &owner.settings, at) {
        if let Err(error) = play.view.require_minimap_preferences_writer() {
            owner.status = error.to_string();
            return false;
        }
        let mut preferences = owner.settings.preferences.clone();
        preferences.minimap.remove_mark(index);
        owner
            .settings
            .apply_minimap_marks_in_background(preferences);
        if let Err(error) = play
            .view
            .set_preferences(owner.settings.preferences.clone())
        {
            owner.status = error.to_string();
        }
    }
    true
}

pub(super) fn has_world_mark(view: &crate::play::PlayView, at: (i32, i32, i16)) -> bool {
    view.satellite_map(|_, marks| marks.iter().any(|mark| (mark.x, mark.y, mark.floor) == at))
}

fn remembered_index(
    view: &crate::play::PlayView,
    settings: &crate::play::SettingsState,
    at: (i32, i32, i16),
) -> Option<usize> {
    let mark = view.remembered_minimap_mark(at)?;
    (settings.preferences.minimap.binding == view.preferences().minimap.binding)
        .then(|| {
            settings
                .preferences
                .minimap
                .marks
                .iter()
                .position(|saved| saved == mark)
        })
        .flatten()
}
