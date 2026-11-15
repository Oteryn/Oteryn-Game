use super::*;
use crate::play::{PlayLink, play_channel};
use oteryn_session::{AppliedDelta, CharacterInventory, SessionEvent};
type TestResult = Result<(), Box<dyn std::error::Error>>;
pub(super) fn entry(handle: u64, count: u32) -> Result<ItemEntry, Box<dyn std::error::Error>> {
    Ok(ItemEntry {
        appearance_id: 0,
        handle: handle.try_into()?,
        item_definition_ref: 42.try_into()?,
        count: count.try_into()?,
        sub_type: 0,
    })
}
pub(super) fn host(link: PlayLink) -> Result<HostOwner, Box<dyn std::error::Error>> {
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
pub(super) fn pointer() -> bevy::picking::events::Pointer {
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
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Count: 12\nRight-click for Look and item actions")
    );
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

#[test]
fn modal_fence_context_menu_blocks_background_and_keeps_its_own_open_command() -> TestResult {
    use bevy::asset::AssetPlugin;
    use bevy::ecs::system::RunSystemOnce;
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
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .insert_non_send(owner)
        .init_resource::<InventoryUi>()
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    app.update();
    let outside = app
        .world_mut()
        .spawn(Control::Open(slot))
        .observe(activate)
        .id();
    app.world_mut()
        .run_system_once(move |mut commands: Commands, skins: Res<Skins>| {
            dialogs::context(&mut commands, &skins, slot, Vec2::ZERO)
        })
        .map_err(|error| format!("context menu fixture: {error:?}"))?;
    app.world_mut().trigger(Activate { entity: outside });
    assert!(
        commands.try_recv().is_err(),
        "background open escaped the context menu"
    );
    let child = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            (entity != outside && matches!(control, Control::Open(_))).then_some(entity)
        })
        .ok_or("context Open")?;
    app.world_mut().trigger(Activate { entity: child });
    assert_eq!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::ContainerView(
            oteryn_session::container_tree::ContainerViewIntent::Open {
                handle: item.handle,
                replace_view: None
            }
        )
    );
    Ok(())
}

#[path = "tests/amount.rs"]
mod amount;
#[test]
fn inventory_tooltip_obeys_delay_preferences_and_current_stack() -> TestResult {
    use bevy::{asset::AssetPlugin, math::DVec2, picking::hover::Hovered, window::PrimaryWindow};
    use std::time::Duration;
    let (link, mut commands, outbox) = play_channel();
    let item = entry(71, 12)?;
    inventory(&outbox, vec![item]);
    let mut owner = host(link)?;
    let mut preferences = owner.settings.preferences.clone();
    preferences.tooltips_enabled = true;
    preferences.tooltip_delay_ms = 100;
    owner.settings.apply(preferences);
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
    .insert_non_send(owner)
    .add_systems(
        Startup,
        (crate::bevy_shell::ui::setup, super::super::setup).chain(),
    );
    let mut window = Window {
        focused: true,
        ..default()
    };
    window.set_physical_cursor_position(Some(DVec2::new(40., 40.)));
    app.world_mut().spawn((window, PrimaryWindow));
    install(&mut app);
    app.add_systems(
        Update,
        super::super::projection::project.before(super::tooltips::hover),
    );
    app.update();
    let cell = app
        .world_mut()
        .query::<(Entity, &Slot)>()
        .iter(app.world())
        .find(|(_, slot)| slot.item == Some(item))
        .map(|(entity, _)| entity)
        .ok_or("cell")?;
    app.world_mut().entity_mut(cell).insert(Hovered(true));
    app.update();
    let visible = |app: &mut App| {
        app.world_mut()
            .query_filtered::<&Node, With<super::tooltips::Tooltip>>()
            .iter(app.world())
            .any(|node| node.display == Display::Flex)
    };
    assert!(!visible(&mut app));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(101),
    ));
    app.update();
    assert!(visible(&mut app));
    assert!(commands.try_recv().is_err(), "hover must not send commands");
    assert!(
        app.world_mut()
            .query_filtered::<Option<&ChildOf>, With<super::tooltips::Tooltip>>()
            .iter(app.world())
            .all(|parent| parent.is_none()),
        "tooltip must escape sidebar clipping"
    );
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, Node::default(), GlobalZIndex(200)))
        .id();
    app.update();
    assert!(!visible(&mut app));
    app.world_mut().entity_mut(modal).despawn();
    app.update();
    app.update();
    assert!(visible(&mut app));

    {
        use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
        use oteryn_client_ui::inspection::{InspectedItem, Reply, Request};
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        link.bevy_item_details_selected(true);
        assert!(link.inspect(Request::Catalogue(42)));
        let _ = commands.try_recv()?;
        let reply = Reply {
            status: 1,
            requests: vec![],
            catalogue: None,
            items: vec![InspectedItem {
                slot: 0,
                definition_ref: 42,
                count: 1,
                tier: 0,
                name: "Silver sword".into(),
                attributes: vec![("Attack".into(), "25".into())],
            }],
        };
        link.bevy_item_details_response(&ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: oteryn_client_ui::inspection::encode_reply(&reply).map_err(|_| "reply")?,
        });
    }
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0
                == "Silver sword\nAttack: 25\nCount: 12\nRight-click for Look and item actions")
    );
    let mut preferences = app
        .world()
        .non_send::<HostOwner>()
        .settings
        .preferences
        .clone();
    preferences.tooltips_enabled = false;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .apply(preferences);
    app.update();
    assert!(!visible(&mut app));
    inventory(&outbox, vec![entry(71, 4)?]);
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Count: 4\nRight-click for Look and item actions")
    );
    inventory(&outbox, vec![]);
    app.update();
    assert!(!visible(&mut app));
    Ok(())
}

#[test]
fn equipment_and_open_container_cells_receive_tooltips() -> TestResult {
    use bevy::asset::AssetPlugin;
    use oteryn_session::container_tree::{ContainerView, ContainerViews};
    let (link, _commands, outbox) = play_channel();
    PlayLink::bevy_items_fixture(
        &outbox,
        Some(CharacterInventory {
            main_backpack: None,
            entries: vec![],
            equipment: vec![oteryn_session::item_movement::EquippedItem {
                slot: EquipmentSlot::Head,
                item: entry(91, 2)?,
            }],
        }),
        Some(ContainerViews {
            views: vec![ContainerView {
                view_id: 0,
                container_handle: 90.try_into()?,
                parent_handle: None,
                capacity: 20,
                entries: vec![entry(92, 3)?],
                browse: None,
            }],
        }),
        true,
        true,
    );
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
        (crate::bevy_shell::ui::setup, super::super::setup).chain(),
    );
    install(&mut app);
    app.update();
    let mut texts = app
        .world_mut()
        .query_filtered::<&Text, With<super::tooltips::Tooltip>>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect::<Vec<_>>();
    texts.sort();
    assert_eq!(
        texts,
        vec![
            "Count: 2\nRight-click for Look and item actions",
            "Count: 3\nRight-click for Look and item actions",
        ]
    );
    Ok(())
}
