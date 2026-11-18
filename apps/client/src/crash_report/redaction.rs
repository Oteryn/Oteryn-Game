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
/// Query keys whose assignments can carry secrets.
const QUERY_SECRETS: [&str; 7] = [
    "code",
    "key",
    "sig",
    "signature",
    "session",
    "code_verifier",
    "assertion",
];
const VALUE_END: &[u8] = b" \t\r\n\x0b\x0c\"'&#,;)]}<>";

/// Returns `text` with every secret value replaced by `[redacted]`.
pub fn redact(text: &str) -> String {
    // An encoded authority can otherwise disappear into a key/value match.
    let text = redact_encoded(redact_urls(text), redact_urls);
    let text = redact_pairs(&text);
    redact_encoded(redact_bearer(&text), redact_plain)
}

fn redact_encoded(text: String, redact: fn(&str) -> String) -> String {
    if !text.contains(['%', '\\']) {
        return text;
    }
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        if line.contains(['%', '\\']) && encoded_secret(line, redact) {
            out.push_str(MASK);
            if line.ends_with('\n') {
                out.push('\n');
            }
        } else {
            out.push_str(line);
        }
    }
    out
}

fn encoded_secret(line: &str, redact: fn(&str) -> String) -> bool {
    let mut next = decode_escapes(line);
    // Inspect the decoded form without emitting it. If the nesting is too
    // deep to inspect, remove the entire line.
    for _ in 0..8 {
        let Some(decoded) = next else {
            return false;
        };
        if redact(&decoded) != decoded {
            return true;
        }
        next = decode_escapes(&decoded);
    }
    next.is_some()
}

fn redact_plain(text: &str) -> String {
    let text = redact_urls(text);
    let text = redact_pairs(&text);
    redact_bearer(&text)
}

fn escaped_ascii(bytes: &[u8]) -> Option<(u8, usize)> {
    let (digits, length) = match bytes {
        [b'\\', b'"' | b'\'' | b'/' | b'\\', ..] => return Some((bytes[1], 2)),
        [b'\\', b'n', ..] => return Some((b'\n', 2)),
        [b'\\', b'r', ..] => return Some((b'\r', 2)),
        [b'\\', b't', ..] => return Some((b'\t', 2)),
        [b'\\', b'u', a, b, c, d, ..] => ([*a, *b, *c, *d], 6),
        [b'%', a, b, ..] => ([b'0', b'0', *a, *b], 3),
        _ => return None,
    };
    let decoded = u8::from_str_radix(std::str::from_utf8(&digits).ok()?, 16).ok()?;
    decoded.is_ascii().then_some((decoded, length))
}

fn decode_escapes(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
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

fn identifier(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

fn skip_space(bytes: &[u8], mut start: usize) -> usize {
    while bytes
        .get(start)
        .is_some_and(|byte| b" \t\r\n\x0b\x0c".contains(byte))
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
            let mut end = authority;
            while let Some(byte) = bytes.get(end).filter(|byte| !terminates_url(byte)) {
                end += if *byte == b'\\' && matches!(bytes.get(end + 1), Some(b'"' | b'\\')) {
                    2
                } else {
                    1
                };
            }
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
            search = authority.max(copied);
        }
    }
    out.push_str(&text[copied..]);
    out
}

/// Decode a JSON member name so escaped characters participate in secret-key matching.
fn json_key(text: &str, start: usize) -> Option<(String, usize)> {
    let bytes = text.as_bytes();
    let end = quoted_end(bytes, start + 1, b'"');
    if bytes.get(skip_space(bytes, end + 1)) != Some(&b':') {
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
        let query_key = QUERY_SECRETS.contains(&key.as_str());
        let secret = SECRET_SUFFIXES.iter().any(|suffix| key.ends_with(suffix));
        let in_query = key_start > 0 && matches!(bytes[key_start - 1], b'?' | b'&');
        if !secret && !query_key {
            index += usize::from(bytes[key_start] == b'"');
            continue;
        }
        let mut cursor = index;
        let quoted_key = bytes
            .get(cursor)
            .is_some_and(|byte| matches!(byte, b'"' | b'\''));
        cursor = skip_space(bytes, cursor + usize::from(quoted_key));
        if cursor >= bytes.len() || !matches!(bytes[cursor], b'=' | b':') {
            continue;
        }
        if !secret && bytes[cursor] != b'=' && !in_query {
            continue;
        }
        let header = bytes[cursor] == b':' && !quoted_key;
        cursor = skip_space(bytes, cursor + 1);
        while matches!(bytes.get(cursor), Some(b'=' | b':')) {
            cursor = skip_space(bytes, cursor + 1);
        }
        if text[cursor..].starts_with(MASK) {
            index = cursor + MASK.len();
            continue;
        }
        if matches!(bytes.get(cursor), Some(b'[' | b'{')) {
            let end = json_container_end(text, cursor).unwrap_or(bytes.len());
            out.push_str(&text[copied..cursor]);
            out.push('"');
            out.push_str(MASK);
            out.push('"');
            copied = end;
            index = end;
            continue;
        }
        let (cursor, end) = if header && key.ends_with("cookie") {
            (cursor, line_end(bytes, cursor))
        } else {
            let word_end = value_end(bytes, cursor);
            let word = text[cursor..word_end].to_ascii_lowercase();
            let next = skip_space(bytes, word_end);
            if matches!(word.as_str(), "bearer" | "basic" | "token" | "digest") && next > word_end {
                if header {
                    (next, line_end(bytes, next))
                } else {
                    credential_span(text, next)
                }
            } else {
                credential_span(text, cursor)
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

fn credential_span(text: &str, start: usize) -> (usize, usize) {
    let bytes = text.as_bytes();
    if text[start..].starts_with(MASK) {
        (start, start + MASK.len())
    } else if let Some(&quote @ (b'"' | b'\'')) = bytes.get(start) {
        (start + 1, quoted_end(bytes, start + 1, quote))
    } else if escaped_ascii(&bytes[start..]).is_some() {
        // Encoded delimiters can wrap a credential with raw spaces.
        (start, line_end(bytes, start))
    } else {
        (start, value_end(bytes, start))
    }
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
        let value = skip_space(bytes, after_scheme);
        if value == after_scheme || (start > 0 && identifier(bytes[start - 1])) {
            search = after_scheme;
            continue;
        }
        if text[value..].starts_with(MASK) {
            search = value + MASK.len();
            continue;
        }
        let (value, end) = credential_span(text, value);
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
