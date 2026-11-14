use super::*;
use bevy::picking::{backend::HitData, pointer::Location};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn app(dpi: f32, scale: f32) -> (App, Entity, Entity) {
    let mut app = App::new();
    app.insert_resource(UiScale(scale));
    super::super::game_ui::modal::install(&mut app);
    install(&mut app);
    let mut window = Window {
        resolution: (800, 600).into(),
        focused: true,
        ..default()
    };
    window.resolution.set_scale_factor_override(Some(dpi));
    let window = app.world_mut().spawn((window, PrimaryWindow)).id();
    let factor = dpi * scale;
    let panel = app
        .world_mut()
        .spawn((
            DraggableDialog,
            super::super::game_ui::GameModal,
            GlobalZIndex(100),
            ComputedNode {
                size: Vec2::new(200., 100.) * factor,
                inverse_scale_factor: factor.recip(),
                ..default()
            },
            UiGlobalTransform::from_translation(Vec2::new(400., 300.) * factor),
        ))
        .id();
    (app, window, panel)
}
fn pointer(
    window: Entity,
    position: Vec2,
    id: PointerId,
) -> std::result::Result<Pointer, Box<dyn std::error::Error>> {
    Ok(Pointer::new(
        id,
        Location {
            target: NormalizedRenderTarget::Window(
                WindowRef::Entity(window)
                    .normalize(None)
                    .ok_or("window target")?,
            ),
            position,
        },
    ))
}
fn press_at(
    app: &mut App,
    target: Entity,
    window: Entity,
    position: Vec2,
    id: PointerId,
    button: PointerButton,
) -> Result {
    app.world_mut().trigger(PointerPress {
        entity: target,
        pointer: pointer(window, position, id)?,
        button,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        count: 1,
    });
    Ok(())
}
// Exercise the actual upstream pointer metadata, target and full drag lifecycle.
#[allow(clippy::too_many_arguments)]
fn move_to(
    app: &mut App,
    target: Entity,
    window: Entity,
    origin: Vec2,
    distance: Vec2,
    id: PointerId,
    button: PointerButton,
    start_drag: bool,
) -> Result {
    if start_drag {
        app.world_mut().trigger(PointerDragStart {
            entity: target,
            pointer: pointer(window, origin + distance, id)?,
            button,
            hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        });
    }
    app.world_mut().trigger(PointerDrag {
        entity: target,
        pointer: pointer(window, origin + distance, id)?,
        button,
        distance,
        delta: distance,
    });
    Ok(())
}
fn translation(app: &App, panel: Entity) -> std::result::Result<Vec2, Box<dyn std::error::Error>> {
    offset(app.world().get::<UiTransform>(panel).ok_or("transform")?)
        .ok_or_else(|| "pixel translation".into())
}
#[test]
fn header_press_drag_and_bounds_use_real_pointer_observers() -> Result {
    let (mut app, window, panel) = app(1., 1.);
    let point = Vec2::new(320., 255.);
    press_at(
        &mut app,
        panel,
        window,
        point,
        PointerId::Mouse,
        PointerButton::Primary,
    )?;
    move_to(
        &mut app,
        panel,
        window,
        point,
        Vec2::new(50., 30.),
        PointerId::Mouse,
        PointerButton::Primary,
        true,
    )?;
    assert_eq!(translation(&app, panel)?, Vec2::new(50., 30.));
    move_to(
        &mut app,
        panel,
        window,
        point,
        Vec2::splat(5000.),
        PointerId::Mouse,
        PointerButton::Primary,
        false,
    )?;
    assert_eq!(translation(&app, panel)?, Vec2::new(300., 250.));
    move_to(
        &mut app,
        panel,
        window,
        point,
        Vec2::splat(-5000.),
        PointerId::Mouse,
        PointerButton::Primary,
        false,
    )?;
    assert_eq!(translation(&app, panel)?, Vec2::new(-300., -250.));
    Ok(())
}
#[test]
fn body_press_cannot_become_a_header_drag_after_crossing_the_caption() -> Result {
    let (mut app, window, panel) = app(1., 1.);
    let body = Vec2::new(320., 290.);
    press_at(
        &mut app,
        panel,
        window,
        body,
        PointerId::Mouse,
        PointerButton::Primary,
    )?;
    move_to(
        &mut app,
        panel,
        window,
        body,
        Vec2::new(50., -35.),
        PointerId::Mouse,
        PointerButton::Primary,
        true,
    )?;
    assert_eq!(translation(&app, panel)?, Vec2::ZERO);
    assert!(
        app.world()
            .get::<DragState>(panel)
            .ok_or("state")?
            .gesture
            .is_none()
    );
    Ok(())
}
#[test]
fn controls_and_item_origins_never_arm_dialog_movement() -> Result {
    use std::num::{NonZeroU32, NonZeroU64};
    let (mut app, window, panel) = app(1., 1.);
    let button = app.world_mut().spawn((Button, ChildOf(panel))).id();
    let input = app.world_mut().spawn((TextInput, ChildOf(panel))).id();
    let checkbox = app
        .world_mut()
        .spawn((Checkbox, Node::default(), ChildOf(panel)))
        .id();
    let item = app
        .world_mut()
        .spawn((
            ItemDragSource {
                generation: 0,
                item: oteryn_session::ItemEntry {
                    handle: NonZeroU64::new(1).ok_or("handle")?,
                    item_definition_ref: NonZeroU32::new(1).ok_or("definition")?,
                    count: NonZeroU32::new(1).ok_or("count")?,
                    sub_type: 0,
                },
            },
            Node::default(),
            ChildOf(panel),
        ))
        .id();
    for target in [button, input, checkbox, item] {
        let point = Vec2::new(320., 255.);
        press_at(
            &mut app,
            target,
            window,
            point,
            PointerId::Mouse,
            PointerButton::Primary,
        )?;
        move_to(
            &mut app,
            target,
            window,
            point,
            Vec2::splat(50.),
            PointerId::Mouse,
            PointerButton::Primary,
            true,
        )?;
        assert_eq!(translation(&app, panel)?, Vec2::ZERO);
        assert!(
            app.world()
                .get::<DragState>(panel)
                .ok_or("state")?
                .gesture
                .is_none()
        );
    }
    Ok(())
}
#[test]
fn matching_pointer_and_first_button_keep_secondary_click_from_bouncing_dialog() -> Result {
    let (mut app, window, panel) = app(1., 1.);
    let point = Vec2::new(320., 255.);
    press_at(
        &mut app,
        panel,
        window,
        point,
        PointerId::Mouse,
        PointerButton::Primary,
    )?;
    move_to(
        &mut app,
        panel,
        window,
        point,
        Vec2::new(20., 0.),
        PointerId::Mouse,
        PointerButton::Primary,
        true,
    )?;
    press_at(
        &mut app,
        panel,
        window,
        point,
        PointerId::Mouse,
        PointerButton::Secondary,
    )?;
    move_to(
        &mut app,
        panel,
        window,
        point,
        Vec2::splat(100.),
        PointerId::Mouse,
        PointerButton::Secondary,
        true,
    )?;
    move_to(
        &mut app,
        panel,
        window,
        point,
        Vec2::splat(100.),
        PointerId::Touch(9),
        PointerButton::Primary,
        true,
    )?;
    assert_eq!(translation(&app, panel)?, Vec2::new(20., 0.));
    app.world_mut().trigger(PointerRelease {
        entity: panel,
        pointer: pointer(window, point, PointerId::Mouse)?,
        button: PointerButton::Secondary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
    });
    move_to(
        &mut app,
        panel,
        window,
        point,
        Vec2::new(40., 0.),
        PointerId::Mouse,
        PointerButton::Primary,
        false,
    )?;
    assert_eq!(translation(&app, panel)?, Vec2::new(40., 0.));
    app.world_mut().trigger(PointerDragEnd {
        entity: panel,
        pointer: pointer(window, point, PointerId::Mouse)?,
        button: PointerButton::Primary,
        distance: Vec2::new(40., 0.),
    });
    move_to(
        &mut app,
        panel,
        window,
        point,
        Vec2::splat(100.),
        PointerId::Mouse,
        PointerButton::Primary,
        false,
    )?;
    assert_eq!(translation(&app, panel)?, Vec2::new(40., 0.));
    Ok(())
}
#[test]
fn focus_loss_cancel_and_higher_modal_end_the_existing_gesture() -> Result {
    let (mut app, window, panel) = app(1., 1.);
    let point = Vec2::new(320., 255.);
    press_at(
        &mut app,
        panel,
        window,
        point,
        PointerId::Mouse,
        PointerButton::Secondary,
    )?;
    move_to(
        &mut app,
        panel,
        window,
        point,
        Vec2::new(30., 0.),
        PointerId::Mouse,
        PointerButton::Secondary,
        true,
    )?;
    app.world_mut().write_message(WindowFocused {
        window,
        focused: false,
    });
    app.update();
    move_to(
        &mut app,
        panel,
        window,
        point,
        Vec2::splat(100.),
        PointerId::Mouse,
        PointerButton::Secondary,
        false,
    )?;
    assert_eq!(translation(&app, panel)?, Vec2::new(30., 0.));
    press_at(
        &mut app,
        panel,
        window,
        point,
        PointerId::Mouse,
        PointerButton::Primary,
    )?;
    app.world_mut().trigger(PointerCancel {
        entity: panel,
        pointer: pointer(window, point, PointerId::Mouse)?,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
    });
    move_to(
        &mut app,
        panel,
        window,
        point,
        Vec2::splat(100.),
        PointerId::Mouse,
        PointerButton::Primary,
        true,
    )?;
    assert_eq!(translation(&app, panel)?, Vec2::new(30., 0.));
    press_at(
        &mut app,
        panel,
        window,
        point,
        PointerId::Mouse,
        PointerButton::Primary,
    )?;
    app.world_mut().spawn((
        super::super::game_ui::GameModal,
        Node::default(),
        GlobalZIndex(200),
    ));
    move_to(
        &mut app,
        panel,
        window,
        point,
        Vec2::splat(100.),
        PointerId::Mouse,
        PointerButton::Primary,
        true,
    )?;
    assert_eq!(translation(&app, panel)?, Vec2::new(30., 0.));
    assert!(
        app.world()
            .get::<DragState>(panel)
            .ok_or("state")?
            .gesture
            .is_none()
    );
    Ok(())
}
#[test]
fn wrong_window_and_stale_generation_do_not_move_the_dialog() -> Result {
    let (mut app, window, panel) = app(1., 1.);
    let point = Vec2::new(320., 255.);
    app.insert_non_send(HostOwner::new(None)?);
    let other = app.world_mut().spawn(Window::default()).id();
    press_at(
        &mut app,
        panel,
        other,
        point,
        PointerId::Mouse,
        PointerButton::Primary,
    )?;
    move_to(
        &mut app,
        panel,
        other,
        point,
        Vec2::splat(20.),
        PointerId::Mouse,
        PointerButton::Primary,
        true,
    )?;
    assert_eq!(translation(&app, panel)?, Vec2::ZERO);
    press_at(
        &mut app,
        panel,
        window,
        point,
        PointerId::Mouse,
        PointerButton::Primary,
    )?;
    let mut state = app.world_mut().get_mut::<DragState>(panel).ok_or("state")?;
    state.gesture.as_mut().ok_or("gesture")?.context.generation = Some(1);
    move_to(
        &mut app,
        panel,
        window,
        point,
        Vec2::splat(20.),
        PointerId::Mouse,
        PointerButton::Primary,
        true,
    )?;
    assert_eq!(translation(&app, panel)?, Vec2::ZERO);
    assert!(
        app.world()
            .get::<DragState>(panel)
            .ok_or("state")?
            .gesture
            .is_none()
    );
    Ok(())
}
#[test]
fn primary_window_resize_recenters_without_persisting_geometry() -> Result {
    let (mut app, window, panel) = app(1., 1.);
    let point = Vec2::new(320., 255.);
    press_at(
        &mut app,
        panel,
        window,
        point,
        PointerId::Mouse,
        PointerButton::Primary,
    )?;
    move_to(
        &mut app,
        panel,
        window,
        point,
        Vec2::new(100., 20.),
        PointerId::Mouse,
        PointerButton::Primary,
        true,
    )?;
    app.world_mut()
        .get_mut::<Window>(window)
        .ok_or("window")?
        .resolution
        .set_physical_resolution(500, 400);
    app.update();
    assert_eq!(translation(&app, panel)?, Vec2::ZERO);
    assert!(
        app.world()
            .get::<DragState>(panel)
            .ok_or("state")?
            .gesture
            .is_none()
    );
    Ok(())
}
#[path = "layout_tests.rs"]
mod layout_tests;

#[path = "focus_tests.rs"]
mod focus_tests;
