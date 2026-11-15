use super::picking_tests::{current_pointer, pointer, route};
use super::*;
use bevy::{
    picking::{
        backend::HitData,
        events::{PointerDragDrop, PointerDragStart, PointerPress},
        pointer::{PointerButton, PointerId},
    },
    window::PrimaryWindow,
};
fn blocked_fixture(
    counted: bool,
) -> Result<(Fixture, crate::play::Outbox), Box<dyn std::error::Error>> {
    let (mut owner, ui, gesture, viewport, point, _) = fixture(counted, false)?;
    let (link, rx, outbox) = play_channel();
    link.use_shared_frontend();
    crate::play::PlayLink::bevy_items_fixture(
        &outbox,
        Some(CharacterInventory {
            main_backpack: Some(super::super::super::tests::entry(9, 1)?),
            entries: vec![super::super::super::tests::entry(8, 9)?],
            equipment: vec![],
        }),
        Some(Default::default()),
        counted,
        true,
    );
    owner.play.as_mut().ok_or("play")?.link = link;
    Ok(((owner, ui, gesture, viewport, point, rx), outbox))
}
fn block(owner: &HostOwner, outbox: &crate::play::Outbox, lens: bool) -> TestResult {
    if lens {
        assert!(owner.play.as_ref().ok_or("play")?.link.lens_toggle());
    } else {
        outbox.push(vec![oteryn_session::SessionEvent::ActorVitals(
            oteryn_session::AppliedDelta {
                server_sequence: 2,
                base_revision: 0,
                new_revision: 1,
                value: oteryn_session::ActorVitals {
                    health: 0,
                    max_health: 100,
                    ..default()
                },
            },
        )]);
    }
    let play = owner.play.as_ref().ok_or("play")?;
    assert!(play.view.gameplay_input_blocked(&play.link));
    Ok(())
}
#[test]
fn blocked_gameplay_cannot_start_or_release_a_ground_item_drag() -> TestResult {
    for lens in [false, true] {
        for already_active in [false, true] {
            let ((mut owner, mut ui, mut gesture, viewport, point, mut rx), outbox) =
                blocked_fixture(false)?;
            if already_active {
                start(&mut owner, &mut ui, &mut gesture, viewport, point)?;
                assert!(gesture.active);
            }
            block(&owner, &outbox, lens)?;
            if !already_active {
                start(&mut owner, &mut ui, &mut gesture, viewport, point)?;
            }
            note_world(&mut gesture);
            step(
                &mut owner,
                &mut ui,
                &mut gesture,
                viewport,
                point + Vec2::X * 48.,
                button(ButtonState::Released),
            )?;
            assert!(
                rx.try_recv().is_err(),
                "blocked source lens={lens}, active={already_active}"
            );
            assert!(!gesture.active);
        }
    }
    Ok(())
}
#[test]
fn blocked_gameplay_cannot_confirm_ground_source_or_ground_destination_stack() -> TestResult {
    for lens in [false, true] {
        for ground_source in [false, true] {
            let ((mut owner, mut ui, mut gesture, viewport, point, mut rx), outbox) =
                blocked_fixture(true)?;
            let destination = if ground_source {
                start(&mut owner, &mut ui, &mut gesture, viewport, point)?;
                gesture.picking.drop_on(
                    0,
                    owner.generation(),
                    Entity::PLACEHOLDER,
                    picking::Destination::Slot(Slot {
                        generation: owner.generation(),
                        location: Location::Inventory,
                        item: None,
                        destination: Some(ItemMoveDestination::MainBackpack),
                    }),
                );
                step(
                    &mut owner,
                    &mut ui,
                    &mut gesture,
                    viewport,
                    Vec2::ZERO,
                    button(ButtonState::Released),
                )?;
                ItemMoveDestination::MainBackpack
            } else {
                let play = owner.play.as_ref().ok_or("play")?;
                let carried = super::super::super::tests::entry(8, 9)?;
                assert!(play.link.item_drag_begin(DragItem {
                    equipped: false,
                    handle: carried.handle,
                    count: 9
                }));
                let destination =
                    ItemMoveDestination::Ground(oteryn_session::item_movement::WorldTilePosition {
                        x: 101,
                        y: 100,
                        floor: 0,
                    });
                play.link.item_drag_drop(Some(destination));
                destination
            };
            let (item, actual) = owner
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .item_movement()
                .prompt
                .ok_or("stack prompt")?;
            assert_eq!(actual, destination);
            let prompt = Prompt {
                generation: owner.generation(),
                item,
                destination,
            };
            ui.prompt = Some(prompt);
            ui.split_text = Some("3".into());
            ui.split_count = 3;
            block(&owner, &outbox, lens)?;
            super::super::super::amount::confirm(&owner, &mut ui, prompt);
            assert!(
                rx.try_recv().is_err(),
                "blocked confirm lens={lens}, ground_source={ground_source}"
            );
            assert!(
                owner
                    .play
                    .as_ref()
                    .ok_or("play")?
                    .link
                    .item_movement()
                    .prompt
                    .is_some()
            );
        }
    }
    Ok(())
}
#[test]
fn historical_world_drop_cannot_borrow_frame_final_world_hit() -> TestResult {
    historical_drop(0)
}
#[test]
fn historical_slot_drop_cannot_borrow_frame_final_slot_hit() -> TestResult {
    historical_drop(1)
}
#[test]
fn historical_carried_slot_drop_is_rejected_before_legacy_dispatch() -> TestResult {
    historical_drop(2)
}
fn historical_drop(kind: u8) -> TestResult {
    let (owner, ui, gesture, viewport, point, mut rx) = fixture(false, false)?;
    let generation = owner.generation();
    let mut app = App::new();
    super::super::super::initialize_item_pointer_test_state(&mut app);
    app.insert_non_send(owner)
        .insert_resource(ui)
        .insert_resource(gesture)
        .add_observer(picking::press);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    let world = app
        .world_mut()
        .spawn((
            WorldRegion,
            Node::default(),
            ComputedNode {
                size: viewport.size.as_vec2(),
                ..default()
            },
            UiGlobalTransform::from_translation(
                viewport.origin.as_vec2() + viewport.size.as_vec2() * 0.5,
            ),
        ))
        .observe(picking::world_drop)
        .id();
    let slot = Slot {
        generation,
        location: Location::Inventory,
        item: Some(super::super::super::tests::entry(8, 9)?),
        destination: None,
    };
    let source = if kind == 2 {
        app.world_mut()
            .spawn((
                slot,
                Node::default(),
                ComputedNode {
                    size: Vec2::splat(32.),
                    ..default()
                },
                UiGlobalTransform::from_translation(Vec2::splat(20.)),
            ))
            .observe(super::super::super::actions::drag_start)
            .id()
    } else {
        world
    };
    let target = if kind == 0 {
        world
    } else {
        app.world_mut()
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
            .id()
    };
    let start_point = if kind == 2 { Vec2::splat(20.) } else { point };
    current_pointer(&mut app, window, start_point, 1.);
    app.world_mut().trigger(PointerPress {
        entity: source,
        pointer: pointer(window, start_point, 1.),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        count: 1,
    });
    route(&mut app, viewport, start_point, motion(start_point)?)?;
    route(
        &mut app,
        viewport,
        start_point,
        button(ButtonState::Pressed),
    )?;
    if kind == 2 {
        app.world_mut().trigger(PointerDragStart {
            entity: source,
            pointer: pointer(window, start_point, 1.),
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        });
    }
    let release_a = if kind == 2 {
        Vec2::splat(24.)
    } else {
        point + Vec2::X * 8.
    };
    route(&mut app, viewport, release_a, motion(release_a)?)?;
    let final_b = if kind == 0 {
        point + Vec2::X * 48.
    } else {
        Vec2::splat(60.)
    };
    current_pointer(&mut app, window, final_b, 1.);
    app.world_mut()
        .resource_mut::<bevy::picking::hover::HoverMap>()
        .entry(PointerId::Mouse)
        .or_default()
        .insert(target, HitData::new(Entity::PLACEHOLDER, 0., None, None));
    app.world_mut().trigger(PointerDragDrop {
        entity: target,
        dropped: source,
        pointer: pointer(window, release_a, 1.),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
    });
    assert!(
        rx.try_recv().is_err(),
        "historical release must not dispatch during picking"
    );
    route(&mut app, viewport, release_a, button(ButtonState::Released))?;
    assert!(
        rx.try_recv().is_err(),
        "releaseA cannot borrow finalB target kind={kind}"
    );
    Ok(())
}

#[test]
fn blocked_carried_world_release_keeps_ordinary_inventory_moves_available() -> TestResult {
    for lens in [false, true] {
        for world in [false, true] {
            let ((owner, mut ui, mut gesture, viewport, point, mut rx), outbox) =
                blocked_fixture(false)?;
            let carried = super::super::super::tests::entry(8, 9)?;
            let slot = Slot {
                generation: owner.generation(),
                location: Location::Inventory,
                item: Some(carried),
                destination: None,
            };
            assert!(
                owner
                    .play
                    .as_ref()
                    .ok_or("play")?
                    .link
                    .item_drag_begin(slot.drag(&owner).ok_or("carried source")?)
            );
            ui.dragged = Some(slot);
            gesture.picking.press(
                0,
                owner.generation(),
                Entity::PLACEHOLDER,
                picking::Source::Carried(slot),
                Vec2::splat(20.),
            );
            after_press(
                &mut ui,
                &mut gesture,
                0,
                &owner,
                viewport,
                Some(Vec2::splat(20.)),
                &button(ButtonState::Pressed),
                true,
                false,
            );
            let destination = if world {
                picking::Destination::World
            } else {
                picking::Destination::Slot(Slot {
                    generation: owner.generation(),
                    location: Location::Inventory,
                    item: None,
                    destination: Some(ItemMoveDestination::MainBackpack),
                })
            };
            gesture
                .picking
                .drop_on(0, owner.generation(), Entity::PLACEHOLDER, destination);
            block(&owner, &outbox, lens)?;
            before(
                &mut ui,
                &mut gesture,
                0,
                &owner,
                viewport,
                Some(point),
                &button(ButtonState::Released),
                true,
            );
            if world {
                assert!(
                    rx.try_recv().is_err(),
                    "world destination blocked lens={lens}"
                );
            } else {
                let PlayCommand::MoveItem(request) = rx.try_recv()? else {
                    return Err("ordinary inventory unavailable".into());
                };
                assert_eq!(request.destination, ItemMoveDestination::MainBackpack);
            }
        }
    }
    Ok(())
}
