use super::*;

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
