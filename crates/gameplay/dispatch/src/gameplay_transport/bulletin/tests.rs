use super::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn operator_file_reload_publishes_future_events_and_rejects_invalid_replacements() -> TestResult {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let path = std::env::temp_dir().join(format!("oteryn-bulletin-{}.json", std::process::id()));
    let source = BulletinSource::new(path.clone());
    let first = r#"{"news":[{"title":"First","body":"Operator publication"}],"events":[]}"#;
    std::fs::write(&path, first)?;
    assert_eq!(runtime.block_on(source.load()).as_deref(), Some(first));
    let future = r#"{"news":[],"events":[{"year":2027,"month":1,"day":12,"title":"Gathering","body":"Town square"}]}"#;
    std::fs::write(&path, future)?;
    assert_eq!(runtime.block_on(source.load()).as_deref(), Some(future));
    for invalid in [
        future.replace("\"month\":1", "\"month\":13"),
        future.replace("\"day\":12", "\"day\":32"),
        first.replace("\"news\"", "\"credentials\""),
        "x".repeat(256 * 1024 + 1),
    ] {
        std::fs::write(&path, invalid)?;
        assert!(runtime.block_on(source.load()).is_none());
    }
    std::fs::remove_file(path)?;
    assert!(runtime.block_on(source.load()).is_none());
    Ok(())
}
