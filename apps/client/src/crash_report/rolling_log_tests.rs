use super::{KEEP_FILES, RollingFile, Sink, TOTAL_CAP};
use bevy::log::tracing_subscriber::fmt::MakeWriter;
use std::{fs, io::Write, sync::Arc};

#[test]
fn rolling_log_applies_lexical_record_redaction() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::temp_dir().join(format!("oteryn-lexical-log-{}", std::process::id()));
    let sink = Sink {
        file: Some(Arc::new(RollingFile::new(
            directory.join("logs"),
            KEEP_FILES,
            TOTAL_CAP,
        ))),
        stderr: false,
        ring: false,
    };
    let sensitive = [
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
    ];
    let clean = [
        "value=%252525252525252520",
        "rendering resumed\nżółw 🐢",
        r"sample \u0061 and %20",
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
        let path = fs::read_dir(directory.join("logs"))?
            .next()
            .ok_or("missing rolling log")??
            .path();
        if fs::read_to_string(&path)? != format!("{expected}\n") {
            failures.push(record);
        }
        fs::remove_file(path)?;
    }

    fs::remove_dir_all(directory)?;
    assert!(
        failures.is_empty(),
        "unexpected rolling log output for {failures:?}"
    );
    Ok(())
}
