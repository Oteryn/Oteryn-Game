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
