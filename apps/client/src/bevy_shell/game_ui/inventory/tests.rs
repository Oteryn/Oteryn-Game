use super::*;
use crate::play::{PlayLink, play_channel};
use oteryn_session::{AppliedDelta, CharacterInventory, SessionEvent};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn entry(handle: u64, count: u32) -> Result<ItemEntry, Box<dyn std::error::Error>> {
    Ok(ItemEntry {
        handle: handle.try_into()?,
        item_definition_ref: 42.try_into()?,
        count: count.try_into()?,
        sub_type: 0,
    })
}
fn host(link: PlayLink) -> Result<HostOwner, Box<dyn std::error::Error>> {
    let mut owner = HostOwner::new(None)?;
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    link.use_shared_frontend();
    owner.play = Some(super::super::super::owner::ActivePlay { view, link });
    owner.phase = super::super::super::owner::Phase::Playing;
    Ok(owner)
}
fn inventory(outbox: &crate::play::Outbox, items: Vec<ItemEntry>) {
    outbox.push(vec![SessionEvent::Inventory(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: CharacterInventory {
            main_backpack: None,
            entries: items,
            equipment: vec![],
        },
    })]);
}
#[test]
fn native_activate_opens_current_inventory_handle_once_and_rejects_removed_item() -> TestResult {
    let (link, mut commands, outbox) = play_channel();
    let item = entry(7, 3)?;
    inventory(&outbox, vec![item]);
    outbox.push(vec![SessionEvent::ContainerViews(AppliedDelta {
        server_sequence: 2,
        base_revision: 0,
        new_revision: 1,
        value: Default::default(),
    })]);
    let owner = host(link)?;
    let slot = Slot {
        generation: owner.generation(),
        location: Location::Inventory,
        item: Some(item),
        destination: Some(ItemMoveDestination::MainBackpack),
    };
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_non_send(owner)
        .init_resource::<InventoryUi>();
    let button = app
        .world_mut()
        .spawn(Control::Open(slot))
        .observe(activate)
        .id();
    app.world_mut().trigger(Activate { entity: button });
    assert_eq!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::ContainerView(
            oteryn_session::container_tree::ContainerViewIntent::Open {
                handle: item.handle,
                replace_view: None
            }
        )
    );
    app.world_mut().trigger(Activate { entity: button });
    assert!(commands.try_recv().is_err());
    inventory(&outbox, vec![]);
    app.world_mut().trigger(Activate { entity: button });
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn pointer_drag_uses_latest_stack_and_stale_session_sends_nothing() -> TestResult {
    let (link, mut commands, outbox) = play_channel();
    let old = entry(8, 30)?;
    inventory(&outbox, vec![old]);
    let owner = host(link)?;
    let slot = Slot {
        generation: owner.generation(),
        location: Location::Inventory,
        item: Some(old),
        destination: None,
    };
    let latest = entry(8, 7)?;
    inventory(&outbox, vec![latest]);
    assert_eq!(slot.drag(&owner).ok_or("drag")?.count, 7);
    let mut stale = slot;
    stale.generation = stale.generation.saturating_add(1);
    assert!(stale.drag(&owner).is_none());
    assert!(commands.try_recv().is_err());
    Ok(())
}
fn pointer() -> bevy::picking::events::Pointer {
    bevy::picking::events::Pointer::new(
        bevy::picking::pointer::PointerId::Mouse,
        bevy::picking::pointer::Location {
            target: bevy::camera::NormalizedRenderTarget::None {
                width: 100,
                height: 100,
            },
            position: Vec2::ZERO,
        },
    )
}
#[test]
fn pointer_drop_dispatches_real_move_without_predicting_inventory() -> TestResult {
    use bevy::picking::{
        backend::HitData,
        events::{PointerDragDrop, PointerDragStart},
        pointer::PointerButton,
    };
    let (link, mut commands, outbox) = play_channel();
    let item = entry(8, 7)?;
    inventory(&outbox, vec![item]);
    let owner = host(link)?;
    let generation = owner.generation();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_non_send(owner)
        .init_resource::<InventoryUi>();
    let source = app
        .world_mut()
        .spawn(Slot {
            generation,
            location: Location::Inventory,
            item: Some(item),
            destination: None,
        })
        .observe(actions::drag_start)
        .id();
    let target = app
        .world_mut()
        .spawn(Slot {
            generation,
            location: Location::Inventory,
            item: None,
            destination: Some(ItemMoveDestination::MainBackpack),
        })
        .observe(actions::drag_drop)
        .id();
    app.world_mut().trigger(PointerDragStart {
        entity: source,
        pointer: pointer(),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
    });
    app.world_mut().trigger(PointerDragDrop {
        entity: target,
        pointer: pointer(),
        button: PointerButton::Primary,
        dropped: source,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
    });
    assert_eq!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::MoveItem(oteryn_client_runtime::inventory::MoveRequest {
            source: item.handle,
            destination: ItemMoveDestination::MainBackpack,
            count: None
        })
    );
    let owner = app.world().non_send::<HostOwner>();
    assert_eq!(
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .inventory_panel()
            .rows(),
        vec![item]
    );
    assert!(
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .item_movement()
            .pending
    );
    Ok(())
}
#[test]
fn live_inventory_delta_updates_visible_cells_without_inventing_items() -> TestResult {
    use bevy::asset::AssetPlugin;
    let (link, _commands, outbox) = play_channel();
    let item = entry(11, 12)?;
    inventory(&outbox, vec![item]);
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<ButtonInput<MouseButton>>()
    .insert_non_send(host(link)?)
    .add_systems(
        Startup,
        (super::super::super::ui::setup, super::super::setup).chain(),
    );
    install(&mut app);
    app.update();
    let visible: Vec<_> = app
        .world_mut()
        .query::<&super::super::ItemIcon>()
        .iter(app.world())
        .map(|icon| icon.0)
        .collect();
    assert_eq!(visible, vec![item]);
    let changed = entry(11, 4)?;
    inventory(&outbox, vec![changed]);
    app.update();
    let visible: Vec<_> = app
        .world_mut()
        .query::<&super::super::ItemIcon>()
        .iter(app.world())
        .map(|icon| icon.0)
        .collect();
    assert_eq!(visible, vec![changed]);
    inventory(&outbox, vec![]);
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&super::super::ItemIcon>()
            .iter(app.world())
            .count(),
        0
    );
    Ok(())
}
#[test]
fn split_buttons_dispatch_selected_count_once_and_reject_an_old_prompt() -> TestResult {
    let (link, mut commands, outbox) = play_channel();
    let item = entry(13, 9)?;
    PlayLink::bevy_items_fixture(
        &outbox,
        Some(CharacterInventory {
            main_backpack: None,
            entries: vec![item],
            equipment: vec![],
        }),
        Some(Default::default()),
        true,
        true,
    );
    assert!(link.item_drag_begin(DragItem {
        equipped: false,
        handle: item.handle,
        count: item.count.get()
    }));
    link.item_drag_drop(Some(ItemMoveDestination::MainBackpack));
    let owner = host(link)?;
    let prompt = Prompt {
        generation: owner.generation(),
        item: DragItem {
            equipped: false,
            handle: item.handle,
            count: item.count.get(),
        },
        destination: ItemMoveDestination::MainBackpack,
    };
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_non_send(owner)
        .insert_resource(InventoryUi {
            prompt: Some(prompt),
            split_count: 9,
            ..default()
        });
    let decrease = app
        .world_mut()
        .spawn(Control::Count(prompt, -1))
        .observe(activate)
        .id();
    let mut stale = prompt;
    stale.generation = stale.generation.saturating_add(1);
    let old = app
        .world_mut()
        .spawn(Control::Confirm(stale))
        .observe(activate)
        .id();
    app.world_mut().trigger(Activate { entity: old });
    assert!(commands.try_recv().is_err());
    app.world_mut().trigger(Activate { entity: decrease });
    app.world_mut().trigger(Activate { entity: decrease });
    let confirm = app
        .world_mut()
        .spawn(Control::Confirm(prompt))
        .observe(activate)
        .id();
    app.world_mut().trigger(Activate { entity: confirm });
    assert_eq!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::MoveItem(oteryn_client_runtime::inventory::MoveRequest {
            source: item.handle,
            destination: ItemMoveDestination::MainBackpack,
            count: Some(7)
        })
    );
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(commands.try_recv().is_err());
    Ok(())
}
