//! Installation of the shared in-game presentations.
mod tools;
use super::{
    actions, analyser, bestiary, bosstiary, bug_report, chat, cyclopedia, cyclopedia_items,
    cyclopedia_map, depot, feature_dialogs, forge, highscores, hotkeys, hud, imbuement_tracker,
    imbuing, inventory, item_details, journal, layout, market, minimap, modal, npc_shop, options,
    outfit, party, prey, proficiency, project, projection, quick_loot, reward_wall, rule_violation,
    satellite_map, stash, store, store_inbox, task_board, task_tracker, team_finder, trade,
    tutorial, unjustified_points, wheel,
};
use bevy::prelude::*;
use tools::project_tools;

pub(in crate::bevy_shell) fn install(app: &mut App) {
    install_scroll_memory(app);
    bosstiary::install(app);
    actions::install(app);
    bestiary::install(app);
    bug_report::install(app);
    chat::install(app);
    install_container_tools(app);
    cyclopedia::install(app);
    cyclopedia_items::install(app);
    cyclopedia_map::install(app);
    analyser::install(app);
    depot::install(app);
    feature_dialogs::install(app);
    highscores::install(app);
    forge::install(app);
    hotkeys::install(app);
    hud::install(app);
    imbuement_tracker::install(app);
    imbuing::install(app);
    inventory::install(app);
    item_details::install(app);
    journal::install(app);
    layout::install(app);
    market::install(app);
    minimap::install(app);
    modal::install(app);
    npc_shop::install(app);
    options::install(app);
    outfit::install(app);
    party::install(app);
    prey::install(app);
    proficiency::install(app);
    quick_loot::install(app);
    task_board::install(app);
    task_tracker::install(app);
    stash::install(app);
    rule_violation::install(app);
    store::install(app);
    store_inbox::install(app);
    satellite_map::install(app);
    reward_wall::install(app);
    team_finder::install(app);
    trade::install(app);
    unjustified_points::install(app);
    wheel::install(app);
    tutorial::install(app);
    app.add_systems(
        Update,
        (
            projection::prepare_item_appearances.before(super::super::scene::project),
            projection::item_appearances.after(super::super::scene::project),
            projection::project_regions.after(project),
        ),
    );
}

/// A stable viewport role, independent of the entities used for its changing content.
#[derive(Component, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RememberScroll(pub &'static str, pub u32);
#[derive(Component)]
struct ScrollScope(u64);
#[derive(Resource, Default)]
struct ScrollMemory {
    generation: Option<u64>,
    positions: std::collections::BTreeMap<RememberScroll, ScrollPosition>,
}
pub(super) fn install_scroll_memory(app: &mut App) {
    if app.world().contains_resource::<ScrollMemory>() {
        return;
    }
    app.init_resource::<ScrollMemory>()
        .add_observer(remember_scroll)
        .add_observer(restore_scroll);
}
fn remember_scroll(
    event: On<bevy::ecs::lifecycle::Remove<RememberScroll>>,
    owner: Option<NonSend<super::super::owner::HostOwner>>,
    areas: Query<(&RememberScroll, &ScrollPosition, &ScrollScope)>,
    mut memory: ResMut<ScrollMemory>,
) {
    let Some(owner) = owner else {
        return;
    };
    if let Ok((key, position, scope)) = areas.get(event.entity)
        && scope.0 == owner.generation()
        && memory.generation == Some(scope.0)
    {
        memory.positions.insert(*key, position.clone());
    }
}
fn restore_scroll(
    event: On<bevy::ecs::lifecycle::Add<RememberScroll>>,
    owner: Option<NonSend<super::super::owner::HostOwner>>,
    areas: Query<&RememberScroll>,
    mut memory: ResMut<ScrollMemory>,
    mut commands: Commands,
) {
    let Some(owner) = owner else {
        return;
    };
    let Ok(key) = areas.get(event.entity) else {
        return;
    };
    let generation = owner.generation();
    if memory.generation != Some(generation) {
        memory.positions.clear();
        memory.generation = Some(generation);
    }
    commands
        .entity(event.entity)
        .insert(ScrollScope(generation));
    if let Some(position) = memory.positions.get(key).cloned() {
        commands.entity(event.entity).insert(position);
    }
}

#[cfg(test)]
mod tests;

use super::super::{
    controls::{ContainerTool, ContainerToolRequest},
    owner::HostOwner,
    ui::Skins,
};
use super::inventory::ToolSnapshot;
use super::modal::ModalGuard;
use bevy::{ui::InteractionDisabled, ui_widgets::Activate};
use oteryn_client_ui_bevy as source;
#[derive(Resource, Default)]
struct ToolUi {
    request: Option<ContainerToolRequest>,
    revision: u64,
    root: Option<Entity>,
    seen: Option<ToolSnapshot>,
}
#[derive(Clone, Copy)]
enum ToolAction {
    Close,
    Change(ContainerTool),
    Browse,
    Seek {
        view: u8,
        handle: oteryn_session::ItemHandle,
        index: u16,
    },
    DepotBrowse {
        box_number: u8,
        page: u32,
    },
    Deposit(oteryn_session::ItemHandle),
    Withdraw(usize),
    Search(oteryn_client_runtime::cui3::depot_search::Request),
    Sort {
        view: u8,
        handle: oteryn_session::ItemHandle,
        key: oteryn_client_runtime::container_sort::SortKey,
        descending: bool,
    },
}
#[derive(Component)]
struct ToolControl {
    generation: u64,
    revision: u64,
    tool: ContainerTool,
    action: ToolAction,
}
fn install_container_tools(app: &mut App) {
    app.init_resource::<ToolUi>()
        .add_systems(Update, project_tools);
}
fn tool_button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    state: &ToolUi,
    label: &str,
    action: ToolAction,
    enabled: bool,
) {
    let Some(request) = state.request else {
        return;
    };
    let entity = commands
        .spawn((
            source::button(skin.buttons.clone(), 210.),
            ToolControl {
                generation: request.generation,
                revision: state.revision,
                tool: request.tool,
                action,
            },
        ))
        .observe(tool_activate)
        .id();
    if !enabled {
        commands.entity(entity).insert(InteractionDisabled);
    }
    let caption = commands
        .spawn(source::button_caption(label, skin.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
fn tool_activate(
    event: On<Activate>,
    guard: ModalGuard,
    controls: Query<(&ToolControl, Has<InteractionDisabled>)>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<ToolUi>,
) {
    let Ok((control, false)) = controls.get(event.entity) else {
        return;
    };
    let Some(request) = state.request else {
        return;
    };
    if !guard.allows(event.entity)
        || owner.modal_active()
        || control.generation != owner.generation()
        || control.generation != request.generation
        || control.tool != request.tool
        || control.revision != state.revision
    {
        return;
    }
    if matches!(control.action, ToolAction::Close) {
        state.request = None;
        return;
    }
    if let ToolAction::Change(tool) = control.action {
        owner.request_container_tool(tool, None, None);
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let snapshot = ToolSnapshot::read(&play.link, request.tool);
    if play.view.gameplay_input_blocked(&play.link)
        || !snapshot.ready()
        || state.seen.as_ref() != Some(&snapshot)
    {
        return;
    }
    match (control.action, snapshot) {
        (ToolAction::Browse, ToolSnapshot::Browse(mut panel)) => {
            let (x, y, floor) = request
                .position
                .unwrap_or_else(|| play.view.satellite_player());
            if floor != play.view.satellite_player().2 {
                return;
            }
            panel.x = x;
            panel.y = y;
            panel.floor = floor;
            if let Some(command) = oteryn_client_ui::browse_field::open(&mut panel) {
                play.link.browse_field_request(command.payload);
            }
        }
        (
            ToolAction::Seek {
                view,
                handle,
                index,
            },
            ToolSnapshot::Browse(mut panel),
        ) => {
            if !panel.views.views.iter().any(|entry| {
                entry.view_id == view && entry.container_handle == handle && entry.browse.is_some()
            }) {
                return;
            }
            panel.view_id = view;
            panel.index = index;
            if let Some(command) = oteryn_client_ui::browse_field::seek(&mut panel) {
                play.link.browse_field_request(command.payload);
            }
        }
        (ToolAction::DepotBrowse { box_number, page }, ToolSnapshot::Depot(_)) => play
            .link
            .depot_request(oteryn_client_ui::depot::Intent::Browse { box_number, page }),
        (ToolAction::Deposit(handle), ToolSnapshot::Depot(_)) => play
            .link
            .depot_request(oteryn_client_ui::depot::Intent::Deposit(handle)),
        (ToolAction::Withdraw(row), ToolSnapshot::Depot(_)) => play
            .link
            .depot_request(oteryn_client_ui::depot::Intent::Withdraw(row)),
        (ToolAction::Search(request), ToolSnapshot::Search(_)) => {
            if let Ok(payload) = oteryn_client_runtime::cui3::depot_search::encode_request(request)
            {
                play.link.depot_search_request(payload);
            }
        }
        (
            ToolAction::Sort {
                view,
                handle,
                key,
                descending,
            },
            ToolSnapshot::Sort(_, containers),
        ) => {
            if !containers.views.as_ref().is_some_and(|views| {
                views.views.iter().any(|entry| {
                    entry.view_id == view
                        && entry.container_handle == handle
                        && entry.browse.is_none()
                })
            }) {
                return;
            }
            let request = oteryn_client_runtime::container_sort::Request {
                view,
                key,
                descending,
                recursive: false,
            };
            for (index, value) in request.encode().into_iter().enumerate() {
                play.link.container_sort_edit(index, value.to_string());
            }
            play.link
                .container_sort_request(oteryn_client_ui::container_sort::Intent::Sort);
        }
        _ => {
            return;
        }
    }
    state.seen = None;
}
