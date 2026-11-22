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
fn sign_in_uses_the_settings_merged_from_disk() -> TestResult {
    use crate::account_services::test_server::CHARACTER;

    let temp = Temp::new()?;
    let path = temp.join("login.json");
    let mut external = settings(&temp)?;
    external.save(&path)?;
    let mut owner = HostOwner::new(None)?;
    owner.configure_login(Some(path.clone()), |_| None);
    owner.request_login();

    external.platform_url = "http://127.0.0.1:18567".into();
    external.save(&path)?;
    owner.login.form.settings.character_id = CHARACTER.into();
    let config = owner.login.submit()?;
    let saved = LoginSettings::load(&path)?;
    assert_eq!(saved.platform_url, external.platform_url);
    assert_eq!(saved.character_id, CHARACTER);
    assert_eq!(config.platform_url, saved.platform_url);
    Ok(())
}

#[test]
fn sign_in_uses_explicit_environment_over_valid_saved_fields() -> TestResult {
    use crate::account_services::test_server::CHARACTER;

    let temp = Temp::new()?;
    let path = temp.join("login.json");
    let mut saved = settings(&temp)?;
    saved.world_ref = "saved-world".into();
    saved.character_id = CHARACTER.into();
    saved.save(&path)?;
    let env_root = temp.join("env-root.der");
    std::fs::copy(&saved.dev_root, &env_root)?;
    let expected = LoginSettings {
        platform_url: "http://127.0.0.1:18566".into(),
        gateway_url: "http://127.0.0.1:18567".into(),
        oauth_client_id: "env-client".into(),
        world_ref: "env-world".into(),
        character_id: String::new(),
        dev_root: env_root.to_string_lossy().into_owned(),
    };
    let mut owner = HostOwner::new(None)?;
    owner.configure_login(Some(path.clone()), |key| {
        crate::native_login::FIELDS
            .iter()
            .position(|field| field.1 == key)
            .map(|i| expected.values()[i].to_owned())
    });
    owner.request_login();
    let mut app = login_app(owner);
    let submit = control(&mut app, "login.submit")?;
    app.world_mut().trigger(Activate { entity: submit });

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    let config = loop {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        if let Some((config, _)) = owner.login.poll_save() {
            break config;
        }
        assert!(owner.login.save_pending(), "{}", owner.login.form.message());
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    };
    assert_eq!(config.gateway_url, expected.gateway_url);
    assert_eq!(config.platform_url, expected.platform_url);
    assert_eq!(config.oauth_client_id, expected.oauth_client_id);
    assert_eq!(config.world_ref, expected.world_ref);
    assert_eq!(config.character_id, expected.character_id);
    assert_eq!(config.dev_root, env_root);
    assert_eq!(LoginSettings::load(&path)?.values(), expected.values());
    Ok(())
}

#[test]
fn failed_admission_keeps_merged_gateway_on_editor_retry() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.join("login.json");
    let mut external = settings(&temp)?;
    external.save(&path)?;
    let mut owner = HostOwner::new(None)?;
    owner.configure_login(Some(path.clone()), |_| None);
    owner.request_login();
    let mut app = login_app(owner);
    let submit = control(&mut app, "login.submit")?;
    let dismiss = control(&mut app, "login.alert.ok")?;
    let gateway = app
        .world_mut()
        .query::<(Entity, &LoginField)>()
        .iter(app.world())
        .find_map(|(entity, field)| (field.0 == 1).then_some(entity))
        .ok_or("gateway field")?;

    external.gateway_url = "http://127.0.0.1:18567".into();
    external.save(&path)?;
    for _ in 0..2 {
        app.world_mut().trigger(Activate { entity: submit });
        {
            let mut owner = app.world_mut().non_send_mut::<HostOwner>();
            assert!(owner.login.save_pending());
            owner
                .login
                .set_pending_assets(Some(temp.join("missing-assets").into_os_string()));
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            while owner.login.save_pending() {
                owner.poll([280, 280]);
                assert!(std::time::Instant::now() < deadline);
                std::thread::yield_now();
            }
            assert!(
                owner
                    .login
                    .form
                    .message()
                    .contains("asset root cannot be opened")
            );
        }
        app.update();
        app.world_mut().trigger(Activate { entity: dismiss });
        app.update();
    }

    assert_eq!(
        LoginSettings::load(&path)?.gateway_url,
        external.gateway_url
    );
    assert_eq!(
        app.world()
            .get::<EditableText>(gateway)
            .ok_or("editor")?
            .value()
            .to_string(),
        external.gateway_url
    );
    Ok(())
}
