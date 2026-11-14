//! Read-only history over the same retained owner and formatter as the native client.
#[cfg(test)]
mod tests;
use super::{
    super::{owner::HostOwner, ui::Skins},
    modal::ModalGuard,
};
use crate::play::feature_0008_achievement_notices::view::AchievementNoticesBinding;
use bevy::{input_focus::InputFocus, prelude::*, ui_widgets::Activate};
use oteryn_client_ui::achievement_notices::notices_panel_text;
use oteryn_client_ui_bevy as source;

#[derive(Resource, Default)]
struct State {
    snapshot: Option<(u64, AchievementNoticesBinding)>,
}
#[derive(Component)]
struct Window;
#[derive(Component)]
struct Close {
    generation: u64,
    binding: AchievementNoticesBinding,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if owner.modal_active() {
        return;
    }
    if let Some(play) = &owner.play {
        let binding = play.link.achievement_notices_binding();
        let _ = play
            .link
            .achievement_notices_set_open_bound(&binding, !binding.panel.open);
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    windows: Query<Entity, With<Window>>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    controls: Query<Entity, With<Close>>,
) {
    let binding = owner
        .play
        .as_ref()
        .map(|play| play.link.achievement_notices_binding())
        .filter(|binding| binding.panel.open);
    if let Some(binding) = &binding
        && state.snapshot.as_ref().is_some_and(|(generation, old)| {
            *generation == owner.generation() && old.same_projection(binding)
        })
    {
        return;
    }
    if binding.is_none() && state.snapshot.is_none() {
        return;
    }
    if focus.get().is_some_and(|entity| controls.contains(entity)) {
        focus.clear();
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    state.snapshot = binding
        .as_ref()
        .map(|binding| (owner.generation(), binding.clone()));
    let Some(binding) = binding else {
        return;
    };
    let panel = commands
        .spawn((
            Window,
            bevy::input_focus::tab_navigation::TabGroup {
                order: 2,
                modal: false,
            },
            crate::bevy_shell::window_drag::DraggableDialog,
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
    source::dialog_caption(
        &mut commands,
        panel,
        "Achievement Notices",
        skins.font.clone(),
    );
    let body = commands
        .spawn((
            source::frame(skins.inset.clone(), [1.; 4]),
            bevy::ui_widgets::ScrollArea,
            Node {
                width: percent(100.),
                max_height: px(400.),
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                padding: UiRect::all(px(5.)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    let text = commands
        .spawn(source::text(
            notices_panel_text(&binding.panel),
            source::font(skins.font.clone(), false),
        ))
        .id();
    commands.entity(body).add_child(text);
    let button = commands
        .spawn((
            source::button(skins.buttons.clone(), 80.),
            Close {
                generation: owner.generation(),
                binding,
            },
        ))
        .observe(activate)
        .id();
    let caption = commands
        .spawn(source::button_caption("Close", skins.font.clone()))
        .id();
    commands.entity(button).add_child(caption);
    commands.entity(panel).add_children(&[body, button]);
}
fn activate(
    event: On<Activate>,
    controls: Query<&Close>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
) {
    if !guard.allows(event.entity) || owner.modal_active() {
        return;
    }
    if let Ok(control) = controls.get(event.entity)
        && control.generation == owner.generation()
        && let Some(play) = &owner.play
    {
        let _ = play
            .link
            .achievement_notices_set_open_bound(&control.binding, false);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    guard: ModalGuard,
    windows: Query<Entity, With<Window>>,
    state: Res<State>,
    mut owner: NonSendMut<HostOwner>,
) {
    for event in events.read() {
        if event.state != bevy::input::ButtonState::Pressed || event.repeat {
            continue;
        }
        let window = windows.iter().next();
        let allowed = guard.allows_focus(window);
        let shortcut = event.key_code == KeyCode::KeyM
            && keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
            && keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
            && !keys.any_pressed([
                KeyCode::AltLeft,
                KeyCode::AltRight,
                KeyCode::SuperLeft,
                KeyCode::SuperRight,
            ]);
        if !allowed || owner.modal_active() {
            continue;
        }
        if window.is_some() && (shortcut || event.key_code == KeyCode::Escape) {
            if let Some((generation, binding)) = &state.snapshot
                && *generation == owner.generation()
                && let Some(play) = &owner.play
            {
                let _ = play.link.achievement_notices_set_open_bound(binding, false);
            }
        } else if shortcut {
            open(&mut owner);
        }
    }
}
