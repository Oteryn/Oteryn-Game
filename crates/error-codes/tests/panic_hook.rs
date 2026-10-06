//! Forced panics in a child process: the hook writes exactly one `E4001` line.
#![allow(clippy::unwrap_used, clippy::panic)]

use oteryn_error_codes::{install_panic_hook, parse_line};
use std::process::Command;

const CHILD: &str = "OTERYN_PANIC_CHILD";
const BUILD: &str = "0.1.0+0123456789ab";

#[test]
fn child_panics() {
    let Ok(kind) = std::env::var(CHILD) else {
        return;
    };
    install_panic_hook(
        "probe",
        BUILD.to_owned(),
        Some("0192e0a8-0000-7000-8000-000000000001".to_owned()),
    );
    let secret = "character-42";
    if kind == "static" {
        panic!("static boom");
    }
    panic!("formatted {secret}");
}

fn run(kind: &str) -> (Vec<String>, String) {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_panics", "--nocapture", "--test-threads=1"])
        .env(CHILD, kind)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    let lines = stderr
        .lines()
        .filter(|line| line.starts_with("probe "))
        .map(str::to_owned)
        .collect();
    (lines, stderr)
}

#[test]
fn a_forced_panic_writes_exactly_one_e4001_line_with_a_static_payload() {
    let (lines, stderr) = run("static");
    assert_eq!(lines.len(), 1, "{stderr}");
    assert!(
        !stderr.contains("panicked at"),
        "the default hook must not run: {stderr}"
    );
    let parsed = parse_line(&lines[0]).unwrap();
    assert_eq!(parsed.code, Some(4001));
    assert_eq!(parsed.build.as_deref(), Some(BUILD));
    assert!(parsed.detail.unwrap().contains("payload=static boom"));
}

#[test]
fn a_formatted_payload_is_withheld_from_the_line() {
    let (lines, stderr) = run("formatted");
    assert_eq!(lines.len(), 1, "{stderr}");
    assert!(!stderr.contains("character-42"), "{stderr}");
    assert!(
        parse_line(&lines[0])
            .unwrap()
            .detail
            .unwrap()
            .contains("payload=formatted")
    );
}
