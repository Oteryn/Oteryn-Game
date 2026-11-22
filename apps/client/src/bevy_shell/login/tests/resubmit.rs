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
