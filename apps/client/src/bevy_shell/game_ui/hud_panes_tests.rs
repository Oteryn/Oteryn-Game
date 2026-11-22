use super::*;
use crate::{
    bevy_shell::owner::{ActivePlay, Phase},
    play::{PlayView, play_channel},
};
use bevy::{
    asset::AssetPlugin,
    camera::NormalizedRenderTarget,
    picking::{events::Pointer, pointer::Location},
    window::{PrimaryWindow, WindowRef},
};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn playing_app(preferences: std::path::PathBuf) -> Result<App, Box<dyn std::error::Error>> {
    playing_app_with(preferences, (1280, 720), |_| {})
}
fn playing_app_with(
    preferences: std::path::PathBuf,
    resolution: (u32, u32),
    configure: impl FnOnce(&mut App),
) -> Result<App, Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<UiScale>();
    configure(&mut app);
    let mut host = HostOwner::new(Some(preferences))?;
    let (link, _commands, _outbox) = play_channel();
    host.phase = Phase::Playing;
    host.play = Some(ActivePlay {
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    app.insert_non_send(host);
    app.world_mut().spawn((
        Window {
            focused: true,
            resolution: resolution.into(),
            ..default()
        },
        PrimaryWindow,
    ));
    app.add_systems(
        Startup,
        (crate::bevy_shell::ui::setup, super::super::setup).chain(),
    );
    install(&mut app);
    app.update();
    let mut roots = app
        .world_mut()
        .query_filtered::<&mut Node, With<super::super::GameRoot>>();
    for mut root in roots.iter_mut(app.world_mut()) {
        root.display = Display::Flex;
    }
    app.update();
    Ok(app)
}
fn entity_with<C: Component>(
    app: &mut App,
    found: impl Fn(&C) -> bool,
) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &C)>()
        .iter(app.world())
        .find(|(_, component)| found(component))
        .map(|(entity, _)| entity)
        .ok_or_else(|| "widget entity missing".into())
}
fn activate(app: &mut App, entity: Entity) {
    app.world_mut().trigger(Activate { entity });
    app.update();
    app.update();
}
fn frame_node(app: &mut App, widget: Widget) -> Result<Node, Box<dyn std::error::Error>> {
    let entity = entity_with::<WidgetFrame>(app, |frame| frame.0 == widget)?;
    Ok(app.world().get::<Node>(entity).ok_or("frame node")?.clone())
}
/// The widgets of a pane in display order, with whether each is shown.
fn pane(app: &mut App, pane: Entity) -> Result<Vec<(Widget, bool)>, Box<dyn std::error::Error>> {
    let children: Vec<Entity> = app
        .world()
        .get::<Children>(pane)
        .ok_or("pane children")?
        .iter()
        .collect();
    Ok(children
        .into_iter()
        .filter_map(|child| {
            let frame = app.world().get::<WidgetFrame>(child)?;
            let node = app.world().get::<Node>(child)?;
            Some((frame.0, node.display == Display::Flex))
        })
        .collect())
}
fn pointer(app: &mut App) -> Result<Pointer, Box<dyn std::error::Error>> {
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())?;
    Ok(Pointer::new(
        PointerId::Mouse,
        Location {
            target: WindowRef::Entity(window)
                .normalize(None)
                .map(NormalizedRenderTarget::Window)
                .ok_or("window")?,
            position: Vec2::new(20., 120.),
        },
    ))
}

include!("hud_panes_widget_tests.rs");

fn moved_dock() -> SidebarWidgets {
    let mut state = SidebarWidgets::default();
    state.set_sidebar(Side::Left, true);
    state.place(Widget::Skills, Dock::Left, None);
    state.place(Widget::Vip, Dock::Right, Some(Widget::BattleList));
    state.set_open(Widget::AnalyticsSelector, true);
    if let Some(entry) = state.entry_mut(Widget::BattleList) {
        entry.minimised = true;
        entry.height = Some(120.);
    }
    state
}

#[test]
fn the_dock_layout_round_trips_through_its_file_and_drops_bad_entries() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("dock-file.json");
    let _ = std::fs::remove_file(&path);
    assert_eq!(SidebarWidgets::load(&path)?, SidebarWidgets::default());
    let mut state = moved_dock();
    state.save(&path)?;
    assert_eq!(SidebarWidgets::load(&path)?, state);
    std::fs::write(
        &path,
        r#"{"left":false,"extra":false,"modules":[
            {"module":"module.unknown","dock":"right","open":true,"minimised":false,"height":null},
            {"module":"module.skills","dock":"left","open":true,"minimised":false,"height":1e9},
            {"module":"module.skills","dock":"right","open":false,"minimised":false,"height":null}
        ]}"#,
    )?;
    let loaded = SidebarWidgets::load(&path)?;
    let skills = loaded.entry(Widget::Skills).ok_or("skills")?;
    assert_eq!(
        skills.dock,
        Dock::Right,
        "a hidden sidebar cannot hold modules"
    );
    assert_eq!(skills.height, Some(MAX_CONTENT));
    assert!(skills.open);
    assert_eq!(loaded.entries.len(), Widget::ALL.len());
    std::fs::remove_file(path)?;
    Ok(())
}

fn settle_dock(app: &mut App) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while app.world().resource::<dock::Persistence>().writing() {
        assert!(std::time::Instant::now() < deadline);
        app.update();
        std::thread::yield_now();
    }
}

#[test]
fn each_character_loads_its_own_dock_layout() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("dock-characters.json");
    let mut app = playing_app(path.clone())?;
    let (first, second) = ([1; 16], [2; 16]);
    let files = [first, second].map(|character| {
        let hex: String = character.iter().map(|byte| format!("{byte:02x}")).collect();
        path.with_extension(format!("dock.{hex}.json"))
    });
    for file in &files {
        let _ = std::fs::remove_file(file);
    }
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .enter_character(Some(first));
    app.update();
    assert_eq!(
        *app.world().resource::<SidebarWidgets>(),
        SidebarWidgets::default()
    );
    *app.world_mut().resource_mut::<SidebarWidgets>() = moved_dock();
    app.update();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .enter_character(Some(second));
    app.update();
    assert_eq!(
        *app.world().resource::<SidebarWidgets>(),
        SidebarWidgets::default(),
        "the second character starts from the default dock"
    );
    settle_dock(&mut app);
    assert_eq!(SidebarWidgets::load(&files[0])?, moved_dock());
    app.world_mut()
        .resource_mut::<SidebarWidgets>()
        .set_sidebar(Side::Right, true);
    app.update();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .enter_character(Some(first));
    app.update();
    assert_eq!(*app.world().resource::<SidebarWidgets>(), moved_dock());
    settle_dock(&mut app);
    assert!(SidebarWidgets::load(&files[1])?.extra);
    for file in &files {
        std::fs::remove_file(file)?;
    }
    Ok(())
}

#[test]
fn reset_window_layout_restores_and_saves_the_default_dock() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("dock-reset.json");
    let mut app = playing_app(path.clone())?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .enter_character(Some([3; 16]));
    app.update();
    *app.world_mut().resource_mut::<SidebarWidgets>() = moved_dock();
    app.update();
    app.world_mut()
        .trigger(crate::bevy_shell::ui::window_layout::LayoutReset);
    app.update();
    assert_eq!(
        *app.world().resource::<SidebarWidgets>(),
        SidebarWidgets::default()
    );
    let file = app
        .world()
        .non_send::<HostOwner>()
        .dock_layout_path()
        .ok_or("dock path")?;
    settle_dock(&mut app);
    assert_eq!(SidebarWidgets::load(&file)?, SidebarWidgets::default());
    std::fs::remove_file(file)?;
    Ok(())
}

#[test]
fn module_zoom_scales_the_content_and_the_title_within_bounds() -> TestResult {
    use crate::bevy_shell::ui::window_layout::{MAX_ZOOM, MIN_ZOOM, WindowLayouts};
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("dock-zoom.json");
    let mut app = playing_app(path.clone())?;
    crate::bevy_shell::ui::window_layout::install(&mut app);
    app.update();
    let frame = entity_with::<WidgetFrame>(&mut app, |frame| frame.0 == Widget::Skills)?;
    assert_eq!(
        app.world().get::<WindowId>(frame).map(|id| id.0),
        Some("module.skills")
    );
    let body = entity_with::<Body>(&mut app, |body| body.0 == Widget::Skills)?;
    let bar = entity_with::<TitleBar>(&mut app, |bar| bar.widget == Widget::Skills)?;
    let title = app
        .world()
        .get::<Children>(bar)
        .and_then(|children| children.first().copied())
        .ok_or("title")?;
    let base = match app.world().get::<TextFont>(title).ok_or("font")?.font_size {
        bevy::text::FontSize::Px(size) => size,
        _ => return Err("font size".into()),
    };
    for (requested, zoom) in [(1.5, 1.5), (9., MAX_ZOOM), (0.1, MIN_ZOOM)] {
        app.world_mut()
            .resource_mut::<WindowLayouts>()
            .0
            .get_mut("module.skills")
            .ok_or("layout")?
            .set_zoom(requested, Vec2::splat(4000.));
        app.update();
        app.update();
        assert_eq!(
            app.world().get::<Node>(body).ok_or("body")?.height,
            px(200. * zoom)
        );
        assert_eq!(
            app.world().get::<TextFont>(title).ok_or("font")?.font_size,
            bevy::text::FontSize::Px(base * zoom)
        );
    }
    let battle = entity_with::<Body>(&mut app, |body| body.0 == Widget::BattleList)?;
    assert_eq!(
        app.world().get::<Node>(battle).ok_or("battle")?.height,
        px(75.),
        "zoom is per module"
    );
    Ok(())
}

/// The real Taffy layout of the game shell at one physical resolution.
fn laid_out_app(width: u32, height: u32) -> Result<App, Box<dyn std::error::Error>> {
    laid_out_app_with(width, height, |_| {})
}

pub(in crate::bevy_shell::game_ui) fn laid_out_app_with(
    width: u32,
    height: u32,
    configure: impl FnOnce(&mut App),
) -> Result<App, Box<dyn std::error::Error>> {
    use bevy::{
        app::{HierarchyPropagatePlugin, PropagateSet},
        camera::{ComputedCameraValues, RenderTargetInfo},
        input_focus::InputFocus,
        text::{FontCx, RemSize},
        ui::{
            ComputedUiRenderTargetInfo, ComputedUiTargetCamera, UiSystems, ui_layout_system,
            ui_surface::UiSurface, update::propagate_ui_target_cameras,
        },
    };
    let fixture = tempfile::tempdir()?;
    let mut app = playing_app_with(fixture.path().join("fit.json"), (width, height), |app| {
        configure(app);
        app.add_plugins((
            oteryn_client_ui_bevy::GlobalUiPlugin,
            HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(PostUpdate),
            HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(PostUpdate),
        ))
        .init_resource::<UiSurface>()
        .init_resource::<FontCx>()
        .init_resource::<RemSize>()
        .init_resource::<InputFocus>()
        .init_resource::<Assets<TextureAtlasLayout>>()
        .configure_sets(
            PostUpdate,
            (
                UiSystems::Prepare,
                UiSystems::Propagate,
                UiSystems::Content,
                UiSystems::Layout,
            )
                .chain(),
        )
        .configure_sets(
            PostUpdate,
            PropagateSet::<ComputedUiTargetCamera>::default().in_set(UiSystems::Propagate),
        )
        .configure_sets(
            PostUpdate,
            PropagateSet::<ComputedUiRenderTargetInfo>::default().in_set(UiSystems::Propagate),
        )
        .add_systems(
            PostUpdate,
            (
                propagate_ui_target_cameras.in_set(UiSystems::Prepare),
                bevy::ui::widget::update_image_content_size_system.in_set(UiSystems::Content),
                ui_layout_system.in_set(UiSystems::Layout),
            ),
        );
    })?;
    app.insert_non_send(fixture);
    for mut camera in app
        .world_mut()
        .query::<&mut Camera>()
        .iter_mut(app.world_mut())
    {
        camera.computed = ComputedCameraValues {
            target_info: Some(RenderTargetInfo {
                physical_size: UVec2::new(width, height),
                scale_factor: 1.,
            }),
            ..default()
        };
    }
    app.update();
    app.update();
    Ok(app)
}
pub(in crate::bevy_shell::game_ui) fn rect(
    app: &App,
    entity: Entity,
) -> Result<Rect, Box<dyn std::error::Error>> {
    let node = app
        .world()
        .get::<ComputedNode>(entity)
        .ok_or("computed node")?;
    let transform = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .ok_or("transform")?;
    Ok(Rect::from_center_size(transform.translation, node.size()))
}

#[test]
fn every_module_fits_without_overlap_from_1020x650_to_4k() -> TestResult {
    for (width, height) in [(1020, 650), (3840, 2160)] {
        let mut app = laid_out_app(width, height)?;
        {
            let panels = &mut app
                .world_mut()
                .non_send_mut::<HostOwner>()
                .into_inner()
                .settings
                .preferences
                .visible_panels;
            for id in ["battle", "inventory", "vip"] {
                if !panels.iter().any(|panel| panel == id) {
                    panels.push(id.into());
                }
            }
        }
        {
            let mut state = app.world_mut().resource_mut::<SidebarWidgets>();
            state.set_sidebar(Side::Left, true);
            state.set_sidebar(Side::Right, true);
            for widget in Widget::ALL {
                state.set_open(widget, true);
            }
            state.place(Widget::Skills, Dock::Left, None);
            state.place(Widget::Vip, Dock::Extra, None);
        }
        app.update();
        app.update();
        let screen = Rect::new(0., 0., width as f32, height as f32);
        let world_region = app.world().resource::<super::super::GameRegions>().world;
        let world = rect(&app, world_region)?;
        assert!(
            world.width() > 0. && world.height() > 0.,
            "{width}x{height} world {world:?}"
        );
        let mut frames = Vec::new();
        for (entity, frame, node) in app
            .world_mut()
            .query::<(Entity, &WidgetFrame, &Node)>()
            .iter(app.world())
        {
            if node.display == Display::Flex {
                frames.push((frame.0, entity));
            }
        }
        assert_eq!(frames.len(), Widget::ALL.len(), "{width}x{height}");
        let rects = frames
            .iter()
            .map(|(widget, entity)| Ok((*widget, rect(&app, *entity)?)))
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
        for (index, (widget, area)) in rects.iter().enumerate() {
            assert!(area.width() > 0., "{widget:?} at {width}x{height}");
            assert!(
                area.min.x >= screen.min.x && area.max.x <= screen.max.x,
                "{widget:?} {area:?} leaves {width}x{height}"
            );
            assert!(
                area.intersect(world).is_empty(),
                "{widget:?} {area:?} covers the game window {world:?} at {width}x{height}"
            );
            for (other, other_area) in &rects[index + 1..] {
                assert!(
                    area.intersect(*other_area).is_empty(),
                    "{widget:?} {area:?} overlaps {other:?} {other_area:?} at {width}x{height}"
                );
            }
        }
    }
    Ok(())
}

#[path = "hud_panes_float_tests.rs"]
mod floating;

#[test]
fn logout_retries_failed_dock_save() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let mut app = playing_app(fixture.path().join("dock-retry.json"))?;
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs(2),
    ));
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .enter_character(Some([5; 16]));
    app.update();
    let path = app
        .world()
        .non_send::<HostOwner>()
        .dock_layout_path()
        .ok_or("dock path")?;
    let staging = path.with_extension("json.tmp");
    std::fs::create_dir(&staging)?;
    *app.world_mut().resource_mut::<SidebarWidgets>() = moved_dock();
    app.update();
    app.update();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .enter_character(None);
    app.update();
    std::fs::remove_dir(staging)?;
    app.update();
    settle_dock(&mut app);
    assert_eq!(SidebarWidgets::load(&path)?, moved_dock());
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn close_flushes_unsettled_dock() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let mut app = playing_app(fixture.path().join("dock-close.json"))?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .enter_character(Some([4; 16]));
    app.update();
    let path = app
        .world()
        .non_send::<HostOwner>()
        .dock_layout_path()
        .ok_or("dock path")?;
    *app.world_mut().resource_mut::<SidebarWidgets>() = moved_dock();
    app.world_mut().non_send_mut::<HostOwner>().close();
    app.update();
    settle_dock(&mut app);
    assert_eq!(SidebarWidgets::load(&path)?, moved_dock());
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn c2d_dock_reset_completes_after_frame_releases_lock() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let mut app = playing_app(fixture.path().join("preferences.json"))?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .enter_character(Some([9; 16]));
    let path = app
        .world()
        .non_send::<HostOwner>()
        .dock_layout_path()
        .ok_or("dock path")?;
    moved_dock().save(&path)?;
    app.update();
    let held = crate::atomic_file::lock(&path)?;
    app.world_mut()
        .trigger(crate::bevy_shell::ui::window_layout::LayoutReset);
    app.update();
    assert_eq!(
        *app.world().resource::<SidebarWidgets>(),
        SidebarWidgets::default()
    );
    assert_eq!(SidebarWidgets::load(&path)?, moved_dock());
    drop(held);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while SidebarWidgets::load(&path)? != SidebarWidgets::default() {
        assert!(std::time::Instant::now() < deadline);
        app.update();
    }
    Ok(())
}
