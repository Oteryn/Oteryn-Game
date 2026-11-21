//! Lexical credential detection for complete diagnostic records.

pub(super) const PLACEHOLDER: &str = "[record redacted: possible credential]";

const TRIGGERS: &[&str] = &[
    "@",
    "://",
    "pass",
    "pwd",
    "secret",
    "token",
    "ticket",
    "auth",
    "bearer",
    "negotiate",
    "basic ",
    "cookie",
    "session",
    "credential",
    "apikey",
    "api_key",
    "api-key",
    "private",
    "otp",
    "code",
    "key",
    "sig",
    "assertion",
    "postgres",
    "mysql",
    "redis",
    "mongodb",
    "sslcert",
    "sslkey",
];

/// A log event or crash report is one record, including its continuation lines.
pub fn redact(record: &str) -> String {
    if !record.is_ascii() || has_encoding(record.as_bytes()) {
        return PLACEHOLDER.to_owned();
    }

    let lowercase = record.to_ascii_lowercase();
    if TRIGGERS.iter().any(|trigger| lowercase.contains(trigger)) {
        return PLACEHOLDER.to_owned();
    }
    record.to_owned()
}

fn has_encoding(bytes: &[u8]) -> bool {
    bytes.windows(3).any(
        |window| matches!(window, [b'%', a, b] if a.is_ascii_hexdigit() && b.is_ascii_hexdigit()),
    ) || bytes.windows(2).any(
        |window| matches!(window, [b'\\', next] if *next == b'/' || next.is_ascii_alphanumeric()),
    )
}

#[cfg(test)]
mod tests {
    use super::{PLACEHOLDER, redact};

    #[test]
    fn bearer_and_authorization_header_values_are_removed() {
        assert_eq!(
            redact("Authorization: Bearer eyJhbGciOi.payload.sig"),
            PLACEHOLDER
        );
        assert_eq!(redact("authorization=Basic dXNlcjpwYXNz next"), PLACEHOLDER);
        assert_eq!(redact("sent bearer abc123 to gateway"), PLACEHOLDER);
    }

    #[test]
    fn tokens_and_passwords_are_removed_in_every_pair_form() {
        assert_eq!(
            redact(r#"{"access_token":"a1","refresh_token": "r2","name":"Bob"}"#),
            PLACEHOLDER
        );
        assert_eq!(redact("password=hunter2 user=bob"), PLACEHOLDER);
        assert_eq!(redact("client_secret: s3cr3t"), PLACEHOLDER);
        assert_eq!(redact("session_ticket=T, ok"), PLACEHOLDER);
    }

    #[test]
    fn oauth_codes_and_query_string_secrets_are_removed() {
        assert_eq!(
            redact("GET /callback?code=abc&state=xyz&key=k1#frag"),
            PLACEHOLDER
        );
        assert_eq!(
            redact("https://login.example/authorize?client_id=c&code_verifier=v1"),
            PLACEHOLDER
        );
        assert_eq!(redact("exit code: 3"), PLACEHOLDER);
    }

    #[test]
    fn postgres_urls_and_url_userinfo_are_removed() {
        assert_eq!(
            redact("db postgres://oteryn:pw@db.internal:5432/game?sslmode=require failed"),
            PLACEHOLDER
        );
        assert_eq!(redact("postgresql://localhost/game"), PLACEHOLDER);
        assert_eq!(
            redact("fetch https://user:pass@cdn.example/a/b?x=1"),
            PLACEHOLDER
        );
        assert_eq!(redact("plain https://cdn.example/a@b"), PLACEHOLDER);
    }

    #[test]
    fn ordinary_ascii_text_is_preserved_and_multibyte_text_is_redacted() {
        let text = "Encountered an error in command `Despawn`: Entity despawned";
        assert_eq!(redact(text), text);
        assert_eq!(redact(&format!("{text} — żółw 🐢")), PLACEHOLDER);
        assert_eq!(redact("token=zażółć rest"), PLACEHOLDER);
        assert_eq!(redact(""), "");
        assert_eq!(redact("token="), PLACEHOLDER);
    }

    #[test]
    fn redaction_is_idempotent() {
        let once = redact("Authorization: Bearer x password=y postgres://a:b@c/d ?code=e");
        assert_eq!(redact(&once), once);
    }
}
