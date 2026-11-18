//! Floating module windows: float, resize, stack, dock back and zoom.
use super::*;

fn floating_app(name: &str) -> Result<App, Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    let mut app = playing_app_with(
        fixture.path().join(format!("{name}.json")),
        (1280, 720),
        |app| {
            app.init_resource::<ButtonInput<KeyCode>>()
                .init_resource::<bevy::input_focus::InputFocus>();
            super::super::super::minimap::install(app);
            crate::bevy_shell::ui::window_layout::install(app);
        },
    )?;
    app.insert_non_send(fixture);
    let panels = &mut app
        .world_mut()
        .non_send_mut::<HostOwner>()
        .into_inner()
        .settings
        .preferences
        .visible_panels;
    for id in ["chat", "minimap"] {
        if !panels.iter().any(|panel| panel == id) {
            panels.push(id.into());
        }
    }
    app.update();
    app.update();
    Ok(app)
}
fn settle(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}
fn float_root(app: &mut App, id: &str) -> Result<Entity, Box<dyn std::error::Error>> {
    entity_with::<float::FloatRoot>(app, |root| root.0 == id)
}
fn parent(app: &App, entity: Entity) -> Option<Entity> {
    app.world().get::<ChildOf>(entity).map(ChildOf::parent)
}
fn grips(app: &App, window: Entity) -> usize {
    app.world().get::<Children>(window).map_or(0, |children| {
        children
            .iter()
            .filter(|child| {
                app.world()
                    .get::<Name>(*child)
                    .is_some_and(|name| name.as_str().starts_with("window.resize."))
            })
            .count()
    })
}
fn index_in(app: &App, home: Entity, child: Entity) -> Option<usize> {
    app.world()
        .get::<Children>(home)?
        .iter()
        .position(|entity| entity == child)
}
fn press_escape(app: &mut App) {
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: bevy::input::keyboard::Key::Escape,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    settle(app);
}

#[test]
fn the_minimap_floats_as_a_window_resizable_from_every_edge_with_its_own_zoom() -> TestResult {
    use crate::bevy_shell::ui::window_layout::{WindowLayer, WindowLayouts};
    let mut app = floating_app("float-minimap")?;
    let region = app
        .world()
        .resource::<super::super::super::GameRegions>()
        .minimap;
    let home = app
        .world()
        .get::<float::Floatable>(region)
        .ok_or("floatable")?
        .home;
    let viewport = entity_with::<Name>(&mut app, |name| name.as_str() == "Minimap viewport")?;
    let toggle = entity_with::<float::FloatToggle>(&mut app, |toggle| toggle.0 == float::MINIMAP)?;
    activate(&mut app, toggle);
    settle(&mut app);
    let root = float_root(&mut app, float::MINIMAP)?;
    assert_eq!(parent(&app, region), Some(root));
    assert_eq!(
        app.world().get::<WindowId>(root).map(|id| id.0),
        Some("minimap")
    );
    assert!(app.world().get::<WindowLayer>(root).is_some());
    assert!(
        app.world()
            .get::<crate::bevy_shell::window_drag::DraggableDialog>(root)
            .is_some()
    );
    assert_eq!(grips(&app, root), 8, "edges and corners");
    assert_eq!(
        app.world().get::<Node>(root).ok_or("root")?.display,
        Display::Flex
    );
    app.world_mut()
        .resource_mut::<WindowLayouts>()
        .0
        .get_mut("minimap")
        .ok_or("layout")?
        .set_zoom(1.5, Vec2::splat(4000.));
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(viewport).ok_or("viewport")?.width,
        px(108. * 1.5)
    );
    let mut layouts = app.world_mut().resource_mut::<WindowLayouts>();
    let layout = layouts.0.get_mut("minimap").ok_or("layout")?;
    layout.resize(Vec2::new(10., 10.), Vec2::splat(4000.));
    assert_eq!(
        layout.size,
        Some(layout.min()),
        "resize clamps to the minimum"
    );
    activate(&mut app, toggle);
    settle(&mut app);
    assert_eq!(parent(&app, region), Some(home));
    assert_eq!(index_in(&app, home, region), Some(0), "docks back in place");
    assert_eq!(
        app.world().get::<Node>(root).ok_or("root")?.display,
        Display::None
    );
    assert_eq!(
        app.world().get::<Node>(viewport).ok_or("viewport")?.width,
        px(108.),
        "the docked minimap is not zoomed"
    );
    Ok(())
}

#[test]
fn the_chat_floats_moves_and_docks_back_above_nothing_but_its_splitter() -> TestResult {
    use crate::bevy_shell::ui::window_layout::WindowLayouts;
    let mut app = floating_app("float-chat")?;
    let chat = app
        .world()
        .resource::<super::super::super::GameRegions>()
        .chat;
    let floatable = *app
        .world()
        .get::<float::Floatable>(chat)
        .ok_or("floatable")?;
    let splitter = floatable.companion.ok_or("splitter")?;
    let toggle = entity_with::<float::FloatToggle>(&mut app, |toggle| toggle.0 == float::CHAT)?;
    activate(&mut app, toggle);
    settle(&mut app);
    let root = float_root(&mut app, float::CHAT)?;
    assert_eq!(parent(&app, chat), Some(root));
    assert_eq!(grips(&app, root), 8);
    assert_eq!(
        app.world().get::<Node>(splitter).ok_or("splitter")?.display,
        Display::None
    );
    assert_eq!(
        app.world().get::<Node>(chat).ok_or("chat")?.max_height,
        Val::Auto,
        "the floating chat fills its window"
    );
    app.world_mut()
        .entity_mut(root)
        .insert(UiTransform::from_translation(Val2::px(50., 30.)));
    settle(&mut app);
    assert_eq!(
        app.world()
            .resource::<WindowLayouts>()
            .0
            .get("chat")
            .map(|layout| layout.offset),
        Some(Vec2::new(50., 30.)),
        "the moved chat keeps its place"
    );
    activate(&mut app, toggle);
    settle(&mut app);
    assert_eq!(parent(&app, chat), Some(floatable.home));
    assert_eq!(index_in(&app, floatable.home, chat), Some(floatable.index));
    let node = app.world().get::<Node>(chat).ok_or("chat")?;
    assert_eq!(node.max_height, percent(50.));
    assert_eq!(node.flex_grow, 0.);
    assert_eq!(
        app.world().get::<Node>(splitter).ok_or("splitter")?.display,
        Display::Flex
    );
    Ok(())
}

#[test]
fn a_floated_module_joins_the_window_stack_and_escape_closes_only_the_front_one() -> TestResult {
    use crate::bevy_shell::ui::window_layout::WindowStack;
    let mut app = floating_app("float-escape")?;
    let frame = entity_with::<WidgetFrame>(&mut app, |frame| frame.0 == Widget::Skills)?;
    let float = entity_with::<TitleButton>(&mut app, |button| {
        *button == TitleButton::Float(Widget::Skills)
    })?;
    activate(&mut app, float);
    settle(&mut app);
    let module = float_root(&mut app, "module.skills")?;
    assert_eq!(parent(&app, frame), Some(module));
    assert_eq!(grips(&app, module), 8);
    assert_eq!(app.world().resource::<WindowStack>().front(), Some(module));
    let toggle = entity_with::<float::FloatToggle>(&mut app, |toggle| toggle.0 == float::MINIMAP)?;
    activate(&mut app, toggle);
    settle(&mut app);
    let minimap = float_root(&mut app, float::MINIMAP)?;
    assert_eq!(app.world().resource::<WindowStack>().front(), Some(minimap));
    press_escape(&mut app);
    let state = app.world().resource::<SidebarWidgets>();
    assert!(
        !state.region_floats(float::MINIMAP),
        "the front window closes"
    );
    assert!(
        state
            .entry(Widget::Skills)
            .is_some_and(|entry| entry.open && entry.floating),
        "the window behind stays"
    );
    assert_eq!(app.world().resource::<WindowStack>().front(), Some(module));
    press_escape(&mut app);
    assert!(
        app.world()
            .resource::<SidebarWidgets>()
            .entry(Widget::Skills)
            .is_some_and(|entry| !entry.open)
    );
    press_escape(&mut app);
    assert!(
        !app.world()
            .resource::<SidebarWidgets>()
            .entry(Widget::BattleList)
            .is_some_and(|entry| !entry.open),
        "Escape leaves docked modules alone"
    );
    Ok(())
}

#[test]
fn a_floating_module_docks_again_when_dropped_on_a_column() -> TestResult {
    let mut app = floating_app("float-dock")?;
    let float = entity_with::<TitleButton>(&mut app, |button| {
        *button == TitleButton::Float(Widget::Skills)
    })?;
    activate(&mut app, float);
    settle(&mut app);
    assert!(
        app.world()
            .resource::<SidebarWidgets>()
            .entry(Widget::Skills)
            .is_some_and(|entry| entry.floating)
    );
    app.world_mut()
        .resource_mut::<SidebarWidgets>()
        .place(Widget::Skills, Dock::Right, None);
    settle(&mut app);
    let sidebar = *app.world().resource::<super::super::super::Sidebar>();
    let frame = entity_with::<WidgetFrame>(&mut app, |frame| frame.0 == Widget::Skills)?;
    assert_eq!(parent(&app, frame), Some(sidebar.right_widgets));
    Ok(())
}

#[test]
fn text_spawned_while_docked_follows_the_floating_window_zoom() -> TestResult {
    use crate::bevy_shell::ui::window_layout::WindowLayouts;
    let mut app = floating_app("float-font")?;
    let slot = entity_with::<Name>(&mut app, |name| name.as_str() == "minimap-zoom-in")?;
    let caption = app
        .world()
        .get::<Children>(slot)
        .and_then(|children| children.first().copied())
        .ok_or("caption")?;
    let size = |app: &App| match app
        .world()
        .get::<TextFont>(caption)
        .map(|font| font.font_size)
    {
        Some(bevy::text::FontSize::Px(size)) => Ok(size),
        _ => Err("font size"),
    };
    let base = size(&app)?;
    let toggle = entity_with::<float::FloatToggle>(&mut app, |toggle| toggle.0 == float::MINIMAP)?;
    activate(&mut app, toggle);
    settle(&mut app);
    for zoom in [2., 1.25] {
        app.world_mut()
            .resource_mut::<WindowLayouts>()
            .0
            .get_mut("minimap")
            .ok_or("layout")?
            .set_zoom(zoom, Vec2::splat(4000.));
        settle(&mut app);
        assert_eq!(size(&app)?, base * zoom);
    }
    activate(&mut app, toggle);
    settle(&mut app);
    assert_eq!(size(&app)?, base, "docked text returns to its size");
    Ok(())
}

#[test]
fn sidebar_toggles_raised_bosstiary_closed() -> TestResult {
    use crate::bevy_shell::ui::{Control, toolbar::grid::ControlButton};
    let mut app = floating_app("bosstiary-toggle")?;
    let (link, _commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_hud_fixture(&[], None);
    link.analyser_select(oteryn_client_ui::analyser::Tool::Session.key());
    link.analyser_open();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    super::super::super::analyser::install(&mut app);
    super::super::super::bosstiary::install(&mut app);
    super::super::super::bestiary::install(&mut app);
    let bestiary = entity_with::<ControlButton>(&mut app, |button| {
        matches!(button, ControlButton::Grid(Control::Panel("bestiary")))
    })?;
    activate(&mut app, bestiary);
    settle(&mut app);
    let button = entity_with::<ControlButton>(&mut app, |button| {
        matches!(button, ControlButton::Grid(Control::Panel("bosstiary")))
    })?;
    activate(&mut app, button);
    settle(&mut app);
    let open = |app: &App| {
        app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .panel_visible("bosstiary")
    };
    assert!(open(&app));
    use crate::bevy_shell::ui::window_layout::{WindowLayer, WindowStack};
    let front = |app: &App| {
        app.world()
            .resource::<WindowStack>()
            .front()
            .and_then(|root| app.world().get::<WindowLayer>(root))
            .map(|layer| layer.0)
    };
    let bestiary_root = entity_with::<WindowLayer>(&mut app, |layer| layer.0 == "bestiary")?;
    let boss_root = entity_with::<WindowLayer>(&mut app, |layer| layer.0 == "bosstiary")?;
    for (root, id) in [(bestiary_root, "bestiary"), (boss_root, "bosstiary")] {
        app.world_mut()
            .resource_mut::<bevy::input_focus::InputFocus>()
            .set(root, bevy::input_focus::FocusCause::Navigated);
        settle(&mut app);
        assert_eq!(front(&app), Some(id));
    }
    let reset = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| (text.0 == "Reset session").then_some(parent.parent()))
        .ok_or("analyser reset")?;
    let kills = |app: &App| {
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .map(|play| play.link.analyser_panel().kills)
    };
    activate(&mut app, reset);
    assert_eq!(kills(&app), Some(23));
    activate(&mut app, button);
    settle(&mut app);
    assert!(!open(&app));
    let close = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, control)| {
            (text.0 == "Close"
                && std::iter::successors(Some(control.parent()), |e| parent(&app, *e))
                    .any(|e| e == bestiary_root))
            .then_some(control.parent())
        })
        .ok_or("bestiary close")?;
    activate(&mut app, close);
    assert!(front(&app).is_none());
    activate(&mut app, reset);
    assert_eq!(kills(&app), Some(0));
    Ok(())
}
