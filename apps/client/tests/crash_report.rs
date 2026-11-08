use oteryn_client::crash_report::CrashReport;
#[test]
fn crash_export_requires_consent_and_never_overwrites() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!("cg-login-crash-{}.txt", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let report = CrashReport::capture(Some("/private/profile/source.rs"), 42);
    assert!(!report.preview().contains("/private/profile"));
    assert!(!report.save(&path, false)?);
    assert!(!path.exists());
    assert!(report.save(&path, true)?);
    assert!(std::fs::read_to_string(&path)?.contains("source.rs:42"));
    assert!(report.save(&path, true).is_err());
    std::fs::remove_file(path)?;
    Ok(())
}
