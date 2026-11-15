//! Actual login-owner callers with an explicit asset lookup, without process-environment mutation.
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{cell::Cell, ffi::OsString};
type TestResult = Result<(), Box<dyn std::error::Error>>;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> std::io::Result<Self> {
        let path = std::env::temp_dir().join(format!(
            "bevy-live-assets-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path)?;
        Ok(Self(path))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn valid_login(temp: &Temp) -> Result<HostOwner, Box<dyn std::error::Error>> {
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let root = temp.0.join("root.der");
    std::fs::write(&root, certificate.cert.der())?;
    let mut host = HostOwner::new(None)?;
    host.configure_login(Some(temp.0.join("login.json")), |_| None);
    host.login.form.settings = crate::native_login::LoginSettings {
        platform_url: "http://127.0.0.1:18564".into(),
        gateway_url: "http://127.0.0.1:18565".into(),
        oauth_client_id: "native-test".into(),
        dev_root: root.to_string_lossy().into_owned(),
        ..Default::default()
    };
    host.request_login();
    Ok(host)
}
#[test]
fn valid_login_without_explicit_live_metadata_never_requests_admission() -> TestResult {
    let temp = Temp::new()?;
    let file = temp.0.join("not-directory");
    std::fs::write(&file, b"not an asset directory")?;
    let metadata_root = temp.0.join("pinned-metadata");
    let manifest = metadata_root.join(oteryn_client_assets::world::MANIFEST_PATH);
    std::fs::create_dir_all(manifest.parent().ok_or("manifest parent")?)?;
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join(oteryn_client_assets::world::MANIFEST_PATH),
        manifest,
    )?;
    for asset_root in [
        None,
        Some(OsString::new()),
        Some(temp.0.join("missing").into_os_string()),
        Some(file.into_os_string()),
        Some(temp.0.clone().into_os_string()),
        Some(metadata_root.into_os_string()),
    ] {
        let mut host = valid_login(&temp)?;
        let calls = Cell::new(0);
        host.submit_login_with_assets(|| {
            calls.set(calls.get() + 1);
            asset_root
        });
        assert!(
            !host.entry.as_ref().is_some_and(Reentry::is_entering),
            "bad assets must not spawn native entry"
        );
        assert_eq!(host.phase, Phase::Offline);
        assert_eq!(calls.get(), 1);
        assert!(!host.login.form.pending());
        assert!(host.login.form.can_submit());
        assert!(host.login.visible);
        assert!(host.login.form.message().contains("OTERYN_ASSET_DIR"));
        host.poll([1024, 720]);
        assert_eq!(host.phase, Phase::Offline);
    }
    Ok(())
}
#[test]
fn invalid_form_keeps_validator_precedence_before_asset_lookup() -> TestResult {
    let temp = Temp::new()?;
    let mut host = valid_login(&temp)?;
    host.login.form.settings.platform_url = "not a URL".into();
    let calls = Cell::new(0);
    host.submit_login_with_assets(|| {
        calls.set(calls.get() + 1);
        None
    });
    assert_eq!(calls.get(), 0);
    assert!(!host.login.form.pending());
    assert!(!host.login.form.message().contains("OTERYN_ASSET_DIR"));
    assert_eq!(host.phase, Phase::Offline);
    Ok(())
}
