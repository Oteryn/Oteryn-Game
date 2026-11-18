use super::*;
use crate::bevy_shell::{owner::HostOwner, ui};
use bevy::asset::AssetPlugin;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn app() -> Result<App, Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .insert_non_send(HostOwner::new(None)?)
        .add_systems(Startup, ui::setup);
    install(&mut app);
    app.update();
    Ok(app)
}
#[test]
fn shell_exposes_updates_separately_from_gameplay() -> TestResult {
    let mut app = app()?;
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Updates")
    );
    Ok(())
}

fn report(app: &App) -> &str {
    &app.world().non_send::<State>().report
}
fn button(app: &mut App, action: Action) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find(|(_, candidate)| **candidate == action)
        .map(|(entity, _)| entity)
        .ok_or_else(|| "updater action missing".into())
}
fn click(app: &mut App, action: Action) -> TestResult {
    let entity = button(app, action)?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
#[test]
fn updater_open_check_recovery_and_hide_use_real_buttons() -> TestResult {
    let mut app = app()?;
    app.world_mut().non_send_mut::<State>().directory = None;
    click(&mut app, Action::Open)?;
    let dialog = app
        .world_mut()
        .query_filtered::<&Node, With<Dialog>>()
        .single(app.world())?;
    assert_eq!(dialog.display, Display::Flex);
    click(&mut app, Action::Check)?;
    assert!(report(&app).contains("No managed installation"));
    assert!(app.world().non_send::<State>().pending.is_none());
    click(&mut app, Action::Check)?;
    assert!(report(&app).contains("trusted Oteryn installer"));
    click(&mut app, Action::Close)?;
    assert!(!app.world().non_send::<State>().open);
    assert_eq!(app.world().non_send::<HostOwner>().phase, Phase::Offline);
    Ok(())
}
#[test]
fn failed_launcher_handoff_preserves_client_and_allows_retry() -> TestResult {
    let mut app = app()?;
    let fixture = crate::play::screenshot::TestDir::new()?;
    app.world_mut().non_send_mut::<State>().directory = Some(fixture.join("no-launcher"));
    click(&mut app, Action::Open)?;
    click(&mut app, Action::Maintain)?;
    assert!(report(&app).contains("Could not open launcher"));
    assert_eq!(app.world().non_send::<HostOwner>().phase, Phase::Offline);
    assert_eq!(
        app.world_mut()
            .resource_mut::<Messages<AppExit>>()
            .drain()
            .count(),
        0
    );
    click(&mut app, Action::Close)?;
    Ok(())
}
#[test]
fn real_worker_failure_releases_pending_operation_for_retry() -> TestResult {
    let mut app = app()?;
    let fixture = crate::play::screenshot::TestDir::new()?;
    let file = fixture.join("not-an-installation");
    std::fs::write(&file, b"not an installation directory")?;
    for _ in 0..2 {
        let receiver = worker::start(file.clone(), false)?;
        // Wait on the real worker, then feed its terminal result through the production poller.
        let event = receiver.recv_timeout(std::time::Duration::from_secs(10))?;
        assert!(matches!(event, worker::Event::Finished(Err(_))));
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        assert!(tx.send(event).is_ok());
        app.world_mut().non_send_mut::<State>().pending = Some(rx);
        app.update();
        let state = app.world().non_send::<State>();
        assert!(state.pending.is_none());
        assert!(state.report.contains("Active release unchanged"));
    }
    std::fs::remove_file(file)?;
    Ok(())
}
