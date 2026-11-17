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
        .query::<(&resize::ResizeGrip, &ChildOf)>()
        .iter(app.world())
        .filter(|(_, parent)| parent.parent() == root)
        .count();
    assert_eq!(grips, 8, "the window has eight resize handles");
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

#[test]
fn every_edge_and_corner_resizes_and_keeps_the_opposite_edge() -> TestResult {
    use resize::{SIDES, reshape};
    let screen = Vec2::new(1920., 1080.);
    let origin = (Vec2::new(100., 100.), Vec2::new(900., 520.));
    let drag = Vec2::new(-40., -30.);
    // (handle, expected top-left, expected size) after a drag by (-40, -30).
    let expected = [
        ("n", Vec2::new(100., 70.), Vec2::new(900., 550.)),
        ("ne", Vec2::new(100., 70.), Vec2::new(860., 550.)),
        ("e", Vec2::new(100., 100.), Vec2::new(860., 520.)),
        ("se", Vec2::new(100., 100.), Vec2::new(860., 490.)),
        ("s", Vec2::new(100., 100.), Vec2::new(900., 490.)),
        ("sw", Vec2::new(60., 100.), Vec2::new(940., 490.)),
        ("w", Vec2::new(60., 100.), Vec2::new(940., 520.)),
        ("nw", Vec2::new(60., 70.), Vec2::new(940., 550.)),
    ];
    for ((name, side, _), (label, position, size)) in SIDES.into_iter().zip(expected) {
        assert_eq!(name, label);
        let mut market = layout("market")?;
        let top_left = reshape(&mut market, side, origin, drag, screen);
        assert_eq!((top_left, market.size), (position, Some(size)), "{name}");
    }
    // The minimum holds and the right edge stays put while the left edge stops.
    let mut market = layout("market")?;
    let top_left = reshape(
        &mut market,
        IVec2::new(-1, 0),
        origin,
        Vec2::new(500., 0.),
        screen,
    );
    assert_eq!(
        (top_left, market.size),
        (Vec2::new(200., 100.), Some(Vec2::new(800., 520.)))
    );
    // The screen holds: the left edge stops at x = 0 and the bottom at the screen.
    let mut market = layout("market")?;
    let top_left = reshape(
        &mut market,
        IVec2::new(-1, 1),
        origin,
        Vec2::new(-500., 900.),
        screen,
    );
    assert_eq!(
        (top_left, market.size),
        (Vec2::ZERO.with_y(100.), Some(Vec2::new(1000., 980.)))
    );
    assert_eq!(
        resize::correction(Vec2::new(60., 70.), Vec2::new(80., 70.)),
        Some(Vec2::new(-20., 0.))
    );
    assert_eq!(resize::correction(Vec2::ONE, Vec2::ONE), None);
    Ok(())
}

#[test]
fn a_window_has_eight_handles_with_resize_cursors() -> TestResult {
    use bevy::window::SystemCursorIcon;
    let mut app = app(1920, 1080, 1.);
    let root = app
        .world_mut()
        .spawn(window("options", Node::default()))
        .id();
    app.update();
    let mut handles: Vec<(String, SystemCursorIcon, Node)> = app
        .world_mut()
        .query::<(&Name, &ResizeCursor, &Node, &ChildOf)>()
        .iter(app.world())
        .filter(|(.., parent)| parent.parent() == root)
        .map(|(name, cursor, node, _)| (name.to_string(), cursor.0, node.clone()))
        .collect();
    handles.sort_by(|a, b| a.0.cmp(&b.0));
    let cursors: Vec<(&str, SystemCursorIcon)> = handles
        .iter()
        .map(|(name, cursor, _)| (name.as_str(), *cursor))
        .collect();
    assert_eq!(
        cursors,
        [
            ("window.resize.e", SystemCursorIcon::EwResize),
            ("window.resize.n", SystemCursorIcon::NsResize),
            ("window.resize.ne", SystemCursorIcon::NeswResize),
            ("window.resize.nw", SystemCursorIcon::NwseResize),
            ("window.resize.s", SystemCursorIcon::NsResize),
            ("window.resize.se", SystemCursorIcon::NwseResize),
            ("window.resize.sw", SystemCursorIcon::NeswResize),
            ("window.resize.w", SystemCursorIcon::EwResize),
        ]
    );
    let node = |name: &str| {
        handles
            .iter()
            .find(|(candidate, ..)| candidate == name)
            .map(|(.., node)| node.clone())
            .ok_or(format!("{name} handle"))
    };
    let west = node("window.resize.w")?;
    assert_eq!(
        (west.left, west.width, west.top, west.bottom),
        (
            px(0.),
            px(resize::EDGE),
            px(resize::CORNER),
            px(resize::CORNER)
        )
    );
    let corner = node("window.resize.se")?;
    assert_eq!(
        (corner.right, corner.bottom, corner.width, corner.height),
        (px(0.), px(0.), px(resize::CORNER), px(resize::CORNER))
    );
    Ok(())
}

#[test]
fn zoom_150_scales_window_text() -> TestResult {
    use bevy::text::{FontSize, LineHeight};
    let mut app = app(1920, 1080, 1.);
    let root = app
        .world_mut()
        .spawn(window("cyclopedia", Node::default()))
        .id();
    let body = app
        .world_mut()
        .spawn((
            Text::new("Body"),
            TextFont {
                font_size: FontSize::Px(11.),
                ..default()
            },
            LineHeight::Px(13.),
            ChildOf(root),
        ))
        .id();
    let small = app
        .world_mut()
        .spawn((
            Text::new("Small"),
            TextFont {
                font_size: FontSize::Px(8.),
                ..default()
            },
            ChildOf(body),
        ))
        .id();
    let outside = app
        .world_mut()
        .spawn((
            Text::new("Outside"),
            TextFont {
                font_size: FontSize::Px(11.),
                ..default()
            },
        ))
        .id();
    app.update();
    app.world_mut()
        .resource_mut::<WindowLayouts>()
        .0
        .get_mut("cyclopedia")
        .ok_or("cyclopedia")?
        .set_zoom(1.5, Vec2::new(1920., 1080.));
    app.update();
    let world = app.world();
    let size = |entity| world.get::<TextFont>(entity).map(|font| font.font_size);
    assert_eq!(size(body), Some(FontSize::Px(16.5)));
    assert_eq!(world.get::<LineHeight>(body), Some(&LineHeight::Px(19.5)));
    assert_eq!(size(small), Some(FontSize::Px(12.)));
    assert_eq!(
        size(outside),
        Some(FontSize::Px(11.)),
        "text outside windows keeps its size"
    );
    Ok(())
}

#[test]
fn layouts_save_and_load_position_size_and_zoom() -> TestResult {
    let folder = std::env::temp_dir().join(format!("oteryn-window-layouts-{}", std::process::id()));
    let path = folder.join("windows.json");
    let _ = std::fs::remove_dir_all(&folder);
    assert_eq!(WindowLayouts::load(&path)?, WindowLayouts::default());
    let screen = Vec2::new(1920., 1080.);
    let mut layouts = WindowLayouts::default();
    let market = layouts.0.get_mut("market").ok_or("market")?;
    market.resize(Vec2::new(1200., 700.), screen);
    market.set_zoom(1.5, screen);
    market.offset = Vec2::new(-40., 25.);
    layouts.save(&path)?;
    let loaded = WindowLayouts::load(&path)?;
    assert_eq!(loaded, layouts);
    std::fs::write(
        &path,
        r#"{"market":{"size":[10,10],"zoom":9,"offset":[1,2]},"retired":{"size":null,"zoom":1,"offset":[0,0]}}"#,
    )?;
    let sanitized = WindowLayouts::load(&path)?;
    let market = sanitized.0.get("market").ok_or("market")?;
    assert_eq!(market.zoom, MAX_ZOOM);
    assert_eq!(market.size, Some(market.min()));
    assert_eq!(market.offset, Vec2::new(1., 2.));
    assert_eq!(sanitized.0.len(), WINDOWS.len());
    std::fs::write(&path, "not json")?;
    assert!(WindowLayouts::load(&path).is_err());
    std::fs::remove_dir_all(&folder)?;
    Ok(())
}

#[test]
fn a_window_reopens_where_the_user_moved_it() -> TestResult {
    let mut app = app(1920, 1080, 1.);
    app.world_mut()
        .resource_mut::<WindowLayouts>()
        .0
        .get_mut("selection")
        .ok_or("selection")?
        .offset = Vec2::new(30., -20.);
    let root = app
        .world_mut()
        .spawn((window("selection", Node::default()), UiTransform::default()))
        .id();
    app.update();
    let world = app.world();
    assert_eq!(
        world
            .get::<UiTransform>(root)
            .map(|transform| transform.translation),
        Some(Val2::px(30., -20.))
    );
    app.world_mut()
        .get_mut::<UiTransform>(root)
        .ok_or("transform")?
        .translation = Val2::px(55., 5.);
    app.update();
    let layouts = app.world().resource::<WindowLayouts>();
    assert_eq!(
        layouts.0.get("selection").map(|layout| layout.offset),
        Some(Vec2::new(55., 5.))
    );
    Ok(())
}

#[test]
fn layouts_load_beside_the_preferences_and_save_once_settled() -> TestResult {
    use crate::bevy_shell::owner::HostOwner;
    use bevy::time::TimeUpdateStrategy;
    let folder = std::env::temp_dir().join(format!("oteryn-window-persist-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder)?;
    let owner = HostOwner::new(Some(folder.join("client-preferences.json")))?;
    let path = owner.window_layouts_path().ok_or("layout path")?;
    let screen = Vec2::new(1920., 1080.);
    let mut saved = WindowLayouts::default();
    saved
        .0
        .get_mut("options")
        .ok_or("options")?
        .set_zoom(1.2, screen);
    saved.save(&path)?;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(300),
        ))
        .init_resource::<WindowLayouts>()
        .insert_non_send(owner)
        .add_systems(Update, persist);
    app.update();
    app.update();
    assert_eq!(*app.world().resource::<WindowLayouts>(), saved);
    app.world_mut()
        .resource_mut::<WindowLayouts>()
        .0
        .get_mut("options")
        .ok_or("options")?
        .offset = Vec2::new(12., 34.);
    app.update();
    assert_eq!(
        WindowLayouts::load(&path)?,
        saved,
        "a change is not written mid-drag"
    );
    for _ in 0..5 {
        app.update();
    }
    let written = WindowLayouts::load(&path)?;
    assert_eq!(
        written.0.get("options").map(|layout| layout.offset),
        Some(Vec2::new(12., 34.))
    );
    std::fs::remove_dir_all(&folder)?;
    Ok(())
}
