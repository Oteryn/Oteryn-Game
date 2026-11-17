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
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<UiScale>();
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
            resolution: (1280, 720).into(),
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
fn preferences(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("oteryn-widgets-{name}-{}.json", std::process::id()))
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
    let path = preferences("default");
    let _ = std::fs::remove_file(&path);
    let mut app = playing_app(path.clone())?;
    let sidebar = *app.world().resource::<super::super::Sidebar>();
    let regions = app.world().resource::<super::super::GameRegions>();
    let panel_order = [
        regions.minimap,
        regions.vitals,
        regions.equipment,
    ];
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
    let mut columns = app
        .world_mut()
        .query_filtered::<&Node, With<Column>>();
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
        .query::<(&crate::bevy_shell::ui::toolbar::grid::ControlButton, &art::ArtSlot, &Node)>()
        .iter(app.world())
        .filter(|(_, _, node)| node.display == Display::Flex)
        .map(|(_, slot, _)| slot.key.clone())
        .collect();
    shown.sort();
    assert_eq!(
        shown,
        [
            "control-analysers",
            "control-battle",
            "control-cyclopedia",
            "control-highscores",
            "control-logout",
            "control-manage",
            "control-options",
            "control-prey",
            "control-quest-log",
            "control-skills",
            "control-spellbook",
            "control-store",
            "control-vip",
        ]
    );

    let titles: Vec<&str> = Widget::ALL.iter().map(|widget| widget.title()).collect();
    assert_eq!(
        titles,
        ["Battle List", "Containers", "Skills", "VIP", "Analytics Selector"]
    );
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn title_buttons_minimise_to_the_header_and_close_the_widget() -> TestResult {
    let path = preferences("chrome");
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
    let path = preferences("arrows");
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
    app.world_mut().resource_mut::<SidebarWidgets>().place(
        Widget::Skills,
        Dock::Left,
        None,
    );
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
    let path = preferences("resize");
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
    let path = preferences("controls");
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
