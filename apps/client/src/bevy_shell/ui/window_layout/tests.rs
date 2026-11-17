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
        Some(Vec2::new(1000., 634.))
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

fn install_close(app: &mut App) {
    if !app.is_plugin_added::<bevy::asset::AssetPlugin>() {
        app.add_plugins(bevy::asset::AssetPlugin::default());
    }
    app.init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<bevy::input_focus::InputFocus>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    crate::bevy_shell::game_ui::modal::install(app);
    crate::bevy_shell::exit::install(app);
    app.world_mut()
        .non_send_mut::<crate::bevy_shell::owner::HostOwner>()
        .settings
        .preferences
        .miscellaneous
        .confirm_exit = false;
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
        .insert_non_send(owner);
    install(&mut app);
    install_close(&mut app);
    let screen = app
        .world_mut()
        .spawn((
            Window {
                resolution: (1920, 1080).into(),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    let options = app.world_mut().spawn(window("options", default())).id();
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
    app.world_mut()
        .get_mut::<UiTransform>(options)
        .ok_or("options")?
        .translation = Val2::px(0., 0.);
    app.world_mut()
        .write_message(bevy::window::WindowCloseRequested { window: screen });
    app.update();
    assert_eq!(WindowLayouts::load(&path)?.0["options"].offset, Vec2::ZERO);
    std::fs::remove_dir_all(&folder)?;
    Ok(())
}

fn saved_market(
    name: &str,
) -> Result<(App, std::path::PathBuf, WindowLayouts), Box<dyn std::error::Error>> {
    use crate::bevy_shell::owner::HostOwner;
    let dir = std::env::temp_dir().join(format!("oteryn-layout-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let owner = HostOwner::new(Some(dir.join("preferences.json")))?;
    let path = owner.window_layouts_path().ok_or("layout path")?;
    let mut saved = WindowLayouts::default();
    saved.0.get_mut("market").ok_or("market")?.size = Some(Vec2::new(1400., 600.));
    saved.save(&path)?;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).insert_non_send(owner);
    install(&mut app);
    Ok((app, path, saved))
}

#[test]
fn idle_exit_preserves_another_clients_save() -> TestResult {
    let (mut app, path, mut saved) = saved_market("idle")?;
    app.update();
    app.update();
    saved.0.get_mut("market").ok_or("market")?.size = Some(Vec2::new(1300., 600.));
    saved.save(&path)?;
    app.world_mut().write_message(AppExit::Success);
    app.update();
    assert_eq!(WindowLayouts::load(&path)?, saved);
    std::fs::remove_dir_all(path.parent().ok_or("fixture directory")?)?;
    Ok(())
}

#[test]
fn smoke_exit_preserves_wide_layout() -> TestResult {
    let (mut app, path, saved) = saved_market("smoke")?;
    app.world_mut().spawn((
        Window {
            resolution: (1024, 768).into(),
            ..default()
        },
        PrimaryWindow,
    ));
    app.add_systems(Update, crate::bevy_shell::smoke_exit);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(WindowLayouts::load(&path)?, saved);
    std::fs::remove_dir_all(path.parent().ok_or("fixture directory")?)?;
    Ok(())
}

#[test]
fn exit_after_panel_despawn_saves_resize_anchor() -> TestResult {
    use bevy::picking::{
        backend::HitData,
        events::{PointerDrag, PointerDragEnd, PointerDragStart},
        pointer::PointerButton,
    };
    let (mut app, path, _) = saved_market("anchor")?;
    stacking::ui_layout(&mut app, UVec2::new(1600, 900));
    install_close(&mut app);
    let screen = app
        .world_mut()
        .spawn((
            Window {
                resolution: (1600, 900).into(),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    let container = app
        .world_mut()
        .spawn(Node {
            width: px(1600.),
            height: px(900.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .id();
    let panel = app
        .world_mut()
        .spawn((window("market", default()), ChildOf(container)))
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<ComputedNode>(panel)
            .ok_or("initial layout")?
            .size(),
        Vec2::new(1400., 600.)
    );
    let entity = app
        .world()
        .get::<Children>(panel)
        .ok_or("grips")?
        .iter()
        .find(|e| {
            app.world()
                .get::<resize::ResizeGrip>(*e)
                .is_some_and(|g| g.side == IVec2::NEG_X)
        })
        .ok_or("left grip")?;
    let button = PointerButton::Primary;
    let distance = Vec2::new(46., 0.);
    app.world_mut().trigger(PointerDragStart {
        entity,
        button,
        pointer: stacking::pointer(),
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
    });
    app.world_mut().trigger(PointerDrag {
        entity,
        button,
        pointer: stacking::pointer(),
        distance,
        delta: distance,
    });
    app.world_mut().trigger(PointerDragEnd {
        entity,
        button,
        pointer: stacking::pointer(),
        distance,
    });
    app.update();
    assert_eq!(
        app.world()
            .get::<UiTransform>(panel)
            .ok_or("panel")?
            .translation,
        Val2::px(23., 0.)
    );
    app.world_mut().despawn(panel);
    app.world_mut()
        .write_message(bevy::window::WindowCloseRequested { window: screen });
    app.update();
    let market = WindowLayouts::load(&path)?.0["market"];
    assert_eq!(market.size, Some(Vec2::new(1354., 600.)));
    assert_eq!(market.offset, Vec2::new(23., 0.));
    std::fs::remove_dir_all(path.parent().ok_or("fixture directory")?)?;
    Ok(())
}

#[test]
fn zoom_stops_at_75_and_200_percent_and_a_saved_zoom_is_clamped() -> TestResult {
    let screen = Vec2::new(4000., 4000.);
    let mut market = layout("market")?;
    for (requested, kept) in [(0.5, 0.75), (0.74, 0.75), (0.75, 0.75), (2., 2.), (2.1, 2.)] {
        market.set_zoom(requested, screen);
        assert_eq!(market.zoom, kept, "{requested}");
    }
    let mut saved = serde_json::to_value(WindowLayouts::default())?;
    saved["market"]["zoom"] = serde_json::json!(0.1);
    saved["npc"]["zoom"] = serde_json::json!(2.1);
    let loaded: WindowLayouts = serde_json::from_value(saved)?;
    assert_eq!(loaded.0.get("market").map(|layout| layout.zoom), Some(0.75));
    assert_eq!(loaded.0.get("npc").map(|layout| layout.zoom), Some(2.));
    Ok(())
}

#[test]
fn reset_window_layout_restores_global_position_size_and_zoom_and_saves() -> TestResult {
    let folder = std::env::temp_dir().join(format!("oteryn-window-reset-{}", std::process::id()));
    let path = folder.join("windows.json");
    let mut app = app(1920, 1080, 1.);
    let root = app
        .world_mut()
        .spawn((WindowId("market"), Node::default(), UiTransform::default()))
        .id();
    app.update();
    {
        let mut layouts = app.world_mut().resource_mut::<WindowLayouts>();
        let market = layouts.0.get_mut("market").ok_or("market")?;
        market.resize(Vec2::new(900., 500.), Vec2::new(1920., 1080.));
        market.set_zoom(1.5, Vec2::new(1920., 1080.));
        market.offset = Vec2::new(-200., 40.);
    }
    app.update();
    let moved = app.world().get::<UiTransform>(root).ok_or("transform")?;
    assert_eq!(moved.translation, Val2::px(-200., 40.));

    app.world_mut()
        .resource_mut::<WindowLayouts>()
        .reset(Some(&path))?;
    app.update();
    let layouts = app.world().resource::<WindowLayouts>();
    assert_eq!(layouts.0, WindowLayouts::default().0);
    assert_eq!(WindowLayouts::load(&path)?.0, WindowLayouts::default().0);
    let transform = app.world().get::<UiTransform>(root).ok_or("transform")?;
    assert_eq!(transform.translation, Val2::px(0., 0.));
    let node = app.world().get::<Node>(root).ok_or("node")?;
    assert_eq!(node.width, px(1000.));
    assert_eq!(node.height, px(634.));
    std::fs::remove_dir_all(folder)?;
    Ok(())
}

mod stacking {
    use super::super::{
        MAX_ZOOM, MIN_ZOOM, WINDOWS, WindowId, WindowLayer, WindowLayouts, WindowStack, install,
        menu, resize::ResizeGrip, stack::BAND,
    };
    use super::TestResult;
    use crate::bevy_shell::{
        game_ui::{GameModal, modal::ModalGuard},
        ui::Skins,
    };
    use bevy::{
        input::mouse::AccumulatedMouseScroll,
        picking::{
            backend::HitData,
            events::{Pointer, PointerClick, PointerPress},
            pointer::{Location, PointerButton, PointerId},
        },
        prelude::*,
        ui_widgets::ButtonPlugin,
    };

    /// What each window's own Escape handler does: close only if the modal fence allows it.
    fn close_front(
        keys: Res<ButtonInput<KeyCode>>,
        windows: Query<Entity, With<WindowLayer>>,
        mut access: ParamSet<(ModalGuard, Query<&mut Node, With<WindowLayer>>)>,
    ) {
        if !keys.just_pressed(KeyCode::Escape) {
            return;
        }
        let guard = access.p0();
        let allowed: Vec<Entity> = windows
            .iter()
            .filter(|entity| guard.allows(*entity))
            .collect();
        let mut nodes = access.p1();
        for entity in allowed {
            if let Ok(mut node) = nodes.get_mut(entity) {
                node.display = Display::None;
            }
        }
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<AccumulatedMouseScroll>()
            .insert_resource(UiScale(1.))
            .add_systems(Update, close_front)
            .configure_sets(
                PostUpdate,
                bevy::ui::UiSystems::Layout.before(bevy::ui::UiSystems::Stack),
            );
        crate::bevy_shell::game_ui::modal::install(&mut app);
        install(&mut app);
        app
    }

    /// A full-screen window overlay with its panel, as the market and the bestiary build them.
    fn window(app: &mut App, id: &'static str, z: i32) -> (Entity, Entity) {
        let overlay = app
            .world_mut()
            .spawn((
                GameModal,
                WindowLayer(id),
                GlobalZIndex(z),
                Node {
                    display: Display::None,
                    width: percent(100.),
                    height: percent(100.),
                    ..default()
                },
            ))
            .id();
        let panel = app
            .world_mut()
            .spawn((Node::default(), ChildOf(overlay)))
            .id();
        (overlay, panel)
    }

    fn show(app: &mut App, overlay: Entity) -> TestResult {
        app.world_mut()
            .get_mut::<Node>(overlay)
            .ok_or("overlay")?
            .display = Display::Flex;
        app.update();
        Ok(())
    }

    fn depth(app: &App, entity: Entity) -> Result<i32, String> {
        app.world()
            .get::<GlobalZIndex>(entity)
            .map(|z| z.0)
            .ok_or(format!("{entity} depth"))
    }

    fn open(app: &App, overlay: Entity) -> bool {
        app.world()
            .get::<Node>(overlay)
            .is_some_and(|node| node.display != Display::None)
    }

    pub(super) fn pointer() -> Pointer {
        Pointer::new(
            PointerId::Mouse,
            Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: 1920,
                    height: 1080,
                },
                position: Vec2::new(300., 200.),
            },
        )
    }

    fn press(app: &mut App, entity: Entity) {
        app.world_mut().trigger(PointerPress {
            entity,
            pointer: pointer(),
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
            count: 1,
        });
        app.update();
    }

    fn escape(app: &mut App) {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::Escape);
        keys.clear();
        keys.press(KeyCode::Escape);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::Escape);
        keys.clear();
    }

    #[test]
    fn several_windows_stay_open_together_between_the_hud_and_dialogs() -> TestResult {
        let mut app = app();
        let (market, _) = window(&mut app, "market", 157);
        let (cyclopedia, _) = window(&mut app, "cyclopedia", 165);
        let (options, _) = window(&mut app, "options", 157);
        for overlay in [market, cyclopedia, options] {
            show(&mut app, overlay)?;
        }
        assert!(
            [market, cyclopedia, options]
                .iter()
                .all(|overlay| open(&app, *overlay))
        );
        let depths = [
            depth(&app, market)?,
            depth(&app, cyclopedia)?,
            depth(&app, options)?,
        ];
        assert!(depths[0] < depths[1] && depths[1] < depths[2], "{depths:?}");
        assert!(
            depths
                .iter()
                .all(|z| (BAND.0..=BAND.1).contains(z) && *z < 150)
        );
        assert_eq!(app.world().resource::<WindowStack>().front(), Some(options));
        Ok(())
    }

    #[test]
    fn clicking_a_window_behind_brings_it_to_the_front() -> TestResult {
        let mut app = app();
        let (market, market_panel) = window(&mut app, "market", 157);
        let (cyclopedia, _) = window(&mut app, "cyclopedia", 165);
        show(&mut app, market)?;
        show(&mut app, cyclopedia)?;
        assert!(depth(&app, cyclopedia)? > depth(&app, market)?);

        press(&mut app, market_panel);
        assert_eq!(app.world().resource::<WindowStack>().front(), Some(market));
        assert!(depth(&app, market)? > depth(&app, cyclopedia)?);
        Ok(())
    }

    #[test]
    fn focusing_a_control_in_a_window_behind_brings_it_to_the_front() -> TestResult {
        let mut app = app();
        let (market, market_panel) = window(&mut app, "market", 157);
        let (cyclopedia, _) = window(&mut app, "cyclopedia", 165);
        show(&mut app, market)?;
        show(&mut app, cyclopedia)?;
        let field = app
            .world_mut()
            .spawn((Node::default(), ChildOf(market_panel)))
            .id();
        app.world_mut()
            .resource_mut::<bevy::input_focus::InputFocus>()
            .set(field, bevy::input_focus::FocusCause::Navigated);
        app.update();
        assert_eq!(app.world().resource::<WindowStack>().front(), Some(market));
        assert!(depth(&app, market)? > depth(&app, cyclopedia)?);
        Ok(())
    }

    #[test]
    fn escape_closes_only_the_front_window_then_the_next() -> TestResult {
        let mut app = app();
        let (market, market_panel) = window(&mut app, "market", 157);
        let (cyclopedia, _) = window(&mut app, "cyclopedia", 165);
        let (options, _) = window(&mut app, "options", 157);
        for overlay in [market, cyclopedia, options] {
            show(&mut app, overlay)?;
        }
        press(&mut app, market_panel);

        escape(&mut app);
        assert!(!open(&app, market));
        assert!(open(&app, cyclopedia) && open(&app, options));
        assert_eq!(app.world().resource::<WindowStack>().front(), Some(options));

        escape(&mut app);
        assert!(!open(&app, options));
        assert!(open(&app, cyclopedia));

        escape(&mut app);
        assert!(!open(&app, cyclopedia));
        assert_eq!(app.world().resource::<WindowStack>().front(), None);
        Ok(())
    }

    fn skins() -> Skins {
        Skins {
            buttons: oteryn_client_ui_bevy::ButtonImages {
                up: default(),
                down: default(),
            },
            checked: default(),
            unchecked: default(),
            frame: default(),
            inset: default(),
            font: default(),
        }
    }

    fn right_click(app: &mut App, entity: Entity) {
        app.world_mut().trigger(PointerClick {
            entity,
            pointer: pointer(),
            button: PointerButton::Secondary,
            hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
            duration: std::time::Duration::ZERO,
            count: 1,
        });
        app.update();
    }

    pub(super) fn ui_layout(app: &mut App, size: UVec2) {
        use bevy::camera::{ComputedCameraValues, RenderTargetInfo};

        app.add_plugins((
            AssetPlugin::default(),
            WindowPlugin {
                primary_window: None,
                close_when_requested: false,
                ..default()
            },
            bevy::input::InputPlugin,
            bevy::input_focus::InputFocusPlugin,
            bevy::transform::TransformPlugin,
            ImagePlugin::default(),
            bevy::image::TextureAtlasPlugin,
            bevy::mesh::MeshPlugin,
            bevy::camera::CameraPlugin,
            bevy::text::TextPlugin,
            bevy::picking::DefaultPickingPlugins,
            bevy::ui::UiPlugin,
            ButtonPlugin,
        ));
        app.world_mut().spawn((
            Camera2d,
            IsDefaultUiCamera,
            Camera {
                computed: ComputedCameraValues {
                    target_info: Some(RenderTargetInfo {
                        physical_size: size,
                        scale_factor: 1.,
                    }),
                    ..default()
                },
                ..default()
            },
        ));
    }

    fn click_menu_row(app: &mut App, row: usize) -> TestResult {
        use bevy::{
            camera::RenderTarget,
            picking::pointer::{PointerAction, PointerInput},
            window::PrimaryWindow,
        };
        let primary = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(app.world())?;
        let location = Location {
            target: RenderTarget::default()
                .normalize(Some(primary))
                .ok_or("primary target")?,
            position: Vec2::new(180., 77. + 22. * row as f32),
        };
        for action in [
            PointerAction::Move { delta: Vec2::ZERO },
            PointerAction::Press(PointerButton::Primary),
            PointerAction::Release(PointerButton::Primary),
        ] {
            app.world_mut().write_message(PointerInput::new(
                PointerId::Mouse,
                location.clone(),
                action,
            ));
            app.update();
        }
        Ok(())
    }

    #[test]
    fn window_menu_controls_keep_button_geometry() -> TestResult {
        let mut app = app();
        app.insert_resource(skins());
        app.world_mut().spawn((
            Window {
                resolution: (301, 201).into(),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ));
        let (market, panel) = window(&mut app, "market", 157);
        show(&mut app, market)?;
        right_click(&mut app, panel);
        for node in app
            .world_mut()
            .query_filtered::<&Node, With<menu::Entry>>()
            .iter(app.world())
        {
            assert_eq!((node.width, node.height), (px(120.), px(20.)));
        }
        Ok(())
    }

    #[test]
    fn window_menu_fits_bottom_right() -> TestResult {
        let mut app = app();
        app.insert_resource(skins());
        app.world_mut().spawn((
            Window {
                resolution: (301, 201).into(),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ));
        let (market, panel) = window(&mut app, "market", 157);
        show(&mut app, market)?;
        right_click(&mut app, panel);
        let menu_node = app
            .world_mut()
            .query_filtered::<&Node, With<menu::WindowMenu>>()
            .single(app.world())?;
        assert_eq!((menu_node.left, menu_node.top), (px(173.), px(63.)));
        app.world_mut()
            .query::<&mut Window>()
            .single_mut(app.world_mut())?
            .resolution
            .set(250., 180.);
        app.update();
        let node = app
            .world_mut()
            .query_filtered::<&Node, With<menu::WindowMenu>>()
            .single(app.world())?;
        assert_eq!((node.left, node.top), (px(122.), px(42.)));
        Ok(())
    }

    #[test]
    fn character_frame_menu_is_above_the_picker() -> TestResult {
        let mut app = app();
        app.insert_resource(skins());
        ui_layout(&mut app, UVec2::new(301, 201));
        app.world_mut().spawn((
            Window {
                resolution: (301, 201).into(),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ));
        app.add_systems(Startup, crate::bevy_shell::selection::setup);
        app.update();
        let overlay = app
            .world_mut()
            .query_filtered::<Entity, With<crate::bevy_shell::selection::Picker>>()
            .single(app.world())?;
        show(&mut app, overlay)?;
        let panel = app
            .world_mut()
            .spawn((WindowId("selection"), Node::default(), ChildOf(overlay)))
            .id();
        right_click(&mut app, panel);
        let z = app
            .world_mut()
            .query_filtered::<&GlobalZIndex, With<menu::WindowMenu>>()
            .single(app.world())?;
        assert!(z.0 > depth(&app, overlay)?);
        click_menu_row(&mut app, 0)?;
        assert_eq!(
            app.world().resource::<WindowLayouts>().0["selection"].zoom,
            0.75
        );
        Ok(())
    }

    #[test]
    fn the_window_menu_zooms_75_to_200_percent_and_resets_the_layout() -> TestResult {
        let mut app = app();
        app.insert_resource(skins());
        ui_layout(&mut app, UVec2::new(301, 201));
        app.world_mut().spawn((
            Window {
                resolution: (301, 201).into(),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ));
        let (market, panel) = window(&mut app, "market", 157);
        show(&mut app, market)?;

        right_click(&mut app, panel);
        let children = app
            .world_mut()
            .query_filtered::<&Children, With<menu::WindowMenu>>()
            .single(app.world())?;
        let labels: Vec<&str> = children
            .iter()
            .filter_map(|child| app.world().get::<menu::Entry>(child))
            .map(|entry| match entry {
                menu::Entry::Zoom(label, _) | menu::Entry::Reset(label) => *label,
            })
            .collect();
        assert_eq!(
            labels,
            [
                "Zoom 75%",
                "Zoom 100%",
                "Zoom 125%",
                "Zoom 150%",
                "Zoom 200%",
                "Reset window layout"
            ]
        );
        let zoom = |app: &App| {
            app.world()
                .resource::<super::super::WindowLayouts>()
                .0
                .get("market")
                .map(|layout| layout.zoom)
        };
        let cover = app
            .world_mut()
            .spawn((
                GlobalZIndex(10000),
                Node {
                    position_type: PositionType::Absolute,
                    width: px(301.),
                    height: px(201.),
                    ..default()
                },
            ))
            .id();
        app.update();
        click_menu_row(&mut app, 0)?;
        assert_eq!(zoom(&app), Some(1.), "covered row must not activate");
        app.world_mut().despawn(cover);
        app.update();
        for (row, (label, value)) in menu::ZOOMS.into_iter().enumerate() {
            if !app
                .world_mut()
                .query::<&menu::WindowMenu>()
                .iter(app.world())
                .any(|_| true)
            {
                right_click(&mut app, panel);
            }
            click_menu_row(&mut app, row)?;
            assert_eq!(zoom(&app), Some(value), "{label}");
            assert_eq!(
                app.world_mut()
                    .query::<&menu::WindowMenu>()
                    .iter(app.world())
                    .count(),
                0,
                "{label} closes the menu"
            );
        }

        right_click(&mut app, panel);
        click_menu_row(&mut app, menu::ZOOMS.len())?;
        assert_eq!(zoom(&app), Some(1.));

        right_click(&mut app, panel);
        press(&mut app, panel);
        assert_eq!(
            app.world_mut()
                .query::<&menu::WindowMenu>()
                .iter(app.world())
                .count(),
            0,
            "a press outside closes the menu"
        );
        Ok(())
    }

    #[test]
    fn every_registered_window_opens_as_a_stacked_resizable_window_with_bounded_zoom() -> TestResult
    {
        let mut app = app();
        app.world_mut().spawn((
            Window {
                resolution: bevy::window::WindowResolution::new(4000, 3000),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ));
        let mut opened = Vec::new();
        for (index, (id, _)) in WINDOWS.iter().enumerate() {
            let (overlay, panel) = window(&mut app, id, 150 + index as i32);
            show(&mut app, overlay)?;
            opened.push((*id, overlay, panel));
        }
        for (id, overlay, panel) in opened {
            assert_eq!(app.world().get::<WindowId>(panel).map(|id| id.0), Some(id));
            assert!(app.world().get::<WindowLayer>(overlay).is_some(), "{id}");
            let grips = app.world().get::<Children>(panel).map_or(0, |children| {
                children
                    .iter()
                    .filter(|child| app.world().get::<ResizeGrip>(*child).is_some())
                    .count()
            });
            assert_eq!(grips, 8, "{id} grips");
            let z = depth(&app, overlay)?;
            assert!((BAND.0..=BAND.1).contains(&z), "{id} at {z}");
            let mut layouts = app.world_mut().resource_mut::<WindowLayouts>();
            let layout = layouts.0.get_mut(id).ok_or(format!("{id} layout"))?;
            layout.set_zoom(0.1, Vec2::splat(4000.));
            assert_eq!(layout.zoom, MIN_ZOOM, "{id}");
            layout.set_zoom(9., Vec2::splat(4000.));
            assert_eq!(layout.zoom, MAX_ZOOM, "{id}");
            app.update();
            let node = app.world().get::<Node>(panel).ok_or("panel")?;
            let min = app
                .world()
                .resource::<WindowLayouts>()
                .0
                .get(id)
                .map(|layout| layout.min())
                .ok_or("layout")?;
            assert_eq!(
                (node.min_width, node.min_height),
                (px(min.x), px(min.y)),
                "{id}"
            );
        }
        Ok(())
    }

    #[test]
    fn bestiary_prey_and_market_stack_raise_on_click_and_close_front_to_back() -> TestResult {
        let mut app = app();
        let (bestiary, bestiary_panel) = window(&mut app, "bestiary", 165);
        let (prey, _) = window(&mut app, "prey", 158);
        let (market, _) = window(&mut app, "market", 157);
        for overlay in [bestiary, prey, market] {
            show(&mut app, overlay)?;
        }
        assert_eq!(app.world().resource::<WindowStack>().front(), Some(market));

        press(&mut app, bestiary_panel);
        assert_eq!(
            app.world().resource::<WindowStack>().front(),
            Some(bestiary)
        );
        assert!(depth(&app, bestiary)? > depth(&app, market)?);
        assert!(depth(&app, market)? > depth(&app, prey)?);

        escape(&mut app);
        assert!(!open(&app, bestiary) && open(&app, market) && open(&app, prey));
        escape(&mut app);
        assert!(!open(&app, market) && open(&app, prey));
        escape(&mut app);
        assert!(!open(&app, prey));
        Ok(())
    }
}

#[test]
fn prey_auto_height_fits_remaining_width() -> TestResult {
    let mut app = app(1600, 900, 1.);
    // Other windows already fit the smaller viewport, so only the screen changes.
    for layout in app
        .world_mut()
        .resource_mut::<WindowLayouts>()
        .0
        .values_mut()
    {
        layout.fit(Vec2::new(800., 600.));
    }
    let prey = app
        .world_mut()
        .spawn(window(
            "prey",
            Node {
                left: percent(8.),
                ..default()
            },
        ))
        .id();
    app.update();
    assert_eq!(app.world().get::<Node>(prey).ok_or("prey")?.width, px(860.));
    app.world_mut()
        .query::<&mut Window>()
        .single_mut(app.world_mut())?
        .resolution
        .set(800., 600.);
    app.update();
    let node = app.world().get::<Node>(prey).ok_or("prey")?;
    assert_eq!(node.width, px(736.));
    assert_eq!(node.height, Val::Auto);
    Ok(())
}

#[test]
fn enlarge_viewport_restores_preferred_size() -> TestResult {
    let (mut app, path, _) = saved_market("independent")?;
    let screen = app
        .world_mut()
        .spawn((
            Window {
                resolution: (1024, 768).into(),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    let market = app.world_mut().spawn(window("market", default())).id();
    let prey = app.world_mut().spawn(window("prey", default())).id();
    app.update();
    assert_eq!(
        app.world().get::<Node>(market).ok_or("market")?.width,
        px(1024.)
    );
    app.world_mut()
        .get_mut::<UiTransform>(prey)
        .ok_or("prey")?
        .translation = Val2::px(12., 34.);
    app.world_mut().write_message(AppExit::Success);
    app.update();
    let saved = WindowLayouts::load(&path)?;
    assert_eq!(saved.0["market"].width(), 1400.);
    assert_eq!(saved.0["prey"].offset, Vec2::new(12., 34.));
    app.world_mut()
        .get_mut::<Window>(screen)
        .ok_or("screen")?
        .resolution
        .set(1600., 900.);
    app.update();
    assert_eq!(
        app.world().get::<Node>(market).ok_or("market")?.width,
        px(1400.)
    );
    std::fs::remove_dir_all(path.parent().ok_or("directory")?)?;
    Ok(())
}

#[test]
fn moving_prey_preserves_other_clients_market_edit() -> TestResult {
    let (mut app, path, _) = saved_market("two-active")?;
    let (mut first, _, _) = saved_market("two-active")?;
    let prey = app.world_mut().spawn(window("prey", default())).id();
    app.update();
    let market = first.world_mut().spawn(window("market", default())).id();
    first.update();
    first
        .world_mut()
        .get_mut::<UiTransform>(market)
        .ok_or("market")?
        .translation = Val2::px(22., 44.);
    first.world_mut().write_message(AppExit::Success);
    first.update();
    let other = WindowLayouts::load(&path)?;
    assert_eq!(other.0["market"].offset, Vec2::new(22., 44.));
    app.world_mut()
        .get_mut::<UiTransform>(prey)
        .ok_or("prey")?
        .translation = Val2::px(12., 34.);
    app.world_mut().write_message(AppExit::Success);
    app.update();
    let saved = WindowLayouts::load(&path)?;
    assert_eq!(saved.0["market"], other.0["market"]);
    assert_eq!(saved.0["prey"].offset, Vec2::new(12., 34.));
    std::fs::remove_dir_all(path.parent().ok_or("folder")?)?;
    Ok(())
}

#[test]
fn save_failure_retries_on_exit() -> TestResult {
    let (mut app, path, _) = saved_market("retry")?;
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs(1),
    ));
    let panel = app.world_mut().spawn(window("market", default())).id();
    app.update();
    let parent = path.parent().ok_or("directory")?;
    let backup = parent.with_extension("backup");
    std::fs::rename(parent, &backup)?;
    std::fs::write(parent, b"blocked")?;
    app.world_mut()
        .get_mut::<UiTransform>(panel)
        .ok_or("panel")?
        .translation = Val2::px(14., 20.);
    for _ in 0..4 {
        app.update();
    }
    std::fs::remove_file(parent)?;
    std::fs::rename(backup, parent)?;
    app.world_mut().write_message(AppExit::Success);
    app.update();
    assert_eq!(
        WindowLayouts::load(&path)?.0["market"].offset,
        Vec2::new(14., 20.)
    );
    std::fs::remove_dir_all(parent)?;
    Ok(())
}

#[test]
fn simultaneous_publications_are_complete_documents() -> TestResult {
    let (_, path, layouts) = saved_market("race")?;
    let barrier = std::sync::Barrier::new(4);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..4)
            .map(|index| {
                let mut layouts = layouts.clone();
                let barrier = &barrier;
                let path = &path;
                scope.spawn(move || -> std::io::Result<()> {
                    layouts
                        .0
                        .get_mut("market")
                        .ok_or_else(|| std::io::Error::other("market"))?
                        .offset
                        .x = index as f32;
                    barrier.wait();
                    for _ in 0..20 {
                        layouts.save(path)?;
                        WindowLayouts::load(path)?;
                    }
                    Ok(())
                })
            })
            .collect();
        for worker in workers {
            worker.join().map_err(|_| "writer panic")??;
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    })?;
    std::fs::remove_dir_all(path.parent().ok_or("directory")?)?;
    Ok(())
}
