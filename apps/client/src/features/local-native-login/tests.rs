use super::*;
use std::path::Path;
use std::time::Duration;

#[test]
fn refuses_further_sign_in_and_displays_manual_download_without_update_trust() {
    let mut form = LoginForm::new(LoginSettings::default());
    let url = "https://oteryn.com/download";
    form.finish(Err(GameplayEntryError::ClientUpdateRequired {
        manual_download_url: url.to_owned(),
        updater_started: false,
    }));
    assert!(form.message().contains(url));
    assert!(!form.can_submit());
    assert!(form.submit().is_err());
    form.offline();
    form.feedback("generic offline message".into());
    assert!(form.message().contains(url));
    assert!(!form.can_submit());
    form.inform("About", "Client details".into());
    assert_eq!(form.alert(), Some(("About", "Client details")));
    form.dismiss_alert();
    assert!(form.message().contains(url));
    #[cfg(bevy_shell)]
    assert!(form.show_status());
}

#[test]
fn maintenance_displays_state_and_only_allows_sign_in_after_deadline() {
    let mut form = LoginForm::new(LoginSettings::default());
    let future = Instant::now() + Duration::from_secs(30);
    form.finish(Err(GameplayEntryError::Maintenance { retry_at: future }));
    assert!(form.message().contains("maintenance"));
    assert!(!form.can_submit());
    assert!(form.submit().is_err());
    form.finish(Err(GameplayEntryError::Maintenance {
        retry_at: Instant::now(),
    }));
    assert!(form.can_submit());
}

#[test]
fn legacy_login_settings_keep_the_gameplay_root_without_blocking_sign_in()
-> Result<(), Box<dyn std::error::Error>> {
    let legacy = r#"{"platform_url":"https://platform.example","gateway_url":"https://gateway.example","oauth_client_id":"native-test","world_ref":"","character_id":"","dev_root":"missing.pem"}"#;
    let settings: LoginSettings = serde_json::from_str(legacy)?;
    assert_eq!(settings.config()?.dev_root, Path::new("missing.pem"));
    let with_unknown = legacy.replace(r#""dev_root""#, r#""extra":null,"dev_root""#);
    assert!(serde_json::from_str::<LoginSettings>(&with_unknown).is_err());
    Ok(())
}

#[test]
fn blank_gameplay_root_does_not_block_the_login_form() -> Result<(), String> {
    let mut form = LoginForm::new(LoginSettings {
        platform_url: "https://platform.example".into(),
        gateway_url: "https://gateway.example".into(),
        oauth_client_id: "native-test".into(),
        ..Default::default()
    });
    assert!(form.submit()?.dev_root.as_os_str().is_empty());
    Ok(())
}

#[test]
fn maintenance_message_tracks_the_deadline() {
    let mut form = LoginForm::new(LoginSettings::default());
    let started = Instant::now();
    let retry_at = started + Duration::from_secs(120);
    form.finish(Err(GameplayEntryError::Maintenance { retry_at }));
    form.refresh_gate_message(started);
    assert!(form.message().contains("120 s."));
    form.refresh_gate_message(started + Duration::from_secs(119));
    assert!(form.message().contains("1 s."));
    form.refresh_gate_message(retry_at);
    assert_eq!(
        form.message(),
        "The maintenance wait has ended. You can sign in again."
    );
    assert!(form.can_submit());
}

#[test]
fn legacy_update_refusal_keeps_its_update_instruction() {
    let mut form = LoginForm::new(LoginSettings::default());
    form.finish(Err(GameplayEntryError::Rejected(
        PublicClass::ClientUpdateRequired,
    )));
    form.feedback("generic offline message".into());
    assert_eq!(
        form.message(),
        "Install the current Oteryn client, then sign in again."
    );
    assert!(!form.can_submit());
}

#[test]
fn simultaneous_login_saves_use_unique_staging_files() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("login.json");
    let occupied = path.with_extension("json.tmp");
    std::fs::create_dir(&occupied)?;
    let barrier = std::sync::Barrier::new(2);
    let settings = |world: &str| {
        LoginSettings::from_values([
            "http://localhost:8080".into(),
            "http://localhost:9000".into(),
            "019a5f4a-1000-7000-8000-000000000001".into(),
            world.into(),
            String::new(),
            String::new(),
        ])
    };
    let results = std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            barrier.wait();
            settings("first").save(&path)
        });
        barrier.wait();
        (settings("second").save(&path), first.join())
    });
    assert!(results.0.is_ok(), "{:?}", results.0);
    assert!(results.1.map_err(|_| "first writer panicked")?.is_ok());
    assert!(["first", "second"].contains(&LoginSettings::load(&path)?.world_ref.as_str()));
    assert!(occupied.is_dir());
    std::fs::remove_dir(occupied)?;
    assert!(
        std::fs::read_dir(fixture.path())?.all(|entry| entry.is_ok_and(|entry| {
            entry.file_name() == "login.json" || entry.file_name() == "login.json.lock"
        }))
    );
    Ok(())
}
