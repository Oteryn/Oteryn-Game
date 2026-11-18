//! Lexical credential detection for complete diagnostic records.

pub(super) const PLACEHOLDER: &str = "[record redacted: possible credential]";

const TRIGGERS: &[&str] = &[
    "@",
    "://",
    "pass",
    "pwd",
    "secret",
    "token",
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
    let mut normalized = record.to_lowercase();
    if has_trigger(&normalized) {
        return PLACEHOLDER.to_owned();
    }

    // Inspect each layer because later unescaping can change earlier matches.
    for _ in 0..8 {
        let Some(decoded) = decode_escapes(&normalized) else {
            break;
        };
        normalized = decoded.to_lowercase();
        if has_trigger(&normalized) {
            return PLACEHOLDER.to_owned();
        }
    }
    record.to_owned()
}

fn has_trigger(text: &str) -> bool {
    TRIGGERS.iter().any(|trigger| text.contains(trigger))
}

fn hex(digits: &[u8]) -> Option<u32> {
    if !digits.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    u32::from_str_radix(std::str::from_utf8(digits).ok()?, 16).ok()
}

fn backslash_escape(bytes: &[u8]) -> Option<(char, usize)> {
    match bytes {
        [b'\\', b'"' | b'\'' | b'/' | b'\\', ..] => Some((char::from(bytes[1]), 2)),
        [b'\\', b'b', ..] => Some(('\u{8}', 2)),
        [b'\\', b'f', ..] => Some(('\u{c}', 2)),
        [b'\\', b'n', ..] => Some(('\n', 2)),
        [b'\\', b'r', ..] => Some(('\r', 2)),
        [b'\\', b't', ..] => Some(('\t', 2)),
        [b'\\', b'u', a, b, c, d, ..] => Some((char::from_u32(hex(&[*a, *b, *c, *d])?)?, 6)),
        _ => None,
    }
}

fn decode_escapes(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut copied = 0;
    let mut index = 0;
    while index < bytes.len() {
        let remaining = &bytes[index..];
        if let [b'%', a, b, ..] = remaining
            && let Some(decoded) = hex(&[*a, *b])
        {
            out.extend_from_slice(&bytes[copied..index]);
            out.push(decoded as u8);
            index += 3;
        } else if let Some((decoded, length)) = backslash_escape(remaining) {
            out.extend_from_slice(&bytes[copied..index]);
            out.extend_from_slice(decoded.encode_utf8(&mut [0; 4]).as_bytes());
            index += length;
        } else {
            index += 1;
            continue;
        }
        copied = index;
    }
    if copied == 0 {
        return None;
    }
    out.extend_from_slice(&bytes[copied..]);
    Some(String::from_utf8_lossy(&out).into_owned())
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
    fn ordinary_text_and_multibyte_text_are_unchanged() {
        let text = "Encountered an error in command `Despawn`: Entity despawned — żółw 🐢";
        assert_eq!(redact(text), text);
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
