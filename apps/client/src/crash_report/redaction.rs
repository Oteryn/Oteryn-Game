//! The one redaction pass for every log line and crash text the client keeps.
//!
//! It removes bearer and authorization values, passwords, tokens, OAuth codes and other secret
//! key/value pairs, query-string secrets, URL user information and whole `postgres://` URLs.
//! It works on ASCII delimiters, so multi-byte text around a secret stays intact.

const MASK: &str = "[redacted]";

/// Key suffixes whose value is always secret, in `key=value`, `key: value` or JSON form.
const SECRET_SUFFIXES: [&str; 11] = [
    "token",
    "secret",
    "password",
    "passwd",
    "apikey",
    "api_key",
    "authorization",
    "cookie",
    "ticket",
    "credential",
    "credentials",
];
/// Keys that are secret in a query string (`?code=` / `&code=`) but ordinary words elsewhere.
const QUERY_SECRETS: [&str; 7] = [
    "code",
    "key",
    "sig",
    "signature",
    "session",
    "code_verifier",
    "assertion",
];
const VALUE_END: &[u8] = b" \t\r\n\"'&#,;)]}<>";

/// Returns `text` with every secret value replaced by `[redacted]`.
pub fn redact(text: &str) -> String {
    // Read structured values before URL masking can change their JSON syntax.
    let text = redact_pairs(text);
    let text = redact_urls(&text);
    redact_bearer(&text)
}

fn identifier(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

fn skip_horizontal_space(bytes: &[u8], mut start: usize) -> usize {
    while bytes
        .get(start)
        .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
    {
        start += 1;
    }
    start
}

fn skip_json_space(bytes: &[u8], mut start: usize) -> usize {
    while bytes
        .get(start)
        .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
    {
        start += 1;
    }
    start
}

fn line_end(bytes: &[u8], start: usize) -> usize {
    bytes[start..]
        .iter()
        .position(|byte| matches!(byte, b'\r' | b'\n'))
        .map_or(bytes.len(), |offset| start + offset)
}

fn quoted_end(bytes: &[u8], mut cursor: usize, quote: u8) -> usize {
    while let Some(&byte) = bytes.get(cursor) {
        match byte {
            byte if byte == quote => return cursor,
            b'\\' => cursor += 2,
            _ => cursor += 1,
        }
    }
    bytes.len()
}

fn value_end(bytes: &[u8], start: usize) -> usize {
    bytes[start..]
        .iter()
        .position(|byte| VALUE_END.contains(byte))
        .map_or(bytes.len(), |offset| start + offset)
}

/// `scheme://user:password@host` loses its user information; a PostgreSQL URL loses everything
/// after the scheme, because its path and options name the database and its credentials.
fn redact_urls(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    let mut search = 0;
    while let Some(found) = text[search..].find("://") {
        let marker = search + found;
        let scheme_start = bytes[..marker]
            .iter()
            .rposition(|byte| !(byte.is_ascii_alphanumeric() || b"+.-".contains(byte)))
            .map_or(0, |index| index + 1);
        let scheme = text[scheme_start..marker].to_ascii_lowercase();
        let authority = marker + 3;
        if text[authority..].starts_with(MASK) {
            search = authority + MASK.len();
            continue;
        }
        let terminates_url = |byte: &u8| byte.is_ascii_whitespace() || b"\"<>".contains(byte);
        if scheme.starts_with("postgres") {
            let end = bytes[authority..]
                .iter()
                .position(terminates_url)
                .map_or(bytes.len(), |offset| authority + offset);
            out.push_str(&text[copied..authority]);
            out.push_str(MASK);
            copied = end;
            search = end;
        } else {
            // URL user information permits RFC 3986 sub-delimiters, including commas.
            let host_end = bytes[authority..]
                .iter()
                .position(|byte| terminates_url(byte) || b"/?#".contains(byte))
                .map_or(bytes.len(), |offset| authority + offset);
            if let Some(at) = bytes[authority..host_end].iter().rposition(|b| *b == b'@') {
                out.push_str(&text[copied..authority]);
                out.push_str(MASK);
                copied = authority + at;
            }
            // Continue scanning the path and surrounding text for another URL scheme.
            search = authority;
        }
    }
    out.push_str(&text[copied..]);
    out
}

/// Decode a JSON member name so escaped characters participate in secret-key matching.
fn json_key(text: &str, start: usize) -> Option<(String, usize)> {
    let bytes = text.as_bytes();
    let end = quoted_end(bytes, start + 1, b'"');
    if bytes.get(skip_json_space(bytes, end + 1)) != Some(&b':') {
        return None;
    }
    let key: String = serde_json::from_str(&text[start..=end]).ok()?;
    Some((key.to_ascii_lowercase(), end))
}

fn json_container_end(text: &str, start: usize) -> Option<usize> {
    let mut values =
        serde_json::Deserializer::from_str(&text[start..]).into_iter::<serde::de::IgnoredAny>();
    values.next()?.ok()?;
    Some(start + values.byte_offset())
}

/// `token=...`, `"password": "..."`, `Authorization: Bearer ...` and `?code=...` values.
fn redact_pairs(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    let mut index = 0;
    while index < bytes.len() {
        let key_start = index;
        let key = if bytes[index] == b'"'
            && (index == 0 || bytes[index - 1] != b'\\')
            && let Some((key, end)) = json_key(text, index)
        {
            index = end;
            key
        } else {
            if !identifier(bytes[index]) || (index > 0 && identifier(bytes[index - 1])) {
                index += 1;
                continue;
            }
            while index < bytes.len() && identifier(bytes[index]) {
                index += 1;
            }
            text[key_start..index].to_ascii_lowercase()
        };
        let in_query = key_start > 0 && matches!(bytes[key_start - 1], b'?' | b'&');
        let secret = SECRET_SUFFIXES.iter().any(|suffix| key.ends_with(suffix))
            || (in_query && QUERY_SECRETS.contains(&key.as_str()));
        if !secret {
            index += usize::from(bytes[key_start] == b'"');
            continue;
        }
        let mut cursor = index;
        let quoted_key = bytes
            .get(cursor)
            .is_some_and(|byte| matches!(byte, b'"' | b'\''));
        if quoted_key {
            cursor += 1;
        }
        if quoted_key {
            cursor = skip_json_space(bytes, cursor);
        } else {
            cursor = skip_horizontal_space(bytes, cursor);
        }
        if cursor >= bytes.len() || !matches!(bytes[cursor], b'=' | b':') {
            continue;
        }
        let header = bytes[cursor] == b':' && !quoted_key;
        cursor = if quoted_key {
            skip_json_space(bytes, cursor + 1)
        } else {
            skip_horizontal_space(bytes, cursor + 1)
        };
        if quoted_key
            && matches!(bytes.get(cursor), Some(b'[' | b'{'))
            && let Some(end) = json_container_end(text, cursor)
        {
            out.push_str(&text[copied..cursor]);
            out.push('"');
            out.push_str(MASK);
            out.push('"');
            copied = end;
            index = end;
            continue;
        }
        let end = if header && key.ends_with("cookie") {
            line_end(bytes, cursor)
        } else if let Some(&quote @ (b'"' | b'\'')) = bytes.get(cursor) {
            cursor += 1;
            quoted_end(bytes, cursor, quote)
        } else {
            let end = value_end(bytes, cursor);
            let word = text[cursor..end].to_ascii_lowercase();
            if matches!(word.as_str(), "bearer" | "basic" | "token" | "digest")
                && bytes
                    .get(end)
                    .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
            {
                cursor = skip_horizontal_space(bytes, end);
                if header {
                    line_end(bytes, cursor)
                } else if text[cursor..].starts_with(MASK) {
                    cursor + MASK.len()
                } else {
                    value_end(bytes, cursor)
                }
            } else if text[cursor..].starts_with(MASK) {
                cursor + MASK.len()
            } else {
                end
            }
        };
        if end == cursor {
            index = end.max(index);
            continue;
        }
        out.push_str(&text[copied..cursor]);
        out.push_str(MASK);
        copied = end;
        index = end;
    }
    out.push_str(&text[copied..]);
    out
}

/// A bare `Bearer <credential>` anywhere in the text.
fn redact_bearer(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    let mut search = 0;
    while let Some(found) = lower[search..].find("bearer") {
        let start = search + found;
        let after_scheme = start + "bearer".len();
        let value = skip_horizontal_space(bytes, after_scheme);
        if value == after_scheme || (start > 0 && identifier(bytes[start - 1])) {
            search = after_scheme;
            continue;
        }
        if text[value..].starts_with(MASK) {
            search = value + MASK.len();
            continue;
        }
        let end = value_end(bytes, value);
        if end > value {
            out.push_str(&text[copied..value]);
            out.push_str(MASK);
            copied = end;
        }
        search = end.max(value);
    }
    out.push_str(&text[copied..]);
    out
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn bearer_and_authorization_header_values_are_removed() {
        assert_eq!(
            redact("Authorization: Bearer eyJhbGciOi.payload.sig"),
            "Authorization: Bearer [redacted]"
        );
        assert_eq!(
            redact("authorization=Basic dXNlcjpwYXNz next"),
            "authorization=Basic [redacted] next"
        );
        assert_eq!(
            redact("sent bearer abc123 to gateway"),
            "sent bearer [redacted] to gateway"
        );
    }

    #[test]
    fn tokens_and_passwords_are_removed_in_every_pair_form() {
        assert_eq!(
            redact(r#"{"access_token":"a1","refresh_token": "r2","name":"Bob"}"#),
            r#"{"access_token":"[redacted]","refresh_token": "[redacted]","name":"Bob"}"#
        );
        assert_eq!(
            redact("password=hunter2 user=bob"),
            "password=[redacted] user=bob"
        );
        assert_eq!(redact("client_secret: s3cr3t"), "client_secret: [redacted]");
        assert_eq!(
            redact("session_ticket=T, ok"),
            "session_ticket=[redacted], ok"
        );
    }

    #[test]
    fn oauth_codes_and_query_string_secrets_are_removed() {
        assert_eq!(
            redact("GET /callback?code=abc&state=xyz&key=k1#frag"),
            "GET /callback?code=[redacted]&state=xyz&key=[redacted]#frag"
        );
        assert_eq!(
            redact("https://login.example/authorize?client_id=c&code_verifier=v1"),
            "https://login.example/authorize?client_id=c&code_verifier=[redacted]"
        );
        // Outside a query string these words are ordinary text.
        assert_eq!(redact("exit code: 3"), "exit code: 3");
    }

    #[test]
    fn postgres_urls_and_url_userinfo_are_removed() {
        assert_eq!(
            redact("db postgres://oteryn:pw@db.internal:5432/game?sslmode=require failed"),
            "db postgres://[redacted] failed"
        );
        assert_eq!(
            redact("postgresql://localhost/game"),
            "postgresql://[redacted]"
        );
        assert_eq!(
            redact("fetch https://user:pass@cdn.example/a/b?x=1"),
            "fetch https://[redacted]@cdn.example/a/b?x=1"
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
        assert_eq!(redact("token=zażółć rest"), "token=[redacted] rest");
        assert_eq!(redact(""), "");
        assert_eq!(redact("token="), "token=");
    }

    #[test]
    fn redaction_is_idempotent() {
        let once = redact("Authorization: Bearer x password=y postgres://a:b@c/d ?code=e");
        assert_eq!(redact(&once), once);
    }
}
