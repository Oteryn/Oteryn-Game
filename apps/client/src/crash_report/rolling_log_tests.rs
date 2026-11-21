use super::{KEEP_FILES, RollingFile, Sink, TOTAL_CAP};
use bevy::log::tracing_subscriber::fmt::MakeWriter;
use std::{fs, io::Write, path::Path, sync::Arc};
use tempfile::TempDir;

fn log_fixture(parent: &Path) -> std::io::Result<TempDir> {
    TempDir::new_in(parent)
}

#[test]
fn rolling_log_fixtures_are_exclusive() -> std::io::Result<()> {
    let parent = TempDir::new()?;
    let first = log_fixture(parent.path())?;
    let marker = first.path().join("marker");
    fs::write(&marker, "first fixture")?;
    let second = log_fixture(parent.path())?;
    assert_ne!(first.path(), second.path());
    second.close()?;
    assert_eq!(fs::read_to_string(marker)?, "first fixture");
    Ok(())
}

#[test]
fn rolling_log_applies_lexical_record_redaction() -> Result<(), Box<dyn std::error::Error>> {
    let parent = TempDir::new()?;
    let directory = log_fixture(parent.path())?;
    let sink = Sink {
        file: Some(Arc::new(RollingFile::new(
            directory.path().join("logs"),
            KEEP_FILES,
            TOTAL_CAP,
        ))),
        stderr: false,
        ring: false,
    };
    let sensitive = [
        r#"{"game_login_ticket":"abc123"}"#,
        "%E2%2584%25AAey=abc",
        "%252525252525252574oken=abc",
        r#"{"url":"https:\/\/alice:p%2Fw@host"}"#,
        "GET /callback?code:abc",
        "GET /callback?x=1&key:abc",
        "GET /callback?x=1&sig:abc",
        "GET /callback?x=1&signature:abc",
        "GET /callback?x=1&code_verifier:abc",
        "GET /callback?x=1&assertion:abc",
        "postgresql+asyncpg://db.internal/game?sslcert=/home/alice/client.crt",
        "fetch https://cdn.example failed for bob@example.net",
        "scan-code=42",
        "https://cdn.example/maps",
        "bob@example.net",
        "mySQL\ncontinuation",
        "reDIS",
        "MongoDB",
        "sSLKey",
        "sSLCert",
        r"%255Cu0040",
        "%2525252525252540",
        r"\u212AEY",
        "%E2%84%AAEY",
        r"\u0025%33%61\/%2f",
        r"\u0025%37%34%4F%4B%45%4E",
        "value=%252525252525252520",
        "rendering resumed\nżółw 🐢",
        r"sample \u0061 and %20",
        r"sample \x61",
        r"sample \/",
        r"sample \Z",
        r"sample \7",
        "rendering RÉSUMED",
        "payload %aF\ncontinuation",
    ];
    let clean = [
        "rendering resumed\nnext frame",
        "progress: 20%",
        "value=%aG",
        r"path=\-",
    ];
    let placeholder = "[record redacted: possible credential]";
    let mut failures = Vec::new();

    for (record, expected) in sensitive
        .into_iter()
        .map(|record| (record, placeholder))
        .chain(clean.into_iter().map(|record| (record, record)))
    {
        let mut writer = sink.make_writer();
        writeln!(writer, "{record}")?;
        drop(writer);
        let mut logs =
            fs::read_dir(directory.path().join("logs"))?.collect::<Result<Vec<_>, _>>()?;
        logs.retain(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with("client-") && name.ends_with(".log")
        });
        let [log] = logs.as_slice() else {
            return Err("expected exactly one daily rolling log".into());
        };
        let path = log.path();
        if fs::read_to_string(&path)? != format!("{expected}\n") {
            failures.push(record);
        }
        fs::remove_file(path)?;
    }

    assert!(
        failures.is_empty(),
        "unexpected rolling log output for {failures:?}"
    );
    directory.close()?;
    Ok(())
}
