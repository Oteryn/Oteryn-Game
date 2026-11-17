use oteryn_client::GameplayEntryError;
use oteryn_client::native_login::{LoginForm, LoginSettings, error_text};

#[test]
fn native_login_input_validation_and_retry_projection() {
    let mut form = LoginForm::new(LoginSettings::default());
    assert!(form.submit().is_err());
    assert!(!form.pending());
    form.finish(Err(GameplayEntryError::WorldUnavailable));
    assert!(form.message().contains("World"));
    assert!(form.message().contains("directory"));
    assert!(error_text(GameplayEntryError::InvalidConfiguration).contains("certificate"));
}

#[test]
fn native_login_settings_reject_secrets_in_endpoints() {
    let settings = LoginSettings {
        platform_url: "https://user:secret@example.com/".into(),
        ..LoginSettings::default()
    };
    assert!(settings.validate().is_err());
    let settings = LoginSettings {
        platform_url: "https://example.com/?token=secret".into(),
        ..LoginSettings::default()
    };
    assert!(settings.validate().is_err());
}

#[test]
fn native_login_round_trip_pending_edit_and_retry() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::temp_dir().join(format!("oteryn-c01-{}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let root = directory.join("root.pem");
    std::fs::write(&root, certificate.cert.der().as_ref())?;
    let path = directory.join("login.json");
    let settings = LoginSettings::from_values([
        " https://platform.example/ ".into(),
        "https://gateway.example/".into(),
        "native-public-client".into(),
        "alpha".into(),
        "01890a5d-ac96-774b-bcce-b302099a8058".into(),
        root.to_string_lossy().into_owned(),
    ]);
    settings.save(&path)?;
    let loaded = LoginSettings::load(&path)?;
    assert!(settings == loaded);
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    assert_eq!(json.as_object().map(|object| object.len()), Some(6));
    let mut form =
        LoginForm::new(loaded.with_env(|name| (name == "OTERYN_WORLD").then(|| "beta".into())));
    let config = form.submit()?;
    assert_eq!(config.world_ref, "beta");
    assert!(form.pending());
    assert!(form.submit().is_err());
    form.finish(Err(GameplayEntryError::WorldUnavailable));
    assert!(!form.pending());
    form.settings.world_ref = "alpha".into();
    assert_eq!(form.submit()?.world_ref, "alpha");
    form.finish(Ok(()));
    assert_eq!(form.message(), "Connected.");
    let mut invalid = settings.clone();
    for url in [
        "https://user:secret@example.com/",
        "https://example.com/?token=secret",
        "http://example.com/",
    ] {
        invalid.platform_url = url.into();
        assert!(invalid.save(&path).is_err());
        assert!(LoginSettings::load(&path)? == settings);
    }
    invalid = settings.clone();
    invalid.character_id = "not-a-character".into();
    assert!(invalid.validate().is_err());
    invalid = settings.clone();
    invalid.dev_root = directory.join("absent.pem").to_string_lossy().into_owned();
    assert!(invalid.validate().is_err());
    std::fs::write(&path, b"{bad-json")?;
    assert!(LoginSettings::load(&path).is_err());
    assert!(LoginSettings::default().save(&path).is_err());
    std::fs::remove_dir_all(directory)?;
    Ok(())
}
