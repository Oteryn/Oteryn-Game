//! Portable presentation over the retained local minimap owner and its persistence.
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_local_minimap::state::{
    Marker,
    editor::{Action, Snapshot},
};
use bevy::{
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui_widgets::{Activate, TextInput},
};
use oteryn_client_ui_bevy as source;
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(u64, Snapshot)>,
    root: Option<Entity>,
    draft: Option<Draft>,
    confirm: Option<Draft>,
    error: String,
    rebuild: bool,
}
#[derive(Clone, PartialEq, Eq)]
struct Draft {
    fields: [String; 4],
    color: u8,
    selected: Option<Marker>,
}
impl Draft {
    fn at(position: (i32, i32, i16)) -> Self {
        Self {
            fields: [
                position.0.to_string(),
                position.1.to_string(),
                position.2.to_string(),
                String::new(),
            ],
            color: 0,
            selected: None,
        }
    }
    fn marker(&self) -> Option<Marker> {
        Some(Marker {
            x: self.fields[0].parse().ok()?,
            y: self.fields[1].parse().ok()?,
            floor: self.fields[2].parse().ok()?,
            label: self.fields[3].clone(),
            color: self.color,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Close,
    ZoomIn,
    ZoomOut,
    FloorUp,
    FloorDown,
    Recenter,
    West,
    East,
    North,
    South,
    Centre,
    Save,
    Delete,
    ConfirmDelete,
    GoTo,
    Select(usize),
    Color(u8),
}
#[derive(Component)]
struct Control {
    generation: u64,
    expected: Snapshot,
    kind: Kind,
}
#[derive(Component)]
struct Field(usize);
#[derive(Component)]
struct Confirmation;
#[derive(Component)]
struct Feedback;
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(
            PostUpdate,
            read_draft.after(bevy::text::EditableTextSystems),
        );
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if let Some(play) = &owner.play {
        play.view
            .minimap_editor_open(&play.link, !owner.modal_active());
    }
}
fn panel_text(panel: &Snapshot) -> String {
    format!(
        "Map centre: {}, {} · Floor {} · {} tiles per cell\nGreen: remembered ground. Select a marker to edit its label.",
        panel.centre.0, panel.centre.1, panel.centre.2, panel.zoom
    )
}
fn label(commands: &mut Commands, skins: &Skins, root: Entity, text: &str) -> Entity {
    let child = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(root).add_child(child);
    child
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    root: Entity,
    binding: &(u64, Snapshot),
    kind: Kind,
    caption: &str,
) {
    let child = commands
        .spawn((
            Control {
                generation: binding.0,
                expected: binding.1.clone(),
                kind,
            },
            source::button(skins.buttons.clone(), 240.),
        ))
        .observe(activate)
        .id();
    label(commands, skins, child, caption);
    commands.entity(root).add_child(child);
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
) {
    let current = owner
        .play
        .as_ref()
        .and_then(|p| p.view.minimap_editor_snapshot())
        .map(|p| (owner.generation(), p));
    if state.snapshot == current && !state.rebuild {
        return;
    }
    state.rebuild = false;
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    // Drafts survive owner projections, but never cross a close or session replacement.
    if current.is_none()
        || state.snapshot.as_ref().is_none_or(|old| {
            current
                .as_ref()
                .is_none_or(|new| old.0 != new.0 || !old.1.same_scope(&new.1))
        })
    {
        state.draft = None;
    }
    if state.snapshot != current {
        state.confirm = None;
    }
    state.snapshot = current.clone();
    let Some(binding) = current else {
        return;
    };
    let panel = &binding.1;
    let draft = state
        .draft
        .get_or_insert_with(|| Draft::at(panel.own))
        .clone();
    let root = commands
        .spawn((
            GameModal,
            Name::new("minimap_markers.window"),
            GlobalZIndex(120),
            bevy::ui_widgets::ScrollArea,
            bevy::input_focus::tab_navigation::TabGroup::new(16),
            source::frame(skins.frame.clone(), [2., 17., 2., 2.]),
            Node {
                position_type: PositionType::Absolute,
                left: px(180.),
                top: px(45.),
                width: px(540.),
                max_height: percent(90.),
                overflow: Overflow::scroll_y(),
                padding: px(20.).all(),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                ..default()
            },
        ))
        .id();
    state.root = Some(root);
    source::dialog_caption(
        &mut commands,
        root,
        "Minimap and markers — Ctrl+Alt+Shift+M",
        skins.font.clone(),
    );
    label(&mut commands, &skins, root, &panel_text(panel));
    for (kind, caption) in [
        (Kind::ZoomIn, "Zoom in"),
        (Kind::ZoomOut, "Zoom out"),
        (Kind::FloorUp, "Floor up"),
        (Kind::FloorDown, "Floor down"),
        (Kind::Recenter, "Recenter"),
        (Kind::West, "West"),
        (Kind::East, "East"),
        (Kind::North, "North"),
        (Kind::South, "South"),
        (Kind::Centre, "Use map centre"),
    ] {
        button(&mut commands, &skins, root, &binding, kind, caption);
    }
    for (index, marker) in panel.markers.iter().enumerate() {
        button(
            &mut commands,
            &skins,
            root,
            &binding,
            Kind::Select(index),
            &format!(
                "{} · {}, {} · floor {}",
                marker.label, marker.x, marker.y, marker.floor
            ),
        );
    }
    for (index, caption) in ["X", "Y", "Floor", "Label"].into_iter().enumerate() {
        label(&mut commands, &skins, root, caption);
        let mut editable = EditableText::new(draft.fields[index].clone());
        editable.max_characters = Some(if index == 3 { 128 } else { 8 });
        editable.allow_newlines = false;
        let field = commands
            .spawn((
                Field(index),
                TextInput,
                editable,
                source::font(skins.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::default(),
                TextCursorStyle::default(),
                bevy::input_focus::tab_navigation::TabIndex(0),
                Node {
                    min_height: px(24.),
                    ..default()
                },
            ))
            .id();
        commands.entity(root).add_child(field);
    }
    for (color, caption) in ["Red", "Blue", "Green", "Gold"].into_iter().enumerate() {
        button(
            &mut commands,
            &skins,
            root,
            &binding,
            Kind::Color(color as u8),
            &format!(
                "{}{}",
                if draft.color == color as u8 {
                    "✓ "
                } else {
                    ""
                },
                caption
            ),
        );
    }
    for (kind, caption) in [
        (Kind::Save, "Save marker"),
        (Kind::Delete, "Delete selected"),
        (Kind::GoTo, "Go to selected"),
        (Kind::Close, "Close"),
    ] {
        button(&mut commands, &skins, root, &binding, kind, caption);
    }
    if state.confirm.is_some() {
        button(
            &mut commands,
            &skins,
            root,
            &binding,
            Kind::ConfirmDelete,
            "Confirm deletion",
        );
    }
    let confirm = label(&mut commands, &skins, root, "");
    commands.entity(confirm).insert(Confirmation);
    label(&mut commands, &skins, root, &panel.feedback);
    let feedback = label(&mut commands, &skins, root, &state.error);
    commands.entity(feedback).insert(Feedback);
}
fn fields_into(draft: &mut Draft, fields: &Query<(&Field, &EditableText)>) {
    for (field, text) in fields {
        draft.fields[field.0] = text.editor.text().to_string();
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if control.generation != owner.generation() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    // Session identity is the sole close gate, including behind a newer modal.
    if control.kind == Kind::Close {
        play.view
            .minimap_editor_bound(&play.link, &control.expected, Action::Close, false);
        return;
    }
    let admitted = guard.allows(event.entity) && !owner.modal_active();
    if !admitted
        || play.view.gameplay_input_blocked(&play.link)
        || play.view.minimap_editor_snapshot().as_ref() != Some(&control.expected)
    {
        return;
    }
    let Some(mut draft) = state.draft.clone() else {
        return;
    };
    if !state.rebuild {
        fields_into(&mut draft, &fields);
    }
    state.error.clear();
    if state
        .confirm
        .as_ref()
        .is_some_and(|confirmed| *confirmed != draft)
    {
        state.confirm = None;
    }
    let action = match control.kind {
        Kind::Close => return,
        Kind::Select(index) => {
            let Some(marker) = control.expected.markers.get(index) else {
                return;
            };
            draft.fields = [
                marker.x.to_string(),
                marker.y.to_string(),
                marker.floor.to_string(),
                marker.label.clone(),
            ];
            draft.color = marker.color;
            draft.selected = Some(marker.clone());
            None
        }
        Kind::Color(color) => {
            draft.color = color;
            None
        }
        Kind::Centre => {
            draft.fields[0] = control.expected.centre.0.to_string();
            draft.fields[1] = control.expected.centre.1.to_string();
            draft.fields[2] = control.expected.centre.2.to_string();
            None
        }
        Kind::Save => match draft.marker() {
            Some(marker) => Some(Action::Save(marker)),
            None => {
                state.error = "Enter valid X, Y and floor values".into();
                None
            }
        },
        Kind::Delete => {
            if draft.selected.is_none() {
                return;
            }
            state.confirm = Some(draft.clone());
            state.draft = Some(draft);
            state.rebuild = true;
            return;
        }
        Kind::ConfirmDelete => {
            if state.confirm.as_ref() != Some(&draft) {
                return;
            }
            let Some(marker) = draft.selected.clone() else {
                return;
            };
            Some(Action::Delete(marker))
        }
        Kind::GoTo => draft.selected.clone().map(Action::GoTo),
        Kind::ZoomIn => Some(Action::ZoomIn),
        Kind::ZoomOut => Some(Action::ZoomOut),
        Kind::FloorUp => Some(Action::FloorUp),
        Kind::FloorDown => Some(Action::FloorDown),
        Kind::Recenter => Some(Action::Recenter),
        Kind::West => Some(Action::Pan(-5, 0)),
        Kind::East => Some(Action::Pan(5, 0)),
        Kind::North => Some(Action::Pan(0, -5)),
        Kind::South => Some(Action::Pan(0, 5)),
    };
    state.confirm = None;
    state.draft = Some(draft);
    if let Some(action) = action {
        play.view
            .minimap_editor_bound(&play.link, &control.expected, action, admitted);
    } else {
        state.rebuild = true;
    }
}
fn read_draft(
    fields: Query<(&Field, &EditableText)>,
    mut state: ResMut<State>,
    mut confirmations: Query<&mut Text, With<Confirmation>>,
    mut feedback: Query<&mut Text, (With<Feedback>, Without<Confirmation>)>,
) {
    if !state.rebuild
        && let Some(draft) = &mut state.draft
    {
        fields_into(draft, &fields);
    }
    if state
        .confirm
        .as_ref()
        .is_some_and(|confirm| Some(confirm) != state.draft.as_ref())
    {
        state.confirm = None;
        state.rebuild = true;
    }
    let text = state
        .confirm
        .as_ref()
        .and_then(|d| d.selected.as_ref())
        .map_or_else(String::new, |m| {
            format!(
                "Delete \"{}\" at {}, {}, floor {}? Confirm deletion to continue.",
                m.label, m.x, m.y, m.floor
            )
        });
    for mut label in &mut confirmations {
        label.0.clone_from(&text);
    }
    for mut label in &mut feedback {
        label.0.clone_from(&state.error);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    if events.read().any(|e| {
        e.key_code == KeyCode::Escape && e.state == bevy::input::ButtonState::Pressed && !e.repeat
    }) && state.root.is_some()
        && let Some((generation, expected)) = &state.snapshot
        && *generation == owner.generation()
        && let Some(play) = &owner.play
    {
        play.view
            .minimap_editor_bound(&play.link, expected, Action::Close, false);
    }
}
#[cfg(test)]
mod tests;
