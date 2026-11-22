use super::*;

#[test]
fn regression_pending_writer_resubmits_once() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.join("login.json");
    let mut owner = HostOwner::new(None)?;
    owner.configure_login(Some(path.clone()), |_| None);
    owner.request_login();
    owner.login.form.settings = settings(&temp)?;
    let lock = crate::atomic_file::lock(&path)?;
    owner.submit_login();
    owner.cancel_login();
    let new_url = "http://127.0.0.1:18567";
    owner.login.form.settings.platform_url = new_url.into();
    owner.submit_login_with_assets(|| Some("new-assets".into()));
    drop(lock);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        if let Some((config, assets)) = owner.login.poll_save() {
            assert_eq!(config.platform_url, new_url);
            assert_eq!(assets, Some("new-assets".into()));
            break;
        }
        assert!(std::time::Instant::now() < deadline, "resubmission lost");
        std::thread::yield_now();
    }
    assert!(!owner.login.save_pending());
    assert!(owner.login.poll_save().is_none());
    Ok(())
}

#[test]
fn resubmission_restores_saved_gateway_after_cancellation() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.join("login.json");
    let original = settings(&temp)?;
    original.save(&path)?;
    let mut owner = HostOwner::new(None)?;
    owner.configure_login(Some(path.clone()), |_| None);
    owner.request_login();

    let mut external = original.clone();
    external.oauth_client_id = "another-client".into();
    external.save(&path)?;
    let lock = crate::atomic_file::lock(&path)?;
    owner.login.form.settings.gateway_url = "http://127.0.0.1:18567".into();
    owner.submit_login();
    assert!(owner.login.save_pending());
    owner.cancel_login();
    owner.login.form.settings.gateway_url = original.gateway_url.clone();
    owner.submit_login();
    drop(lock);

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while owner.login.poll_save().is_none() {
        assert!(std::time::Instant::now() < deadline, "resubmission lost");
        std::thread::yield_now();
    }
    let saved = LoginSettings::load(&path)?;
    assert_eq!(saved.gateway_url, original.gateway_url);
    assert_eq!(saved.oauth_client_id, external.oauth_client_id);
    Ok(())
}

#[test]
fn failed_resubmission_retries_the_submitted_gateway() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.join("login.json");
    let original = settings(&temp)?;
    original.save(&path)?;
    let mut owner = HostOwner::new(None)?;
    owner.configure_login(Some(path.clone()), |_| None);
    owner.request_login();

    let held = crate::atomic_file::lock(&path)?;
    let submitted = "http://127.0.0.1:18567";
    owner.login.form.settings.gateway_url = submitted.into();
    owner.submit_login();
    owner.cancel_login();
    owner.login.form.settings.gateway_url = original.gateway_url.clone();
    drop(held);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while LoginSettings::load(&path)?.gateway_url != submitted {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }

    let held = crate::atomic_file::lock(&path)?;
    owner.submit_login();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while owner.login.save_pending() {
        assert!(owner.login.poll_save().is_none());
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    drop(held);
    assert_eq!(owner.login.submit()?.gateway_url, original.gateway_url);
    assert_eq!(
        LoginSettings::load(&path)?.gateway_url,
        original.gateway_url
    );
    Ok(())
}

#[test]
fn submit_repairs_empty_saved_gateway_from_environment() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.join("login.json");
    let mut original = settings(&temp)?;
    let gateway = std::mem::take(&mut original.gateway_url);
    std::fs::write(&path, serde_json::to_vec(&original)?)?;
    let mut owner = HostOwner::new(None)?;
    owner.configure_login(Some(path.clone()), |key| {
        (key == "OTERYN_GATEWAY_URL").then(|| gateway.clone())
    });
    owner.request_login();
    owner.submit_login();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        if let Some((config, _)) = owner.login.poll_save() {
            assert_eq!(config.gateway_url, gateway);
            break;
        }
        assert!(owner.login.save_pending(), "{}", owner.login.form.message());
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert_eq!(LoginSettings::load(&path)?.gateway_url, gateway);
    Ok(())
}

#[test]
fn gateway_edit_signs_in_with_the_concurrent_oauth_client() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.join("login.json");
    let original = settings(&temp)?;
    original.save(&path)?;
    let mut owner = HostOwner::new(None)?;
    owner.configure_login(Some(path.clone()), |_| None);
    owner.request_login();

    let mut external = original;
    external.oauth_client_id = "another-client".into();
    external.save(&path)?;
    let gateway_url = "http://127.0.0.1:18567";
    owner.login.form.settings.gateway_url = gateway_url.into();
    let config = owner.login.submit()?;
    assert_eq!(config.gateway_url, gateway_url);
    assert_eq!(
        LoginSettings::load(&path)?.oauth_client_id,
        external.oauth_client_id
    );
    assert_eq!(config.oauth_client_id, external.oauth_client_id);
    Ok(())
}

#[test]
fn review_round_environment_overrides_survive_save() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.join("login.json");
    let mut original = settings(&temp)?;
    original.world_ref = "saved-world".into();
    original.character_id = "29020304-0506-7008-800a-0b0c0d0e0f29".into();
    original.save(&path)?;
    let mut state = LoginState::default();
    let root = temp.join("override.der");
    std::fs::copy(&original.dev_root, &root)?;
    let gateway = "http://127.0.0.1:18567";
    let character = "29020304-0506-7008-800a-0b0c0d0e0f30";
    state.load(Some(path.clone()), |key| {
        Some(match key {
            "OTERYN_GATEWAY_URL" => gateway.into(),
            "OTERYN_OAUTH_CLIENT_ID" => "override-client".into(),
            "OTERYN_WORLD" => "override-world".into(),
            "OTERYN_CHARACTER_ID" => character.into(),
            "OTERYN_DEV_ROOT" => root.to_string_lossy().into_owned(),
            _ => return None,
        })
    });
    let config = state.submit()?;
    assert_eq!(config.gateway_url, gateway);
    assert_eq!(config.oauth_client_id, "override-client");
    assert_eq!(config.world_ref, "override-world");
    assert_eq!(config.character_id, character);
    assert_eq!(config.dev_root, root);
    assert!(LoginSettings::load(&path)? == original);
    Ok(())
}

#[test]
fn review_round_platform_switch_saves_a_coherent_selection() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.join("login.json");
    let original = settings(&temp)?;
    original.save(&path)?;
    let mut state = LoginState::default();
    state.load(Some(path.clone()), |_| None);

    let mut external = original.clone();
    external.oauth_client_id = "concurrent-client".into();
    external.save(&path)?;
    state.form.settings.platform_url = "http://127.0.0.1:18569".into();
    let config = state.submit()?;
    assert_eq!(config.oauth_client_id, original.oauth_client_id);
    assert!(LoginSettings::load(&path)? == state.form.settings);
    Ok(())
}

#[test]
fn review_round_stale_gateway_edit_leaves_new_platform_selection_intact() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.join("login.json");
    let original = settings(&temp)?;
    original.save(&path)?;
    let mut state = LoginState::default();
    state.load(Some(path.clone()), |_| None);

    let mut external = original.clone();
    external.platform_url = "http://127.0.0.1:18569".into();
    external.oauth_client_id = "concurrent-client".into();
    external.save(&path)?;
    let gateway = "http://127.0.0.1:18567";
    state.form.settings.gateway_url = gateway.into();
    let config = state.submit()?;
    assert_eq!(config.platform_url, original.platform_url);
    assert_eq!(config.gateway_url, gateway);
    assert_eq!(config.oauth_client_id, original.oauth_client_id);
    assert!(LoginSettings::load(&path)? == external);
    Ok(())
}

#[test]
fn review_round_retry_keeps_concurrent_settings_after_asset_failure() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.join("login.json");
    let original = settings(&temp)?;
    original.save(&path)?;
    let mut host = HostOwner::new(None)?;
    host.configure_login(Some(path.clone()), |_| None);
    host.request_login();
    let mut app = login_app(host);
    app.add_systems(PreUpdate, |mut owner: NonSendMut<HostOwner>| {
        owner.poll([800, 600]);
    });

    let mut external = original;
    external.oauth_client_id = "concurrent-client".into();
    external.save(&path)?;

    for _ in 0..2 {
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .submit_login_with_assets(|| Some(temp.join("missing-assets").into_os_string()));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            app.update();
            let owner = app.world().non_send::<HostOwner>();
            if !owner.login.save_pending() {
                assert!(owner.login.form.message().contains("live asset root"));
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
    }
    assert_eq!(
        LoginSettings::load(&path)?.oauth_client_id,
        external.oauth_client_id
    );
    Ok(())
}
