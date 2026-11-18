use super::super::{GameModal, GameRegions, GameRoot, setup, source};
use super::*;
use crate::bevy_shell::ui::Skins;
use bevy::picking::{backend::HitData, pointer::Location};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn app() -> App {
    app_with(LayoutStore::default())
}
fn app_with(store: LayoutStore) -> App {
    let mut app = App::new();
    app.insert_resource(store);
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
    Ok(node.height)
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
fn chat_float_toggle_stacks_with_its_window() -> TestResult {
    let mut app = app();
    let toggle = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, name)| name.as_str() == "Chat float")
        .ok_or("chat toggle")?
        .0;
    assert!(app.world().get::<GlobalZIndex>(toggle).is_none());
    assert_eq!(app.world().get::<ZIndex>(toggle), Some(&ZIndex(1)));
    use super::super::{hud_panes, pointer_layout};
    app.insert_non_send(crate::bevy_shell::owner::HostOwner::new(None)?);
    pointer_layout::install(&mut app, UVec2::new(1280, 720));
    app.world_mut().resource_mut::<UiScale>().0 = 1.;
    hud_panes::install(&mut app);
    app.world_mut()
        .resource_mut::<hud_panes::SidebarWidgets>()
        .toggle_region("chat");
    for _ in 0..4 {
        app.update();
    }
    let chat = app.world().resource::<GameRegions>().chat;
    let parent = app.world().get::<ChildOf>(chat).ok_or("parent")?.parent();
    assert_eq!(
        app.world().get::<hud_panes::float::FloatRoot>(parent),
        Some(&hud_panes::float::FloatRoot("chat"))
    );
    app.world_mut().entity_mut(parent).insert(GlobalZIndex(100));
    app.update();
    let point = pointer_layout::center(&app, toggle)?;
    pointer_layout::hit(&mut app, point, toggle)?;
    Ok(())
}

#[test]
fn console_drag_resizes_the_console_and_nothing_resizes_the_sidebar() -> TestResult {
    let mut app = app();
    let handles = handles(&mut app);
    let chat = app.world().resource::<GameRegions>().chat;
    assert_eq!(
        handles
            .iter()
            .map(|(_, splitter)| (splitter.id, splitter.target))
            .collect::<Vec<_>>(),
        [("console", chat)]
    );
    for (entity, splitter) in handles {
        geometry(&mut app, splitter);
        start_drag(&mut app, entity, PointerButton::Primary)?;
        move_drag(&mut app, entity, Vec2::splat(-120.))?;
        assert_eq!(size(&app, splitter)?, px(210.));
        // Distance is cumulative from drag start, rather than added to each prior frame.
        move_drag(&mut app, entity, Vec2::splat(-140.))?;
        assert_eq!(size(&app, splitter)?, px(220.));
        move_drag(&mut app, entity, Vec2::splat(10000.))?;
        assert_eq!(size(&app, splitter)?, px(90.));
        move_drag(&mut app, entity, Vec2::splat(-10000.))?;
        assert_eq!(size(&app, splitter)?, px(600.));
        let node = app.world().get::<Node>(splitter.target).ok_or("region")?;
        assert_eq!(node.max_height, percent(50.));
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
fn game_shell_exposes_only_the_console_handle_and_the_widget_resize_edges()
-> Result<(), Box<dyn std::error::Error>> {
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
    let mut names: Vec<_> = handles
        .iter(app.world())
        .filter(|name| name.as_str().starts_with("Resize "))
        .map(|name| name.as_str().to_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "Resize console",
            "Resize widget",
            "Resize widget",
            "Resize widget",
            "Resize widget",
            "Resize widget"
        ]
    );
    Ok(())
}

#[test]
fn window_geometry_survives_restart_and_rejects_bad_bounds() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("cg-gamewin-layout.json");
    let mut layout = WindowLayout::load(&path)?;
    layout.record("Spell catalogue", [40, 80, 680, 760])?;
    assert!(layout.record("bad", [0, 0, 0, 760]).is_err());
    layout.save(&path)?;
    let restored = WindowLayout::load(&path)?;
    assert_eq!(restored.bounds("Spell catalogue"), Some([40, 80, 680, 760]));
    assert_eq!(restored.bounds("bad"), None);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn resized_regions_and_main_window_geometry_survive_a_restart() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("o033-bevy-layout.json");
    let mut app = app_with(LayoutStore::load(Some(path.clone())));
    for (entity, splitter) in handles(&mut app) {
        geometry(&mut app, splitter);
        start_drag(&mut app, entity, PointerButton::Primary)?;
        move_drag(&mut app, entity, Vec2::splat(-120.))?;
        app.update();
        let dragged = [0, 0, 1, 210];
        assert_ne!(
            LayoutStore::load(Some(path.clone()))
                .layout
                .bounds(splitter.id),
            Some(dragged),
            "a drag in progress is not written before it ends"
        );
        let pointer = pointer(&mut app)?;
        app.world_mut().trigger(PointerDragEnd {
            entity,
            pointer,
            button: PointerButton::Primary,
            distance: Vec2::ZERO,
        });
    }
    let mut windows = app
        .world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>();
    let mut window = windows.single_mut(app.world_mut())?;
    window.position = WindowPosition::At(IVec2::new(40, 80));
    window.resolution = (1280, 900).into();
    app.update();
    let mut restarted = app_with(LayoutStore::load(Some(path.clone())));
    let mut sizes = handles(&mut restarted)
        .into_iter()
        .map(|(_, splitter)| Ok((splitter.id, size(&restarted, splitter)?)))
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    sizes.sort_by_key(|(id, _)| *id);
    assert_eq!(sizes, vec![("console", px(210.))]);
    assert_eq!(
        LayoutStore::load(Some(path.clone())).window(),
        Some([40, 80, 1280, 900])
    );
    for (width, height) in [(5120, 2880), (4096, 2880), (1280, 900)] {
        let mut window = windows.single_mut(app.world_mut())?;
        window.resolution = (width, height).into();
        app.update();
        assert_eq!(
            LayoutStore::load(Some(path.clone())).window(),
            Some([40, 80, width as i32, height as i32])
        );
    }
    let mut window = windows.single_mut(app.world_mut())?;
    window.resolution.set_scale_factor(2.);
    window.resolution.set_physical_resolution(2400, 1400);
    app.update();
    let [_, _, width, height] = LayoutStore::load(Some(path.clone()))
        .window()
        .ok_or("window")?;
    let mut restored = Window {
        resolution: (u32::try_from(width)?, u32::try_from(height)?).into(),
        ..default()
    };
    // Windowed creation requests this logical size from winit, which scales it
    // to the monitor before reporting the physical resolution to Bevy.
    restored
        .resolution
        .set_scale_factor_and_apply_to_physical_size(2.);
    assert_eq!(restored.physical_size(), UVec2::new(2400, 1400));
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn failed_layout_save_retries_after_directory_recovers() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let dir = fixture.path().join("layout-retry");
    std::fs::write(&dir, [])?;

    let path = dir.join("layout.json");
    let mut app = app_with(LayoutStore::load(Some(path.clone())));
    std::fs::remove_file(&dir)?;
    app.update();
    assert!(!path.exists(), "retry must back off");
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(std::time::Duration::from_secs(1));
    app.update();
    assert!(WindowLayout::load(&path)?.bounds("console").is_some());
    Ok(())
}
