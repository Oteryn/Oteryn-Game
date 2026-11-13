use super::*;
use crate::{
    bevy_shell::{
        game_ui::{GameRegions, viewport::WorldViewport},
        owner::{ActivePlay, Phase},
    },
    play::{PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, math::DVec2, window::PrimaryWindow};

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn fixture(delay: u16) -> Result<App, Box<dyn std::error::Error>> {
    let tile = oteryn_renderer::TileCoord::new(100, 100);
    let mut view =
        PlayView::new(tile, 0, &[])?.with_entities(vec![oteryn_session::WorldSpatialEntity {
            entity: oteryn_session::EntityRef {
                identity: [7; 16],
                generation: 1,
            },
            kind: oteryn_session::EntityKind::Creature,
            display_name: Some(oteryn_session::EntityDisplayName::new("Rat").map_err(|_| "name")?),
            position: oteryn_session::ActorPosition {
                x: 100,
                y: 100,
                floor: 0,
            },
            detail: oteryn_session::EntityDetail::Actor {
                direction: oteryn_session::StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        }])?;
    let [x, y] = view.scene().view().tile_to_screen(tile);
    let (width, height) = view.scene().view().viewport_px();
    let (link, _, _) = play_channel();
    let mut owner = HostOwner::new(None)?;
    let mut preferences = owner.settings.preferences.clone();
    preferences.tooltips_enabled = true;
    preferences.tooltip_delay_ms = delay;
    owner.settings.apply(preferences.clone());
    view.set_preferences(preferences)?;
    owner.play = Some(ActivePlay { view, link });
    owner.phase = Phase::Playing;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        source::GlobalUiPlugin,
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .init_resource::<UiScale>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    let world = app.world_mut().spawn(Node::default()).id();
    // Only the map anchor is observed by this isolated bridge.
    app.insert_resource(GameRegions {
        vitals: world,
        minimap: world,
        equipment: world,
        containers: world,
        battle: world,
        skills: world,
        vip: world,
        analyzers: world,
        chat: world,
        bars: world,
        world,
    })
    .insert_resource(WorldViewport {
        origin: UVec2::new(80, 40),
        size: UVec2::new(width + 40, height + 20),
    });
    let mut window = Window {
        resolution: (1024, 720).into(),
        focused: true,
        ..default()
    };
    window.set_physical_cursor_position(Some(DVec2::new(
        f64::from(80. + 20. + x + 1.),
        f64::from(40. + 10. + y + 1.),
    )));
    app.world_mut().spawn((window, PrimaryWindow));
    install(&mut app);
    app.update();
    Ok(app)
}

#[test]
fn physical_map_hover_projects_confirmed_name_from_existing_owner() -> TestResult {
    let mut app = fixture(0)?;
    let visible = app
        .world_mut()
        .query::<(&HintRoot, &Node)>()
        .iter(app.world())
        .any(|(_, node)| node.display == Display::Flex);
    assert!(visible, "confirmed map hover must display a tooltip");
    assert!(
        app.world_mut()
            .query::<(&HintText, &Text)>()
            .iter(app.world())
            .any(|(_, text)| text.0 == "Rat")
    );
    Ok(())
}

fn shown(app: &mut App) -> bool {
    app.world_mut()
        .query_filtered::<&Node, With<HintRoot>>()
        .iter(app.world())
        .any(|node| node.display == Display::Flex)
}
fn cursor(app: &mut App) -> Result<SystemCursorIcon, Box<dyn std::error::Error>> {
    use bevy::window::CursorIcon;
    app.world_mut()
        .query_filtered::<&CursorIcon, With<PrimaryWindow>>()
        .iter(app.world())
        .find_map(|icon| icon.as_system().copied())
        .ok_or_else(|| "cursor".into())
}

#[test]
fn accepted_delay_entity_generation_focus_and_disable_reset_hover_without_sleeping() -> TestResult {
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;
    let mut app = fixture(500)?;
    assert!(!shown(&mut app));
    let start = app
        .world()
        .resource::<Time<Real>>()
        .last_update()
        .ok_or("time")?;
    app.insert_resource(TimeUpdateStrategy::ManualInstant(
        start + Duration::from_millis(499),
    ));
    app.update();
    assert!(!shown(&mut app));
    app.insert_resource(TimeUpdateStrategy::ManualInstant(
        start + Duration::from_millis(500),
    ));
    app.update();
    assert!(shown(&mut app));
    app.world_mut().resource_mut::<HintState>().generation = Some(u64::MAX);
    app.update();
    assert!(
        !shown(&mut app),
        "the previous host generation cannot retain its elapsed hover"
    );
    app.insert_resource(TimeUpdateStrategy::ManualInstant(
        start + Duration::from_millis(1000),
    ));
    app.update();
    assert!(shown(&mut app));
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let play = owner.play.as_mut().ok_or("play")?;
        let entity = oteryn_session::WorldSpatialEntity {
            entity: oteryn_session::EntityRef {
                identity: [7; 16],
                generation: 2,
            },
            kind: oteryn_session::EntityKind::Creature,
            display_name: Some(oteryn_session::EntityDisplayName::new("Rat").map_err(|_| "name")?),
            position: oteryn_session::ActorPosition {
                x: 100,
                y: 100,
                floor: 0,
            },
            detail: oteryn_session::EntityDetail::Actor {
                direction: oteryn_session::StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        };
        play.view = play.view.clone().with_entities(vec![entity])?;
    }
    app.update();
    assert!(
        !shown(&mut app),
        "a replaced entity must start a fresh hover delay"
    );
    app.insert_resource(TimeUpdateStrategy::ManualInstant(
        start + Duration::from_millis(1500),
    ));
    app.update();
    assert!(shown(&mut app));
    for mut window in app
        .world_mut()
        .query::<&mut Window>()
        .iter_mut(app.world_mut())
    {
        window.focused = false;
    }
    app.update();
    assert!(!shown(&mut app));
    assert_eq!(cursor(&mut app)?, SystemCursorIcon::Default);
    for mut window in app
        .world_mut()
        .query::<&mut Window>()
        .iter_mut(app.world_mut())
    {
        window.focused = true;
    }
    app.update();
    assert!(!shown(&mut app));
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    let mut preferences = owner.settings.preferences.clone();
    preferences.tooltips_enabled = false;
    owner.settings.apply(preferences);
    drop(owner);
    app.insert_resource(TimeUpdateStrategy::ManualInstant(
        start + Duration::from_millis(2000),
    ));
    app.update();
    assert!(!shown(&mut app));
    Ok(())
}

#[test]
fn cursor_help_and_crosshair_stay_in_map_and_yield_to_text_and_modals() -> TestResult {
    use bevy::{picking::hover::Hovered, text::EditableText};
    let mut app = fixture(0)?;
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    let mut preferences = owner.settings.preferences.clone();
    preferences.cursor_crosshair = true;
    owner.settings.apply(preferences);
    drop(owner);
    app.update();
    assert_eq!(cursor(&mut app)?, SystemCursorIcon::Crosshair);
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .lens_toggle();
    app.update();
    assert_eq!(cursor(&mut app)?, SystemCursorIcon::Help);
    let original = app
        .world_mut()
        .query::<&Window>()
        .iter(app.world())
        .next()
        .and_then(Window::physical_cursor_position)
        .ok_or("point")?;
    for mut window in app
        .world_mut()
        .query::<&mut Window>()
        .iter_mut(app.world_mut())
    {
        window.set_physical_cursor_position(Some(DVec2::new(900., 600.)));
    }
    app.update();
    assert_eq!(cursor(&mut app)?, SystemCursorIcon::Default);
    assert!(!shown(&mut app));
    for mut window in app
        .world_mut()
        .query::<&mut Window>()
        .iter_mut(app.world_mut())
    {
        window.set_physical_cursor_position(Some(original.as_dvec2()));
    }
    let modal = app
        .world_mut()
        .spawn((
            crate::bevy_shell::game_ui::GameModal,
            Node::default(),
            GlobalZIndex(500),
        ))
        .id();
    app.update();
    assert_eq!(cursor(&mut app)?, SystemCursorIcon::Default);
    assert!(!shown(&mut app));
    let input = app
        .world_mut()
        .spawn((
            EditableText::new(""),
            Node::default(),
            Hovered(true),
            ChildOf(modal),
        ))
        .id();
    app.update();
    assert_eq!(cursor(&mut app)?, SystemCursorIcon::Text);
    assert!(!shown(&mut app));
    app.world_mut().entity_mut(input).despawn();
    app.world_mut()
        .get_mut::<Node>(modal)
        .ok_or("modal")?
        .display = Display::None;
    app.update();
    assert_eq!(cursor(&mut app)?, SystemCursorIcon::Help);
    Ok(())
}

#[test]
fn measured_tooltip_is_clamped_in_physical_window_at_scaled_ui() -> TestResult {
    use bevy::ecs::system::RunSystemOnce;
    let mut app = fixture(0)?;
    app.world_mut().resource_mut::<UiScale>().0 = 2.;
    {
        let mut state = app.world_mut().resource_mut::<HintState>();
        state.pointer = Some(Vec2::new(1023., 719.));
        state.fresh = false;
    }
    for mut computed in app
        .world_mut()
        .query_filtered::<&mut ComputedNode, With<HintRoot>>()
        .iter_mut(app.world_mut())
    {
        computed.size = Vec2::new(240., 84.);
    }
    app.world_mut()
        .run_system_once(projection::present)
        .map_err(|error| format!("present: {error:?}"))?;
    let node = app
        .world_mut()
        .query_filtered::<&Node, With<HintRoot>>()
        .iter(app.world())
        .next()
        .ok_or("hint")?;
    let (Val::Px(left), Val::Px(top)) = (node.left, node.top) else {
        return Err("tooltip position".into());
    };
    assert!(left >= 0. && top >= 0.);
    assert!(left * 2. + 240. <= 1024.);
    assert!(top * 2. + 84. <= 720.);
    Ok(())
}
