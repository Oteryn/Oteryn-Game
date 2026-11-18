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

#[test]
fn one_fixed_right_sidebar_holds_the_panel_then_the_default_widgets() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("default.json");
    let _ = std::fs::remove_file(&path);
    let mut app = playing_app(path.clone())?;
    let sidebar = *app.world().resource::<super::super::Sidebar>();
    let regions = app.world().resource::<super::super::GameRegions>();
    let panel_order = [regions.minimap, regions.vitals, regions.equipment];
    let panel: Vec<Entity> = app
        .world()
        .get::<Children>(
            app.world()
                .get::<ChildOf>(regions.minimap)
                .ok_or("panel")?
                .parent(),
        )
        .ok_or("panel children")?
        .iter()
        .collect();
    assert_eq!(panel[..3], panel_order);
    assert!(
        app.world()
            .get::<super::super::ToolbarRegion>(panel[3])
            .is_some(),
        "the control buttons close the fixed panel"
    );
    for (column, shown) in [(sidebar.left, false), (sidebar.extra, false)] {
        let node = app.world().get::<Node>(column).ok_or("column")?;
        assert_eq!(node.display == Display::Flex, shown);
    }
    let mut columns = app.world_mut().query_filtered::<&Node, With<Column>>();
    for node in columns.iter(app.world()) {
        assert_eq!(
            (node.width, node.min_width, node.max_width, node.height),
            (px(176.), px(176.), px(176.), percent(100.))
        );
    }
    // battleList, container, skills and vip in Tibia order; the VIP panel itself is closed
    // by the default settings until it is opened, and the analytics selector stays closed.
    let widgets = pane(&mut app, sidebar.right_widgets)?;
    assert_eq!(
        widgets,
        [
            (Widget::BattleList, true),
            (Widget::Containers, true),
            (Widget::Skills, true),
            (Widget::Vip, false),
            (Widget::AnalyticsSelector, false),
        ]
    );
    let state = app.world().resource::<SidebarWidgets>();
    assert!(state.entry(Widget::Vip).is_some_and(|entry| entry.open));
    // The control buttons: Store over the grid of the default set, Options, Logout and the
    // button manager on the right; no text toolbar while playing.
    let mut shown: Vec<String> = app
        .world_mut()
        .query::<(
            &crate::bevy_shell::ui::toolbar::grid::ControlButton,
            &art::ArtSlot,
            &Node,
        )>()
        .iter(app.world())
        .filter(|(_, _, node)| node.display == Display::Flex)
        .map(|(_, slot, _)| slot.key.clone())
        .collect();
    shown.sort();
    assert_eq!(
        shown,
        [
            "control-action-bars",
            "control-analysers",
            "control-battle",
            "control-bestiary-tracker",
            "control-boss-slots",
            "control-bosstiary",
            "control-bosstiary-tracker",
            "control-compendium",
            "control-cyclopedia",
            "control-daily-rewards",
            "control-forge",
            "control-friends",
            "control-highscores",
            "control-imbuement-tracker",
            "control-lens-help",
            "control-logout",
            "control-manage",
            "control-options",
            "control-player-guide",
            "control-prey",
            "control-prey-widget",
            "control-quest-log",
            "control-skills",
            "control-spellbook",
            "control-store",
            "control-task-board",
            "control-vip",
            "control-weapon-proficiency",
            "control-wheel",
        ]
    );

    let titles: Vec<&str> = Widget::ALL.iter().map(|widget| widget.title()).collect();
    assert_eq!(
        titles,
        [
            "Battle List",
            "Containers",
            "Skills",
            "VIP",
            "Analytics Selector"
        ]
    );
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn title_buttons_minimise_to_the_header_and_close_the_widget() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("chrome.json");
    let _ = std::fs::remove_file(&path);
    let mut app = playing_app(path.clone())?;
    let minimise = entity_with::<TitleButton>(&mut app, |button| {
        *button == TitleButton::Minimise(Widget::Skills)
    })?;
    activate(&mut app, minimise);
    assert_eq!(frame_node(&mut app, Widget::Skills)?.height, px(MINIMISED));
    let body = entity_with::<Body>(&mut app, |body| body.0 == Widget::Skills)?;
    assert_eq!(
        app.world().get::<Node>(body).ok_or("body")?.display,
        Display::None
    );
    activate(&mut app, minimise);
    assert_eq!(frame_node(&mut app, Widget::Skills)?.height, Val::Auto);
    assert_eq!(
        app.world().get::<Node>(body).ok_or("body")?.height,
        px(200.)
    );
    let close = entity_with::<TitleButton>(&mut app, |button| {
        *button == TitleButton::Close(Widget::Skills)
    })?;
    activate(&mut app, close);
    assert_eq!(frame_node(&mut app, Widget::Skills)?.display, Display::None);
    assert!(
        app.world_mut()
            .resource_mut::<SidebarWidgets>()
            .toggle(Widget::Skills)
    );
    app.update();
    assert_eq!(frame_node(&mut app, Widget::Skills)?.display, Display::Flex);
    // Closing the battle list goes through the saved panel setting.
    let close = entity_with::<TitleButton>(&mut app, |button| {
        *button == TitleButton::Close(Widget::BattleList)
    })?;
    activate(&mut app, close);
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .panel_visible("battle")
    );
    assert_eq!(
        frame_node(&mut app, Widget::BattleList)?.display,
        Display::None
    );
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn sidebar_arrows_add_a_sidebar_and_removing_it_returns_its_widgets() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("arrows.json");
    let _ = std::fs::remove_file(&path);
    let mut app = playing_app(path.clone())?;
    let sidebar = *app.world().resource::<super::super::Sidebar>();
    let add_left = entity_with::<SidebarArrow>(&mut app, |arrow| {
        *arrow
            == SidebarArrow {
                side: Side::Left,
                add: true,
            }
    })?;
    activate(&mut app, add_left);
    assert_eq!(
        app.world().get::<Node>(sidebar.left).ok_or("left")?.display,
        Display::Flex
    );
    app.world_mut()
        .resource_mut::<SidebarWidgets>()
        .place(Widget::Skills, Dock::Left, None);
    app.update();
    app.update();
    assert_eq!(
        pane(&mut app, sidebar.left_widgets)?,
        [(Widget::Skills, true)]
    );
    let remove_left = entity_with::<SidebarArrow>(&mut app, |arrow| {
        *arrow
            == SidebarArrow {
                side: Side::Left,
                add: false,
            }
    })?;
    activate(&mut app, remove_left);
    assert_eq!(
        app.world().get::<Node>(sidebar.left).ok_or("left")?.display,
        Display::None
    );
    assert_eq!(pane(&mut app, sidebar.left_widgets)?, []);
    assert_eq!(
        pane(&mut app, sidebar.right_widgets)?
            .into_iter()
            .map(|(widget, _)| widget)
            .collect::<Vec<_>>(),
        [
            Widget::BattleList,
            Widget::Containers,
            Widget::Vip,
            Widget::AnalyticsSelector,
            Widget::Skills,
        ]
    );
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn widgets_reorder_before_another_widget_or_at_the_end_of_a_dock() {
    let mut state = SidebarWidgets::default();
    state.place(Widget::Vip, Dock::Right, Some(Widget::BattleList));
    state.place(Widget::BattleList, Dock::Extra, None);
    state.place(Widget::Skills, Dock::Extra, Some(Widget::BattleList));
    let order: Vec<_> = state
        .entries
        .iter()
        .map(|entry| (entry.widget, entry.dock))
        .collect();
    assert_eq!(
        order,
        [
            (Widget::Vip, Dock::Right),
            (Widget::Containers, Dock::Right),
            (Widget::AnalyticsSelector, Dock::Right),
            (Widget::Skills, Dock::Extra),
            (Widget::BattleList, Dock::Extra),
        ]
    );
    state.set_open(Widget::Vip, false);
    assert!(!state.entry(Widget::Vip).is_some_and(|entry| entry.open));
    assert!(state.toggle(Widget::Vip));
}

#[test]
fn the_bottom_edge_resizes_the_content_within_bounds() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("resize.json");
    let _ = std::fs::remove_file(&path);
    let mut app = playing_app(path.clone())?;
    let body = entity_with::<Body>(&mut app, |body| body.0 == Widget::BattleList)?;
    app.world_mut().entity_mut(body).insert(ComputedNode {
        size: Vec2::new(160., 150.),
        inverse_scale_factor: 0.5,
        ..default()
    });
    let handle = entity_with::<Resize>(&mut app, |handle| handle.widget == Widget::BattleList)?;
    let pointer = pointer(&mut app)?;
    let hit = bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0., None, None);
    app.world_mut().trigger(PointerDragStart {
        entity: handle,
        pointer: pointer.clone(),
        button: PointerButton::Primary,
        hit: hit.clone(),
    });
    for (distance, height) in [(25., 100.), (-1000., MIN_CONTENT), (1000., 600.)] {
        app.world_mut().trigger(PointerDrag {
            entity: handle,
            pointer: pointer.clone(),
            button: PointerButton::Primary,
            distance: Vec2::new(0., distance),
            delta: Vec2::ZERO,
        });
        app.update();
        assert_eq!(
            app.world().get::<Node>(body).ok_or("body")?.height,
            px(height)
        );
    }
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn control_buttons_toggle_the_sidebar_widgets() -> TestResult {
    use crate::bevy_shell::ui::{Control, toolbar::grid::ControlButton};
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("controls.json");
    let _ = std::fs::remove_file(&path);
    let mut app = playing_app(path.clone())?;
    let skills = entity_with::<ControlButton>(&mut app, |button| {
        matches!(button, ControlButton::Grid(Control::Skills))
    })?;
    activate(&mut app, skills);
    assert_eq!(frame_node(&mut app, Widget::Skills)?.display, Display::None);
    activate(&mut app, skills);
    assert_eq!(frame_node(&mut app, Widget::Skills)?.display, Display::Flex);
    let analytics = entity_with::<ControlButton>(&mut app, |button| {
        matches!(button, ControlButton::Grid(Control::Analysers))
    })?;
    activate(&mut app, analytics);
    assert_eq!(
        frame_node(&mut app, Widget::AnalyticsSelector)?.display,
        Display::Flex
    );
    let battle = entity_with::<ControlButton>(&mut app, |button| {
        matches!(button, ControlButton::Grid(Control::Panel("battle")))
    })?;
    activate(&mut app, battle);
    assert_eq!(
        frame_node(&mut app, Widget::BattleList)?.display,
        Display::None
    );
    let _ = std::fs::remove_file(path);
    Ok(())
}
#[test]
fn tibia_control_buttons_open_their_oteryn_windows() -> TestResult {
    use crate::bevy_shell::ui::toolbar::grid::{ControlButton, Extra};
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("extra-controls.json");
    let _ = std::fs::remove_file(&path);
    let mut app = playing_app(path.clone())?;
    let press = |app: &mut App, extra: Extra| -> TestResult {
        let entity = entity_with::<ControlButton>(
            app,
            |button| matches!(button, ControlButton::Extra(found) if *found == extra),
        )?;
        assert!(
            !app.world()
                .entity(entity)
                .contains::<bevy::ui::InteractionDisabled>()
        );
        activate(app, entity);
        Ok(())
    };
    fn link<T>(
        app: &App,
        read: impl Fn(&crate::play::PlayLink) -> T,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let owner = app.world().non_send::<HostOwner>();
        Ok(read(&owner.play.as_ref().ok_or("play")?.link))
    }
    press(&mut app, Extra::LensHelp)?;
    assert!(link(&app, |link| link.lens_active())?);
    link(&app, |link| link.lens_toggle())?;
    press(&mut app, Extra::PreyWidget)?;
    assert!(link(&app, |link| link.prey_panel().open)?);
    press(&mut app, Extra::ImbuementTracker)?;
    assert!(link(&app, |link| link.imbue_panel().tracker_open)?);
    press(&mut app, Extra::PlayerGuide)?;
    assert!(link(&app, |link| link.notices_panel().open)?);
    press(&mut app, Extra::Friends)?;
    assert!(link(&app, |link| link.vip_panel().account_open)?);
    press(&mut app, Extra::BossSlots)?;
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .panel_visible("bosstiary")
    );
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn dragging_a_module_title_drops_it_into_the_other_column() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("drag-column.json");
    let mut app = playing_app(path.clone())?;
    app.world_mut()
        .resource_mut::<SidebarWidgets>()
        .set_sidebar(Side::Left, true);
    app.update();
    let sidebar = *app.world().resource::<super::super::Sidebar>();
    for (pane, x) in [(sidebar.left_widgets, 88.), (sidebar.right_widgets, 1192.)] {
        app.world_mut().entity_mut(pane).insert((
            ComputedNode {
                size: Vec2::new(176., 600.),
                inverse_scale_factor: 1.,
                ..default()
            },
            UiGlobalTransform::from_translation(Vec2::new(x, 300.)),
        ));
    }
    let bar = entity_with::<TitleBar>(&mut app, |bar| bar.widget == Widget::Skills)?;
    let pointer = pointer(&mut app)?;
    let hit = bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0., None, None);
    app.world_mut().trigger(PointerDragStart {
        entity: bar,
        pointer: pointer.clone(),
        button: PointerButton::Primary,
        hit,
    });
    app.world_mut().trigger(PointerDragEnd {
        entity: bar,
        pointer,
        button: PointerButton::Primary,
        distance: Vec2::new(-1000., 0.),
    });
    app.update();
    let state = app.world().resource::<SidebarWidgets>();
    assert_eq!(
        state.entry(Widget::Skills).map(|entry| entry.dock),
        Some(Dock::Left)
    );
    assert!(pane(&mut app, sidebar.left_widgets)?.contains(&(Widget::Skills, true)));
    assert!(
        !pane(&mut app, sidebar.right_widgets)?
            .iter()
            .any(|(widget, _)| *widget == Widget::Skills)
    );
    Ok(())
}

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
    let state = moved_dock();
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
    assert_eq!(SidebarWidgets::load(&path)?, moved_dock());
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn scaled_context_menu_keeps_its_last_entry_inside_the_window() -> TestResult {
    check_scaled_context_menus(bevy::text::DEFAULT_FONT_DATA, None)
}

#[test]
fn scaled_context_menus_fit_production_skin_font_or_wider_fixture() -> TestResult {
    if let Some(root) = std::env::var_os("OTERYN_UI_ASSETS") {
        let font = std::fs::read(std::path::Path::new(&root).join("fonts/verdanab.ttf"))?;
        check_scaled_context_menus(&font, None)
    } else {
        // At 13 px Bevy's bundled Fira Mono is wider on these menu captions than
        // the production 11 px Verdana Bold, including "Communication Lists".
        check_scaled_context_menus(bevy::text::DEFAULT_FONT_DATA, Some(13.))
    }
}

fn check_scaled_context_menus(font_bytes: &[u8], fixture_size: Option<f32>) -> TestResult {
    use super::super::{actions, chat};
    use crate::bevy_shell::gameplay_dialogs::popup;
    use bevy::ui::UiSystems;
    for (pointer, forced, speech) in [
        (Vec2::new(1190., 715.), false, true),
        (Vec2::new(1190., 715.), true, false),
        (Vec2::new(150., 90.), false, false),
    ] {
        let mut app = laid_out_app_with(1200, 720, |app| {
            app.add_plugins(bevy::text::TextPlugin)
                .add_message::<bevy::input::keyboard::KeyboardInput>()
                .add_systems(
                    PostUpdate,
                    bevy::ui::widget::measure_text_system
                        .after(bevy::text::detect_text_needs_rerender)
                        .in_set(UiSystems::Content),
                );
            actions::install(app);
            chat::install(app);
        })?;
        app.insert_resource(UiScale(1.5))
            .add_systems(Update, popup::layout);
        let font = app.world().resource::<Skins>().font.clone();
        assert_eq!(
            font.path().map(|path| path.path()),
            Some(std::path::Path::new("fonts/verdanab.ttf"))
        );
        app.world_mut()
            .resource_mut::<Assets<Font>>()
            .insert(font.id(), Font::from_bytes(font_bytes.to_vec()))?;
        let target = if speech {
            let owner = app.world().non_send::<HostOwner>();
            crate::play::feature_0007_chat::protocol::edit_chat_fixture(
                &owner.play.as_ref().ok_or("play")?.link,
                |panel| {
                    panel.selected = true;
                    panel.lines.push(oteryn_session::ChatLine::Private {
                        speaker_name: "Alice".into(),
                        text: "bounds fixture".into(),
                    });
                },
            );
            app.update();
            app.update();
            app.world_mut()
                .query::<(Entity, &bevy::text::EditableText)>()
                .iter(app.world())
                .find_map(|(entity, text)| {
                    text.value()
                        .to_string()
                        .contains("bounds fixture")
                        .then_some(entity)
                })
                .ok_or("transcript row")?
        } else {
            actions::tests::slot(&mut app, 0)?
        };
        let mut click = actions::tests::secondary_click(target);
        click.pointer = bevy::picking::events::Pointer::new(
            bevy::picking::pointer::PointerId::Mouse,
            bevy::picking::pointer::Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: 1200,
                    height: 720,
                },
                position: pointer,
            },
        );
        app.world_mut().trigger(click);
        app.world_mut().flush();
        let last = if speech {
            "Mirror This Tab"
        } else {
            "Reset Zoom"
        };
        let (caption, entry) = app
            .world_mut()
            .query::<(Entity, &Text, &ChildOf)>()
            .iter(app.world())
            .find_map(|(entity, text, parent)| {
                (text.0 == last
                    && app
                        .world()
                        .get::<popup::PopupEntry>(parent.parent())
                        .is_some())
                .then_some((entity, parent.parent()))
            })
            .ok_or("final menu entry")?;
        let frame = app
            .world()
            .get::<ChildOf>(entry)
            .ok_or("popup frame")?
            .parent();
        let scrim = app
            .world_mut()
            .query_filtered::<Entity, With<popup::PopupScrim>>()
            .single(app.world())?;
        if let Some(size) = fixture_size {
            let mut labels = app.world_mut().query::<(Entity, &ChildOf, &TextFont)>();
            let labels: Vec<_> = labels
                .iter(app.world())
                .filter(|(_, parent, _)| {
                    app.world()
                        .get::<popup::PopupEntry>(parent.parent())
                        .is_some()
                })
                .map(|(entity, _, _)| entity)
                .collect();
            for label in labels {
                app.world_mut()
                    .get_mut::<TextFont>(label)
                    .ok_or("menu label font")?
                    .font_size = bevy::text::FontSize::Px(size);
            }
        }
        if forced {
            app.world_mut().get_mut::<Node>(frame).ok_or("frame")?.width = px(150.);
        }
        app.update();
        let caption_font = &app
            .world()
            .get::<TextFont>(caption)
            .ok_or("caption font")?
            .font;
        let bevy::text::FontSource::Handle(handle) = caption_font else {
            return Err("popup caption must use the skin font handle".into());
        };
        assert_eq!(handle.id(), font.id());
        let fonts = app.world().resource::<Assets<Font>>();
        assert!(
            fonts.get(handle).ok_or("loaded popup font")?.data.as_ref() == font_bytes,
            "popup caption must be measured with the supplied font bytes"
        );
        let menu = rect(&app, frame)?;
        assert!(menu.width() > 0. && menu.height() > 0.);
        let last = rect(&app, entry)?;
        assert!(menu.contains(last.min) && menu.contains(last.max));
        let mut labels = app
            .world_mut()
            .query::<(Entity, &ChildOf, &bevy::text::ComputedTextBlock)>();
        for (label, parent, block) in labels.iter(app.world()) {
            if app
                .world()
                .get::<popup::PopupEntry>(parent.parent())
                .is_some()
            {
                let intrinsic = block.buffer().calculate_content_widths().max;
                let bounds = rect(&app, label)?;
                assert!(
                    intrinsic > 0. && bounds.width() + 1. >= intrinsic,
                    "{intrinsic} in {bounds:?}"
                );
                assert!(menu.contains(bounds.min) && menu.contains(bounds.max));
            }
        }
        assert!(rect(&app, caption)?.width() > 40.);
        assert!(menu.max.x <= 1200. && menu.max.y <= 720., "{menu:?}");
        assert_eq!(rect(&app, scrim)?, Rect::new(0., 0., 1200., 720.));
        assert_eq!(menu.min, pointer.min(Vec2::new(1200., 720.) - menu.size()));
        if forced {
            assert!(menu.width() >= 225.);
        }
    }
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
    assert_eq!(SidebarWidgets::load(&path)?, moved_dock());
    std::fs::remove_file(path)?;
    Ok(())
}
