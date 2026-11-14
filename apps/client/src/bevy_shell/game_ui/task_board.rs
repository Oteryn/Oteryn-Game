//! Task Board presentation over the retained owner.
use super::{
    super::{owner::HostOwner, ui::Skins},
    modal::ModalGuard,
};
use crate::play::feature_0072_task_board::view::TaskBoardControl;
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::task_board::{self, FullIntent, TaskBoardPanel};
use oteryn_client_ui_bevy as source;
#[derive(Resource, Default)]
struct State {
    snapshot: Option<TaskBoardPanel>,
    generation: u64,
    root: Option<Entity>,
}
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    snapshot: TaskBoardPanel,
    action: TaskBoardControl,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>().add_systems(Update, project);
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
) {
    let panel = owner
        .play
        .as_ref()
        .map(|p| p.link.task_board_panel())
        .filter(|p| p.open);
    if state.generation == owner.generation() && state.snapshot == panel {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.generation = owner.generation();
    state.snapshot = panel;
    let Some(panel) = &state.snapshot else {
        return;
    };
    let root = commands
        .spawn((
            source::frame(skins.frame.clone(), [2., 17., 2., 2.]),
            Node {
                position_type: PositionType::Absolute,
                left: px(200.),
                top: px(50.),
                width: px(700.),
                max_height: percent(85.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                padding: px(20.).all(),
                ..default()
            },
            GlobalZIndex(10),
        ))
        .id();
    source::dialog_caption(&mut commands, root, "Task Board", skins.font.clone());
    let text = commands
        .spawn(source::text(
            task_board::panel_text(panel),
            source::font(skins.font.clone(), false),
        ))
        .id();
    commands.entity(root).add_child(text);
    for (label, action) in [
        ("Refresh", TaskBoardControl::Request(FullIntent::Refresh)),
        (
            "Bounty Tasks",
            TaskBoardControl::Request(FullIntent::OpenBounty),
        ),
        ("Claim", TaskBoardControl::Request(FullIntent::ClaimBounty)),
        ("Close", TaskBoardControl::Close),
    ] {
        let button = commands
            .spawn((
                source::button(skins.buttons.clone(), 140.),
                Control {
                    generation: owner.generation(),
                    snapshot: panel.clone(),
                    action,
                },
            ))
            .observe(activate)
            .id();
        let caption = commands
            .spawn(source::button_caption(label, skins.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(root).add_child(button);
    }
    state.root = Some(root);
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity) || owner.modal_active() {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if control.generation != owner.generation() {
        return;
    }
    if let Some(play) = &owner.play {
        play.link
            .task_board_action_bound(&control.snapshot, control.action.clone());
    }
}
