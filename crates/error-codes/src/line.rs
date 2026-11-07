//! The §1.5 diagnostic line: rendering, `detail` escaping and the field-order parser.

use crate::ErrorCode;

/// Longest escaped `detail` value in bytes.
const DETAIL_MAX: usize = 512;
const ELLIPSIS: &str = "...";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
}

impl Level {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "error" => Self::Error,
            "warn" => Self::Warn,
            "info" => Self::Info,
            "debug" => Self::Debug,
            _ => return None,
        })
    }
}

/// Unix milliseconds of the wall clock; `0` before the epoch.
#[must_use]
pub fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| u64::try_from(elapsed.as_millis()).ok())
        .unwrap_or(0)
}

/// One diagnostic line. Every field but `detail` is a token: anything outside
/// `[A-Za-z0-9._:/+-]` is replaced by `_`, so no value holds a space, a quote or a control
/// character. No player-linked identifier belongs in any field, `detail` included.
#[derive(Debug, Clone)]
pub struct Line<'a> {
    pub level: Level,
    pub module: &'a str,
    pub event: &'a str,
    pub build: Option<&'a str>,
    pub code: Option<ErrorCode>,
    pub wire: Option<u32>,
    pub trace: Option<&'a str>,
    pub world: Option<&'a str>,
    pub channel: Option<&'a str>,
    pub session_gen: Option<u64>,
    pub detail: Option<&'a str>,
}

impl<'a> Line<'a> {
    #[must_use]
    pub const fn new(level: Level, module: &'a str, event: &'a str) -> Self {
        Self {
            level,
            module,
            event,
            build: None,
            code: None,
            wire: None,
            trace: None,
            world: None,
            channel: None,
            session_gen: None,
            detail: None,
        }
    }

    #[must_use]
    pub const fn build(mut self, build: &'a str) -> Self {
        self.build = Some(build);
        self
    }

    #[must_use]
    pub const fn code(mut self, code: ErrorCode) -> Self {
        self.code = Some(code);
        self
    }

    #[must_use]
    pub const fn wire(mut self, wire: u32) -> Self {
        self.wire = Some(wire);
        self
    }

    #[must_use]
    pub const fn trace(mut self, trace: &'a str) -> Self {
        self.trace = Some(trace);
        self
    }

    #[must_use]
    pub const fn scope(mut self, world: &'a str, channel: &'a str) -> Self {
        self.world = Some(world);
        self.channel = Some(channel);
        self
    }

    #[must_use]
    pub const fn session_gen(mut self, generation: u64) -> Self {
        self.session_gen = Some(generation);
        self
    }

    #[must_use]
    pub const fn detail(mut self, detail: &'a str) -> Self {
        self.detail = Some(detail);
        self
    }

    /// Whether the level filter must keep this line whatever its level: it carries a code or is
    /// `warn` or above.
    #[must_use]
    pub fn always_written(&self) -> bool {
        self.code.is_some() || self.level <= Level::Warn
    }

    /// The single line, without a trailing newline, in the fixed field order. `process` is the
    /// binary name.
    #[must_use]
    pub fn render(&self, process: &str, ts_ms: u64) -> String {
        let mut out = format!(
            "{} ts={ts_ms} level={} module={} event={}",
            token(process),
            self.level.as_str(),
            token(self.module),
            token(self.event),
        );
        if let Some(build) = self.build {
            out.push_str(" build=");
            out.push_str(&token(build));
        }
        if let Some(code) = self.code {
            out.push_str(&format!(
                " code=E{:04} name={} cat={}",
                code.number,
                token(code.name),
                code.category().as_str()
            ));
        }
        if let Some(trace) = self.trace {
            out.push_str(" trace=");
            out.push_str(&token(trace));
        }
        if self.code.is_some()
            && let Some(wire) = self.wire
        {
            out.push_str(&format!(" wire=E{wire:04}"));
        }
        if let Some(world) = self.world {
            out.push_str(" world=");
            out.push_str(&token(world));
        }
        if let Some(channel) = self.channel {
            out.push_str(" channel=");
            out.push_str(&token(channel));
        }
        if let Some(generation) = self.session_gen {
            out.push_str(&format!(" session_gen={generation}"));
        }
        if let Some(detail) = self.detail {
            out.push_str(" detail=\"");
            out.push_str(&escape_detail(detail));
            out.push('"');
        }
        out
    }
}

fn token(value: &str) -> String {
    let token: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "._:/+-".contains(character) {
                character
            } else {
                '_'
            }
        })
        .collect();
    if token.is_empty() {
        "_".to_owned()
    } else {
        token
    }
}

fn escaped(character: char) -> String {
    match character {
        '\\' => "\\\\".to_owned(),
        '"' => "\\\"".to_owned(),
        '\r' => "\\r".to_owned(),
        '\n' => "\\n".to_owned(),
        control if control.is_ascii_control() => format!("\\u{{{:02X}}}", control as u32),
        other => other.to_string(),
    }
}

/// The escaped `detail` value (without its quotes), at most 512 bytes. A longer value is cut at
/// a character boundary that leaves room for `...`, and an escape sequence is never split.
#[must_use]
pub fn escape_detail(detail: &str) -> String {
    let pieces: Vec<String> = detail.chars().map(escaped).collect();
    let total: usize = pieces.iter().map(String::len).sum();
    if total <= DETAIL_MAX {
        return pieces.concat();
    }
    let room = DETAIL_MAX - ELLIPSIS.len();
    let mut out = String::new();
    for piece in &pieces {
        if out.len() + piece.len() > room {
            break;
        }
        out.push_str(piece);
    }
    out.push_str(ELLIPSIS);
    out
}

fn unescape(text: &str) -> Result<String, ParseError> {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(character) = chars.next() {
        match character {
            '"' => return Err(ParseError::BadDetail),
            '\\' => match chars.next() {
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('r') => out.push('\r'),
                Some('n') => out.push('\n'),
                Some('u') => {
                    if chars.next() != Some('{') {
                        return Err(ParseError::BadDetail);
                    }
                    let mut digits = String::new();
                    loop {
                        match chars.next() {
                            Some('}') => break,
                            Some(digit) if digit.is_ascii_hexdigit() && digits.len() < 2 => {
                                digits.push(digit);
                            }
                            _ => return Err(ParseError::BadDetail),
                        }
                    }
                    let value =
                        u32::from_str_radix(&digits, 16).map_err(|_| ParseError::BadDetail)?;
                    out.push(char::from_u32(value).ok_or(ParseError::BadDetail)?);
                }
                _ => return Err(ParseError::BadDetail),
            },
            other => out.push(other),
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// A required field is missing or a field is out of order or repeated.
    FieldOrder(String),
    /// A field value is malformed.
    BadValue(String),
    /// `detail` is not one quoted, correctly escaped value that ends the line.
    BadDetail,
}

/// A parsed §1.5 line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLine {
    pub process: String,
    pub ts: u64,
    pub level: Level,
    pub module: String,
    pub event: String,
    pub build: Option<String>,
    pub code: Option<u32>,
    pub name: Option<String>,
    pub cat: Option<String>,
    pub trace: Option<String>,
    pub wire: Option<u32>,
    pub world: Option<String>,
    pub channel: Option<String>,
    pub session_gen: Option<u64>,
    pub detail: Option<String>,
}

const ORDER: [&str; 14] = [
    "ts",
    "level",
    "module",
    "event",
    "build",
    "code",
    "name",
    "cat",
    "trace",
    "wire",
    "world",
    "channel",
    "session_gen",
    "detail",
];

fn code_number(value: &str, key: &str) -> Result<u32, ParseError> {
    value
        .strip_prefix('E')
        .filter(|digits| digits.len() >= 4 && digits.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|digits| digits.parse().ok())
        .ok_or_else(|| ParseError::BadValue(key.to_owned()))
}

/// Parse one line, rejecting a line whose fields are out of the fixed order, repeated, missing
/// (`ts`, `level`, `module` and `event` are always present) or inconsistent (`name` and `cat`
/// accompany `code`; `wire` needs `code`).
pub fn parse_line(line: &str) -> Result<ParsedLine, ParseError> {
    if line.contains(['\n', '\r']) {
        return Err(ParseError::BadDetail);
    }
    let (head, detail) = match line.find(" detail=\"") {
        Some(at) => {
            let raw = &line[at + " detail=\"".len()..];
            let inner = raw.strip_suffix('"').ok_or(ParseError::BadDetail)?;
            (&line[..at], Some(unescape(inner)?))
        }
        None => (line, None),
    };
    let mut words = head.split(' ');
    let process = words
        .next()
        .filter(|word| !word.is_empty() && !word.contains('='))
        .ok_or_else(|| ParseError::FieldOrder("process".to_owned()))?;
    let mut parsed = ParsedLine {
        process: process.to_owned(),
        ts: 0,
        level: Level::Info,
        module: String::new(),
        event: String::new(),
        build: None,
        code: None,
        name: None,
        cat: None,
        trace: None,
        wire: None,
        world: None,
        channel: None,
        session_gen: None,
        detail,
    };
    let mut next = 0;
    let mut seen = [false; 14];
    for word in words {
        let (key, value) = word
            .split_once('=')
            .filter(|(_, value)| !value.is_empty())
            .ok_or_else(|| ParseError::BadValue(word.to_owned()))?;
        let index = ORDER
            .iter()
            .position(|candidate| *candidate == key)
            .filter(|index| *index < 13)
            .ok_or_else(|| ParseError::FieldOrder(key.to_owned()))?;
        if index < next || seen[index] {
            return Err(ParseError::FieldOrder(key.to_owned()));
        }
        next = index + 1;
        seen[index] = true;
        let bad = || ParseError::BadValue(key.to_owned());
        match key {
            "ts" => parsed.ts = value.parse().map_err(|_| bad())?,
            "level" => parsed.level = Level::parse(value).ok_or_else(bad)?,
            "module" => parsed.module = value.to_owned(),
            "event" => parsed.event = value.to_owned(),
            "build" => parsed.build = Some(value.to_owned()),
            "code" => parsed.code = Some(code_number(value, key)?),
            "name" => parsed.name = Some(value.to_owned()),
            "cat" => parsed.cat = Some(value.to_owned()),
            "trace" => parsed.trace = Some(value.to_owned()),
            "wire" => parsed.wire = Some(code_number(value, key)?),
            "world" => parsed.world = Some(value.to_owned()),
            "channel" => parsed.channel = Some(value.to_owned()),
            _ => parsed.session_gen = Some(value.parse().map_err(|_| bad())?),
        }
    }
    for required in 0..4 {
        if !seen[required] {
            return Err(ParseError::FieldOrder(ORDER[required].to_owned()));
        }
    }
    if !(seen[5] == seen[6] && seen[6] == seen[7]) || (seen[9] && !seen[5]) {
        return Err(ParseError::FieldOrder("code".to_owned()));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::{Category, Progression};

    const CODE: ErrorCode = ErrorCode::new(
        2001,
        "CONFIG_INVALID",
        Category::InvalidInput,
        Progression::Terminal,
    );

    fn failure(detail: &str) -> String {
        Line::new(Level::Error, "node", "boot_failed")
            .code(CODE)
            .trace("0192e0a8-0000-7000-8000-000000000001")
            .wire(1050)
            .scope("world-1", "channel-2")
            .session_gen(7)
            .detail(detail)
            .render("oteryn-game-server", 1_700_000_000_000)
    }

    #[test]
    fn a_failure_line_has_the_fixed_field_order_and_parses_back() {
        let text = failure("bad key");
        assert_eq!(
            text,
            "oteryn-game-server ts=1700000000000 level=error module=node event=boot_failed \
             code=E2001 name=CONFIG_INVALID cat=INVALID_INPUT \
             trace=0192e0a8-0000-7000-8000-000000000001 wire=E1050 world=world-1 \
             channel=channel-2 session_gen=7 detail=\"bad key\""
        );
        let parsed = parse_line(&text).unwrap();
        assert_eq!(parsed.code, Some(2001));
        assert_eq!(parsed.wire, Some(1050));
        assert_eq!(parsed.cat.as_deref(), Some("INVALID_INPUT"));
        assert_eq!(parsed.session_gen, Some(7));
        assert_eq!(parsed.detail.as_deref(), Some("bad key"));
    }

    #[test]
    fn a_line_without_a_failure_omits_code_name_cat_and_wire() {
        let text = Line::new(Level::Info, "node", "process_start")
            .build("0.1.0+abcdef012345.local")
            .wire(1050)
            .render("oteryn-game-server", 5);
        assert_eq!(
            text,
            "oteryn-game-server ts=5 level=info module=node event=process_start \
             build=0.1.0+abcdef012345.local"
        );
        assert!(parse_line(&text).is_ok());
    }

    #[test]
    fn out_of_order_repeated_or_missing_fields_are_rejected() {
        let good = "p ts=1 level=info module=node event=x";
        assert!(parse_line(good).is_ok());
        for bad in [
            "p level=info ts=1 module=node event=x",
            "p ts=1 level=info module=node event=x event=y",
            "p ts=1 level=info module=node",
            "p ts=1 level=info module=node event=x name=A cat=CONFLICT",
            "p ts=1 level=info module=node event=x wire=E1050",
            "p ts=1 level=info module=node event=x trace=a code=E2001 name=A cat=CONFLICT",
            "p ts=1 level=loud module=node event=x",
            "p ts=1 level=info module=node event=x extra=1",
            "p ts=1 level=info module=node event=x detail=\"a\" world=w",
        ] {
            assert!(parse_line(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn adversarial_detail_yields_one_line_that_parses_to_the_original_fields() {
        let attacks = [
            "quote \" inside",
            "back\\slash and \\\" mixed",
            "cr\rlf\nend",
            "ctl \u{0}\u{1}\u{1f}\u{7f} tab\t",
            "forged code=E9999 name=FAKE cat=CONFLICT trace=x",
            "forged \" detail=\"x\" world=evil",
            "\\u{41} looks like an escape",
            "ünïcödé 日本語 🦀",
        ];
        for attack in attacks {
            let text = failure(attack);
            assert!(!text.contains(['\n', '\r']), "{attack:?}");
            let parsed = parse_line(&text).unwrap();
            assert_eq!(parsed.code, Some(2001), "{attack:?}");
            assert_eq!(parsed.world.as_deref(), Some("world-1"));
            assert_eq!(parsed.channel.as_deref(), Some("channel-2"));
            assert_eq!(parsed.session_gen, Some(7));
            assert_eq!(parsed.detail.as_deref(), Some(attack), "{attack:?}");
        }
    }

    #[test]
    fn tokens_never_hold_a_space_quote_or_control_character() {
        let text = Line::new(Level::Info, "no de", "ev\"ent\n")
            .scope("w w", "c\u{1}")
            .render("p p", 1);
        let parsed = parse_line(&text).unwrap();
        assert_eq!(parsed.process, "p_p");
        assert_eq!(parsed.module, "no_de");
        assert_eq!(parsed.event, "ev_ent_");
    }

    #[test]
    fn a_long_detail_is_cut_at_a_character_boundary_with_an_ellipsis() {
        // 3-byte characters: the cut never splits one.
        let long = "日".repeat(400);
        let escaped = escape_detail(&long);
        assert!(escaped.len() <= DETAIL_MAX && escaped.ends_with(ELLIPSIS));
        assert_eq!(escaped.len(), 3 * 169 + 3);
        let parsed = parse_line(&failure(&long)).unwrap();
        assert_eq!(parsed.detail.unwrap(), format!("{}...", "日".repeat(169)));
        // Escape sequences are never split.
        let newlines = "\n".repeat(400);
        let escaped = escape_detail(&newlines);
        assert!(escaped.len() <= DETAIL_MAX && escaped.ends_with("\\n..."));
        assert!(parse_line(&failure(&newlines)).is_ok());
        let controls = "\u{1}".repeat(400);
        let parsed = parse_line(&failure(&controls)).unwrap();
        assert!(parsed.detail.unwrap().ends_with("\u{1}..."));
        // Exactly at the limit nothing is cut.
        let exact = "a".repeat(DETAIL_MAX);
        assert_eq!(escape_detail(&exact), exact);
        assert!(escape_detail(&"a".repeat(DETAIL_MAX + 1)).ends_with("a..."));
    }
}
