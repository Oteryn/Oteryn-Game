//! Fail-closed credential detection for complete diagnostic records.

pub(super) const PLACEHOLDER: &str = "[record redacted: possible credential]";

const TRIGGERS: &[&str] = &[
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
];

/// A log event or crash report is one record, including its continuation lines.
pub fn redact(record: &str) -> String {
    if contains_credential(record) {
        return PLACEHOLDER.to_owned();
    }

    // Check nested encodings on a copy. Fail closed if the existing decoding
    // limit is reached before all escape layers can be inspected.
    let mut remaining = record;
    let mut decoded;
    for _ in 0..8 {
        let Some(next) = decode_escapes(remaining) else {
            return record.to_owned();
        };
        if contains_credential(&next) {
            return PLACEHOLDER.to_owned();
        }
        decoded = next;
        remaining = &decoded;
    }
    if decode_escapes(remaining).is_some() {
        PLACEHOLDER.to_owned()
    } else {
        record.to_owned()
    }
}

fn contains_credential(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    TRIGGERS.iter().any(|trigger| lower.contains(trigger))
        || lower.match_indices("://").any(|(index, marker)| {
            let authority = lower[index + marker.len()..]
                .split('/')
                .next()
                .unwrap_or_default();
            authority.contains('@')
        })
        // Keep blocking connection strings and legacy OAuth/query assignments
        // that were already treated as sensitive.
        || lower.contains("postgres://")
        || lower.contains("postgresql://")
        || lower.match_indices(['=', ':']).any(|(index, delimiter)| {
            let key = lower[..index]
                .trim_end()
                .trim_end_matches(['"', '\''])
                .rsplit(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .next()
                .unwrap_or_default();
            key.ends_with("ticket")
                || (delimiter == "="
                    && matches!(key, "code" | "key" | "sig" | "signature" | "code_verifier" | "assertion"))
        })
}

fn escaped_ascii(bytes: &[u8]) -> Option<(u8, usize)> {
    let (digits, length) = match bytes {
        [b'\\', b'"' | b'\'' | b'/' | b'\\', ..] => return Some((bytes[1], 2)),
        [b'\\', b'b', ..] => return Some((8, 2)),
        [b'\\', b'f', ..] => return Some((12, 2)),
        [b'\\', b'n', ..] => return Some((b'\n', 2)),
        [b'\\', b'r', ..] => return Some((b'\r', 2)),
        [b'\\', b't', ..] => return Some((b'\t', 2)),
        [b'\\', b'u', a, b, c, d, ..] => ([*a, *b, *c, *d], 6),
        [b'%', a, b, ..] => ([b'0', b'0', *a, *b], 3),
        _ => return None,
    };
    if !digits.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    let decoded = u8::from_str_radix(std::str::from_utf8(&digits).ok()?, 16).ok()?;
    decoded.is_ascii().then_some((decoded, length))
}

fn decode_escapes(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = String::new();
    let mut copied = 0;
    let mut index = 0;
    while index < bytes.len() {
        if let Some((decoded, length)) = escaped_ascii(&bytes[index..]) {
            out.push_str(&text[copied..index]);
            out.push(char::from(decoded));
            index += length;
            copied = index;
        } else {
            index += 1;
        }
    }
    if copied == 0 {
        return None;
    }
    out.push_str(&text[copied..]);
    Some(out)
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
        // Outside a query string these words are ordinary text.
        assert_eq!(redact("exit code: 3"), "exit code: 3");
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
        assert_eq!(
            redact("plain https://cdn.example/a@b"),
            "plain https://cdn.example/a@b"
        );
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
