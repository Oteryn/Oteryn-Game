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
        pointer::{Location, Pointer, PointerId},
    },
    window::WindowRef,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn app(preferences: std::path::PathBuf) -> Result<App, Box<dyn std::error::Error>> {
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
    let mut app = app(path.clone())?;
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
    let mut again = app(path.clone())?;
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
