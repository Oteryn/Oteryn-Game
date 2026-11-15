use super::super::{GameModal, GameRegions, GameRoot, setup, source};
use super::*;
use crate::bevy_shell::ui::Skins;
use bevy::picking::{backend::HitData, pointer::Location};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn app() -> App {
    let mut app = App::new();
    app.insert_resource(UiScale(2.));
    app.insert_resource(Skins {
        buttons: oteryn_client_ui_bevy::ButtonImages {
            up: default(),
            down: default(),
        },
        checked: default(),
        unchecked: default(),
        frame: default(),
        inset: default(),
        font: default(),
    });
    install(&mut app);
    app.add_systems(Startup, setup);
    app.world_mut().spawn((
        Window {
            focused: true,
            ..default()
        },
        PrimaryWindow,
    ));
    app.update();
    let mut roots = app
        .world_mut()
        .query_filtered::<&mut Node, With<GameRoot>>();
    for mut root in roots.iter_mut(app.world_mut()) {
        root.display = Display::Flex;
    }
    app
}
fn handles(app: &mut App) -> Vec<(Entity, Splitter)> {
    let mut query = app.world_mut().query::<(Entity, &Splitter)>();
    query
        .iter(app.world())
        .map(|(entity, splitter)| (entity, *splitter))
        .collect()
}
fn pointer(app: &mut App) -> Result<Pointer, Box<dyn std::error::Error>> {
    let mut windows = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>();
    let window = windows.single(app.world())?;
    Ok(Pointer::new(
        PointerId::Mouse,
        Location {
            target: WindowRef::Entity(window)
                .normalize(None)
                .map(NormalizedRenderTarget::Window)
                .ok_or("window")?,
            position: Vec2::new(500., 400.),
        },
    ))
}
fn start_drag(app: &mut App, entity: Entity, button: PointerButton) -> TestResult {
    let pointer = pointer(app)?;
    app.world_mut().trigger(PointerDragStart {
        entity,
        pointer,
        button,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
    });
    Ok(())
}
fn move_drag(app: &mut App, entity: Entity, distance: Vec2) -> TestResult {
    let pointer = pointer(app)?;
    app.world_mut().trigger(PointerDrag {
        entity,
        pointer,
        button: PointerButton::Primary,
        distance,
        delta: distance,
    });
    Ok(())
}
fn size(app: &App, splitter: Splitter) -> Result<Val, Box<dyn std::error::Error>> {
    let node = app.world().get::<Node>(splitter.target).ok_or("region")?;
    Ok(if splitter.vertical {
        node.height
    } else {
        node.width
    })
}
fn geometry(app: &mut App, splitter: Splitter) {
    app.world_mut()
        .entity_mut(splitter.target)
        .insert(ComputedNode {
            size: Vec2::new(352., 300.),
            inverse_scale_factor: 0.5,
            ..default()
        });
}

#[test]
fn main_panel_drag_resizes_console_and_both_sidebars() -> TestResult {
    let mut app = app();
    let handles = handles(&mut app);
    assert_eq!(handles.len(), 3);
    let chat = app.world().resource::<GameRegions>().chat;
    assert_eq!(
        handles
            .iter()
            .filter(|(_, splitter)| splitter.vertical && splitter.target == chat)
            .count(),
        1
    );
    for (entity, splitter) in handles {
        geometry(&mut app, splitter);
        start_drag(&mut app, entity, PointerButton::Primary)?;
        move_drag(&mut app, entity, Vec2::splat(-120.))?;
        assert_eq!(
            size(&app, splitter)?,
            px(if splitter.vertical { 210. } else { 236. })
        );
        // Distance is cumulative from drag start, rather than added to each prior frame.
        move_drag(&mut app, entity, Vec2::splat(-140.))?;
        assert_eq!(
            size(&app, splitter)?,
            px(if splitter.vertical { 220. } else { 246. })
        );
        move_drag(&mut app, entity, Vec2::splat(10000.))?;
        assert_eq!(
            size(&app, splitter)?,
            px(if splitter.vertical { 90. } else { 176. })
        );
        move_drag(&mut app, entity, Vec2::splat(-10000.))?;
        assert_eq!(
            size(&app, splitter)?,
            px(if splitter.vertical { 600. } else { 480. })
        );
        let node = app.world().get::<Node>(splitter.target).ok_or("region")?;
        assert_eq!(
            if splitter.vertical {
                node.max_height
            } else {
                node.max_width
            },
            percent(if splitter.vertical { 50. } else { 30. })
        );
    }
    Ok(())
}

#[test]
fn splitter_rejects_secondary_and_stops_after_cancel_focus_loss_or_modal() -> TestResult {
    let mut app = app();
    let (entity, splitter) = handles(&mut app).into_iter().next().ok_or("splitter")?;
    geometry(&mut app, splitter);
    let initial = size(&app, splitter)?;
    start_drag(&mut app, entity, PointerButton::Secondary)?;
    move_drag(&mut app, entity, Vec2::splat(-100.))?;
    assert_eq!(size(&app, splitter)?, initial);
    start_drag(&mut app, entity, PointerButton::Primary)?;
    let pointer = pointer(&mut app)?;
    let world = app.world().resource::<GameRegions>().world;
    app.world_mut().trigger(PointerCancel {
        entity: world,
        pointer,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
    });
    move_drag(&mut app, entity, Vec2::splat(-100.))?;
    assert_eq!(size(&app, splitter)?, initial);
    start_drag(&mut app, entity, PointerButton::Primary)?;
    let mut windows = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>();
    let window = windows.single(app.world())?;
    app.world_mut().write_message(WindowFocused {
        window,
        focused: false,
    });
    app.update();
    move_drag(&mut app, entity, Vec2::splat(-100.))?;
    assert_eq!(size(&app, splitter)?, initial);
    start_drag(&mut app, entity, PointerButton::Primary)?;
    app.world_mut()
        .spawn((GameModal, Node::default(), GlobalZIndex(100)));
    move_drag(&mut app, entity, Vec2::splat(-100.))?;
    assert_eq!(size(&app, splitter)?, initial);
    Ok(())
}

#[test]
fn splitter_cancels_when_ui_scale_changes_and_ends_on_release() -> TestResult {
    let mut app = app();
    let (entity, splitter) = handles(&mut app).into_iter().next().ok_or("splitter")?;
    geometry(&mut app, splitter);
    let initial = size(&app, splitter)?;
    start_drag(&mut app, entity, PointerButton::Primary)?;
    app.world_mut().resource_mut::<UiScale>().0 = 1.;
    move_drag(&mut app, entity, Vec2::splat(-100.))?;
    assert_eq!(size(&app, splitter)?, initial);
    start_drag(&mut app, entity, PointerButton::Primary)?;
    let pointer = pointer(&mut app)?;
    app.world_mut().trigger(PointerDragEnd {
        entity,
        pointer,
        button: PointerButton::Primary,
        distance: Vec2::ZERO,
    });
    move_drag(&mut app, entity, Vec2::splat(-100.))?;
    assert_eq!(size(&app, splitter)?, initial);
    Ok(())
}

#[test]
fn game_shell_exposes_three_resize_handles() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.insert_resource(Skins {
        buttons: source::ButtonImages {
            up: default(),
            down: default(),
        },
        checked: default(),
        unchecked: default(),
        frame: default(),
        inset: default(),
        font: default(),
    });
    app.add_systems(Startup, setup);
    app.update();
    let mut handles = app.world_mut().query::<&Name>();
    assert_eq!(
        handles
            .iter(app.world())
            .filter(|name| name.as_str().starts_with("Resize "))
            .count(),
        3
    );
    Ok(())
}
