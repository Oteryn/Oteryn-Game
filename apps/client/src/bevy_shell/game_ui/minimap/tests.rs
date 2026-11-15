use super::*;
use crate::{
    bevy_shell::owner::{ActivePlay, Phase},
    play::{PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, ui_widgets::Activate};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn fixture() -> Result<App, Box<dyn std::error::Error>> {
    let (link, _, _) = play_channel();
    let view = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), -7, &[])?;
    let mut owner = HostOwner::new(None)?;
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
    .init_resource::<bevy::input_focus::InputFocus>()
    .init_resource::<ButtonInput<KeyCode>>()
    .insert_non_send(owner)
    .add_systems(
        Startup,
        (crate::bevy_shell::ui::setup, super::super::setup).chain(),
    );
    app.add_systems(Update, super::super::project);
    install(&mut app);
    app.update();
    Ok(app)
}
fn activate(app: &mut App, label: &str) -> TestResult {
    let text = app
        .world_mut()
        .query::<(Entity, &Text)>()
        .iter(app.world())
        .find_map(|(e, t)| (t.0 == label).then_some(e))
        .ok_or("missing minimap control")?;
    let button = app
        .world()
        .get::<ChildOf>(text)
        .ok_or("button parent")?
        .parent();
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    Ok(())
}
#[test]
fn sidebar_exposes_all_navigation_controls() -> TestResult {
    let mut app = fixture()?;
    for label in [
        "N",
        "W",
        "Centre",
        "E",
        "S",
        "Zoom out",
        "Zoom in",
        "Floor up",
        "Floor down",
    ] {
        activate(&mut app, label)?;
    }
    Ok(())
}

type CameraState = (i32, i32, i16, i32, bool);
fn camera(app: &App) -> Result<CameraState, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .view
        .minimap_camera())
}
#[test]
fn sidebar_navigation_changes_shared_camera_and_centre_restores_following() -> TestResult {
    let mut app = fixture()?;
    assert_eq!(camera(&app)?, (100, 200, -7, 1, true));
    activate(&mut app, "W")?;
    activate(&mut app, "N")?;
    assert_eq!(camera(&app)?, (95, 195, -7, 1, false));
    activate(&mut app, "Zoom out")?;
    activate(&mut app, "E")?;
    activate(&mut app, "S")?;
    activate(&mut app, "Floor up")?;
    assert_eq!(camera(&app)?, (105, 205, -6, 2, false));
    activate(&mut app, "Centre")?;
    assert_eq!(camera(&app)?, (100, 200, -7, 2, true));
    activate(&mut app, "Zoom in")?;
    activate(&mut app, "Zoom in")?;
    assert_eq!(camera(&app)?.3, 1);
    for _ in 0..20 {
        activate(&mut app, "Zoom out")?;
        activate(&mut app, "Floor up")?;
    }
    assert_eq!(camera(&app)?.2, 0);
    assert_eq!(camera(&app)?.3, 8);
    for _ in 0..20 {
        activate(&mut app, "Floor down")?;
    }
    assert_eq!(camera(&app)?.2, -15);
    Ok(())
}
fn key(app: &mut App, key: KeyCode) {
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset_all();
        keys.press(KeyCode::AltLeft);
        keys.press(key);
    }
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
}
#[test]
fn alt_navigation_shortcuts_respect_modal_and_hidden_sidebar() -> TestResult {
    let mut app = fixture()?;
    key(&mut app, KeyCode::ArrowLeft);
    key(&mut app, KeyCode::Home);
    key(&mut app, KeyCode::PageDown);
    assert_eq!(camera(&app)?, (95, 200, -8, 2, false));
    let modal = app
        .world_mut()
        .spawn((Node::default(), super::super::GameModal))
        .id();
    key(&mut app, KeyCode::ArrowRight);
    activate(&mut app, "Centre")?;
    assert_eq!(camera(&app)?, (95, 200, -8, 2, false));
    app.world_mut().despawn(modal);
    let region = app.world().resource::<GameRegions>().minimap;
    app.world_mut()
        .get_mut::<Node>(region)
        .ok_or("region")?
        .display = Display::None;
    key(&mut app, KeyCode::PageUp);
    activate(&mut app, "Centre")?;
    assert_eq!(camera(&app)?, (95, 200, -8, 2, false));
    app.world_mut()
        .get_mut::<Node>(region)
        .ok_or("region")?
        .display = Display::Flex;
    activate(&mut app, "Centre")?;
    key(&mut app, KeyCode::End);
    key(&mut app, KeyCode::PageUp);
    assert_eq!(camera(&app)?, (100, 200, -6, 1, false));
    Ok(())
}
