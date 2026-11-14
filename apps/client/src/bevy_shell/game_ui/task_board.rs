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
    tab: Tab,
    confirmation: Option<TaskBoardControl>,
}
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    snapshot: TaskBoardPanel,
    action: TaskBoardControl,
    confirm: bool,
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
            board_text(panel, state.tab),
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
                    confirm: false,
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
    for (label, tab) in [
        ("Bounty Tasks", Tab::Bounty),
        ("Weekly tasks", Tab::Weekly),
        ("Soulseals", Tab::Soulseals),
        ("Weekly results", Tab::Results),
    ] {
        let button = commands
            .spawn((source::button(skins.buttons.clone(), 140.), TabControl(tab)))
            .observe(change_tab)
            .id();
        let caption = commands
            .spawn(source::button_caption(label, skins.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(root).add_child(button);
    }
    let mut actions = Vec::new();
    if let Some(reply) = &panel.weekly {
        match state.tab {
            Tab::Bounty => {
                for (i, offer) in reply.bounty.offers.iter().enumerate() {
                    if let Ok(i) = u8::try_from(i) {
                        actions.push((
                            format!("Select {}", offer.race),
                            TaskBoardControl::Request(FullIntent::SelectBounty(i)),
                        ));
                    }
                }
                actions.push((
                    "Reroll (Bounty Reroll Token)".into(),
                    TaskBoardControl::Request(FullIntent::Reroll),
                ));
            }
            Tab::Weekly | Tab::Results => {
                for i in 0..4 {
                    actions.push((
                        format!("Start weekly: {}", task_board::difficulty_name(i)),
                        TaskBoardControl::Request(FullIntent::StartWeekly(i)),
                    ));
                }
                for (i, delivery) in reply.weekly.deliveries.iter().enumerate() {
                    if !delivery.delivered
                        && let Ok(i) = u8::try_from(i)
                    {
                        actions.push((
                            format!("Deliver item {}", delivery.item),
                            TaskBoardControl::Request(FullIntent::Deliver(i)),
                        ));
                    }
                }
            }
            Tab::Soulseals => {
                actions.push(("Refresh Soulseals".into(), TaskBoardControl::Catalogue));
            }
        }
    }
    if let Some(action) = &state.confirmation {
        actions = vec![("Confirm payment".into(), action.clone())];
    }
    for (label, action) in actions {
        let button = commands
            .spawn((
                source::button(skins.buttons.clone(), 260.),
                Control {
                    generation: owner.generation(),
                    snapshot: panel.clone(),
                    action,
                    confirm: state.confirmation.is_some(),
                },
            ))
            .observe(activate)
            .id();
        let caption = commands
            .spawn(source::button_caption(&label, skins.font.clone()))
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
    mut state: ResMut<State>,
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
    if paid(&control.action) && !control.confirm {
        state.confirmation = Some(control.action.clone());
        state.snapshot = None;
        return;
    }
    state.confirmation = None;
    if let Some(play) = &owner.play {
        play.link
            .task_board_action_bound(&control.snapshot, control.action.clone());
    }
}

#[cfg(test)]
mod tests;

#[derive(Default, Clone, Copy)]
enum Tab {
    #[default]
    Bounty,
    Weekly,
    Soulseals,
    Results,
}
#[derive(Component)]
struct TabControl(Tab);
fn change_tab(
    event: On<Activate>,
    tabs: Query<&TabControl>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    if guard.allows(event.entity)
        && let Ok(tab) = tabs.get(event.entity)
    {
        state.tab = tab.0;
        state.snapshot = None;
        state.confirmation = None;
    }
}
fn paid(action: &TaskBoardControl) -> bool {
    matches!(
        action,
        TaskBoardControl::Request(
            FullIntent::Reroll
                | FullIntent::Deliver(_)
                | FullIntent::UnlockList
                | FullIntent::BuyShop(_)
        )
    )
}
fn board_text(panel: &TaskBoardPanel, tab: Tab) -> String {
    match tab {
        Tab::Bounty | Tab::Weekly => task_board::panel_text(panel),
        Tab::Soulseals => task_board::soulseals_text(panel),
        Tab::Results => panel
            .weekly
            .as_ref()
            .and_then(|r| r.weekly_results.as_ref())
            .map_or_else(
                || "No weekly results received".into(),
                |r| {
                    format!(
                        "Weekly rewards received\nTask points: {}\nSoulseals: {}",
                        r.points, r.soulseals
                    )
                },
            ),
    }
}
