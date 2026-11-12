//! C01 headless scenario: first-run configuration, failed connection, edit, retry, admission.
use oteryn_client::GameplayEntryError;
use oteryn_client::native_login::{LoginForm, LoginSettings};

#[test]
fn configure_connect_correct_and_retry() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!("oteryn-c01-harness-{}.json", std::process::id()));
    let root = path.with_extension("pem");
    std::fs::write(
        &root,
        rcgen::generate_simple_self_signed(vec!["localhost".into()])?
            .cert
            .der()
            .as_ref(),
    )?;
    let mut form = LoginForm::new(LoginSettings::default());
    assert!(form.submit().is_err());
    form.settings = LoginSettings::from_values([
        "https://platform.example/".into(),
        "https://gateway.example/".into(),
        "native-client".into(),
        "wrong-world".into(),
        "01890a5d-ac96-774b-bcce-b302099a8058".into(),
        root.to_string_lossy().into_owned(),
    ]);
    form.settings.save(&path)?;
    form.settings = LoginSettings::load(&path)?;
    assert_eq!(form.submit()?.world_ref, "wrong-world");
    assert!(form.pending());
    assert!(form.submit().is_err());
    form.finish(Err(GameplayEntryError::WorldUnavailable));
    assert!(form.message().contains("directory"));
    form.settings.world_ref = "alpha".into();
    form.settings.save(&path)?;
    assert_eq!(form.submit()?.world_ref, "alpha");
    form.finish(Ok(()));
    assert!(!form.pending());
    assert_eq!(form.message(), "Connected.");
    assert_eq!(LoginSettings::load(&path)?.world_ref, "alpha");
    std::fs::remove_file(path)?;
    std::fs::remove_file(root)?;
    Ok(())
}
