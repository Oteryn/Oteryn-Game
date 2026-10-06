//! Process-level checks of the diagnostic line (ERR-NODE-1): every binary writes
//! `process_start` first, refuses a malformed `OTERYN_LOG` with its registered code, and
//! reports a failure as exactly one coded line in the documented field order.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::process::{Command, Output, Stdio};

use oteryn_error_codes::{Level, ParsedLine, parse_line};

const SERVER: &str = env!("CARGO_BIN_EXE_oteryn-game-server");
const MIGRATE: &str = env!("CARGO_BIN_EXE_oteryn-game-migrate");
const IMPORT: &str = env!("CARGO_BIN_EXE_oteryn-game-import-proficiencies");
const OPS: &str = env!("CARGO_BIN_EXE_oteryn-game-ops");

fn run(binary: &str, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(binary);
    command
        .args(args)
        .env_remove("OTERYN_LOG")
        .env_remove("OTERYN_BUILD_SHA");
    command.env_remove("OTERYN_GAME_MIGRATION_DATABASE_URL");
    for (key, value) in env {
        command.env(key, value);
    }
    command.output().expect("process runs")
}

fn lines(output: &Output) -> Vec<ParsedLine> {
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .map(|line| parse_line(line).unwrap_or_else(|error| panic!("{line:?}: {error:?}")))
        .collect()
}

#[test]
fn version_prints_the_process_and_build() {
    for (binary, process) in [
        (SERVER, "oteryn-game-server"),
        (MIGRATE, "oteryn-game-migrate"),
        (IMPORT, "oteryn-game-import-proficiencies"),
        (OPS, "oteryn-game-ops"),
    ] {
        let output = run(binary, &["--version"], &[]);
        assert!(output.status.success(), "{process}");
        let text = String::from_utf8_lossy(&output.stdout);
        let (name, build) = text.trim().split_once(' ').expect("process and build");
        assert_eq!(name, process);
        assert!(build.starts_with(env!("CARGO_PKG_VERSION")), "{build}");
        assert!(build.contains('+'), "{build}");
    }
}

#[test]
fn the_first_line_is_process_start_with_the_build() {
    let output = run(
        SERVER,
        &["serve", "--config", "/nonexistent/oteryn.toml"],
        &[],
    );
    let parsed = lines(&output);
    assert_eq!(parsed[0].event, "process_start");
    assert!(
        parsed[0]
            .build
            .as_deref()
            .is_some_and(|build| build.contains('+'))
    );
    assert_eq!(parsed[0].process, "oteryn-game-server");
}

#[test]
fn a_malformed_log_spec_fails_the_start_with_its_code() {
    for (binary, code) in [(SERVER, 2014), (MIGRATE, 2014), (IMPORT, 2014), (OPS, 6007)] {
        let output = run(binary, &[], &[("OTERYN_LOG", "loud=")]);
        assert_eq!(output.status.code(), Some(2), "{binary}");
        let parsed = lines(&output);
        assert_eq!(parsed[0].event, "process_start", "{binary}");
        let coded: Vec<_> = parsed.iter().filter(|line| line.code.is_some()).collect();
        assert_eq!(coded.len(), 1, "{binary}");
        assert_eq!(coded[0].code, Some(code), "{binary}");
    }
}

#[cfg(unix)]
#[test]
fn a_non_utf8_log_spec_is_malformed_not_unset() {
    use std::os::unix::ffi::OsStrExt;
    for (binary, code) in [(SERVER, 2014), (MIGRATE, 2014), (IMPORT, 2014), (OPS, 6007)] {
        let output = Command::new(binary)
            .env_remove("OTERYN_BUILD_SHA")
            .env("OTERYN_LOG", std::ffi::OsStr::from_bytes(b"warn\xff"))
            .output()
            .expect("process runs");
        assert_eq!(output.status.code(), Some(2), "{binary}");
        let parsed = lines(&output);
        assert_eq!(parsed[0].event, "process_start", "{binary}");
        let coded: Vec<_> = parsed.iter().filter(|line| line.code.is_some()).collect();
        assert_eq!(coded.len(), 1, "{binary}");
        assert_eq!(coded[0].code, Some(code), "{binary}");
    }
}

#[test]
fn a_warn_filter_drops_info_lines_but_keeps_coded_lines() {
    let output = run(
        SERVER,
        &["serve", "--config", "/nonexistent/oteryn.toml"],
        &[("OTERYN_LOG", "warn")],
    );
    let parsed = lines(&output);
    assert_eq!(parsed[0].event, "process_start");
    assert!(
        parsed
            .iter()
            .all(|line| line.level <= Level::Warn || line.event == "process_start")
    );
    assert!(parsed.iter().any(|line| line.code == Some(2001)));
}

#[test]
fn a_boot_failure_is_one_coded_line_with_the_boot_exit_status() {
    let output = run(
        SERVER,
        &["serve", "--config", "/nonexistent/oteryn.toml"],
        &[],
    );
    assert_eq!(output.status.code(), Some(10));
    let parsed = lines(&output);
    let failed: Vec<_> = parsed
        .iter()
        .filter(|line| line.event == "boot_failed")
        .collect();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].code, Some(2001));
    assert_eq!(failed[0].level, Level::Error);
    assert!(failed[0].trace.is_some());
}

#[test]
fn migrate_without_a_database_url_writes_one_coded_line() {
    let output = run(MIGRATE, &[], &[]);
    assert_eq!(output.status.code(), Some(1));
    let parsed = lines(&output);
    let coded: Vec<_> = parsed.iter().filter(|line| line.code.is_some()).collect();
    assert_eq!(coded.len(), 1);
    assert_eq!(coded[0].code, Some(3010));
    assert_eq!(parsed.len(), 2, "process_start plus the failure line");
}

#[cfg(target_os = "linux")]
#[test]
fn import_proficiencies_reports_a_write_failure_as_one_coded_line() {
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .expect("/dev/full");
    let mut command = Command::new(IMPORT);
    command.env_remove("OTERYN_LOG").stdout(Stdio::from(full));
    let output = command.output().expect("process runs");
    assert_eq!(output.status.code(), Some(1));
    let parsed = lines(&output);
    let coded: Vec<_> = parsed.iter().filter(|line| line.code.is_some()).collect();
    assert_eq!(coded.len(), 1);
    assert_eq!(coded[0].code, Some(3011));
}
