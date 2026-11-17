use super::*;
use bevy::window::WindowResolution;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn layout(id: &str) -> Result<Layout, String> {
    WindowLayouts::default()
        .0
        .get(id)
        .copied()
        .ok_or(format!("{id} layout"))
}

fn app(width: u32, height: u32, scale: f32) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<WindowLayouts>()
        .insert_resource(UiScale(scale))
        .add_systems(Update, project);
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(width, height),
            ..default()
        },
        PrimaryWindow,
    ));
    app
}

#[test]
fn every_window_starts_at_its_global_size_and_has_a_minimum() -> TestResult {
    for (id, size) in [
        ("cyclopedia", Some(Vec2::new(700., 538.))),
        ("market", Some(Vec2::new(1000., 634.))),
        ("npc", None),
        ("options", Some(Vec2::new(685., 534.))),
        ("selection", None),
    ] {
        let layout = layout(id)?;
        assert_eq!(layout.size, size, "{id}");
        assert!(layout.min().cmple(Vec2::new(1020., 650.)).all(), "{id}");
        assert!(layout.min().cmpgt(Vec2::ZERO).all(), "{id}");
    }
    Ok(())
}

#[test]
fn resizing_stops_at_the_minimum_and_at_the_screen() -> TestResult {
    let mut market = layout("market")?;
    let screen = Vec2::new(1020., 650.);
    market.resize(Vec2::new(100., 100.), screen);
    assert_eq!(market.size, Some(Vec2::new(800., 480.)));
    market.resize(Vec2::new(5000., 5000.), screen);
    assert_eq!(market.size, Some(screen));
    market.resize(Vec2::new(900., 520.), screen);
    assert_eq!(market.size, Some(Vec2::new(900., 520.)));
    market.resize(Vec2::NAN, screen);
    assert_eq!(market.size, Some(Vec2::new(800., 480.)));
    Ok(())
}

#[test]
fn zoom_150_scales_rows_icons_and_the_minimum() -> TestResult {
    let mut app = app(3840, 2160, 1.);
    let root = app.world_mut().spawn(window("npc", Node::default())).id();
    let row = app
        .world_mut()
        .spawn((
            Scaled {
                min_height: Some(35.),
                ..default()
            },
            Node::default(),
            ChildOf(root),
        ))
        .id();
    let icon = app
        .world_mut()
        .spawn((Scaled::square(32.), Node::default(), ChildOf(row)))
        .id();
    app.update();
    let screen = Vec2::new(3840., 2160.);
    app.world_mut()
        .resource_mut::<WindowLayouts>()
        .0
        .get_mut("npc")
        .ok_or("npc")?
        .set_zoom(1.5, screen);
    app.update();
    let world = app.world();
    let node = |entity| world.get::<Node>(entity).ok_or("node");
    assert_eq!(node(row)?.min_height, px(52.5));
    assert_eq!((node(icon)?.width, node(icon)?.height), (px(48.), px(48.)));
    let window = node(root)?;
    assert_eq!((window.min_width, window.min_height), (px(630.), px(525.)));
    assert_eq!(window.width, px(900.));
    let mut npc = layout("npc")?;
    npc.set_zoom(9., screen);
    assert_eq!(npc.zoom, MAX_ZOOM);
    npc.reset_zoom(screen);
    assert_eq!((npc.zoom, npc.min()), (1., Vec2::new(420., 350.)));
    Ok(())
}

#[test]
fn a_smaller_screen_or_larger_ui_scale_clamps_every_window() -> TestResult {
    // 1920x1080 at UI scale 2 leaves a 960x540 logical screen.
    let mut app = app(1920, 1080, 2.);
    let market = app
        .world_mut()
        .spawn(window("market", Node::default()))
        .id();
    app.update();
    let layouts = app.world().resource::<WindowLayouts>();
    assert_eq!(
        layouts.0.get("market").ok_or("market")?.size,
        Some(Vec2::new(960., 540.))
    );
    assert_eq!(
        layouts.0.get("options").ok_or("options")?.size,
        Some(Vec2::new(685., 534.))
    );
    let node = app.world().get::<Node>(market).ok_or("market node")?;
    assert_eq!((node.width, node.height), (px(960.), px(540.)));
    Ok(())
}

#[test]
fn ctrl_wheel_over_a_window_zooms_it_and_ctrl_0_resets() -> TestResult {
    use bevy::{
        input::mouse::{AccumulatedMouseScroll, MouseScrollUnit},
        picking::{backend::HitData, hover::HoverMap, pointer::PointerId},
    };
    let mut app = app(1920, 1080, 1.);
    app.add_systems(Update, zoom.before(project));
    let root = app
        .world_mut()
        .spawn(window("market", Node::default()))
        .id();
    let row = app
        .world_mut()
        .spawn((Scaled::square(32.), Node::default(), ChildOf(root)))
        .id();
    app.update();
    let grips = app
        .world_mut()
        .query::<(&ResizeGrip, &ChildOf)>()
        .iter(app.world())
        .filter(|(_, parent)| parent.parent() == root)
        .count();
    assert_eq!(grips, 1, "the window has one corner resize grip");
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::ControlLeft);
    let mut hover = HoverMap::default();
    hover.0.insert(
        PointerId::Mouse,
        [(row, HitData::new(Entity::PLACEHOLDER, 0., None, None))]
            .into_iter()
            .collect(),
    );
    app.insert_resource(keys).insert_resource(hover);
    for _ in 0..5 {
        app.insert_resource(AccumulatedMouseScroll {
            unit: MouseScrollUnit::Line,
            delta: Vec2::new(0., 1.),
        });
        app.update();
    }
    let market = *app
        .world()
        .resource::<WindowLayouts>()
        .0
        .get("market")
        .ok_or("market")?;
    assert!((market.zoom - 1.5).abs() < 1e-5, "zoom {}", market.zoom);
    let icon = app.world().get::<Node>(row).ok_or("row")?;
    assert_eq!(icon.width, px(32. * market.zoom));
    app.insert_resource(AccumulatedMouseScroll::default());
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::ControlLeft);
    keys.press(KeyCode::Digit0);
    app.insert_resource(keys);
    app.update();
    let market = app
        .world()
        .resource::<WindowLayouts>()
        .0
        .get("market")
        .copied()
        .ok_or("market")?;
    assert_eq!(market.zoom, 1.);
    Ok(())
}
