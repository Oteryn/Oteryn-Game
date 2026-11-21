use super::super::{
    MAX_ZOOM, MIN_ZOOM, WINDOWS, WindowId, WindowLayer, WindowLayouts, WindowStack, install, menu,
    resize::ResizeGrip, stack::BAND,
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

pub(in crate::bevy_shell) fn ui_layout(app: &mut App, size: UVec2) {
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
fn reset_menu_publishes_once_before_another_client_edit() -> TestResult {
    let fixture_dir = crate::play::screenshot::TestDir::new()?;
    let directory = fixture_dir.join("layout");
    let (mut app, path, mut saved) = super::saved_market(&directory)?;
    app.insert_resource(skins());
    ui_layout(&mut app, UVec2::new(800, 600));
    app.world_mut().spawn((
        Window {
            resolution: (800, 600).into(),
            ..default()
        },
        bevy::window::PrimaryWindow,
    ));
    let panel = app
        .world_mut()
        .spawn(super::super::window("market", default()))
        .id();
    app.update();
    right_click(&mut app, panel);
    let current_zoom = app
        .world_mut()
        .query::<(Entity, &menu::Entry)>()
        .iter(app.world())
        .find_map(|(entity, entry)| matches!(entry, menu::Entry::Zoom(_, 1.)).then_some(entity))
        .ok_or("current zoom")?;
    app.world_mut().trigger(bevy::ui_widgets::Activate {
        entity: current_zoom,
    });
    assert_eq!(
        app.world().resource::<WindowLayouts>().0["market"].size,
        saved.0["market"].size
    );
    right_click(&mut app, panel);
    let entry = app
        .world_mut()
        .query::<(Entity, &menu::Entry)>()
        .iter(app.world())
        .find_map(|(entity, entry)| matches!(entry, menu::Entry::Reset(_)).then_some(entity))
        .ok_or("reset")?;
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: entry });
    assert_eq!(WindowLayouts::load(&path)?, WindowLayouts::default());
    saved.0.get_mut("market").ok_or("market")?.offset.x = 200.;
    saved.save(&path)?;
    app.world_mut().write_message(AppExit::Success);
    app.update();
    assert_eq!(WindowLayouts::load(&path)?, saved);
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
fn every_registered_window_opens_as_a_stacked_resizable_window_with_bounded_zoom() -> TestResult {
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
