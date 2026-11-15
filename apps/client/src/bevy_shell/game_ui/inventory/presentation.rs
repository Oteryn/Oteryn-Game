//! Session-local container geometry and authoritative browse-page navigation.
use super::*;
use bevy::ui_widgets::ScrollArea;
use oteryn_client_ui_bevy as source;
use oteryn_session::{
    ItemHandle,
    browse_field::{BrowseFieldIntent, BrowseFieldPage, encode_browse_field_intent},
    container_tree::{ContainerView, ContainerViews},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Window {
    pub handle: ItemHandle,
    pub minimized: bool,
    pub rows: u8,
    pub name: Option<String>,
    name_requested: bool,
}
#[derive(Clone, Copy)]
struct Binding {
    view: u8,
    handle: ItemHandle,
}
#[derive(Component, Clone, Copy)]
struct Action {
    generation: u64,
    target: Target,
}
#[derive(Clone, Copy)]
enum Target {
    Ground,
    Container(Binding, Kind),
}
#[derive(Clone, Copy)]
enum Kind {
    Minimize,
    Resize(i8),
    Page(BrowseFieldPage, u16),
}

impl Window {
    fn new(view: &ContainerView) -> Self {
        Self {
            handle: view.container_handle,
            minimized: false,
            rows: view.capacity.div_ceil(4).min(5),
            name: None,
            name_requested: false,
        }
    }
}
pub(super) fn retain(ui: &mut InventoryUi, owner: &HostOwner, views: Option<&ContainerViews>) {
    ui.windows.retain(|id, window| {
        views.is_some_and(|views| {
            views
                .views
                .iter()
                .any(|view| view.view_id == *id && view.container_handle == window.handle)
        })
    });
    let (Some(views), Some(play)) = (views, &owner.play) else {
        return;
    };
    let inspection = play.link.inspection_panel();
    // Share the existing single-flight inspection owner only while both user presenters are idle.
    let mut request = inspection.selected
        && owner.phase == crate::bevy_shell::owner::Phase::Playing
        && !inspection.open
        && !inspection.catalogue_open
        && !inspection.pending
        && !owner.modal_active()
        && ui.menu.is_none()
        && ui.prompt.is_none()
        && !play.view.gameplay_input_blocked(&play.link);
    for view in &views.views {
        let window = ui
            .windows
            .entry(view.view_id)
            .or_insert_with(|| Window::new(view));
        window.rows = window.rows.clamp(1, view.capacity.div_ceil(4));
        if view.browse.is_some() {
            continue;
        }
        if !inspection.pending
            && inspection.last_request
                == Some(oteryn_client_ui::inspection::Request::Item(
                    view.container_handle,
                ))
            && let Some(reply) = &inspection.reply
            && reply.status == 1
            && reply.items.len() == 1
            && window.name.as_ref() != Some(&reply.items[0].name)
        {
            window
                .name
                .get_or_insert_default()
                .clone_from(&reply.items[0].name);
        }
        if request && window.name.is_none() && !window.name_requested {
            window.name_requested = play
                .link
                .inspect(oteryn_client_ui::inspection::Request::Item(
                    view.container_handle,
                ));
            request = !window.name_requested;
        }
    }
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let row = commands
        .spawn(Node {
            column_gap: px(3.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(row);
    row
}
fn action(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    name: &str,
    caption: &str,
    action: Action,
) {
    let (name, width) = match action.target {
        Target::Ground => ("inventory.container.browse".into(), 110.),
        Target::Container(binding, _) => (
            format!("inventory.container.{}.{}", binding.view, name),
            50.,
        ),
    };
    let entity = commands
        .spawn((
            Name::new(name),
            action,
            source::button(skins.buttons.clone(), width),
        ))
        .observe(activate)
        .id();
    let text = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(text);
    commands.entity(parent).add_child(entity);
}
pub(super) fn browse_entry(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    generation: u64,
) {
    let entry = Action {
        generation,
        target: Target::Ground,
    };
    action(commands, skins, parent, "browse", "Browse ground", entry);
}
pub(super) fn body(
    commands: &mut Commands,
    skins: &Skins,
    panel: Entity,
    generation: u64,
    view: &ContainerView,
    window: Option<&Window>,
) -> Entity {
    let fallback = Window::new(view);
    let window = window.unwrap_or(&fallback);
    let controls = row(commands, panel);
    let binding = Binding {
        view: view.view_id,
        handle: view.container_handle,
    };
    let bound = Action {
        generation,
        target: Target::Container(binding, Kind::Minimize),
    };
    let caption = if window.minimized {
        "Restore"
    } else {
        "Minimize"
    };
    action(commands, skins, controls, "minimize", caption, bound);
    for (name, caption, delta) in [("shrink", "− row", -1), ("grow", "+ row", 1)] {
        if !window.minimized
            && (1..=i16::from(view.capacity.div_ceil(4)))
                .contains(&(i16::from(window.rows) + i16::from(delta)))
        {
            let resize = Action {
                target: Target::Container(binding, Kind::Resize(delta)),
                ..bound
            };
            action(commands, skins, controls, name, caption, resize);
        }
    }
    if !window.minimized
        && let Some(page) = view.browse
    {
        let capacity = u16::from(view.capacity);
        let text = format!(
            "Page {} of {}",
            page.index / capacity + 1,
            page.total.max(1).div_ceil(capacity)
        );
        super::view::label(commands, skins, panel, &text);
        let controls = row(commands, panel);
        for (name, caption, index) in [
            ("previous", "Previous", page.index.checked_sub(capacity)),
            (
                "next",
                "Next",
                page.index
                    .checked_add(capacity)
                    .filter(|index| *index < page.total),
            ),
        ] {
            if let Some(index) = index {
                let kind = Kind::Page(page, index);
                let page = Action {
                    target: Target::Container(binding, kind),
                    ..bound
                };
                action(commands, skins, controls, name, caption, page);
            }
        }
    }
    let body = commands
        .spawn((
            Name::new(format!("inventory.container.{}.body", view.view_id)),
            Node {
                display: if window.minimized {
                    Display::None
                } else {
                    Display::Flex
                },
                height: px(f32::from(window.rows) * 37. - 3.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.,
                ..default()
            },
            ScrollArea,
            ScrollPosition::default(),
        ))
        .id();
    commands.entity(panel).add_child(body);
    body
}
fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    controls: Query<&Action>,
    owner: NonSend<HostOwner>,
    mut ui: ResMut<InventoryUi>,
) {
    if !guard.allows(event.entity)
        || owner.phase != crate::bevy_shell::owner::Phase::Playing
        || owner.modal_active()
        || ui.prompt.is_some()
        || ui.menu.is_some()
        || ui.dragged.is_some()
    {
        return;
    }
    let Ok(action) = controls.get(event.entity) else {
        return;
    };
    if owner.generation() != action.generation {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let intent = match action.target {
        Target::Ground => {
            let (x, y, floor) = play.view.map_center();
            BrowseFieldIntent::Open {
                position: oteryn_session::item_view::WorldTilePosition { x, y, floor },
            }
        }
        Target::Container(binding, kind) => {
            let panel = play.link.container_panel();
            let Some(view) = panel.views.as_ref().and_then(|views| {
                views.views.iter().find(|view| {
                    view.view_id == binding.view && view.container_handle == binding.handle
                })
            }) else {
                return;
            };
            if let Kind::Page(current, index) = kind {
                let capacity = u16::from(view.capacity);
                if panel.pending
                    || view.browse != Some(current)
                    || !index.is_multiple_of(capacity)
                    || index >= current.total
                {
                    return;
                }
                BrowseFieldIntent::Seek {
                    view_id: binding.view,
                    index,
                }
            } else {
                let window = ui
                    .windows
                    .entry(binding.view)
                    .or_insert_with(|| Window::new(view));
                if let Kind::Resize(delta) = kind {
                    window.rows = (i16::from(window.rows) + i16::from(delta))
                        .clamp(1, i16::from(view.capacity.div_ceil(4)))
                        as u8;
                } else {
                    window.minimized = !window.minimized;
                }
                return;
            }
        }
    };
    if let Ok(payload) = encode_browse_field_intent(&intent) {
        play.link.browse_field_request(payload);
    }
}
