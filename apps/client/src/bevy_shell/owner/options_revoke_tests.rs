use super::HostOwner;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn turning_remember_device_off_forgets_the_device_and_logs_the_character_out() -> TestResult {
    use std::sync::Arc;
    let temp = tempfile::tempdir()?;
    let mut host = HostOwner::new(Some(temp.path().join("preferences.json")))?;
    let config = crate::NativeLoginConfig::from_env(|name| {
        Some(match name {
            "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => "https://localhost:1".into(),
            "OTERYN_OAUTH_CLIENT_ID" => "019a5f4a-1000-7000-8000-000000000001".into(),
            "OTERYN_DEV_ROOT" => "/tmp/reentry-unused-root".into(),
            _ => String::new(),
        })
    })?
    .ok_or("native login config")?;
    let device = Arc::clone(&config.device_sessions);
    // Holding the device session keeps a scheduled revocation task (and its owner) alive.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let held_session = runtime.block_on(device.hold_session_for_reentry_test());
    assert!(host.entry.as_mut().ok_or("entry")?.configure(config));
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.set_preference_character([7; 16]);
    let (link, mut commands, _events) = crate::play::play_channel();
    assert!(host.install_play(view, link));
    let mut preferences = host.settings.preferences.clone();
    preferences.miscellaneous.remember_device = true;
    host.settings.apply(preferences);
    let owners = Arc::strong_count(&device);
    host.open_graphics();
    host.edit_preferences(|preferences| preferences.miscellaneous.remember_device = false);
    host.apply(true);
    host.finish_forget_intent();
    assert!(!host.settings.preferences.miscellaneous.remember_device);
    assert_eq!(
        Arc::strong_count(&device),
        owners + 1,
        "turning remember-device off must schedule the device revocation"
    );
    let play = host.play.as_mut().ok_or("play")?;
    play.view
        .tick(&play.link)
        .map_err(|end| format!("unexpected end {end:?}"))?;
    assert!(
        std::iter::from_fn(|| commands.try_recv().ok())
            .any(|command| matches!(command, crate::combat_input::PlayCommand::Logout))
    );
    drop(held_session);
    host.close();
    Ok(())
}
