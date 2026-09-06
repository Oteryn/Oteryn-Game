//! Closed, bounded native evidence wire data. Nothing here authenticates a source
//! or constructs a Foundation owning/current capability.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidWire;
type Result<T> = std::result::Result<T, InvalidWire>;

/// V1 purpose/scope values must come from independent configuration, never a response.
#[derive(Debug, Clone, Copy)]
pub enum Request<'a> {
    Account {
        recovery: bool,
        account_id: &'a str,
        purpose: &'a str,
        scope: &'a str,
    },
    Trust {
        recovery: bool,
        key_id: &'a str,
        key_purpose: &'a str,
    },
}
impl Request<'_> {
    fn recovery(&self) -> bool {
        match self {
            Self::Account { recovery, .. } | Self::Trust { recovery, .. } => *recovery,
        }
    }
    fn operation(&self) -> &'static str {
        match self {
            Self::Account {
                recovery: false, ..
            } => "ReadAccountSecurityV1",
            Self::Account { recovery: true, .. } => "ReadRecoveryAccountSecurityV2",
            Self::Trust {
                recovery: false, ..
            } => "ReadFreshSigningTrustV1",
            Self::Trust { recovery: true, .. } => "ReadRecoverySigningTrustV2",
        }
    }
    fn bindings(&self) -> Result<[Option<(&'static str, &str)>; 4]> {
        match self {
            Self::Account {
                recovery,
                account_id,
                purpose,
                scope,
            } => {
                if !uuid(account_id)
                    || !binding(purpose)
                    || !binding(scope)
                    || (*recovery
                        && (*purpose != "platform_security" || *scope != "existing_actor_recovery"))
                {
                    return Err(InvalidWire);
                }
                Ok([
                    Some(("account_id", account_id)),
                    Some(("purpose", purpose)),
                    Some(("scope", scope)),
                    None,
                ])
            }
            Self::Trust {
                recovery,
                key_id,
                key_purpose,
            } => {
                if key_id.is_empty()
                    || key_id.len() > 64
                    || !key_id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                    || !binding(key_purpose)
                    || (*recovery && *key_purpose != "existing_actor_recovery")
                {
                    return Err(InvalidWire);
                }
                let (issuer, profile) = if *recovery {
                    (
                        "urn:oteryn:platform:game-recovery",
                        "oteryn-reauth-recovery-v1",
                    )
                } else {
                    (
                        "urn:oteryn:platform:game-admission",
                        "oteryn-pre-admission-v1",
                    )
                };
                Ok([
                    Some(("issuer", issuer)),
                    Some(("profile", profile)),
                    Some(("key_purpose", key_purpose)),
                    Some(("key_id", key_id)),
                ])
            }
        }
    }
}
fn binding(s: &str) -> bool {
    !s.is_empty() && s.len() <= 256
}
fn authority(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}
fn uuid(s: &str) -> bool {
    s.len() == 36
        && s.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
        && s.as_bytes()[14] == b'7'
        && b"89ab".contains(&s.as_bytes()[19])
}

/// Fixed storage avoids peer-sized heap allocations, including escaped JSON strings.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Text {
    bytes: [u8; 256],
    len: usize,
}
impl std::fmt::Debug for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
impl Text {
    const EMPTY: Self = Self {
        bytes: [0; 256],
        len: 0,
    };
    #[allow(
        clippy::expect_used,
        reason = "Private fixed buffer is exposed only after UTF-8 validation"
    )]
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).expect("parser validates UTF-8")
    }
    fn push(&mut self, bytes: &[u8], max: usize) -> Result<()> {
        let end = self.len.checked_add(bytes.len()).ok_or(InvalidWire)?;
        if end > max {
            return Err(InvalidWire);
        }
        self.bytes[self.len..end].copy_from_slice(bytes);
        self.len = end;
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    NotFound,
    Unavailable,
    Unauthorized,
    Unsupported,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facts {
    Account {
        allowed: bool,
        minimum_valid_generation: u64,
    },
    Trust {
        trusted: bool,
        public_key: [u8; 32],
    },
}
/// Historical wire facts only. Expected bindings are validated, never granted authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Observation {
    pub source_authority: Text,
    pub source_revision: u64,
    pub decision_identity: Text,
    pub source_observed_at: i64,
    pub clock_uncertainty_seconds: u64,
    pub facts: Facts,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    clippy::large_enum_variant,
    reason = "Fixed inline storage intentionally prevents hostile-input heap allocation"
)]
pub enum Response {
    Observed(Observation),
    Failure(Failure),
}

struct Output {
    bytes: [u8; 1024],
    len: usize,
}
impl Output {
    fn add(&mut self, s: &[u8]) -> Result<()> {
        let end = self.len.checked_add(s.len()).ok_or(InvalidWire)?;
        if end > 1024 {
            return Err(InvalidWire);
        }
        self.bytes[self.len..end].copy_from_slice(s);
        self.len = end;
        Ok(())
    }
    fn string(&mut self, s: &str) -> Result<()> {
        self.add(b"\"")?;
        for ch in s.chars() {
            match ch {
                '"' => self.add(b"\\\"")?,
                '\\' => self.add(b"\\\\")?,
                c if (c as u32) < 32 => {
                    let n = c as u8;
                    let h = b"0123456789abcdef";
                    self.add(&[
                        b'\\',
                        b'u',
                        b'0',
                        b'0',
                        h[usize::from(n >> 4)],
                        h[usize::from(n & 15)],
                    ])?
                }
                c => {
                    let mut buf = [0; 4];
                    self.add(c.encode_utf8(&mut buf).as_bytes())?
                }
            }
        }
        self.add(b"\"")
    }
}
pub fn encode_request(request: &Request<'_>) -> Result<String> {
    let fields = request.bindings()?;
    let mut out = Output {
        bytes: [0; 1024],
        len: 0,
    };
    out.add(if request.recovery() {
        b"{\"version\":2,\"operation\":"
    } else {
        b"{\"version\":1,\"operation\":"
    })?;
    out.string(request.operation())?;
    for (key, value) in fields.into_iter().flatten() {
        out.add(b",")?;
        out.string(key)?;
        out.add(b":")?;
        out.string(value)?;
    }
    out.add(b"}")?;
    String::from_utf8(out.bytes[..out.len].to_vec()).map_err(|_| InvalidWire)
}

const NAMES: [&str; 19] = [
    "version",
    "operation",
    "result",
    "source_authority",
    "source_revision",
    "decision_identity",
    "source_observed_at",
    "clock_uncertainty_seconds",
    "account_id",
    "purpose",
    "scope",
    "allowed",
    "minimum_valid_generation",
    "issuer",
    "profile",
    "key_purpose",
    "key_id",
    "trusted",
    "public_key",
];
#[derive(Clone, Copy)]
#[allow(
    clippy::large_enum_variant,
    reason = "Fixed parser slots intentionally avoid per-value heap allocation"
)]
enum Value {
    Empty,
    Text(Text),
    Bool(bool),
    Version(u8),
}
struct Parser<'a> {
    raw: &'a [u8],
    pos: usize,
}
impl Parser<'_> {
    fn ws(&mut self) {
        while self
            .raw
            .get(self.pos)
            .is_some_and(|b| b" \n\r\t".contains(b))
        {
            self.pos += 1;
        }
    }
    fn byte(&mut self, b: u8) -> Result<()> {
        self.ws();
        if self.raw.get(self.pos) != Some(&b) {
            return Err(InvalidWire);
        }
        self.pos += 1;
        Ok(())
    }
    fn hex(&mut self) -> Result<u16> {
        let mut n = 0u16;
        for _ in 0..4 {
            let b = *self.raw.get(self.pos).ok_or(InvalidWire)?;
            self.pos += 1;
            let d = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => return Err(InvalidWire),
            };
            n = n * 16 + u16::from(d);
        }
        Ok(n)
    }
    fn text(&mut self, max: usize) -> Result<Text> {
        self.byte(b'"')?;
        let mut out = Text::EMPTY;
        loop {
            let b = *self.raw.get(self.pos).ok_or(InvalidWire)?;
            self.pos += 1;
            match b {
                b'"' => break,
                b'\\' => {
                    let e = *self.raw.get(self.pos).ok_or(InvalidWire)?;
                    self.pos += 1;
                    match e {
                        b'"' | b'\\' | b'/' => out.push(&[e], max)?,
                        b'b' => out.push(&[8], max)?,
                        b'f' => out.push(&[12], max)?,
                        b'n' => out.push(b"\n", max)?,
                        b'r' => out.push(b"\r", max)?,
                        b't' => out.push(b"\t", max)?,
                        b'u' => {
                            let first = self.hex()?;
                            let code = if (0xd800..=0xdbff).contains(&first) {
                                if self.raw.get(self.pos..self.pos + 2) != Some(b"\\u") {
                                    return Err(InvalidWire);
                                }
                                self.pos += 2;
                                let second = self.hex()?;
                                if !(0xdc00..=0xdfff).contains(&second) {
                                    return Err(InvalidWire);
                                }
                                0x10000
                                    + ((u32::from(first) - 0xd800) << 10)
                                    + (u32::from(second) - 0xdc00)
                            } else {
                                u32::from(first)
                            };
                            let ch = char::from_u32(code).ok_or(InvalidWire)?;
                            let mut buf = [0; 4];
                            out.push(ch.encode_utf8(&mut buf).as_bytes(), max)?
                        }
                        _ => return Err(InvalidWire),
                    }
                }
                0..=31 => return Err(InvalidWire),
                _ => out.push(&[b], max)?,
            }
        }
        std::str::from_utf8(&out.bytes[..out.len]).map_err(|_| InvalidWire)?;
        Ok(out)
    }
    fn literal(&mut self, s: &[u8]) -> Result<()> {
        if self.raw.get(self.pos..self.pos + s.len()) != Some(s) {
            return Err(InvalidWire);
        }
        self.pos += s.len();
        Ok(())
    }
}
fn number(s: &str, positive: bool) -> Result<u64> {
    if s.is_empty()
        || s.len() > 20
        || !s.bytes().all(|b| b.is_ascii_digit())
        || (s.len() > 1 && s.starts_with('0'))
    {
        return Err(InvalidWire);
    }
    let n = s.parse::<u64>().map_err(|_| InvalidWire)?;
    if positive && n == 0 {
        return Err(InvalidWire);
    }
    Ok(n)
}
fn text(values: &[Value; 19], i: usize) -> Result<Text> {
    match values[i] {
        Value::Text(s) => Ok(s),
        _ => Err(InvalidWire),
    }
}
fn boolean(values: &[Value; 19], i: usize) -> Result<bool> {
    match values[i] {
        Value::Bool(b) => Ok(b),
        _ => Err(InvalidWire),
    }
}

pub fn decode_response(
    request: &Request<'_>,
    expected_source: &str,
    raw: &[u8],
) -> Result<Response> {
    if raw.len() > 8192 || !authority(expected_source) {
        return Err(InvalidWire);
    }
    let bindings = request.bindings()?;
    let mut parser = Parser { raw, pos: 0 };
    let mut values = [Value::Empty; 19];
    let mut count = 0;
    parser.byte(b'{')?;
    loop {
        if count > 0 {
            parser.byte(b',')?;
        }
        if count == 16 {
            return Err(InvalidWire);
        }
        let key = parser.text(64)?;
        if !key.as_str().is_ascii() {
            return Err(InvalidWire);
        }
        let i = NAMES[..19]
            .iter()
            .position(|n| *n == key.as_str())
            .ok_or(InvalidWire)?;
        if !matches!(values[i], Value::Empty) {
            return Err(InvalidWire);
        }
        parser.byte(b':')?;
        parser.ws();
        values[i] = match parser.raw.get(parser.pos) {
            Some(b'"') => Value::Text(parser.text(256)?),
            Some(b't') => {
                parser.literal(b"true")?;
                Value::Bool(true)
            }
            Some(b'f') => {
                parser.literal(b"false")?;
                Value::Bool(false)
            }
            Some(b'1' | b'2') if i == 0 => {
                let n = parser.raw[parser.pos] - b'0';
                parser.pos += 1;
                Value::Version(n)
            }
            _ => return Err(InvalidWire),
        };
        count += 1;
        parser.ws();
        if parser.raw.get(parser.pos) == Some(&b'}') {
            parser.pos += 1;
            break;
        }
    }
    parser.ws();
    if parser.pos != raw.len() {
        return Err(InvalidWire);
    }
    if !matches!(values[0],Value::Version(v) if v==if request.recovery(){2}else{1})
        || text(&values, 1)?.as_str() != request.operation()
    {
        return Err(InvalidWire);
    }
    let result = text(&values, 2)?;
    if result.as_str() != "observed" {
        if count != 3 {
            return Err(InvalidWire);
        }
        return Ok(Response::Failure(match result.as_str() {
            "not_found" => Failure::NotFound,
            "unavailable" => Failure::Unavailable,
            "unauthorized" => Failure::Unauthorized,
            "unsupported" => Failure::Unsupported,
            _ => return Err(InvalidWire),
        }));
    }
    let mut mask = 0xffu32;
    for (name, expected) in bindings.into_iter().flatten() {
        let i = NAMES.iter().position(|s| *s == name).ok_or(InvalidWire)?;
        if text(&values, i)?.as_str() != expected {
            return Err(InvalidWire);
        }
        mask |= 1 << i;
    }
    let facts = match request {
        Request::Account { .. } => {
            mask |= (1 << 11) | (1 << 12);
            Facts::Account {
                allowed: boolean(&values, 11)?,
                minimum_valid_generation: number(text(&values, 12)?.as_str(), true)?,
            }
        }
        Request::Trust { .. } => {
            mask |= (1 << 17) | (1 << 18);
            let encoded = text(&values, 18)?;
            if encoded.len != 43 {
                return Err(InvalidWire);
            }
            let mut key = [0; 32];
            let written = URL_SAFE_NO_PAD
                .decode_slice(encoded.as_str(), &mut key)
                .map_err(|_| InvalidWire)?;
            if written != 32 {
                return Err(InvalidWire);
            }
            Facts::Trust {
                trusted: boolean(&values, 17)?,
                public_key: key,
            }
        }
    };
    for (i, value) in values.iter().enumerate() {
        if matches!(value, Value::Empty) == (mask & (1 << i) != 0) {
            return Err(InvalidWire);
        }
    }
    let source = text(&values, 3)?;
    if source.as_str() != expected_source {
        return Err(InvalidWire);
    }
    let revision = text(&values, 4)?;
    let source_revision = number(revision.as_str(), true)?;
    let decision_identity = text(&values, 5)?;
    if decision_identity != revision {
        return Err(InvalidWire);
    }
    let timestamp = text(&values, 6)?;
    if timestamp.len > 19 {
        return Err(InvalidWire);
    }
    let source_observed_at =
        i64::try_from(number(timestamp.as_str(), false)?).map_err(|_| InvalidWire)?;
    Ok(Response::Observed(Observation {
        source_authority: source,
        source_revision,
        decision_identity,
        source_observed_at,
        clock_uncertainty_seconds: number(text(&values, 7)?.as_str(), false)?,
        facts,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_buffers_reject_before_growth() {
        let mut output = Output {
            bytes: [0; 1024],
            len: 0,
        };
        assert!(output.add(&[b'x'; 1024]).is_ok());
        assert_eq!(output.add(b"x"), Err(InvalidWire));
        assert_eq!(output.len, 1024);
        let mut text = Text::EMPTY;
        assert!(text.push(&[b'x'; 256], 256).is_ok());
        assert_eq!(text.push(b"x", 256), Err(InvalidWire));
        assert_eq!(text.len, 256);
        let mut key = Text::EMPTY;
        assert!(key.push(&[b'x'; 64], 64).is_ok());
        assert_eq!(key.push(b"x", 64), Err(InvalidWire));
        assert_eq!(key.len, 64);
    }
}
