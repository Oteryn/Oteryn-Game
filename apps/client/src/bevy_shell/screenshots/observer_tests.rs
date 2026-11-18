use super::tests::{TestResult, app, frame, wait_for_writes};
use super::*;
use crate::play::screenshot::TestDir;

#[test]
fn f12_observer_returns_before_reporting_the_write_result() -> TestResult {
    let fixture = TestDir::new()?;
    let mut app = app(fixture.path(), true)?;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::F12);
    app.update();
    let entity = app
        .world_mut()
        .query_filtered::<Entity, With<Screenshot>>()
        .single(app.world())?;
    app.world_mut().trigger(ScreenshotCaptured {
        entity,
        image: frame(),
    });
    assert_eq!(
        app.world().non_send::<HostOwner>().status,
        "Capturing screenshot…"
    );
    app.world_mut().despawn(entity);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    wait_for_writes(&mut app)?;
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .status
            .starts_with("Screenshot saved:")
    );
    Ok(())
}

fn backlog_app(folder: &std::path::Path) -> Result<App, Box<dyn std::error::Error>> {
    let mut app = app(folder, false)?;
    let (link, _, _) = crate::play::play_channel();
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    owner.settings.preferences.screenshots.backlog_frames = 2;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
    });
    Ok(app)
}

fn next_sample(app: &mut App) -> TestResult {
    let mut clock = app.world_mut().resource_mut::<Time<Real>>();
    let now = clock.last_update().ok_or("capture clock")?;
    clock.update_with_instant(now + std::time::Duration::from_secs(1));
    Ok(())
}

#[test]
fn f12_saves_the_buffered_frames_and_the_current_frame() -> TestResult {
    let fixture = TestDir::new()?;
    let mut app = backlog_app(fixture.path())?;
    for sample in 0..2 {
        if sample == 1 {
            next_sample(&mut app)?;
        }
        app.update();
        let entity = app
            .world_mut()
            .query_filtered::<Entity, With<Screenshot>>()
            .single(app.world())?;
        app.world_mut().trigger(ScreenshotCaptured {
            entity,
            image: frame(),
        });
        app.world_mut().despawn(entity);
        wait_for_writes(&mut app)?;
        assert_eq!(std::fs::read_dir(fixture.path())?.count(), 0);
    }
    super::tests::capture(&mut app)?;
    assert_eq!(std::fs::read_dir(fixture.path())?.count(), 3);
    Ok(())
}

#[test]
fn f12_saves_when_an_automatic_sample_is_due() -> TestResult {
    let fixture = TestDir::new()?;
    let mut app = backlog_app(fixture.path())?;
    super::tests::capture(&mut app)?;
    assert_eq!(std::fs::read_dir(fixture.path())?.count(), 1);
    Ok(())
}

#[test]
fn audit_f12_shares_a_readback_with_the_due_backlog_sample() -> TestResult {
    let fixture = TestDir::new()?;
    let mut app = backlog_app(fixture.path())?;
    let audit = fixture.join("audit.json");
    app.insert_resource(super::super::audit::AuditOutput::new(Some(audit.clone())));
    super::tests::capture(&mut app)?;
    assert!(audit.with_extension("png").is_file());
    app.insert_resource(super::super::audit::AuditOutput::new(None));
    super::tests::capture(&mut app)?;
    assert_eq!(std::fs::read_dir(fixture.path())?.count(), 3);
    Ok(())
}
