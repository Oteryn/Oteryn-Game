//! Replay the exact command results captured during one verified M01 journey.

use crate::target_move::{WalkDisposition, WalkResponse, project};
use oteryn_session::{CommandStatus, GatedCommandResult};

#[path = "../../../../../../tools/qualification/e2e_all/m01_replay.rs"]
mod m01_replay;

type ReplayResult<T> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn rejects_inconsistent_m01_receipt() -> ReplayResult<()> {
    let bytes = b"real-session 7 1 20\n\x08\x03";
    assert!(m01_replay::read_result(bytes, "other-session").is_err());
    let result = m01_replay::read_result(bytes, "real-session")?;
    assert!(project(&result).is_err());
    Ok(())
}

#[test]
fn rejects_missing_stale_or_wrong_batch_m01_capture() -> ReplayResult<()> {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let root = std::env::temp_dir().join(format!("m01-replay-{}-{nonce}", std::process::id()));
    std::fs::create_dir(&root)?;
    let missing = m01_replay::replay(&root, "run-a", "128:1", "session-a");
    std::fs::write(root.join("m01-run"), b"M01.client 128:1 run-a session-a\n")?;
    let stale = m01_replay::replay(&root, "run-b", "128:1", "session-a");
    let wrong_batch = m01_replay::replay(&root, "run-a", "127:2", "session-a");
    let incomplete = m01_replay::replay(&root, "run-a", "128:1", "session-a");
    std::fs::remove_dir_all(&root)?;
    assert!(missing.is_err());
    for result in [stale, wrong_batch] {
        assert_eq!(
            result.err().map(|error| error.to_string()).as_deref(),
            Some("M01 capture run, batch, or session mismatch")
        );
    }
    assert!(incomplete.is_err());
    Ok(())
}
