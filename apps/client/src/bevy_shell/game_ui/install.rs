//! Installation of the shared in-game presentations.
use super::{
    actions, analyser, bug_report, chat, cyclopedia, feature_dialogs, forge, highscores, hotkeys,
    hud, imbuing, inventory, item_details, journal, market, minimap, modal, npc_shop, options,
    outfit, party, prey, proficiency, project, projection, quick_loot, reward_wall, rule_violation,
    satellite_map, stash, store, task_board, trade, tutorial, wheel,
};
use bevy::prelude::*;

pub(in crate::bevy_shell) fn install(app: &mut App) {
    install_scroll_memory(app);
    actions::install(app);
    bug_report::install(app);
    chat::install(app);
    install_container_tools(app);
    cyclopedia::install(app);
    analyser::install(app);
    feature_dialogs::install(app);
    highscores::install(app);
    forge::install(app);
    hotkeys::install(app);
    hud::install(app);
    imbuing::install(app);
    inventory::install(app);
    item_details::install(app);
    journal::install(app);
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
    stash::install(app);
    rule_violation::install(app);
    store::install(app);
    satellite_map::install(app);
    reward_wall::install(app);
    trade::install(app);
    wheel::install(app);
    tutorial::install(app);
    app.add_systems(
        Update,
        (
            projection::missing_item_appearances,
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
        (ToolAction::Sort { view, handle, key }, ToolSnapshot::Sort(_, containers)) => {
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
                descending: false,
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
fn project_tools(
    mut commands: Commands,
    mut owner: NonSendMut<HostOwner>,
    skin: Res<Skins>,
    guard: ModalGuard,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut state: ResMut<ToolUi>,
) {
    if let Some(request) = owner.take_container_tool_request()
        && request.generation == owner.generation()
        && owner.play.is_some()
    {
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        state.request = Some(request);
        state.seen = None;
    }
    if owner.play.is_none()
        || state
            .request
            .is_some_and(|request| request.generation != owner.generation())
        || (!owner.modal_active()
            && state.root.is_some_and(|root| guard.allows(root))
            && keys
                .as_ref()
                .is_some_and(|keys| keys.just_pressed(KeyCode::Escape)))
    {
        state.request = None;
    }
    let Some(request) = state.request else {
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        state.seen = None;
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    let snapshot = ToolSnapshot::read(&play.link, request.tool);
    if state.seen.as_ref() == Some(&snapshot) {
        return;
    }
    let Some(revision) = state.revision.checked_add(1) else {
        state.request = None;
        return;
    };
    state.revision = revision;
    let root = *state.root.get_or_insert_with(|| {
        commands
            .spawn((
                super::GameModal,
                GlobalZIndex(155),
                Name::new(request.tool.title()),
                bevy::ui_widgets::ScrollArea,
                RememberScroll("container_tools", request.tool as u32),
                source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(20.),
                    top: percent(10.),
                    width: percent(60.),
                    max_height: percent(80.),
                    padding: px(20.).all(),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4.),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
            ))
            .id()
    });
    commands.entity(root).despawn_children();
    source::dialog_caption(&mut commands, root, request.tool.title(), skin.font.clone());
    let text = commands
        .spawn(source::text(
            snapshot.text(),
            source::font(skin.font.clone(), false),
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(root).add_child(text);
    let mut button = |label: &str, action, enabled| {
        tool_button(&mut commands, &skin, root, &state, label, action, enabled)
    };
    for tool in [
        ContainerTool::BrowseField,
        ContainerTool::Depot,
        ContainerTool::Search,
        ContainerTool::Sort,
    ] {
        button(tool.title(), ToolAction::Change(tool), true);
    }
    let ready = snapshot.ready();
    match &snapshot {
        ToolSnapshot::Browse(panel) => {
            button("Browse selected tile", ToolAction::Browse, ready);
            for view in &panel.views.views {
                if let Some(page) = view.browse {
                    for (label, index, enabled) in [
                        (
                            "Previous page",
                            page.index.saturating_sub(30),
                            page.index > 0,
                        ),
                        (
                            "Next page",
                            page.index.saturating_add(30),
                            page.index.saturating_add(30) < page.total,
                        ),
                    ] {
                        button(
                            label,
                            ToolAction::Seek {
                                view: view.view_id,
                                handle: view.container_handle,
                                index,
                            },
                            ready && enabled,
                        );
                    }
                }
            }
        }
        ToolSnapshot::Depot(panel) => {
            let (box_number, page) = panel
                .reply
                .as_ref()
                .map_or((1, 0), |reply| (reply.box_number, reply.page));
            for (label, box_number, page, enabled) in [
                ("Refresh box", box_number, page, true),
                (
                    "Previous box",
                    box_number.saturating_sub(1),
                    0,
                    box_number > 1,
                ),
                ("Next box", box_number.saturating_add(1), 0, box_number < 17),
                (
                    "Previous page",
                    box_number,
                    page.saturating_sub(1),
                    page > 0,
                ),
                (
                    "Next page",
                    box_number,
                    page.saturating_add(1),
                    page < 468
                        && panel
                            .reply
                            .as_ref()
                            .is_some_and(|reply| !reply.entries.is_empty()),
                ),
            ] {
                button(
                    label,
                    ToolAction::DepotBrowse { box_number, page },
                    ready && enabled,
                );
            }
            if let Some(reply) = &panel.reply {
                for (row, entry) in reply.entries.iter().enumerate() {
                    button(
                        &format!("Withdraw item {} ×{}", entry.definition, entry.count),
                        ToolAction::Withdraw(row),
                        ready && reply.town.is_some(),
                    );
                }
                for item in oteryn_client_ui::depot::deposit_items(panel) {
                    button(
                        &format!("Deposit item {} ×{}", item.item_definition_ref, item.count),
                        ToolAction::Deposit(item.handle),
                        ready && reply.town.is_some(),
                    );
                }
            }
        }
        ToolSnapshot::Search(panel) => {
            use oteryn_client_runtime::cui3::depot_search::{Outcome, Request};
            button(
                "List depot holdings",
                ToolAction::Search(Request::Open { after: 0 }),
                ready,
            );
            if let Some(reply) = &panel.reply {
                if reply.outcome == Outcome::More
                    && let Some(last) = reply.results.last()
                {
                    button(
                        "Next page",
                        ToolAction::Search(Request::Open {
                            after: last.client_id,
                        }),
                        ready,
                    );
                }
                for row in &reply.results {
                    for all in [false, true] {
                        button(
                            &format!(
                                "Retrieve {} of item {}",
                                if all { "all" } else { "one" },
                                row.client_id
                            ),
                            ToolAction::Search(Request::Retrieve {
                                client_id: row.client_id,
                                all,
                            }),
                            ready,
                        );
                    }
                    for tier in &row.tiers {
                        button(
                            &format!("Item {} tier {} details", row.client_id, tier.tier),
                            ToolAction::Search(Request::Detail {
                                client_id: row.client_id,
                                tier: tier.tier,
                            }),
                            ready,
                        );
                    }
                }
            }
        }
        ToolSnapshot::Sort(_, containers) => {
            if let Some(views) = &containers.views {
                for view in &views.views {
                    if view.browse.is_some()
                        || request
                            .view
                            .is_some_and(|source| source != (view.view_id, view.container_handle))
                    {
                        continue;
                    }
                    use oteryn_client_runtime::container_sort::SortKey;
                    for (key, label) in [
                        (SortKey::Name, "name"),
                        (SortKey::Weight, "weight"),
                        (SortKey::Expiry, "expiry"),
                        (SortKey::StackSize, "stack size"),
                    ] {
                        button(
                            &format!("Sort container {} by {label}", view.view_id),
                            ToolAction::Sort {
                                view: view.view_id,
                                handle: view.container_handle,
                                key,
                            },
                            ready,
                        );
                    }
                }
            }
        }
    }
    button("Close", ToolAction::Close, true);
    state.seen = Some(snapshot);
}
