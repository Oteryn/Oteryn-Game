//! Pre-admission account screens over the authenticated controllers: Character Bazaar,
//! world transfer, coin packages and the Premium overview.
#[cfg(test)]
mod bazaar_tests;
mod bridge;
mod screens;
#[cfg(test)]
mod tests;
use super::{
    game_ui::{GameModal, modal::ModalGuard},
    owner::HostOwner,
    ui::Skins,
};
use bevy::{
    input_focus::{
        FocusCause, InputFocus,
        tab_navigation::{TabGroup, TabIndex},
    },
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui::InteractionDisabled,
    ui_widgets::{Activate, ScrollArea, TextInput},
};
use bridge::Reply;
pub(super) use bridge::Services;
use oteryn_client_ui_bevy as source;

const ROW: Color = Color::srgba(0., 0., 0., 0.);
const CHOSEN: Color = Color::srgb(0.25, 0.25, 0.32);

#[derive(Component)]
struct Root(Option<u64>);
#[derive(Component)]
struct Body;
#[derive(Component)]
struct Control {
    request: u64,
    button: &'static str,
}
#[derive(Component)]
struct Row {
    request: u64,
    list: usize,
    index: usize,
}
#[derive(Component)]
struct Input(usize);
/// Every widget of a shown screen, so focus is released with it.
type Widgets = Or<(With<Control>, With<Row>, With<Input>)>;
/// The rows the player chose on the shown screen, and a press waiting for typed text to settle.
#[derive(Resource, Default)]
struct Choices {
    request: Option<u64>,
    selected: Vec<Option<usize>>,
    press: Option<(u64, &'static str)>,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<Choices>()
        .add_systems(Startup, setup.after(super::ui::setup))
        .add_systems(PreUpdate, poll.before(super::poll_owner))
        .add_systems(Update, (project, keyboard).chain())
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}
fn setup(mut commands: Commands) {
    commands.spawn((
        Root(None),
        GameModal,
        GlobalZIndex(210),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100.),
            height: percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0., 0., 0., 0.7)),
    ));
}
fn poll(mut owner: NonSendMut<HostOwner>) {
    owner.services.poll();
}
// The screen is rebuilt from independent widget projections and focus state.
#[allow(clippy::too_many_arguments)]
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut roots: Query<(Entity, &mut Node, &mut Root)>,
    mut texts: Query<&mut Text, With<Body>>,
    mut focus: ResMut<InputFocus>,
    mut choices: ResMut<Choices>,
    owned: Query<(), Widgets>,
) {
    let Some(request) = owner.services.pending() else {
        if focus.get().is_some_and(|e| owned.contains(e)) {
            focus.clear();
        }
        for (root, mut node, mut rendered) in &mut roots {
            node.display = Display::None;
            if rendered.0.is_some() {
                commands.entity(root).despawn_children();
            }
            rendered.0 = None;
        }
        *choices = Choices::default();
        return;
    };
    let screen = &request.screen;
    let text = screen.text();
    for mut body in &mut texts {
        if body.0 != text {
            body.0.clone_from(&text);
        }
    }
    for (root, mut node, mut rendered) in &mut roots {
        node.display = Display::Flex;
        if rendered.0 == Some(request.id) {
            continue;
        }
        commands.entity(root).despawn_children();
        *choices = Choices {
            request: Some(request.id),
            selected: screen.lists.iter().map(|l| l.selected).collect(),
            press: None,
        };
        let panel = commands
            .spawn((
                super::window_drag::DraggableDialog,
                TabGroup {
                    order: 6,
                    modal: true,
                },
                source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
                Node {
                    width: px(if screen.lists.is_empty() { 620. } else { 900. }),
                    max_width: percent(95.),
                    max_height: percent(90.),
                    padding: UiRect {
                        top: px(29.),
                        bottom: px(16.),
                        left: px(16.),
                        right: px(16.),
                    },
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8.),
                    ..default()
                },
            ))
            .id();
        commands.entity(root).add_child(panel);
        source::dialog_caption(&mut commands, panel, screen.title, skin.font.clone());
        let scroll = commands
            .spawn((
                ScrollArea,
                Node {
                    min_height: px(0.),
                    flex_shrink: 1.,
                    overflow: Overflow::scroll_y(),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(6.),
                    ..default()
                },
            ))
            .id();
        commands.entity(panel).add_child(scroll);
        let font = source::font(skin.font.clone(), false);
        for (index, list) in screen.lists.iter().enumerate() {
            let label = commands.spawn(source::text(list.label, font.clone())).id();
            let rows = commands
                .spawn((
                    source::frame(skin.inset.clone(), [1.; 4]),
                    Node {
                        flex_direction: FlexDirection::Column,
                        max_height: px(150.),
                        overflow: Overflow::scroll_y(),
                        padding: px(2.).all(),
                        ..default()
                    },
                ))
                .id();
            commands.entity(scroll).add_children(&[label, rows]);
            for (at, value) in list.rows.iter().enumerate() {
                let row = commands
                    .spawn((
                        bevy::ui_widgets::Button,
                        Row {
                            request: request.id,
                            list: index,
                            index: at,
                        },
                        Node {
                            padding: UiRect::axes(px(4.), px(1.)),
                            ..default()
                        },
                        BackgroundColor(if list.selected == Some(at) {
                            CHOSEN
                        } else {
                            ROW
                        }),
                    ))
                    .observe(select)
                    .id();
                let caption = commands
                    .spawn(source::text(value.clone(), font.clone()))
                    .id();
                commands.entity(row).add_child(caption);
                commands.entity(rows).add_child(row);
            }
        }
        for (index, field) in screen.fields.iter().enumerate() {
            let label = commands.spawn(source::text(field.label, font.clone())).id();
            let mut editable = EditableText::new(field.value.as_str());
            editable.max_characters = Some(32);
            editable.visible_lines = Some(1.);
            let input = commands
                .spawn((
                    Input(index),
                    TextInput,
                    editable,
                    TabIndex(i32::try_from(index).unwrap_or(i32::MAX)),
                    font.clone(),
                    TextColor(source::TEXT_COLOR),
                    TextLayout::no_wrap(),
                    TextCursorStyle::default(),
                    Node {
                        width: px(260.),
                        min_height: px(25.),
                        padding: px(4.).all(),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                ))
                .id();
            commands.entity(scroll).add_children(&[label, input]);
        }
        let body = commands
            .spawn((Body, source::text(text.clone(), font.clone())))
            .id();
        commands.entity(scroll).add_child(body);
        let footer = commands
            .spawn(Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(8.),
                row_gap: px(6.),
                justify_content: JustifyContent::End,
                ..default()
            })
            .id();
        for entry in &screen.buttons {
            let button = commands
                .spawn((
                    source::button(skin.buttons.clone(), 150.),
                    Control {
                        request: request.id,
                        button: entry.id,
                    },
                ))
                .observe(activate)
                .id();
            if !entry.enabled {
                commands.entity(button).insert(InteractionDisabled);
            }
            let caption = commands
                .spawn(source::button_caption(entry.label, skin.font.clone()))
                .id();
            commands.entity(button).add_child(caption);
            commands.entity(footer).add_child(button);
            if entry.id == screen.escape {
                focus.set(button, FocusCause::Navigated);
            }
        }
        commands.entity(panel).add_child(footer);
        rendered.0 = Some(request.id);
    }
}
fn select(
    event: On<Activate>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut choices: ResMut<Choices>,
    mut rows: Query<(Entity, &Row, &mut BackgroundColor)>,
    mut inputs: Query<(&Input, &mut EditableText)>,
) {
    let Ok((_, row, _)) = rows.get(event.entity) else {
        return;
    };
    let (list, index) = (row.list, row.index);
    let Some(request) = owner.services.pending() else {
        return;
    };
    if !guard.allows(event.entity)
        || choices.request != Some(row.request)
        || request.id != row.request
    {
        return;
    }
    if let Some(at) = choices.selected.get_mut(list) {
        *at = Some(index);
    }
    for (entity, other, mut color) in &mut rows {
        if other.list == list {
            color.0 = if entity == event.entity { CHOSEN } else { ROW };
        }
    }
    if let Some((field, values)) = request.screen.lists.get(list).and_then(|l| l.fill.as_ref())
        && let Some(value) = values.get(index)
    {
        for (input, mut text) in &mut inputs {
            if input.0 == *field {
                text.pending_edits.clear();
                text.pending_paste = None;
                text.editor_mut().set_text(value);
            }
        }
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    mut choices: ResMut<Choices>,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
        && choices.request == Some(control.request)
    {
        choices.press = Some((control.request, control.button));
    }
}
/// Answers once the text editors settled this frame, with the values the player sees.
fn finish(
    mut owner: NonSendMut<HostOwner>,
    mut choices: ResMut<Choices>,
    inputs: Query<(&Input, &EditableText)>,
) {
    let Some((request, button)) = choices.press else {
        return;
    };
    if inputs.iter().any(|(_, text)| {
        text.pending_paste.is_some()
            || text
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
    }) {
        return;
    }
    choices.press = None;
    let Some(screen) = owner
        .services
        .pending()
        .filter(|r| r.id == request)
        .map(|r| &r.screen)
    else {
        return;
    };
    let mut fields: Vec<String> = screen.fields.iter().map(|f| f.value.clone()).collect();
    for (input, text) in &inputs {
        if let Some(value) = fields.get_mut(input.0) {
            *value = text.value().to_string();
        }
    }
    let reply = Reply {
        button,
        selected: choices.selected.clone(),
        fields,
    };
    owner.services.respond(request, reply);
}
fn keyboard(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    roots: Query<Entity, With<Root>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut choices: ResMut<Choices>,
) {
    if !roots.iter().any(|e| guard.allows(e)) {
        keys.clear();
        return;
    }
    if keys.read().any(|k| {
        k.key_code == KeyCode::Escape && k.state == bevy::input::ButtonState::Pressed && !k.repeat
    }) && let Some(request) = owner.services.pending()
        && choices.request == Some(request.id)
    {
        choices.press = Some((request.id, request.screen.escape));
    }
}
