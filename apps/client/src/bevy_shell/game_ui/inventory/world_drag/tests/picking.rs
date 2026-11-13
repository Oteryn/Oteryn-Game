//! Headless upstream picking interfaces + normalized native bridge; no OS delivery claim.
use super::*;
use bevy::{
    ecs::system::RunSystemOnce,
    picking::{
        backend::HitData,
        events::{Pointer, PointerDragDrop, PointerDragStart, PointerPress},
        pointer::{Location as PointerLocation, PointerButton, PointerId},
    },
    window::PrimaryWindow,
};
#[derive(Resource)]
struct Packet {
    point: Vec2,
    event: NormalizedInputEvent,
    viewport: WorldViewport,
}
fn native(
    mut capture: super::super::super::ItemPointerCapture,
    mut owner: NonSendMut<HostOwner>,
    packet: Res<Packet>,
) {
    let consumed = capture.before(
        &owner,
        packet.viewport,
        Some(packet.point),
        &packet.event,
        true,
    );
    let mapped = owner.play.as_ref().and_then(|play| {
        let (w, h) = play.view.scene().view().viewport_px();
        packet.viewport.scene_point(packet.point, UVec2::new(w, h))
    });
    if let Some(local) = mapped {
        let event = if matches!(packet.event, NormalizedInputEvent::PointerMoved { .. }) {
            motion(local).ok()
        } else {
            Some(packet.event.clone())
        };
        if let Some(event) = event {
            owner.route_input(&[event], consumed);
        }
    } else {
        owner.route_input(&[NormalizedInputEvent::DeviceLost], true);
    }
    capture.after_press(
        &owner,
        packet.viewport,
        Some(packet.point),
        &packet.event,
        true,
        mapped.is_some(),
    );
}
fn route(
    app: &mut App,
    viewport: WorldViewport,
    point: Vec2,
    event: NormalizedInputEvent,
) -> TestResult {
    app.insert_resource(Packet {
        viewport,
        point,
        event,
    });
    app.world_mut()
        .run_system_once(native)
        .map_err(|error| format!("native bridge: {error:?}"))?;
    Ok(())
}
pub(super) fn pointer(window: Entity, physical: Vec2, dpi: f32) -> Pointer {
    Pointer::new(
        PointerId::Mouse,
        PointerLocation {
            target: bevy::window::WindowRef::Entity(window)
                .normalize(None)
                .map(bevy::camera::NormalizedRenderTarget::Window)
                .unwrap_or(bevy::camera::NormalizedRenderTarget::None {
                    width: 1,
                    height: 1,
                }),
            position: physical / dpi,
        },
    )
}
#[test]
fn bevy_press_and_drop_supply_bound_container_target_then_normalized_release_queues_once()
-> TestResult {
    let (owner, ui, gesture, viewport, point, mut rx) = fixture(false, false)?;
    let generation = owner.generation();
    let mut app = App::new();
    super::super::super::initialize_item_pointer_test_state(&mut app);
    app.insert_non_send(owner)
        .insert_resource(ui)
        .insert_resource(gesture)
        .init_resource::<FrameCount>()
        .add_observer(picking::press);
    let mut window = Window::default();
    window.resolution.set_scale_factor_override(Some(2.));
    let window = app.world_mut().spawn((window, PrimaryWindow)).id();
    let world = app.world_mut().spawn((WorldRegion, Node::default())).id();
    geometry(
        &mut app,
        world,
        viewport.origin.as_vec2() + viewport.size.as_vec2() * 0.5,
        viewport.size.as_vec2(),
    );
    let child = app.world_mut().spawn(Node::default()).id();
    geometry(&mut app, child, point, Vec2::splat(16.));
    app.world_mut().entity_mut(world).add_child(child);
    let target = app
        .world_mut()
        .spawn((
            Slot {
                generation,
                location: Location::Inventory,
                item: None,
                destination: Some(ItemMoveDestination::MainBackpack),
            },
            Node::default(),
        ))
        .observe(super::super::super::actions::drag_drop)
        .id();
    current_pointer(&mut app, window, point, 2.);
    app.world_mut().trigger(PointerPress {
        entity: child,
        pointer: pointer(window, point, 2.),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        count: 1,
    });
    route(&mut app, viewport, point, motion(point)?)?;
    route(&mut app, viewport, point, button(ButtonState::Pressed))?;
    let end = Vec2::new(20., 20.);
    route(&mut app, viewport, end, motion(end)?)?;
    app.world_mut().trigger(PointerDragDrop {
        entity: target,
        dropped: child,
        pointer: pointer(window, end, 2.),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
    });
    route(&mut app, viewport, end, button(ButtonState::Released))?;
    let PlayCommand::MoveItem(request) = rx.try_recv()? else {
        return Err("container move missing".into());
    };
    assert_eq!(request.source, 7.try_into()?);
    assert_eq!(request.destination, ItemMoveDestination::MainBackpack);
    route(&mut app, viewport, end, button(ButtonState::Released))?;
    assert!(rx.try_recv().is_err());
    Ok(())
}
#[test]
fn bevy_carried_drag_to_world_uses_physical_letterboxed_tile_and_unmatched_overlay_is_refused()
-> TestResult {
    for hit in 0..3 {
        let (owner, ui, gesture, viewport, point, mut rx) = fixture(false, false)?;
        let generation = owner.generation();
        let slot = Slot {
            generation,
            location: Location::Inventory,
            item: Some(super::super::super::tests::entry(8, 9)?),
            destination: None,
        };
        let mut app = App::new();
        app.insert_non_send(owner)
            .insert_resource(ui)
            .insert_resource(gesture)
            .init_resource::<FrameCount>()
            .add_observer(picking::press);
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        let world = app
            .world_mut()
            .spawn((WorldRegion, Node::default()))
            .observe(picking::world_drop)
            .id();
        let source = app
            .world_mut()
            .spawn((slot, Node::default()))
            .observe(super::super::super::actions::drag_start)
            .id();
        let start = Vec2::new(20., 20.);
        geometry(&mut app, source, start, Vec2::splat(32.));
        current_pointer(&mut app, window, start, 1.);
        app.world_mut().trigger(PointerPress {
            entity: source,
            pointer: pointer(window, start, 1.),
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
            count: 1,
        });
        route(&mut app, viewport, start, button(ButtonState::Pressed))?;
        app.world_mut().trigger(PointerDragStart {
            entity: source,
            pointer: pointer(window, start + Vec2::X * 8., 1.),
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        });
        route(&mut app, viewport, point, motion(point)?)?;
        if hit != 1 {
            let target = app.world_mut().spawn(Node::default()).id();
            app.world_mut().entity_mut(world).add_child(target);
            if hit == 2 {
                app.world_mut()
                    .entity_mut(target)
                    .insert(bevy::ui_widgets::Button);
            }
            app.world_mut().trigger(PointerDragDrop {
                entity: target,
                dropped: source,
                pointer: pointer(window, point, 1.),
                button: PointerButton::Primary,
                hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
            });
        }
        route(&mut app, viewport, point, button(ButtonState::Released))?;
        if hit == 0 {
            let PlayCommand::MoveItem(request) = rx.try_recv()? else {
                return Err("carried ground drop missing".into());
            };
            assert_eq!(request.source, 8.try_into()?);
            assert_eq!(
                request.destination,
                ItemMoveDestination::Ground(oteryn_session::item_movement::WorldTilePosition {
                    x: 100,
                    y: 100,
                    floor: 0
                })
            );
        } else {
            assert!(rx.try_recv().is_err());
        }
        assert!(rx.try_recv().is_err());
    }
    Ok(())
}
#[test]
fn press_tokens_do_not_cross_two_same_frame_gestures_or_session_generation() -> TestResult {
    let mut ledger = picking::Ledger::default();
    let entity = Entity::PLACEHOLDER;
    let point = Vec2::new(10., 20.);
    ledger.press(0, 3, entity, picking::Source::World, point);
    ledger.drop_on(0, 3, entity, picking::Destination::World);
    ledger.press(0, 3, entity, picking::Source::World, point);
    let first = ledger.claim(0, 3, point).ok_or("first press")?;
    let second = ledger.claim(0, 3, point).ok_or("second press")?;
    assert!(ledger.take_drop(0, second.token).is_none());
    assert!(ledger.take_drop(0, first.token).is_some());
    assert!(ledger.take_drop(0, first.token).is_none());
    ledger.press(1, 3, entity, picking::Source::World, point);
    assert!(ledger.claim(1, 4, point).is_none());
    Ok(())
}

#[test]
fn projected_carried_slot_survives_drag_and_authoritative_removal_cancels() -> TestResult {
    let (owner, _, _, _, _, _) = fixture(false, false)?;
    let (link, mut rx, outbox) = play_channel();
    link.use_shared_frontend();
    let item = super::super::super::tests::entry(8, 9)?;
    crate::play::PlayLink::bevy_items_fixture(
        &outbox,
        Some(CharacterInventory {
            main_backpack: Some(super::super::super::tests::entry(9, 1)?),
            entries: vec![item],
            equipment: vec![],
        }),
        Some(Default::default()),
        false,
        true,
    );
    let mut owner = owner;
    owner.play.as_mut().ok_or("play")?.link = link;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<bevy::input_focus::InputFocus>()
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<ButtonInput<bevy::input::mouse::MouseButton>>()
    .insert_non_send(owner)
    .add_systems(
        Startup,
        (
            crate::bevy_shell::ui::setup,
            super::super::super::super::setup,
        )
            .chain(),
    );
    super::super::super::install(&mut app);
    app.update();
    let source = app
        .world_mut()
        .query::<(Entity, &Slot)>()
        .iter(app.world())
        .find_map(|(entity, slot)| (slot.item == Some(item)).then_some(entity))
        .ok_or("carried source")?;
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    geometry(&mut app, source, Vec2::splat(20.), Vec2::splat(32.));
    current_pointer(&mut app, window, Vec2::splat(20.), 1.);
    app.world_mut().trigger(PointerPress {
        entity: source,
        pointer: pointer(window, Vec2::splat(20.), 1.),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        count: 1,
    });
    app.world_mut()
        .resource_mut::<ButtonInput<bevy::input::mouse::MouseButton>>()
        .press(bevy::input::mouse::MouseButton::Left);
    app.world_mut().trigger(PointerDragStart {
        entity: source,
        pointer: super::super::super::tests::pointer(),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
    });
    app.update();
    assert!(
        app.world().get_entity(source).is_ok(),
        "project must retain live native source"
    );
    outbox.push(vec![oteryn_session::SessionEvent::Inventory(
        oteryn_session::AppliedDelta {
            server_sequence: 2,
            base_revision: 0,
            new_revision: 1,
            value: CharacterInventory {
                main_backpack: None,
                entries: vec![],
                equipment: vec![],
            },
        },
    )]);
    app.update();
    assert!(app.world().get_entity(source).is_err());
    assert!(app.world().resource::<InventoryUi>().dragged.is_none());
    assert!(rx.try_recv().is_err());
    Ok(())
}

fn geometry(app: &mut App, entity: Entity, center: Vec2, size: Vec2) {
    app.world_mut().entity_mut(entity).insert((
        ComputedNode { size, ..default() },
        UiGlobalTransform::from_translation(center),
    ));
}

pub(super) fn current_pointer(app: &mut App, window: Entity, physical: Vec2, dpi: f32) {
    app.init_resource::<bevy::picking::hover::HoverMap>()
        .init_resource::<bevy::picking::pointer::PointerMap>();
    let pointer = pointer(window, physical, dpi);
    app.world_mut().spawn((
        PointerId::Mouse,
        bevy::picking::pointer::PointerLocation::new(PointerLocation {
            target: pointer.target,
            position: pointer.position,
        }),
    ));
}
