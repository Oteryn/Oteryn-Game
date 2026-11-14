//! In-game Premium presentation over the retained native owner and formatter.
#[cfg(test)]
mod tests;
use super::super::{owner::HostOwner, ui::Skins};
use super::modal::ModalGuard;
use crate::premium::game::Binding;
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui_bevy as source;

#[derive(Resource, Default)]
struct State {
    snapshot: Option<(u64, Binding)>,
    root: Option<Entity>,
}
#[derive(Component)]
struct Control {
    generation: u64,
    binding: Binding,
    close: bool,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if let Some(play) = &owner.play {
        let expected = play.link.premium_overview_binding();
        let _ = play.link.premium_overview_action_bound(
            &expected,
            expected.open,
            !owner.modal_active(),
        );
    }
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
        .map(|p| p.link.premium_overview_binding())
        .filter(|p| p.open);
    if current.as_ref().is_some_and(|binding| {
        state.snapshot.as_ref().is_some_and(|(generation, old)| {
            *generation == owner.generation() && old.same_projection(binding)
        })
    }) || (current.is_none() && state.snapshot.is_none())
    {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.snapshot = current
        .as_ref()
        .map(|binding| (owner.generation(), binding.clone()));
    let Some(binding) = current else {
        return;
    };
    let root = commands
        .spawn((
            Name::new("premium_overview.window"),
            crate::bevy_shell::window_drag::DraggableDialog,
            bevy::input_focus::tab_navigation::TabGroup {
                order: 2,
                modal: false,
            },
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            GlobalZIndex(150),
            Node {
                position_type: PositionType::Absolute,
                left: px(200.),
                top: px(70.),
                width: px(440.),
                max_width: percent(95.),
                max_height: percent(85.),
                padding: UiRect {
                    left: px(12.),
                    right: px(12.),
                    top: px(24.),
                    bottom: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    state.root = Some(root);
    source::dialog_caption(
        &mut commands,
        root,
        "Oteryn Premium overview",
        skins.font.clone(),
    );
    let body = commands
        .spawn((
            bevy::ui_widgets::ScrollArea,
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                max_height: px(400.),
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                padding: px(5.).all(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    let text = commands
        .spawn(source::text(
            &binding.text,
            source::font(skins.font.clone(), false),
        ))
        .id();
    commands.entity(body).add_child(text);
    commands.entity(root).add_child(body);
    for (close, caption) in [(false, "Refresh Premium status"), (true, "Close")] {
        let button = commands
            .spawn((
                source::button(skins.buttons.clone(), 180.),
                Control {
                    generation: owner.generation(),
                    binding: binding.clone(),
                    close,
                },
            ))
            .observe(activate)
            .id();
        let caption = commands
            .spawn(source::button_caption(caption, skins.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(root).add_child(button);
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
) {
    if let Ok(control) = controls.get(event.entity)
        && control.generation == owner.generation()
        && let Some(play) = &owner.play
    {
        let admitted = guard.allows(event.entity) && !owner.modal_active();
        let _ = play
            .link
            .premium_overview_action_bound(&control.binding, control.close, admitted);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    state: Res<State>,
    owner: NonSend<HostOwner>,
) {
    for event in events.read() {
        if event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
            && event.key_code == KeyCode::Escape
            && let Some((generation, binding)) = &state.snapshot
            && *generation == owner.generation()
            && let Some(play) = &owner.play
        {
            let _ = play.link.premium_overview_action_bound(binding, true, true);
        }
    }
}
