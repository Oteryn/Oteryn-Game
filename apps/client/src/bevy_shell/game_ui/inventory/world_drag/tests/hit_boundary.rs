use super::*;
use bevy::{
    picking::{
        backend::HitData,
        events::{PointerDragDrop, PointerDragStart, PointerPress},
        pointer::PointerButton,
    },
    window::PrimaryWindow,
};
#[test]
fn final_hover_slot_outside_real_press_bounds_cannot_move_a_different_carried_item() -> TestResult {
    reject_ambiguous_press(Vec2::splat(200.), Vec2::splat(200.))
}
#[test]
fn final_hover_slot_with_covered_press_and_uncovered_final_position_cannot_move() -> TestResult {
    reject_ambiguous_press(Vec2::splat(20.), Vec2::new(25., 20.))
}
fn reject_ambiguous_press(historical: Vec2, current: Vec2) -> TestResult {
    let (owner, ui, gesture, _, _, mut rx) = fixture(false, false)?;
    let generation = owner.generation();
    let mut app = App::new();
    app.insert_non_send(owner)
        .insert_resource(ui)
        .insert_resource(gesture)
        .init_resource::<FrameCount>()
        .init_resource::<bevy::picking::pointer::PointerMap>()
        .init_resource::<bevy::picking::hover::HoverMap>()
        .add_observer(picking::press);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    let source = app
        .world_mut()
        .spawn((
            Slot {
                generation,
                location: Location::Inventory,
                item: Some(super::super::super::tests::entry(8, 9)?),
                destination: None,
            },
            Node::default(),
            ComputedNode {
                size: Vec2::splat(32.),
                ..default()
            },
            UiGlobalTransform::from_translation(Vec2::splat(20.)),
        ))
        .observe(super::super::super::actions::drag_start)
        .id();
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
    let latest = picking_tests::pointer(window, current, 1.);
    app.world_mut().spawn((
        bevy::picking::pointer::PointerId::Mouse,
        bevy::picking::pointer::PointerLocation::new(bevy::picking::pointer::Location {
            target: latest.target,
            position: latest.position,
        }),
    ));
    let pointer = picking_tests::pointer(window, historical, 1.);
    app.world_mut().trigger(PointerPress {
        entity: source,
        pointer: pointer.clone(),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        count: 1,
    });
    app.world_mut().trigger(PointerDragStart {
        entity: source,
        pointer: pointer.clone(),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
    });
    app.world_mut().trigger(PointerDragDrop {
        entity: target,
        dropped: source,
        pointer,
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
    });
    assert!(
        rx.try_recv().is_err(),
        "a final-hover source must not own an unmatched historical press"
    );
    Ok(())
}
