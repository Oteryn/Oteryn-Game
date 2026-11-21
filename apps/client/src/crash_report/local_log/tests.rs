use super::*;
use std::time::Duration;
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;
/// The ring is process-global: the tests that fill it or read it back take turns.
static RING: Mutex<()> = Mutex::new(());

#[test]
fn utc_calendar_matches_known_instants() {
    let at = |seconds: u64| Utc::at(UNIX_EPOCH + Duration::from_millis(seconds));
    assert_eq!(at(0).rfc3339(), "1970-01-01T00:00:00.000Z");
    assert_eq!(at(951_782_400_000).date(), "2000-02-29");
    assert_eq!(
        at(1_794_909_296_789).file_stamp(),
        "2026-11-17T09-54-56.789Z"
    );
}

#[test]
fn the_ring_keeps_the_newest_lines_and_counts_levels() {
    let _ring = lock(&RING);
    let before = log_counters();
    for index in 0..(RECENT_LINES + 5) {
        record_log_line(LineLevel::Other, &format!("ring line {index}"));
    }
    record_log_line(LineLevel::Warn, "ring warning");
    let error = redact("ring error password=value");
    record_log_line(LineLevel::Error, &error);
    let lines = recent_log_lines(RECENT_LINES);
    assert_eq!(lines.len(), RECENT_LINES);
    assert!(lines.contains(&PLACEHOLDER.to_owned()));
    let after = log_counters();
    assert!(after.warnings > before.warnings);
    assert!(after.errors > before.errors);
    assert_eq!(recent_log_lines(1).len(), 1);
}

#[test]
fn a_crash_file_is_redacted_and_carries_message_context_backtrace_and_log() -> TestResult {
    let _ring = lock(&RING);
    let directory = TempDir::new()?;
    record_log_line(LineLevel::Error, "before the crash");
    record_log_line(LineLevel::Warn, PLACEHOLDER);
    set_crash_context(CrashContext {
        lifecycle: "playing".into(),
        adapter: Some("Test Adapter (Vulkan)".into()),
    });
    let backtrace = std::backtrace::Backtrace::force_capture().to_string();
    let crash = Crash {
        message: "test crash unique-7f3e",
        location: "apps/client/src/crash_report/local_log.rs:1:1",
        thread: "crash-test",
        backtrace: &backtrace,
    };
    let redacted = write_crash_file(
        directory.path(),
        &Crash {
            message: "test crash token=abc123 unique-7f3e",
            ..crash
        },
        UNIX_EPOCH,
    )?;
    let redacted_text = fs::read_to_string(redacted)?;
    assert_eq!(redacted_text, PLACEHOLDER);
    assert!(!redacted_text.contains("abc123"));
    let path = write_crash_file(
        directory.path(),
        &crash,
        UNIX_EPOCH + Duration::from_secs(1),
    )?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("name")?;
    assert!(
        name.starts_with("crash-") && name.ends_with("Z.log"),
        "{name}"
    );
    let text = fs::read_to_string(&path)?;
    assert!(text.contains("Message: test crash unique-7f3e"));
    assert!(text.contains("Thread: crash-test"));
    assert!(text.contains("Location: apps/client/src/crash_report/local_log.rs"));
    assert!(text.contains("Backtrace:\n") && text.contains("local_log"));
    assert!(text.contains(crate::CLIENT_BUILD));
    assert!(text.contains("before the crash"));
    assert!(text.contains(PLACEHOLDER));
    // The context is process-global; another test may have replaced it meanwhile.
    assert!(text.contains("Lifecycle: "));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&path)?.permissions().mode() & 0o777, 0o600);
    }
    directory.close()?;
    Ok(())
}

#[test]
fn a_new_crash_file_is_reported_once_on_the_next_start() -> TestResult {
    let directory = TempDir::new()?;
    assert_eq!(take_unseen_crash(directory.path())?, None);
    let crash = Crash {
        message: "m",
        location: "l",
        thread: "t",
        backtrace: "b",
    };
    let first = write_crash_file(
        directory.path(),
        &crash,
        UNIX_EPOCH + Duration::from_secs(10),
    )?;
    assert_eq!(take_unseen_crash(directory.path())?, Some(first));
    assert_eq!(take_unseen_crash(directory.path())?, None);
    let second = write_crash_file(
        directory.path(),
        &crash,
        UNIX_EPOCH + Duration::from_secs(20),
    )?;
    let again = write_crash_file(
        directory.path(),
        &crash,
        UNIX_EPOCH + Duration::from_secs(20),
    )?;
    assert_ne!(second, again, "same instant gets its own file");
    assert_eq!(take_unseen_crash(directory.path())?, Some(again));
    assert_eq!(take_unseen_crash(directory.path())?, None);
    directory.close()?;
    Ok(())
}

#[test]
fn oversized_seen_marker_is_rejected() -> TestResult {
    let directory = TempDir::new()?;
    std::fs::write(directory.path().join("crash-test.log"), "")?;
    std::fs::write(directory.path().join(SEEN_MARKER), vec![b' '; 1025])?;
    assert!(matches!(take_unseen_crash(directory.path()), Err(error) if error.kind() == io::ErrorKind::InvalidData));
    Ok(())
}
