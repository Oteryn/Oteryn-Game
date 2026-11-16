use super::*;
use crate::{
    bevy_shell::owner::{ActivePlay, Phase},
    play::{HudPane, PlayView, play_channel},
};
use bevy::{
    asset::AssetPlugin,
    camera::NormalizedRenderTarget,
    picking::{
        backend::HitData,
        events::Pointer,
        pointer::{Location, PointerId},
    },
    window::WindowRef,
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
    let mut regions = Vec::new();
    for _ in 0..11 {
        regions.push(app.world_mut().spawn(Node::default()).id());
    }
    app.insert_resource(super::super::GameRegions {
        vitals: regions[0],
        minimap: regions[1],
        equipment: regions[2],
        containers: regions[3],
        battle: regions[4],
        skills: regions[5],
        vip: regions[6],
        analyzers: regions[7],
        chat: regions[8],
        bars: regions[9],
        world: regions[10],
    });
    app.world_mut().spawn((
        Window {
            focused: true,
            resolution: (1280, 720).into(),
            ..default()
        },
        PrimaryWindow,
    ));
    app.add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.update();
    Ok(app)
}
fn entity_with<C: Component + Copy>(
    app: &mut App,
    found: impl Fn(&C) -> bool,
) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &C)>()
        .iter(app.world())
        .find(|(_, component)| found(component))
        .map(|(entity, _)| entity)
        .ok_or_else(|| "HUD pane entity missing".into())
}
fn click(app: &mut App, command: PaneCommand) -> TestResult {
    let entity = entity_with::<PaneCommand>(app, |candidate| *candidate == command)?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
fn pane_node(app: &mut App, panel: HudPanel) -> Result<Node, Box<dyn std::error::Error>> {
    let entity = entity_with::<Pane>(app, |pane| pane.0 == panel)?;
    Ok(app.world().get::<Node>(entity).ok_or("pane node")?.clone())
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
fn hud_panes_show_dock_and_drag_then_come_back_after_a_restart() -> TestResult {
    let path = std::env::temp_dir().join(format!(
        "oteryn-hud-panes-{}/preferences.json",
        std::process::id()
    ));
    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory)?;
    }
    let _ = std::fs::remove_file(&path);
    let mut app = playing_app(path.clone())?;
    assert_eq!(
        pane_node(&mut app, HudPanel::Combat)?.display,
        Display::None
    );

    click(&mut app, PaneCommand::Toggle(HudPanel::Combat))?;
    let combat = pane_node(&mut app, HudPanel::Combat)?;
    assert_eq!(combat.display, Display::Flex);
    // Docked left below the Status pane docked on top (8 + 100 + 4).
    assert_eq!((combat.left, combat.top), (px(8.), px(112.)));
    let shown = {
        let owner = app.world().non_send::<HostOwner>();
        owner.play.as_ref().ok_or("play")?.view.combat_status()
    };
    let body = entity_with::<PaneText>(&mut app, |text| text.0 == HudPanel::Combat)?;
    assert_eq!(app.world().get::<Text>(body).ok_or("combat text")?.0, shown);

    click(
        &mut app,
        PaneCommand::Dock(HudPanel::Combat, HudDock::Right),
    )?;
    let right = pane_node(&mut app, HudPanel::Combat)?;
    assert_eq!(right.left, px(1280. - 240. - 8.));

    let bar = entity_with::<TitleBar>(&mut app, |bar| bar.panel == HudPanel::Combat)?;
    let pointer = pointer(&mut app)?;
    app.world_mut().trigger(PointerDragStart {
        entity: bar,
        pointer: pointer.clone(),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
    });
    let distance = Vec2::new(300. - (1280. - 240. - 8.), 200.);
    app.world_mut().trigger(PointerDrag {
        entity: bar,
        pointer: pointer.clone(),
        button: PointerButton::Primary,
        distance,
        delta: distance,
    });
    app.update();
    let dragged = pane_node(&mut app, HudPanel::Combat)?;
    assert_eq!((dragged.left, dragged.top), (px(300.), px(312.)));
    app.world_mut().trigger(PointerDragEnd {
        entity: bar,
        pointer,
        button: PointerButton::Primary,
        distance,
    });
    app.update();
    let expected = HudPane {
        dock: HudDock::Floating,
        bounds: [300, 312, 240, 120],
        visible: true,
    };
    let dropped = pane_node(&mut app, HudPanel::Combat)?;
    assert_eq!((dropped.left, dropped.top), (px(300.), px(312.)));

    // A new session reads the saved layout back from the preference file.
    let restarted = HostOwner::new(Some(path.clone()))?;
    assert_eq!(
        restarted
            .settings
            .preferences
            .effective_hud_layout()
            .pane(HudPanel::Combat),
        &expected
    );
    let mut again = playing_app(path.clone())?;
    {
        let mut owner = again.world_mut().non_send_mut::<HostOwner>();
        let preferences = owner.settings.preferences.clone();
        owner
            .play
            .as_mut()
            .ok_or("play")?
            .view
            .set_preferences(preferences)?;
    }
    again.update();
    let restored = pane_node(&mut again, HudPanel::Combat)?;
    assert_eq!(
        (restored.display, restored.left, restored.top),
        (Display::Flex, px(300.), px(312.))
    );

    click(&mut again, PaneCommand::Hide(HudPanel::Combat))?;
    assert_eq!(
        pane_node(&mut again, HudPanel::Combat)?.display,
        Display::None
    );
    let _ = std::fs::remove_file(&path);
    Ok(())
}

fn temp_preferences(name: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let directory = std::env::temp_dir().join(format!("oteryn-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    let path = directory.join("preferences.json");
    let _ = std::fs::remove_file(&path);
    Ok(path)
}
fn layout_pane(app: &App, panel: HudPanel) -> Result<HudPane, Box<dyn std::error::Error>> {
    let owner = app.world().non_send::<HostOwner>();
    Ok(owner
        .play
        .as_ref()
        .ok_or("play")?
        .view
        .preferences()
        .effective_hud_layout()
        .pane(panel)
        .clone())
}
fn drag_bar(app: &mut App, panel: HudPanel, distance: Vec2, end: bool) -> TestResult {
    let bar = entity_with::<TitleBar>(app, |bar| bar.panel == panel)?;
    let pointer = pointer(app)?;
    app.world_mut().trigger(PointerDrag {
        entity: bar,
        pointer: pointer.clone(),
        button: PointerButton::Primary,
        distance,
        delta: distance,
    });
    app.update();
    if end {
        app.world_mut().trigger(PointerDragEnd {
            entity: bar,
            pointer,
            button: PointerButton::Primary,
            distance,
        });
        app.update();
    }
    Ok(())
}
fn start_drag(app: &mut App, panel: HudPanel) -> TestResult {
    let bar = entity_with::<TitleBar>(app, |bar| bar.panel == panel)?;
    let pointer = pointer(app)?;
    app.world_mut().trigger(PointerDragStart {
        entity: bar,
        pointer,
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
    });
    Ok(())
}

#[test]
fn fresh_login_status_pane_leaves_the_vitals_and_pane_toggle_strip_uncovered() -> TestResult {
    let path = temp_preferences("hud-panes-header")?;
    let mut app = playing_app(path.clone())?;
    // The laid-out header (vitals and the pane-toggle strip) is 60 UI units tall.
    let vitals = app.world().resource::<super::super::GameRegions>().vitals;
    app.world_mut().entity_mut(vitals).insert(ComputedNode {
        size: Vec2::new(1280., 60.),
        inverse_scale_factor: 1.,
        ..default()
    });
    app.update();
    let status = pane_node(&mut app, HudPanel::Resources)?;
    assert_eq!(
        status.display,
        Display::Flex,
        "Status shows on a fresh login"
    );
    assert_eq!((status.left, status.top), (px(8.), px(68.)));

    // A pane dragged up onto the header stops below it; dropped there it docks first on
    // top, still below the header, and the Status pane moves down under it.
    click(&mut app, PaneCommand::Toggle(HudPanel::Combat))?;
    assert_eq!(pane_node(&mut app, HudPanel::Combat)?.top, px(172.));
    start_drag(&mut app, HudPanel::Combat)?;
    drag_bar(&mut app, HudPanel::Combat, Vec2::new(300., -400.), false)?;
    assert_eq!(pane_node(&mut app, HudPanel::Combat)?.top, px(60.));
    drag_bar(&mut app, HudPanel::Combat, Vec2::new(300., -400.), true)?;
    assert_eq!(layout_pane(&app, HudPanel::Combat)?.dock, HudDock::Top);
    assert_eq!(pane_node(&mut app, HudPanel::Combat)?.top, px(68.));
    assert_eq!(pane_node(&mut app, HudPanel::Resources)?.top, px(192.));
    let _ = std::fs::remove_file(&path);
    Ok(())
}

#[test]
fn interface_scale_from_options_is_applied_once_to_docked_and_dragged_panes() -> TestResult {
    let path = temp_preferences("hud-panes-scale")?;
    let mut app = playing_app(path.clone())?;
    // Options at 150%, applied through the options draft; the shell keeps Bevy's UiScale
    // in step with it.
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.open_graphics();
        owner.edit_preferences(|preferences| preferences.interface_scale = 150);
        owner.apply(true);
        // The shell hands the applied preferences to the play view every frame.
        let HostOwner { play, settings, .. } = &mut *owner;
        let play = play.as_mut().ok_or("play")?;
        settings.capture(&mut play.view)?;
        assert_eq!(play.view.preferences().interface_scale, 150);
    }
    app.insert_resource(UiScale(1.5));
    click(&mut app, PaneCommand::Toggle(HudPanel::Combat))?;
    click(
        &mut app,
        PaneCommand::Dock(HudPanel::Combat, HudDock::Right),
    )?;
    let combat = pane_node(&mut app, HudPanel::Combat)?;
    // Rendered: 360 px wide, 8 px from the right edge of the 1280 px window.
    assert_eq!(
        (combat.left, combat.top, combat.width, combat.height),
        (px(608.), px(108.), px(240.), px(120.))
    );

    start_drag(&mut app, HudPanel::Combat)?;
    drag_bar(&mut app, HudPanel::Combat, Vec2::new(-600., 300.), false)?;
    let dragged = pane_node(&mut app, HudPanel::Combat)?;
    assert_eq!((dragged.left, dragged.top), (px(208.), px(308.)));
    drag_bar(&mut app, HudPanel::Combat, Vec2::new(-600., 300.), true)?;
    assert_eq!(
        layout_pane(&app, HudPanel::Combat)?,
        HudPane {
            dock: HudDock::Floating,
            bounds: [208, 308, 240, 120],
            visible: true,
        }
    );
    let dropped = pane_node(&mut app, HudPanel::Combat)?;
    assert_eq!(
        (dropped.left, dropped.top, dropped.width),
        (px(208.), px(308.), px(240.))
    );
    let _ = std::fs::remove_file(&path);
    Ok(())
}

#[test]
fn cancelled_or_stale_title_bar_drags_end_without_freezing_the_pane() -> TestResult {
    let path = temp_preferences("hud-panes-cancel")?;
    let mut app = playing_app(path.clone())?;
    click(&mut app, PaneCommand::Toggle(HudPanel::Combat))?;

    // A touch drag the platform cancels: no drag end ever arrives.
    start_drag(&mut app, HudPanel::Combat)?;
    drag_bar(&mut app, HudPanel::Combat, Vec2::new(400., 100.), false)?;
    let bar = entity_with::<TitleBar>(&mut app, |bar| bar.panel == HudPanel::Combat)?;
    let pointer = pointer(&mut app)?;
    app.world_mut().trigger(PointerCancel {
        entity: bar,
        pointer,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
    });
    app.update();
    let restored = pane_node(&mut app, HudPanel::Combat)?;
    assert_eq!((restored.left, restored.top), (px(8.), px(112.)));
    click(&mut app, PaneCommand::Hide(HudPanel::Combat))?;
    assert_eq!(
        pane_node(&mut app, HudPanel::Combat)?.display,
        Display::None,
        "Hide still reaches a pane whose drag was cancelled"
    );

    // The window is resized under a running drag: it neither moves nor saves the pane.
    click(&mut app, PaneCommand::Toggle(HudPanel::Combat))?;
    start_drag(&mut app, HudPanel::Combat)?;
    app.world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>()
        .single_mut(app.world_mut())?
        .resolution = (1024, 700).into();
    drag_bar(&mut app, HudPanel::Combat, Vec2::new(400., 100.), true)?;
    let kept = pane_node(&mut app, HudPanel::Combat)?;
    assert_eq!((kept.left, kept.top), (px(8.), px(112.)));
    assert_eq!(layout_pane(&app, HudPanel::Combat)?.dock, HudDock::Left);

    // A modal opened over a running drag ends it the same way.
    start_drag(&mut app, HudPanel::Combat)?;
    app.world_mut()
        .spawn((super::super::GameModal, Node::default(), GlobalZIndex(1000)));
    app.update();
    drag_bar(&mut app, HudPanel::Combat, Vec2::new(400., 100.), true)?;
    assert_eq!(layout_pane(&app, HudPanel::Combat)?.dock, HudDock::Left);
    let _ = std::fs::remove_file(&path);
    Ok(())
}

#[test]
fn a_title_bar_drag_ends_when_its_pointer_is_released_without_a_drag_end() -> TestResult {
    let path = temp_preferences("hud-panes-release")?;
    let mut app = playing_app(path.clone())?;
    click(&mut app, PaneCommand::Toggle(HudPanel::Combat))?;
    app.init_resource::<bevy::picking::pointer::PointerMap>();
    let held = app
        .world_mut()
        .spawn((
            PointerId::Mouse,
            bevy::picking::pointer::PointerPressState::default(),
        ))
        .id();
    start_drag(&mut app, HudPanel::Combat)?;
    drag_bar(&mut app, HudPanel::Combat, Vec2::new(400., 100.), false)?;
    let restored = pane_node(&mut app, HudPanel::Combat)?;
    assert_eq!((restored.left, restored.top), (px(8.), px(112.)));
    assert_eq!(layout_pane(&app, HudPanel::Combat)?.dock, HudDock::Left);
    app.world_mut().despawn(held);
    let _ = std::fs::remove_file(&path);
    Ok(())
}
