use super::super::{GameRegions, ItemIcon};
use super::dialogs::{button, context};
use super::*;
use bevy::ui_widgets::Button;
use oteryn_client_ui_bevy as source;

#[derive(PartialEq)]
struct Snapshot {
    generation: u64,
    inventory: oteryn_client_ui::inventory::InventoryPanel,
    containers: oteryn_client_ui::container_tree::ContainerPanel,
    slots: Vec<oteryn_client_ui::equipment::Slot>,
    movement: oteryn_client_ui::inventory::movement::Movement,
    selection: Option<Slot>,
    menu: Option<(Slot, Vec2)>,
    world_feedback: Option<String>,
    sort_available: bool,
    windows: std::collections::BTreeMap<u8, presentation::Window>,
    browse: bool,
    /// The soul points of the status widget's vitals and the inventory widget state.
    soul: Option<u32>,
    widget: super::super::hud::EquipmentWidget,
    /// The writing (115) and timed item (39) windows the item menu can open.
    write: bool,
    timed: bool,
    /// Tibia Global's Manual Sort Mode: orange container titles, and drops name their slot.
    manual_sort: bool,
}
#[derive(Component)]
pub(super) struct Projection;
#[derive(Default)]
pub(super) struct ProjectionState(Option<Snapshot>);

// The projection reads independent retained regions and slot decorations.
#[allow(clippy::too_many_arguments)]
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    regions: Res<GameRegions>,
    mut ui: ResMut<InventoryUi>,
    widget: Option<Res<super::super::hud::EquipmentWidget>>,
    mut previous: Local<ProjectionState>,
    roots: Query<Entity, With<Projection>>,
    mut cells: Query<(&Slot, &mut BorderColor)>,
) {
    let Some(play) = &owner.play else {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
        previous.0 = None;
        *ui = InventoryUi::default();
        return;
    };
    // Retain picking identities during a gesture; refresh the authoritative stack again on drop.
    if ui
        .dragged
        .is_some_and(|slot| slot.current_item(&owner).is_some())
    {
        return;
    }
    if ui
        .selection
        .is_some_and(|slot| slot.current_item(&owner).is_none())
    {
        ui.selection = None;
    }
    if ui
        .menu
        .is_some_and(|(slot, _)| slot.current_item(&owner).is_none())
    {
        ui.menu = None;
    }
    let movement = play.link.item_movement();
    if movement.prompt.is_none() && movement.dragged.is_none() {
        ui.ground_source = None;
    }
    let prompt = movement.prompt.map(|(item, destination)| Prompt {
        generation: owner.generation(),
        item,
        destination,
    });
    if ui.prompt != prompt {
        ui.prompt = prompt;
        ui.split_count = prompt.map_or(0, |prompt| prompt.item.count.min(100));
        ui.split_text = prompt.map(|prompt| prompt.item.count.min(100).to_string());
        ui.split_entry = prompt.and_then(|prompt| super::amount::source_entry(&owner, prompt));
        ui.split_feedback.clear();
        ui.pending_amount_confirm = None;
    }
    let generation = owner.generation();
    if previous.0.as_ref().map(|snapshot| snapshot.generation) != Some(generation) {
        ui.windows.clear();
    }
    let containers = play.link.container_panel();
    presentation::retain(&mut ui, &owner, containers.views.as_ref());
    let snapshot = Snapshot {
        generation: owner.generation(),
        inventory: play.link.inventory_panel(),
        containers,
        windows: ui.windows.clone(),
        browse: play.link.browse_field_panel().selected,
        soul: play.link.actor_vitals().map(|vitals| vitals.soul),
        widget: widget.as_deref().copied().unwrap_or_default(),
        write: play.link.mail_panel().selected,
        timed: play.link.timed_item_panel().selected,
        slots: play.link.equipment_slots(),
        movement,
        selection: ui.selection,
        menu: ui.menu,
        world_feedback: play.link.world_context_feedback(),
        sort_available: play.link.container_sort_panel().selected,
        manual_sort: play.link.container_manual_sort_active(),
    };
    // Selection changes decoration, not the pointer identities used for double clicks.
    for (slot, mut border) in &mut cells {
        *border = BorderColor::all(
            if snapshot.selection.is_some_and(|selected| {
                selected.item.is_some()
                    && selected.item.map(|item| item.handle) == slot.item.map(|item| item.handle)
            }) {
                Color::WHITE
            } else {
                Color::NONE
            },
        );
    }
    if previous.0.as_ref().is_some_and(|old| {
        old.generation == snapshot.generation
            && old.inventory.inventory == snapshot.inventory.inventory
            && old.inventory.capacity == snapshot.inventory.capacity
            && old.containers.views == snapshot.containers.views
            && old.containers.feedback == snapshot.containers.feedback
            && old.slots == snapshot.slots
            && old.movement == snapshot.movement
            && old.menu == snapshot.menu
            && old.world_feedback == snapshot.world_feedback
            && old.sort_available == snapshot.sort_available
            && old.windows == snapshot.windows
            && old.browse == snapshot.browse
            && old.soul == snapshot.soul
            && old.widget == snapshot.widget
            && old.write == snapshot.write
            && old.timed == snapshot.timed
            && old.manual_sort == snapshot.manual_sort
    }) {
        previous.0 = Some(snapshot);
        return;
    }
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    let equipment = commands
        .spawn((
            Projection,
            Node {
                width: percent(100.),
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.,
                row_gap: px(2.),
                ..default()
            },
        ))
        .id();
    commands.entity(regions.equipment).add_child(equipment);
    equipment_grid(&mut commands, &skins, equipment, &snapshot);
    if snapshot.inventory.inventory.is_none() {
        label(&mut commands, &skins, equipment, "Inventory unavailable");
    }
    // The initial main-backpack snapshot remains usable before a container view is opened.
    if let Some(inventory) = &snapshot.inventory.inventory {
        let open = inventory.main_backpack.is_some_and(|backpack| {
            snapshot.containers.views.as_ref().is_some_and(|views| {
                views
                    .views
                    .iter()
                    .any(|view| view.container_handle == backpack.handle)
            })
        });
        if !open && !inventory.entries.is_empty() {
            let panel = panel(&mut commands, &skins, regions.containers, "Backpack");
            // In Manual Sort Mode the main backpack is named by its handle, so a slot drop sends
            // the slot (capability 510); otherwise it is the main backpack as before.
            let destination = match inventory.main_backpack {
                Some(backpack) if snapshot.manual_sort => {
                    ItemMoveDestination::Container(backpack.handle, None)
                }
                _ => ItemMoveDestination::MainBackpack,
            };
            grid(
                &mut commands,
                &skins,
                panel,
                snapshot.generation,
                Location::Inventory,
                &inventory.entries,
                inventory.entries.len(),
                Some(destination),
                snapshot.selection,
            );
        }
    }
    if let Some(views) = &snapshot.containers.views {
        for view in &views.views {
            container(&mut commands, &skins, regions.containers, view, &snapshot);
        }
    }
    if let Some(message) = snapshot.containers.feedback.or(snapshot.movement.feedback) {
        label(&mut commands, &skins, equipment, message);
    }
    if let Some(message) = &snapshot.world_feedback {
        label(&mut commands, &skins, equipment, message);
    }
    if snapshot.movement.pending {
        label(&mut commands, &skins, equipment, "Moving item…");
    }
    if let Some((slot, position)) = snapshot.menu {
        context(
            &mut commands,
            &skins,
            slot,
            position,
            (snapshot.write, snapshot.timed),
        );
    }
    previous.0 = Some(snapshot);
}
fn panel(commands: &mut Commands, skins: &Skins, parent: Entity, title: &str) -> Entity {
    titled_panel(commands, skins, parent, title, source::TEXT_COLOR)
}
fn titled_panel(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    title: &str,
    color: Color,
) -> Entity {
    let entity = commands
        .spawn((
            Projection,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: percent(100.),
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.,
                padding: UiRect::all(px(3.)),
                row_gap: px(3.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
    let title = commands
        .spawn((
            source::text(title, source::font(skins.font.clone(), false)),
            Node {
                height: px(14.),
                ..default()
            },
        ))
        .insert(TextColor(color))
        .id();
    commands.entity(entity).add_child(title);
    entity
}
pub(super) fn label(commands: &mut Commands, skins: &Skins, parent: Entity, text: &str) {
    let entity = commands
        .spawn(source::text(text, source::font(skins.font.clone(), true)))
        .insert((
            TextLayout::default(),
            Node {
                max_width: percent(100.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
}
/// Inventory.qml: the 108x142 equipment (46x42 minimised) with 32x32 slot silhouettes, the
/// 34x20 Soul and Cap boxes beside the feet and the 34x12 Blessings and Store inbox buttons.
const EQUIPMENT: Vec2 = Vec2::new(108., 142.);
const MINIMISED: Vec2 = Vec2::new(46., 42.);
const STAT_BOX: Vec2 = Vec2::new(34., 20.);
const SMALL_BUTTON: Vec2 = Vec2::new(34., 12.);
/// The silhouette of each equipment slot, in the order of the equipment slot list.
const SILHOUETTES: [&str; 10] = [
    "slot-head",
    "slot-necklace",
    "slot-backpack",
    "slot-armor",
    "slot-right-hand",
    "slot-left-hand",
    "slot-legs",
    "slot-feet",
    "slot-ring",
    "slot-ammo",
];
/// A button of the inventory widget.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum WidgetButton {
    Minimise,
    Blessings,
    Purse,
}
fn widget_button(
    event: On<bevy::ui_widgets::Activate>,
    buttons: Query<&WidgetButton>,
    guard: super::super::modal::ModalGuard,
    owner: NonSend<HostOwner>,
    widget: Option<ResMut<super::super::hud::EquipmentWidget>>,
) {
    if !guard.allows(event.entity) || owner.modal_active() {
        return;
    }
    let (Ok(button), Some(mut widget)) = (buttons.get(event.entity), widget) else {
        return;
    };
    match button {
        WidgetButton::Minimise => widget.minimised = !widget.minimised,
        WidgetButton::Blessings => widget.blessings = !widget.blessings,
        WidgetButton::Purse => {
            if let Some(play) = &owner.play {
                play.link.store_inbox_set_open(true);
            }
        }
    }
}
fn art_button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    button: WidgetButton,
    key: &'static str,
    size: Vec2,
) {
    let slot = super::super::art::fixed(commands, &skins.font, key, size);
    commands
        .entity(slot)
        .insert((button, Button))
        .observe(widget_button);
    commands.entity(parent).add_child(slot);
}
/// A Soul or Cap box: an 8 px caption over the value.
fn stat_box(commands: &mut Commands, skins: &Skins, parent: Entity, caption: &str, value: &str) {
    let entity = commands
        .spawn((
            Name::new(caption.to_owned()),
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: px(STAT_BOX.x),
                height: px(STAT_BOX.y),
                flex_shrink: 0.,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let caption = commands
        .spawn(source::text(
            caption,
            source::font(skins.font.clone(), true),
        ))
        .id();
    let value = commands
        .spawn(source::text(value, source::font(skins.font.clone(), true)))
        .id();
    commands.entity(entity).add_children(&[caption, value]);
    commands.entity(parent).add_child(entity);
}
mod slots;
use slots::{container, equipment_grid, grid};
