use super::*;

#[cfg(test)]
#[test]
fn review_crash_redacts_complete_credentials() {
    let text = crash_text(
        &Crash {
            thread: "test",
            location: "test",
            message: r#"{"password":"p4-first p4-second\" p4-third"} https://user:comma,secret@example.test/path"#,
            backtrace: "Cookie: a=first_cookie; b=second_cookie\nnext line",
        },
        SystemTime::now(),
    );
    for secret in [
        "p4-first",
        "p4-second",
        "p4-third",
        "first_cookie",
        "second_cookie",
        "comma,secret",
    ] {
        assert!(!text.contains(secret), "{secret} escaped redaction");
    }
    assert_eq!(text, PLACEHOLDER);
}

#[cfg(test)]
#[test]
fn review_crash_redacts_adjacent_urls_and_multiline_json() {
    let cases = [
        "https://one.test/,https://user:adjacent-secret@two.test/",
        "{\"password\"\n:\r\n\t\"json-secret\"}",
    ];
    for message in cases {
        assert_eq!(review_crash_text(message), PLACEHOLDER);
    }
}

#[cfg(test)]
#[test]
fn review_crash_redacts_structured_and_spaced_credentials() {
    let cases = [
        r#"{"pass\u0077ord":"example-one"}"#,
        r#"{"credentials":["example-two",{"nested":"another-secret"}],"name":"Bob"}"#,
        r#"{"credentials":{"url":"postgres://example.test/a\"value","value":"secret"}}"#,
        "Bearer  example-three",
        "Bearer\texample-four",
    ];
    for message in cases {
        assert_eq!(review_crash_text(message), PLACEHOLDER);
    }
}

#[cfg(test)]
const EARLIER_REVIEW_CASES: [(&str, &[&str]); 23] = [
    (
        "https://alice:hunter2;token=abc@example.test/map",
        &["alice", "hunter2", "abc"],
    ),
    (
        "HTTPS://user:private%3Bvalue;;ToKeN=private2@example.test",
        &["private", "user:"],
    ),
    (
        "https://user:comma,secret@example.test/path",
        &["comma,secret"],
    ),
    (
        r#"{"message":"{\"password\":\"example-one\"}"}"#,
        &["example-one"],
    ),
    (
        r#"{"message":"{\"PaSs\\u0077oRd\": \"nested\\\" secret\"}"}"#,
        &[r#"nested\""#, "nested\" secret", " secret\""],
    ),
    (r#"{"credentials":["example-two""#, &["example-two"]),
    (
        r#"{"CrEdEnTiAlS":["one-secret", {"nested":"two-secret"}"#,
        &["one-secret", "two-secret"],
    ),
    (r#"{"pass\u0077ord":"unicode-secret"}"#, &["unicode-secret"]),
    (
        r#"{"credentials":["array-secret",{"nested":"nested-secret"}]}"#,
        &["array-secret", "nested-secret"],
    ),
    (
        "Bearer  space-secret Bearer\ttab-secret",
        &["space-secret", "tab-secret"],
    ),
    (
        "https://example.test/?ok=1;;CoDe=query-secret&&%74oKeN=encoded-secret",
        &["query-secret", "encoded-secret"],
    ),
    (
        "https://example.test/?ok=1, SiG=comma-secret",
        &["comma-secret"],
    ),
    (
        r#"{"message":"https:\/\/user:escaped-secret@example.test"}"#,
        &["escaped-secret"],
    ),
    (
        "https://hidden-user:hidden-password;token=abc%40host.test",
        &["hidden-user", "hidden-password"],
    ),
    (
        "https://one.test/,https://user:adjacent-secret@two.test/",
        &["adjacent-secret"],
    ),
    (
        "{\"password\"\n:\r\n\t\"multiline-secret\"}",
        &["multiline-secret"],
    ),
    (
        r#"{"password":"first-secret second-secret\" third-secret"}"#,
        &["first-secret", "second-secret", "third-secret"],
    ),
    (
        "Cookie: a=first_cookie; b=second_cookie",
        &["first_cookie", "second_cookie"],
    ),
    (
        r#"{"credentials":{"url":"postgres://example.test/a\"value","value":"structured-secret"}}"#,
        &["structured-secret", "example.test"],
    ),
    (
        "pAsSwOrD == quoted-secret; &ToKeN = 'another secret'",
        &["quoted-secret", "another secret"],
    ),
    (
        r#"authorization=Basic "quoted auth secret""#,
        &["quoted auth secret"],
    ),
    ("tOkEn\r\n=\n \twhitespace-secret", &["whitespace-secret"]),
    (
        "Bearer %2522encoded leading-quote-secret",
        &["encoded", "leading-quote-secret"],
    ),
];

#[cfg(test)]
#[test]
fn review_crash_redacts_credentials_across_formats() {
    let results = EARLIER_REVIEW_CASES.map(|(message, secrets)| {
        let text = crash_text(
            &Crash {
                message,
                location: "test",
                thread: "test",
                backtrace: "",
            },
            SystemTime::now(),
        );
        secrets.iter().all(|secret| !text.contains(secret))
    });
    assert_eq!(results, [true; 23]);
}

#[cfg(test)]
#[test]
fn review_redacts_whole_records() {
    let cases = [
        (
            "encoded URL and multiline JSON",
            "{\"url\":\"https:\\/\\/alice:pw@host\",\"password\":\n\"example-one\"}",
            "example-one",
        ),
        (
            "encoded key and continuation",
            "%70assword=\nexample-one",
            "example-one",
        ),
        (
            "apostrophe in URL query",
            "https://host/?token=first'last",
            "'last",
        ),
        (
            "literal mask prefix",
            "password=[redacted]example-two",
            "example-two",
        ),
        (
            "Negotiate header",
            "Authorization: Negotiate dGVzdA==",
            "dGVzdA==",
        ),
    ];
    let placeholder = "[record redacted: possible credential]";
    let mut failures = Vec::new();
    for (label, message, leaked) in cases {
        let text = review_crash_text(message);
        if text != placeholder {
            failures.push((label, text.contains(leaked)));
        }
    }
    assert!(failures.is_empty(), "(case, leaked value): {failures:?}");

    for (message, _) in EARLIER_REVIEW_CASES {
        assert_eq!(review_crash_text(message), placeholder, "{message:?}");
    }
    for (message, expected) in [
        ("rendering resumed\nżółw 🐢", placeholder),
        (r#"sample \u0061 and %20"#, placeholder),
        ("https://host/maps/a@b", placeholder),
        ("https://alice:pw@host", placeholder),
        ("https://alice:pw%40host", placeholder),
        (r#"https:\/\/alice:pw@host"#, placeholder),
        (r#"%68ttps:\u002f%2falice:pw%40host"#, placeholder),
        (r#"\u0070%61\u0073%73 = value"#, placeholder),
    ] {
        assert_eq!(redact(message), expected, "{message:?}");
    }
    for trigger in [
        "PASS",
        "pWd",
        "SECRET",
        "Token",
        "TiCkEt",
        "AuTh",
        "BEARER",
        "Negotiate",
        "BaSiC ",
        "COOKIE",
        "Session",
        "CREDENTIAL",
        "ApiKey",
        "API_KEY",
        "api-KEY",
        "PRIVATE",
        "OTP",
    ] {
        let message = format!("before {trigger}after\ncontinuation");
        assert_eq!(redact(&message), placeholder, "{trigger:?}");
    }
}

#[cfg(test)]
fn review_crash_text(message: &str) -> String {
    crash_text(
        &Crash {
            message,
            location: "test",
            thread: "test",
            backtrace: "",
        },
        UNIX_EPOCH,
    )
}
