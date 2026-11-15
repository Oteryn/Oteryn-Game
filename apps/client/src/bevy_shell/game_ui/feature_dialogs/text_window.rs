//! House access lists use the retained catalogue, ACL reads and mutation intents.
use super::*;
use crate::play::PlayLink;
use bevy::{
    input::keyboard::KeyboardInput,
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui::house::Panel;
use oteryn_client_ui_bevy as source;

#[derive(Component)]
pub(super) struct Field(pub usize);
#[derive(Component)]
struct Control {
    generation: u64,
    root: Entity,
    selection: Option<usize>,
}
#[derive(Resource, Default)]
struct HouseWindow {
    generation: Option<u64>,
    open: bool,
    root: Option<Entity>,
    model: Option<Panel>,
    fields: [String; 7],
    request: Option<usize>,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<HouseWindow>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}
fn keyboard(
    mut events: MessageReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    focus: Res<InputFocus>,
    editors: Query<(), With<EditableText>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<HouseWindow>,
) {
    let mut close = false;
    let open = events.read().any(|event| {
        close |= event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat;
        event.key_code == KeyCode::KeyH
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    });
    if close && state.root.is_some_and(|root| guard.allows(root)) {
        state.open = false;
        state.request = None;
    }
    if open
        && keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
        && keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
        && !state.open
        && !owner.modal_active()
        && guard.allows_focus(focus.get())
        && focus.get().is_none_or(|entity| !editors.contains(entity))
        && owner
            .play
            .as_ref()
            .is_some_and(|play| !play.view.gameplay_input_blocked(&play.link))
    {
        state.generation = Some(owner.generation());
        state.open = true;
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<HouseWindow>,
    mut focus: ResMut<InputFocus>,
    fields: Query<(), With<Field>>,
) {
    if state.generation != Some(owner.generation()) || owner.play.is_none() {
        state.open = false;
    }
    let projected = owner
        .play
        .as_ref()
        .filter(|_| state.open)
        .map(|play| play.link.house_panel());
    if let Some(panel) = &projected
        && state.model.as_ref() == Some(&panel.model)
        && state.fields == panel.fields
    {
        return;
    }
    if let Some(root) = state.root.take() {
        if focus.get().is_some_and(|entity| fields.contains(entity)) {
            focus.clear();
        }
        commands.entity(root).despawn();
    }
    state.model = None;
    state.request = None;
    let Some(panel) = projected else {
        return;
    };
    let message = oteryn_client_ui::house::panel_text(&panel.model);
    state.root = Some(build(
        &mut commands,
        &skin,
        owner.generation(),
        &panel.fields,
        &message,
    ));
    state.fields = panel.fields;
    state.model = Some(panel.model);
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<HouseWindow>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity)
        || control.generation != owner.generation()
        || state.root != Some(control.root)
        || owner.modal_active()
    {
        return;
    }
    if let Some(selection) = control.selection {
        state.request = Some(selection);
    } else {
        state.open = false;
    }
}
fn finish(
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<HouseWindow>,
    fields: Query<(&Field, &EditableText)>,
) {
    let Some(selection) = state.request else {
        return;
    };
    if fields.iter().any(|(_, text)| {
        text.pending_paste.is_some()
            || text
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
    }) {
        return;
    }
    state.request = None;
    if !state.open
        || state.generation != Some(owner.generation())
        || owner.modal_active()
        || state.root.is_none_or(|root| !guard.allows(root))
    {
        return;
    }
    let (Some(play), Some(model)) = (&owner.play, &state.model) else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let mut values = state.fields.clone();
    for (field, text) in &fields {
        values[field.0] = text.value().to_string();
    }
    submit(&play.link, model, &state.fields, values, selection);
}
pub(super) fn submit(
    link: &PlayLink,
    expected: &Panel,
    expected_fields: &[String; 7],
    fields: [String; 7],
    selection: usize,
) {
    let current = link.house_panel();
    if current.model != *expected
        || current.fields != *expected_fields
        || current.model.pending.is_some()
    {
        return;
    }
    for (index, value) in fields.into_iter().enumerate() {
        link.house_edit(index, value);
    }
    link.house_request(selection);
}

pub(super) fn build(
    commands: &mut Commands,
    skin: &Skins,
    generation: u64,
    fields: &[String; 7],
    message: &str,
) -> Entity {
    let root = commands
        .spawn((
            GameModal,
            GlobalZIndex(165),
            TabGroup {
                order: 3,
                modal: true,
            },
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.55)),
        ))
        .id();
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(540.),
                max_width: percent(96.),
                max_height: percent(94.),
                padding: UiRect {
                    top: px(24.),
                    bottom: px(10.),
                    left: px(12.),
                    right: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(panel);
    source::dialog_caption(commands, panel, "Edit House List", skin.font.clone());
    let content = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                min_height: px(0.),
                flex_shrink: 1.,
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(content);
    label(commands, skin, content, message);
    for (index, prompt) in [
        (0, "World UUID"),
        (1, "Town key"),
        (2, "House row (1 onwards)"),
        (4, "Enter one name per line."),
        (5, "Door: x y floor"),
        (6, "Type CONFIRM for changes"),
    ] {
        label(commands, skin, content, prompt);
        if index == 4 {
            label(
                commands,
                skin,
                content,
                "This server requires member UUIDs; names cannot be resolved.",
            );
        }
        let mut editable = EditableText::new(&fields[index]);
        editable.allow_newlines = index == 4;
        editable.max_characters = Some(8192);
        editable.visible_lines = Some(if index == 4 { 4. } else { 1. });
        let field = commands
            .spawn((
                Field(index),
                TextInput,
                editable,
                TabIndex(index as i32),
                source::font(skin.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::default(),
                TextCursorStyle::default(),
                Node {
                    width: percent(100.),
                    min_height: px(26.),
                    padding: px(4.).all(),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
            ))
            .id();
        commands.entity(content).add_child(field);
    }
    let footer = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(5.),
            row_gap: px(5.),
            ..default()
        })
        .id();
    commands.entity(panel).add_child(footer);
    for (selection, caption) in [
        (Some(0), "Query"),
        (Some(1), "Next"),
        (Some(11), "Load guests"),
        (Some(12), "Load subowners"),
        (Some(13), "Load door"),
        (Some(3), "Save guests"),
        (Some(4), "Save subowners"),
        (Some(5), "Save door"),
        (None, "Close"),
    ] {
        let button = commands
            .spawn((
                source::button(skin.buttons.clone(), 120.),
                Control {
                    generation,
                    root,
                    selection,
                },
            ))
            .observe(activate)
            .id();
        let text = commands
            .spawn(source::button_caption(caption, skin.font.clone()))
            .id();
        commands.entity(button).add_child(text);
        commands.entity(footer).add_child(button);
    }
    root
}
fn label(commands: &mut Commands, skin: &Skins, parent: Entity, text: &str) {
    let entity = commands
        .spawn(source::text(text, source::font(skin.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(entity);
}
