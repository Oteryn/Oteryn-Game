//! Pre-admission account menu and Premium overview over the authenticated controllers.
mod bridge;
mod commerce;
#[cfg(test)]
mod tests;
use super::{
    game_ui::{GameModal, modal::ModalGuard},
    owner::HostOwner,
    ui::Skins,
};
use bevy::{
    input_focus::{FocusCause, InputFocus, tab_navigation::TabGroup},
    prelude::*,
    ui_widgets::{Activate, ScrollArea},
};
pub(super) use bridge::Services;
use bridge::{Action, Screen};
use oteryn_client_ui_bevy as source;

#[derive(Component)]
struct Root(Option<u64>);
#[derive(Component)]
struct Body;
#[derive(Component)]
struct Control {
    request: u64,
    action: Action,
}

pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup.after(super::ui::setup))
        .add_systems(PreUpdate, poll.before(super::poll_owner))
        .add_systems(Update, (project, keyboard).chain());
    commerce::install(app);
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
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut roots: Query<(Entity, &mut Node, &mut Root)>,
    mut texts: Query<&mut Text, With<Body>>,
    mut focus: ResMut<InputFocus>,
    controls: Query<&Control>,
) {
    let Some(request) = owner.services.pending() else {
        if focus.get().is_some_and(|e| controls.contains(e)) {
            focus.clear();
        }
        for (root, mut node, mut rendered) in &mut roots {
            node.display = Display::None;
            if rendered.0.is_some() {
                commands.entity(root).despawn_children();
            }
            rendered.0 = None;
        }
        return;
    };
    // Re-evaluate remaining time and freshness using the controller's original clock.
    let text = match &request.screen {
        Screen::Account(view) => view.status_text(),
        Screen::Coin(view) => view.status_text(),
        Screen::Premium(view) => view.status_text(),
    };
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
        let panel = commands
            .spawn((
                super::window_drag::DraggableDialog,
                TabGroup {
                    order: 6,
                    modal: true,
                },
                source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
                Node {
                    width: px(620.),
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
        let (title, primary) = match &request.screen {
            Screen::Premium(_) => ("Premium overview", Some((Action::Refresh, "Refresh"))),
            Screen::Account(_) => ("Account Services", Some((Action::Premium, "Premium"))),
            Screen::Coin(_) => ("Get Oteryn coins", None),
        };
        source::dialog_caption(&mut commands, panel, title, skin.font.clone());
        let scroll = commands
            .spawn((
                ScrollArea,
                Node {
                    min_height: px(0.),
                    flex_shrink: 1.,
                    overflow: Overflow::scroll_y(),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
            ))
            .id();
        let body = commands
            .spawn((
                Body,
                source::text(text.clone(), source::font(skin.font.clone(), false)),
            ))
            .id();
        commands.entity(scroll).add_child(body);
        commerce::build(&mut commands, &skin, scroll, request);
        commands.entity(panel).add_child(scroll);
        let footer = commands
            .spawn(Node {
                column_gap: px(8.),
                justify_content: JustifyContent::End,
                ..default()
            })
            .id();
        for (action, label) in primary.into_iter().chain([(Action::Back, "Back")]) {
            let button = commands
                .spawn((
                    source::button(skin.buttons.clone(), 90.),
                    Control {
                        request: request.id,
                        action,
                    },
                ))
                .observe(activate)
                .id();
            let caption = commands
                .spawn(source::button_caption(label, skin.font.clone()))
                .id();
            commands.entity(button).add_child(caption);
            commands.entity(footer).add_child(button);
            if action == Action::Back {
                focus.set(button, FocusCause::Navigated);
            }
        }
        commands.entity(panel).add_child(footer);
        rendered.0 = Some(request.id);
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        owner.services.respond(control.request, control.action);
    }
}
fn keyboard(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    roots: Query<Entity, With<Root>>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
) {
    if !roots.iter().any(|e| guard.allows(e)) {
        keys.clear();
        return;
    }
    if keys.read().any(|k| {
        k.key_code == KeyCode::Escape && k.state == bevy::input::ButtonState::Pressed && !k.repeat
    }) && let Some(id) = owner.services.pending().map(|r| r.id)
    {
        owner.services.respond(id, Action::Back);
    }
}
