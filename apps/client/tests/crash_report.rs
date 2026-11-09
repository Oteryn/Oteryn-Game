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

#[test]
fn pending_crash_review_declines_discards_and_retains_on_failed_export()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client::crash_report::review_pending_at;
    let directory = std::env::temp_dir().join(format!("crash-review-{}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    let pending = directory.join("pending.txt");
    let destination = directory.join("export.txt");
    let report = CrashReport::capture(Some("source.rs"), 7);
    report.save(&pending, true)?;
    assert!(!review_pending_at(&pending, &destination, |preview| {
        assert!(preview.contains("source.rs:7"));
        false
    })?);
    assert!(!pending.exists());
    assert!(!destination.exists());
    report.save(&pending, true)?;
    assert!(review_pending_at(&pending, &destination, |_| true)?);
    assert!(!pending.exists());
    assert_eq!(std::fs::read_to_string(&destination)?, report.preview());
    report.save(&pending, true)?;
    assert!(review_pending_at(&pending, &destination, |_| true).is_err());
    assert!(pending.exists());
    std::fs::remove_dir_all(directory)?;
    Ok(())
}
