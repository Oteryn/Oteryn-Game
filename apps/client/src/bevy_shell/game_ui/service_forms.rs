//! Shared presentations use the same forms and authoritative request owners as native UI.
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0115_mail::view::forms::{self, Form, Snapshot};
use bevy::{
    input::keyboard::KeyboardInput,
    input_focus::{
        InputFocus,
        tab_navigation::{TabGroup, TabIndex},
    },
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui::InteractionDisabled,
    ui_widgets::{Activate, ScrollArea, TextInput},
};
use oteryn_client_ui_bevy as source;
#[derive(Component)]
struct Field(usize);
#[derive(Component)]
struct Control {
    generation: u64,
    root: Entity,
    action: Option<usize>,
}
#[derive(Resource, Default)]
struct State {
    generation: u64,
    form: Option<Form>,
    snapshot: Option<Snapshot>,
    root: Option<Entity>,
    request: Option<usize>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}
fn keyboard(
    mut events: MessageReader<KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    if events.read().any(|e| {
        e.key_code == KeyCode::Escape && e.state == bevy::input::ButtonState::Pressed && !e.repeat
    }) && state.root.is_some_and(|root| guard.allows(root))
        && let (Some(form), Some(play)) = (state.form, &owner.play)
    {
        form.close(&play.link);
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    fields: Query<(), With<Field>>,
) {
    let current = owner.play.as_ref().and_then(|play| {
        forms::all().into_iter().find_map(|form| {
            let snapshot = form.read(&play.link);
            snapshot.open.then_some((form, snapshot))
        })
    });
    if state.generation == owner.generation()
        && current.as_ref().map(|(f, s)| (f.title, s))
            == state
                .form
                .as_ref()
                .zip(state.snapshot.as_ref())
                .map(|(f, s)| (f.title, s))
    {
        return;
    }
    if let Some(root) = state.root.take() {
        if focus.get().is_some_and(|entity| fields.contains(entity)) {
            focus.clear();
        }
        commands.entity(root).despawn();
    }
    state.form = None;
    state.snapshot = None;
    state.request = None;
    state.generation = owner.generation();
    if let Some((form, snapshot)) = current {
        state.root = Some(build(
            &mut commands,
            &skin,
            owner.generation(),
            form,
            &snapshot,
        ));
        state.form = Some(form);
        state.snapshot = Some(snapshot);
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Ok(c) = controls.get(event.entity) else {
        return;
    };
    if c.generation != owner.generation()
        || state.root != Some(c.root)
        || !guard.allows(event.entity)
        || owner.modal_active()
    {
        return;
    }
    if let Some(action) = c.action {
        state.request = Some(action);
    } else if let (Some(form), Some(play)) = (state.form, &owner.play) {
        form.close(&play.link);
    }
}
fn finish(
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    mut state: ResMut<State>,
    fields: Query<(&Field, &EditableText)>,
) {
    if state.request.is_none()
        || fields.iter().any(|(_, text)| {
            text.pending_paste.is_some()
                || text
                    .pending_edits
                    .iter()
                    .any(bevy::text::TextEdit::is_destructive)
        })
    {
        return;
    }
    let Some(action) = state.request.take() else {
        return;
    };
    if state.generation != owner.generation()
        || owner.modal_active()
        || state.root.is_none_or(|root| !guard.allows(root))
    {
        return;
    }
    let (Some(play), Some(form), Some(snapshot)) = (&owner.play, state.form, &state.snapshot)
    else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let mut values = snapshot.fields.clone();
    for (field, text) in &fields {
        if let Some(value) = values.get_mut(field.0) {
            *value = text.value().to_string();
        }
    }
    form.dispatch(&play.link, snapshot, action, values);
}
fn label(commands: &mut Commands, skin: &Skins, parent: Entity, value: &str) {
    let entity = commands
        .spawn(source::text(value, source::font(skin.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(entity);
}
fn build(
    commands: &mut Commands,
    skin: &Skins,
    generation: u64,
    form: Form,
    snapshot: &Snapshot,
) -> Entity {
    let root = commands
        .spawn((
            GameModal,
            GlobalZIndex(166),
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
            BackgroundColor(Color::srgba(0., 0., 0., 0.5)),
        ))
        .id();
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(660.),
                max_width: percent(96.),
                max_height: percent(94.),
                padding: UiRect {
                    top: px(24.),
                    bottom: px(12.),
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
    source::dialog_caption(commands, panel, form.title, skin.font.clone());
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
    label(commands, skin, content, &snapshot.text);
    if let Some(drawing) = &snapshot.drawing {
        canvas(commands, skin, content, drawing);
    }
    for (i, prompt) in form.labels.iter().enumerate() {
        label(commands, skin, content, prompt);
        let mut text = EditableText::new(snapshot.fields.get(i).map_or("", String::as_str));
        text.allow_newlines = true;
        text.max_characters = Some(8192);
        let field = commands
            .spawn((
                Field(i),
                TextInput,
                text,
                TabIndex(i as i32),
                source::font(skin.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::default(),
                TextCursorStyle::default(),
                Node {
                    width: percent(100.),
                    min_height: px(26.),
                    padding: px(4.).all(),
                    flex_shrink: 0.,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
            ))
            .id();
        if snapshot.pending {
            commands.entity(field).insert(InteractionDisabled);
        }
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
    commands.entity(content).add_child(footer);
    for (action, caption) in form
        .buttons
        .iter()
        .enumerate()
        .map(|(i, t)| (Some(i), *t))
        .chain(std::iter::once((None, "Close")))
    {
        let button = commands
            .spawn((
                source::button(skin.buttons.clone(), 155.),
                Control {
                    generation,
                    root,
                    action,
                },
            ))
            .observe(activate)
            .id();
        if snapshot.pending && action.is_some() {
            commands.entity(button).insert(InteractionDisabled);
        }
        let caption = commands
            .spawn(source::button_caption(caption, skin.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(footer).add_child(button);
    }
    root
}
fn canvas(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    drawing: &oteryn_client_ui::cyclopedia_map::atlas_view::Drawing,
) {
    label(commands, skin, parent, &drawing.caption);
    let canvas = commands
        .spawn((
            Node {
                width: px(640.),
                height: px(512.),
                flex_shrink: 0.,
                position_type: PositionType::Relative,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.08, 0.08)),
        ))
        .id();
    commands.entity(parent).add_child(canvas);
    for rect in &drawing.rects {
        let [x, y, w, h] = rect.bounds;
        let colors = [
            Color::srgb(0.47, 0.39, 0.3),
            Color::srgb(0.44, 0.42, 0.27),
            Color::srgb(0.4, 0.47, 0.28),
            Color::srgb(0.51, 0.46, 0.31),
            Color::srgb(0.43, 0.38, 0.33),
            Color::srgb(0.48, 0.42, 0.29),
        ];
        let entity = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(x as f32),
                    top: px(y as f32),
                    width: px(w as f32),
                    height: px(h as f32),
                    ..default()
                },
                BackgroundColor(colors[usize::from(rect.color).min(colors.len() - 1)]),
            ))
            .id();
        commands.entity(canvas).add_child(entity);
    }
    if let Some([x, y]) = drawing.player {
        for [dx, dy, w, h] in [[-5., -1., 11., 3.], [-1., -5., 3., 11.]] {
            let entity = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(x as f32 + dx),
                        top: px(y as f32 + dy),
                        width: px(w),
                        height: px(h),
                        ..default()
                    },
                    BackgroundColor(Color::WHITE),
                ))
                .id();
            commands.entity(canvas).add_child(entity);
        }
    }
    for [x, y] in &drawing.points {
        let entity = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(*x as f32),
                    top: px(*y as f32),
                    width: px(5.),
                    height: px(5.),
                    ..default()
                },
                BackgroundColor(Color::srgb(1., 0.9, 0.1)),
            ))
            .id();
        commands.entity(canvas).add_child(entity);
    }
}
