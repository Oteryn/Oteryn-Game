//! Account achievements panel typed payloads (`OTERYN_ACHIEVEMENT_DISPLAY_CONTRACT_V1` §3:
//! D223-D228, #162 owner decision record 5911933242; schema in
//! `docs/contracts/protocol-oteryn/v1/account_achievements_v1.proto`). Command type 10
//! `ACCOUNT_ACHIEVEMENTS_QUERY`: the client asks for one page, the server answers with the earned
//! facts of the session's own account.
//!
//! Decoding is strict: unknown or repeated singular fields, a key outside the
//! `oteryn:achievement/<slug>` grammar, invalid UTF-8, a grade outside 1-4, a bool other than 0
//! or 1, a field above its byte bound, more than [`ACCOUNT_ACHIEVEMENTS_PAGE_ROWS`] rows, and
//! `has_more` on a page that is not full all fail closed. A standard proto3 encoder omits a field
//! holding its default value, so decoding accepts that omission and defaults the field (FND-02
//! §7). Encoding is strict too: a row the decoder would refuse is a server fault, refused before
//! any byte is emitted, and never truncated (§3.3).

// The client-side codecs (query encode, result decode) are exercised by the round-trip tests; the
// server composes only its own direction.
#![cfg_attr(not(test), allow(dead_code))]

pub const COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY: u32 = 10;
/// The fixed server-side page size (§3.3).
pub const ACCOUNT_ACHIEVEMENTS_PAGE_ROWS: usize = 64;
pub const MAX_ACHIEVEMENT_KEY_BYTES: usize = 160;
pub const MAX_ACHIEVEMENT_NAME_BYTES: usize = 64;
pub const MAX_ACHIEVEMENT_DESCRIPTION_BYTES: usize = 256;
pub const MAX_ACHIEVEMENT_GRADE: u32 = 4;
/// `page`: 1 tag + a 5-byte uint32 varint.
pub const MAX_ACCOUNT_ACHIEVEMENTS_QUERY_BYTES: usize = 6;
/// One row at every field bound: key 1 + 2 + 160, name 1 + 1 + 64, description 1 + 2 + 256,
/// grade 1 + 1, points 1 + 5, earned_at 1 + 10 (a negative int64), secret 1 + 1.
pub const MAX_ACCOUNT_ACHIEVEMENT_ROW_BYTES: usize = 509;
/// 64 rows of 512 bytes (the row, its tag and its 2-byte length) and 20 bytes of `total_points`
/// (6), `fact_count` (6), `page` (6) and `has_more` (2).
pub const MAX_ACCOUNT_ACHIEVEMENTS_RESULT_BYTES: usize = 32_788;

// Both bounds sit inside the FND-02 §19 parents (64 KiB command and result payloads).
const _: () = assert!(MAX_ACCOUNT_ACHIEVEMENTS_QUERY_BYTES <= crate::MAX_COMMAND_PAYLOAD_BYTES);
const _: () =
    assert!(MAX_ACCOUNT_ACHIEVEMENTS_RESULT_BYTES <= crate::MAX_COMMAND_RESULT_PAYLOAD_BYTES);

const KEY_PREFIX: &str = "oteryn:achievement/";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountAchievementsError {
    /// Not a valid encoding of the schema, or a value outside its declared domain.
    Malformed,
    /// The payload, a text field or the row count is over its bound.
    LimitExceeded,
}

/// `AccountAchievementsQuery`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountAchievementsQuery {
    pub page: u32,
}

/// `AccountAchievementRow`: one earned fact of the account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountAchievementRow {
    pub key: String,
    pub name: String,
    pub description: String,
    pub grade: u32,
    pub points: u32,
    pub earned_at: i64,
    pub secret: bool,
}

/// `AccountAchievementsResult`: one page of the account's facts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccountAchievementsResult {
    pub total_points: u32,
    pub fact_count: u32,
    pub page: u32,
    pub has_more: bool,
    pub rows: Vec<AccountAchievementRow>,
}

/// `oteryn:achievement/<slug>`, the catalogue key grammar: lowercase ASCII letters and digits in
/// non-empty parts joined by single underscores, at most 160 bytes in all.
pub fn valid_achievement_key(key: &str) -> bool {
    key.len() <= MAX_ACHIEVEMENT_KEY_BYTES
        && key.strip_prefix(KEY_PREFIX).is_some_and(|slug| {
            slug.split('_').all(|part| {
                !part.is_empty()
                    && part
                        .bytes()
                        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
            })
        })
}

/// The row rules shared by both directions.
fn check_row(row: &AccountAchievementRow) -> Result<(), AccountAchievementsError> {
    if row.key.len() > MAX_ACHIEVEMENT_KEY_BYTES
        || row.name.len() > MAX_ACHIEVEMENT_NAME_BYTES
        || row.description.len() > MAX_ACHIEVEMENT_DESCRIPTION_BYTES
    {
        return Err(AccountAchievementsError::LimitExceeded);
    }
    if !valid_achievement_key(&row.key) || !(1..=MAX_ACHIEVEMENT_GRADE).contains(&row.grade) {
        return Err(AccountAchievementsError::Malformed);
    }
    Ok(())
}

/// The page rules shared by both directions: at most one page of rows, and `has_more` only on a
/// full page (a page holds `min(64, remaining)` rows).
fn check_page(result: &AccountAchievementsResult) -> Result<(), AccountAchievementsError> {
    if result.rows.len() > ACCOUNT_ACHIEVEMENTS_PAGE_ROWS {
        return Err(AccountAchievementsError::LimitExceeded);
    }
    if result.has_more && result.rows.len() != ACCOUNT_ACHIEVEMENTS_PAGE_ROWS {
        return Err(AccountAchievementsError::Malformed);
    }
    Ok(())
}

fn push_varint(output: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        output.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}

/// Omits the field at its proto3 default (0), as a standard encoder does.
fn push_varint_field(output: &mut Vec<u8>, field: u64, value: u64) {
    if value != 0 {
        push_varint(output, field << 3);
        push_varint(output, value);
    }
}

fn push_bytes_field(output: &mut Vec<u8>, field: u64, value: &[u8]) {
    push_varint(output, (field << 3) | 2);
    push_varint(output, value.len() as u64);
    output.extend_from_slice(value);
}

/// Omits an empty string, as a standard proto3 encoder does.
fn push_string_field(output: &mut Vec<u8>, field: u64, value: &str) {
    if !value.is_empty() {
        push_bytes_field(output, field, value.as_bytes());
    }
}

fn read_varint(input: &[u8], cursor: &mut usize) -> Result<u64, AccountAchievementsError> {
    let mut value = 0_u64;
    for shift in (0..70).step_by(7) {
        let byte = *input
            .get(*cursor)
            .ok_or(AccountAchievementsError::Malformed)?;
        *cursor += 1;
        if shift == 63 && byte > 1 {
            return Err(AccountAchievementsError::Malformed);
        }
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(AccountAchievementsError::Malformed)
}

fn read_uint32(input: &[u8], cursor: &mut usize) -> Result<u32, AccountAchievementsError> {
    u32::try_from(read_varint(input, cursor)?).map_err(|_| AccountAchievementsError::Malformed)
}

fn read_bool(input: &[u8], cursor: &mut usize) -> Result<bool, AccountAchievementsError> {
    match read_varint(input, cursor)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(AccountAchievementsError::Malformed),
    }
}

fn read_bytes<'a>(
    input: &'a [u8],
    cursor: &mut usize,
) -> Result<&'a [u8], AccountAchievementsError> {
    let len = usize::try_from(read_varint(input, cursor)?)
        .map_err(|_| AccountAchievementsError::Malformed)?;
    let end = cursor
        .checked_add(len)
        .filter(|end| *end <= input.len())
        .ok_or(AccountAchievementsError::Malformed)?;
    let value = &input[*cursor..end];
    *cursor = end;
    Ok(value)
}

/// A UTF-8 string of at most `maximum` bytes; the bound is checked before the copy.
fn read_string(
    input: &[u8],
    cursor: &mut usize,
    maximum: usize,
) -> Result<String, AccountAchievementsError> {
    let bytes = read_bytes(input, cursor)?;
    if bytes.len() > maximum {
        return Err(AccountAchievementsError::LimitExceeded);
    }
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| AccountAchievementsError::Malformed)
}

/// Stores a singular field once; a repeated field fails closed.
fn set_once<T>(slot: &mut Option<T>, value: T) -> Result<(), AccountAchievementsError> {
    if slot.is_some() {
        return Err(AccountAchievementsError::Malformed);
    }
    *slot = Some(value);
    Ok(())
}

/// `ClientCommand.payload` of command type 10. Page 0 is the empty payload.
pub fn encode_account_achievements_query(query: AccountAchievementsQuery) -> Vec<u8> {
    let mut output = Vec::with_capacity(MAX_ACCOUNT_ACHIEVEMENTS_QUERY_BYTES);
    push_varint_field(&mut output, 1, u64::from(query.page));
    output
}

pub fn decode_account_achievements_query(
    payload: &[u8],
) -> Result<AccountAchievementsQuery, AccountAchievementsError> {
    if payload.len() > MAX_ACCOUNT_ACHIEVEMENTS_QUERY_BYTES {
        return Err(AccountAchievementsError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut page = None;
    while cursor < payload.len() {
        if read_varint(payload, &mut cursor)? != 0x08 {
            return Err(AccountAchievementsError::Malformed);
        }
        let value = read_uint32(payload, &mut cursor)?;
        set_once(&mut page, value)?;
    }
    Ok(AccountAchievementsQuery {
        page: page.unwrap_or(0),
    })
}

fn encode_row(output: &mut Vec<u8>, row: &AccountAchievementRow) {
    let mut inner = Vec::with_capacity(MAX_ACCOUNT_ACHIEVEMENT_ROW_BYTES);
    push_string_field(&mut inner, 1, &row.key);
    push_string_field(&mut inner, 2, &row.name);
    push_string_field(&mut inner, 3, &row.description);
    push_varint_field(&mut inner, 4, u64::from(row.grade));
    push_varint_field(&mut inner, 5, u64::from(row.points));
    // int64 is the two's-complement varint, so a negative value takes 10 bytes.
    push_varint_field(&mut inner, 6, row.earned_at as u64);
    push_varint_field(&mut inner, 7, u64::from(row.secret));
    push_bytes_field(output, 5, &inner);
}

/// `CommandResult.payload` of command type 10. A row or page the decoder would refuse is a server
/// fault: it is refused before any byte is emitted, never truncated.
pub fn encode_account_achievements_result(
    result: &AccountAchievementsResult,
) -> Result<Vec<u8>, AccountAchievementsError> {
    check_page(result)?;
    for row in &result.rows {
        check_row(row)?;
    }
    let mut output = Vec::new();
    push_varint_field(&mut output, 1, u64::from(result.total_points));
    push_varint_field(&mut output, 2, u64::from(result.fact_count));
    push_varint_field(&mut output, 3, u64::from(result.page));
    push_varint_field(&mut output, 4, u64::from(result.has_more));
    for row in &result.rows {
        encode_row(&mut output, row);
    }
    if output.len() > MAX_ACCOUNT_ACHIEVEMENTS_RESULT_BYTES {
        return Err(AccountAchievementsError::LimitExceeded);
    }
    Ok(output)
}

fn decode_row(input: &[u8]) -> Result<AccountAchievementRow, AccountAchievementsError> {
    if input.len() > MAX_ACCOUNT_ACHIEVEMENT_ROW_BYTES {
        return Err(AccountAchievementsError::LimitExceeded);
    }
    let mut cursor = 0;
    let (mut key, mut name, mut description) = (None, None, None);
    let (mut grade, mut points, mut earned_at, mut secret) = (None, None, None, None);
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            0x0a => {
                let value = read_string(input, &mut cursor, MAX_ACHIEVEMENT_KEY_BYTES)?;
                set_once(&mut key, value)?;
            }
            0x12 => {
                let value = read_string(input, &mut cursor, MAX_ACHIEVEMENT_NAME_BYTES)?;
                set_once(&mut name, value)?;
            }
            0x1a => {
                let value = read_string(input, &mut cursor, MAX_ACHIEVEMENT_DESCRIPTION_BYTES)?;
                set_once(&mut description, value)?;
            }
            0x20 => {
                let value = read_uint32(input, &mut cursor)?;
                set_once(&mut grade, value)?;
            }
            0x28 => {
                let value = read_uint32(input, &mut cursor)?;
                set_once(&mut points, value)?;
            }
            0x30 => {
                let value = read_varint(input, &mut cursor)? as i64;
                set_once(&mut earned_at, value)?;
            }
            0x38 => {
                let value = read_bool(input, &mut cursor)?;
                set_once(&mut secret, value)?;
            }
            _ => return Err(AccountAchievementsError::Malformed),
        }
    }
    let row = AccountAchievementRow {
        key: key.unwrap_or_default(),
        name: name.unwrap_or_default(),
        description: description.unwrap_or_default(),
        grade: grade.unwrap_or(0),
        points: points.unwrap_or(0),
        earned_at: earned_at.unwrap_or(0),
        secret: secret.unwrap_or(false),
    };
    check_row(&row)?;
    Ok(row)
}

pub fn decode_account_achievements_result(
    payload: &[u8],
) -> Result<AccountAchievementsResult, AccountAchievementsError> {
    if payload.len() > MAX_ACCOUNT_ACHIEVEMENTS_RESULT_BYTES {
        return Err(AccountAchievementsError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut fields: [Option<u32>; 3] = [None; 3];
    let mut has_more = None;
    let mut rows = Vec::new();
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            key @ (0x08 | 0x10 | 0x18) => {
                // Field numbers 1..=3 index 0..=2.
                let index = usize::try_from((key >> 3) - 1)
                    .map_err(|_| AccountAchievementsError::Malformed)?;
                let value = read_uint32(payload, &mut cursor)?;
                let slot = fields
                    .get_mut(index)
                    .ok_or(AccountAchievementsError::Malformed)?;
                set_once(slot, value)?;
            }
            0x20 => {
                let value = read_bool(payload, &mut cursor)?;
                set_once(&mut has_more, value)?;
            }
            0x2a => {
                // The row count is bounded before the row is decoded.
                if rows.len() == ACCOUNT_ACHIEVEMENTS_PAGE_ROWS {
                    return Err(AccountAchievementsError::LimitExceeded);
                }
                rows.push(decode_row(read_bytes(payload, &mut cursor)?)?);
            }
            _ => return Err(AccountAchievementsError::Malformed),
        }
    }
    let [total_points, fact_count, page] = fields.map(|value| value.unwrap_or(0));
    let result = AccountAchievementsResult {
        total_points,
        fact_count,
        page,
        has_more: has_more.unwrap_or(false),
        rows,
    };
    check_page(&result)?;
    Ok(result)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use serde_json::Value;

    const PROTOCOL_REGISTRY: &str =
        include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
    const SCHEMA: &str =
        include_str!("../../../docs/contracts/protocol-oteryn/v1/account_achievements_v1.proto");

    const COOKIES: &str = "oteryn:achievement/allow_cookies";
    const MERRIER: &str = "oteryn:achievement/the_more_the_merrier";

    fn row(key: &str, name: &str, grade: u32, points: u32) -> AccountAchievementRow {
        AccountAchievementRow {
            key: key.into(),
            name: name.into(),
            description: String::new(),
            grade,
            points,
            earned_at: 0,
            secret: false,
        }
    }

    /// A row at every field bound: 509 bytes.
    fn worst_row() -> AccountAchievementRow {
        AccountAchievementRow {
            key: format!("{KEY_PREFIX}{}", "a".repeat(MAX_ACHIEVEMENT_KEY_BYTES - 19)),
            name: "n".repeat(MAX_ACHIEVEMENT_NAME_BYTES),
            description: "d".repeat(MAX_ACHIEVEMENT_DESCRIPTION_BYTES),
            grade: MAX_ACHIEVEMENT_GRADE,
            points: u32::MAX,
            earned_at: i64::MIN,
            secret: true,
        }
    }

    fn worst_result() -> AccountAchievementsResult {
        AccountAchievementsResult {
            total_points: u32::MAX,
            fact_count: u32::MAX,
            page: u32::MAX,
            has_more: true,
            rows: vec![worst_row(); ACCOUNT_ACHIEVEMENTS_PAGE_ROWS],
        }
    }

    /// Canonical bytes produced by the reference protobuf runtime (protoc-generated Python,
    /// protobuf 7.36.2, `SerializeToString(deterministic=True)`) outside the repository, not by the
    /// encoder under test.
    fn result_fixtures() -> Vec<(AccountAchievementsResult, Vec<u8>)> {
        let two_rows = [
            &[0x08, 0x0a, 0x10, 0x02, 0x2a, 0x40, 0x0a, 0x20][..],
            COOKIES.as_bytes(),
            &[0x12, 0x0e],
            b"Allow Cookies?",
            // description "D", grade 1, points 10, earned_at 1_700_000_000_000.
            &[0x1a, 0x01, 0x44, 0x20, 0x01, 0x28, 0x0a],
            &[0x30, 0x80, 0xd0, 0x95, 0xff, 0xbc, 0x31],
            // A retired row: empty description and points 0 omitted, earned_at 5, secret.
            &[0x2a, 0x45, 0x0a, 0x27],
            MERRIER.as_bytes(),
            &[0x12, 0x14],
            b"The More the Merrier",
            &[0x20, 0x01, 0x30, 0x05, 0x38, 0x01],
        ]
        .concat();
        let edge = [
            &[
                0x08, 0xff, 0xff, 0xff, 0xff, 0x0f, 0x10, 0xff, 0xff, 0xff, 0xff, 0x0f,
            ][..],
            &[0x18, 0xff, 0xff, 0xff, 0xff, 0x0f, 0x2a, 0x38, 0x0a, 0x15],
            b"oteryn:achievement/x1",
            // "Été" and "✓" in UTF-8, grade 4, points u32::MAX, earned_at -1 (10 bytes), secret.
            &[
                0x12, 0x05, 0xc3, 0x89, 0x74, 0xc3, 0xa9, 0x1a, 0x03, 0xe2, 0x9c, 0x93,
            ],
            &[0x20, 0x04, 0x28, 0xff, 0xff, 0xff, 0xff, 0x0f],
            &[
                0x30, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01, 0x38, 0x01,
            ],
        ]
        .concat();
        vec![
            // An empty account, page 0: every field omitted.
            (AccountAchievementsResult::default(), vec![]),
            // A page past the end: no rows, has_more false.
            (
                AccountAchievementsResult {
                    total_points: 12,
                    fact_count: 3,
                    page: 9,
                    has_more: false,
                    rows: vec![],
                },
                vec![0x08, 0x0c, 0x10, 0x03, 0x18, 0x09],
            ),
            (
                AccountAchievementsResult {
                    total_points: 10,
                    fact_count: 2,
                    page: 0,
                    has_more: false,
                    rows: vec![
                        AccountAchievementRow {
                            description: "D".into(),
                            earned_at: 1_700_000_000_000,
                            ..row(COOKIES, "Allow Cookies?", 1, 10)
                        },
                        AccountAchievementRow {
                            earned_at: 5,
                            secret: true,
                            ..row(MERRIER, "The More the Merrier", 1, 0)
                        },
                    ],
                },
                two_rows,
            ),
            (
                AccountAchievementsResult {
                    total_points: u32::MAX,
                    fact_count: u32::MAX,
                    page: u32::MAX,
                    has_more: false,
                    rows: vec![AccountAchievementRow {
                        key: "oteryn:achievement/x1".into(),
                        name: "\u{c9}t\u{e9}".into(),
                        description: "\u{2713}".into(),
                        grade: 4,
                        points: u32::MAX,
                        earned_at: -1,
                        secret: true,
                    }],
                },
                edge,
            ),
        ]
    }

    #[test]
    fn query_matches_independent_fixtures_and_refuses_everything_else() {
        let fixtures: [(u32, &[u8]); 4] = [
            (0, &[]),
            (1, &[0x08, 0x01]),
            (300, &[0x08, 0xac, 0x02]),
            (u32::MAX, &[0x08, 0xff, 0xff, 0xff, 0xff, 0x0f]),
        ];
        for (page, bytes) in fixtures {
            let query = AccountAchievementsQuery { page };
            assert_eq!(encode_account_achievements_query(query), bytes, "{page}");
            assert_eq!(decode_account_achievements_query(bytes), Ok(query));
        }
        // An explicit proto3 default decodes as the omitted one.
        assert_eq!(
            decode_account_achievements_query(&[0x08, 0x00]),
            Ok(AccountAchievementsQuery { page: 0 })
        );
        let malformed: &[(&str, &[u8])] = &[
            ("page repeated", &[0x08, 0x01, 0x08, 0x02]),
            ("page above u32", &[0x08, 0x80, 0x80, 0x80, 0x80, 0x10]),
            ("unknown field 2", &[0x10, 0x01]),
            ("field 0", &[0x00, 0x01]),
            ("page wrong wire type", &[0x0a, 0x00]),
            ("truncated key", &[0x88]),
            ("truncated varint", &[0x08, 0x81]),
        ];
        for (case, bytes) in malformed {
            assert_eq!(
                decode_account_achievements_query(bytes),
                Err(AccountAchievementsError::Malformed),
                "{case}"
            );
        }
        assert_eq!(
            decode_account_achievements_query(&[0; MAX_ACCOUNT_ACHIEVEMENTS_QUERY_BYTES + 1]),
            Err(AccountAchievementsError::LimitExceeded)
        );
    }

    #[test]
    fn result_matches_independent_fixtures() {
        for (value, bytes) in result_fixtures() {
            assert_eq!(
                encode_account_achievements_result(&value),
                Ok(bytes.clone()),
                "{value:?}"
            );
            assert_eq!(
                decode_account_achievements_result(&bytes),
                Ok(value),
                "{bytes:02x?}"
            );
        }
        // Field order is not significant, and explicit proto3 defaults decode as omitted ones:
        // page 0, then a row of secret, earned_at, points, grade 2, name, description and key,
        // then fact_count 1 and total_points 0.
        let reordered = [
            &[
                0x18, 0x00, 0x2a, 0x22, 0x38, 0x00, 0x30, 0x00, 0x28, 0x00, 0x20, 0x02,
            ][..],
            &[0x12, 0x00, 0x1a, 0x00, 0x0a, 0x14],
            b"oteryn:achievement/b",
            &[0x10, 0x01, 0x08, 0x00],
        ]
        .concat();
        assert_eq!(
            decode_account_achievements_result(&reordered),
            Ok(AccountAchievementsResult {
                fact_count: 1,
                rows: vec![row("oteryn:achievement/b", "", 2, 0)],
                ..AccountAchievementsResult::default()
            })
        );
    }

    /// FND-02 §22 item 2: a raw walker, independent of the production decoder, checks the framing
    /// of the canonical worst case: 3 scalars, `has_more`, then 64 rows of exactly 509 bytes.
    #[test]
    fn worst_case_fits_the_registered_bound_by_an_independent_walk() {
        let bytes = encode_account_achievements_result(&worst_result()).expect("worst case");
        assert_eq!(bytes.len(), MAX_ACCOUNT_ACHIEVEMENTS_RESULT_BYTES);
        let (mut cursor, mut tags, mut row_lengths) = (0, Vec::new(), Vec::new());
        let varint = |cursor: &mut usize| {
            let (mut value, mut shift) = (0_u64, 0);
            loop {
                let byte = bytes[*cursor];
                *cursor += 1;
                value |= u64::from(byte & 0x7f) << shift;
                shift += 7;
                if byte & 0x80 == 0 {
                    return value;
                }
            }
        };
        while cursor < bytes.len() {
            let tag = varint(&mut cursor);
            tags.push(tag);
            if tag & 7 == 2 {
                let length = varint(&mut cursor) as usize;
                row_lengths.push(length);
                cursor += length;
            } else {
                varint(&mut cursor);
            }
        }
        assert_eq!(cursor, bytes.len());
        assert_eq!(tags[..4], [0x08, 0x10, 0x18, 0x20]);
        assert_eq!(tags.len(), 4 + ACCOUNT_ACHIEVEMENTS_PAGE_ROWS);
        assert!(tags[4..].iter().all(|tag| *tag == 0x2a));
        assert!(
            row_lengths
                .iter()
                .all(|length| *length == MAX_ACCOUNT_ACHIEVEMENT_ROW_BYTES)
        );
        // The first row starts with its tag, the 2-byte length 509 and the key header 0a a0 01.
        assert_eq!(bytes[20..26], [0x2a, 0xfd, 0x03, 0x0a, 0xa0, 0x01]);
        assert_eq!(
            decode_account_achievements_result(&bytes),
            Ok(worst_result())
        );
    }

    /// Wraps one raw row as the only row of a result.
    fn one_row(inner: &[u8]) -> Vec<u8> {
        let mut output = vec![0x2a];
        push_varint(&mut output, inner.len() as u64);
        output.extend_from_slice(inner);
        output
    }

    /// A raw row with `key`, grade 1 and `rest` appended.
    fn raw_row(key: &[u8], rest: &[u8]) -> Vec<u8> {
        let mut inner = vec![0x0a];
        push_varint(&mut inner, key.len() as u64);
        inner.extend_from_slice(key);
        inner.extend_from_slice(&[0x20, 0x01]);
        inner.extend_from_slice(rest);
        one_row(&inner)
    }

    fn raw_text(tag: u8, length: usize) -> Vec<u8> {
        let mut output = vec![tag];
        push_varint(&mut output, length as u64);
        output.extend(std::iter::repeat_n(b'x', length));
        output
    }

    #[test]
    fn result_malformed_and_oversize_corpus_fails_closed() {
        use AccountAchievementsError::{LimitExceeded, Malformed};
        let key = COOKIES.as_bytes();
        let long_key = format!("{KEY_PREFIX}{}", "a".repeat(MAX_ACHIEVEMENT_KEY_BYTES - 18));
        let keyed = |rest: &[u8]| one_row(&[&[0x0a, 0x20][..], key, rest].concat());
        let full_page_minus_one: Vec<u8> = [0x20, 0x01]
            .into_iter()
            .chain((1..ACCOUNT_ACHIEVEMENTS_PAGE_ROWS).flat_map(|_| raw_row(key, &[])))
            .collect();
        let over_page: Vec<u8> = (0..=ACCOUNT_ACHIEVEMENTS_PAGE_ROWS)
            .flat_map(|_| raw_row(key, &[]))
            .collect();
        let long_earned_at = [
            0x30, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x02,
        ];
        let cases: Vec<(&str, Vec<u8>, AccountAchievementsError)> = vec![
            ("unknown field 6", vec![0x30, 0x01], Malformed),
            ("field 0", vec![0x00, 0x01], Malformed),
            ("total repeated", vec![0x08, 0x01, 0x08, 0x01], Malformed),
            ("page repeated", vec![0x18, 0x01, 0x18, 0x01], Malformed),
            ("has_more repeated", vec![0x20, 0x00, 0x20, 0x00], Malformed),
            ("has_more not a bool", vec![0x20, 0x02], Malformed),
            (
                "fact_count above u32",
                vec![0x10, 0x80, 0x80, 0x80, 0x80, 0x10],
                Malformed,
            ),
            ("rows as varint", vec![0x28, 0x01], Malformed),
            ("has_more on an empty page", vec![0x20, 0x01], Malformed),
            ("has_more on 63 rows", full_page_minus_one, Malformed),
            ("65 rows", over_page, LimitExceeded),
            ("key absent", one_row(&[0x20, 0x01]), Malformed),
            (
                "key outside the grammar",
                raw_row(b"canary:achievement/a", &[]),
                Malformed,
            ),
            (
                "key upper case",
                raw_row(b"oteryn:achievement/Allow", &[]),
                Malformed,
            ),
            (
                "key over 160 bytes",
                raw_row(long_key.as_bytes(), &[]),
                LimitExceeded,
            ),
            ("key not UTF-8", raw_row(&[0xff], &[]), Malformed),
            (
                "name over 64 bytes",
                raw_row(key, &raw_text(0x12, 65)),
                LimitExceeded,
            ),
            (
                "description over 256 bytes",
                raw_row(key, &raw_text(0x1a, 257)),
                LimitExceeded,
            ),
            (
                "name not UTF-8",
                raw_row(key, &[0x12, 0x02, 0xc3, 0x28]),
                Malformed,
            ),
            ("grade absent", keyed(&[]), Malformed),
            ("grade 0", keyed(&[0x20, 0x00]), Malformed),
            ("grade 5", keyed(&[0x20, 0x05]), Malformed),
            ("grade repeated", raw_row(key, &[0x20, 0x01]), Malformed),
            (
                "points above u32",
                raw_row(key, &[0x28, 0x80, 0x80, 0x80, 0x80, 0x10]),
                Malformed,
            ),
            ("secret not a bool", raw_row(key, &[0x38, 0x02]), Malformed),
            (
                "earned_at over 10 bytes",
                raw_row(key, &long_earned_at),
                Malformed,
            ),
            (
                "row unknown field 8",
                raw_row(key, &[0x40, 0x01]),
                Malformed,
            ),
            ("row name as varint", raw_row(key, &[0x10, 0x01]), Malformed),
            ("row length past end", vec![0x2a, 0x05, 0x0a], Malformed),
            (
                "text length past end",
                vec![0x2a, 0x03, 0x0a, 0x05, 0x6f],
                Malformed,
            ),
            ("truncated key", vec![0x88], Malformed),
            ("truncated varint", vec![0x08, 0x81], Malformed),
        ];
        for (case, bytes, error) in cases {
            assert_eq!(
                decode_account_achievements_result(&bytes),
                Err(error),
                "{case}"
            );
        }
        // One byte over the row bound, and one over the payload bound.
        let mut over_row = raw_text(0x1a, MAX_ACCOUNT_ACHIEVEMENT_ROW_BYTES);
        over_row.truncate(MAX_ACCOUNT_ACHIEVEMENT_ROW_BYTES + 1);
        assert_eq!(
            decode_account_achievements_result(&one_row(&over_row)),
            Err(LimitExceeded)
        );
        assert_eq!(
            decode_account_achievements_result(&[0; MAX_ACCOUNT_ACHIEVEMENTS_RESULT_BYTES + 1]),
            Err(LimitExceeded)
        );
    }

    #[test]
    fn the_encoder_refuses_what_the_decoder_refuses() {
        use AccountAchievementsError::{LimitExceeded, Malformed};
        let valid = row(COOKIES, "Allow Cookies?", 1, 10);
        let page = |rows: Vec<AccountAchievementRow>, has_more: bool| AccountAchievementsResult {
            has_more,
            rows,
            ..AccountAchievementsResult::default()
        };
        let with = |change: fn(&mut AccountAchievementRow)| {
            let mut changed = valid.clone();
            change(&mut changed);
            page(vec![changed], false)
        };
        let cases = [
            (
                "65 rows",
                page(vec![valid.clone(); 65], true),
                LimitExceeded,
            ),
            (
                "has_more on 63 rows",
                page(vec![valid.clone(); 63], true),
                Malformed,
            ),
            ("has_more on no row", page(vec![], true), Malformed),
            ("grade 0", with(|row| row.grade = 0), Malformed),
            ("grade 5", with(|row| row.grade = 5), Malformed),
            (
                "key outside the grammar",
                with(|row| row.key = "oteryn:achievement/a-b".into()),
                Malformed,
            ),
            ("key empty", with(|row| row.key.clear()), Malformed),
            (
                "key over 160 bytes",
                with(|row| row.key.push_str(&"a".repeat(129))),
                LimitExceeded,
            ),
            (
                "name over 64 bytes",
                with(|row| row.name = "\u{e9}".repeat(33)),
                LimitExceeded,
            ),
            (
                "description over 256 bytes",
                with(|row| row.description = "d".repeat(257)),
                LimitExceeded,
            ),
        ];
        for (case, value, error) in cases {
            assert_eq!(
                encode_account_achievements_result(&value),
                Err(error),
                "{case}"
            );
        }
        // The full page with has_more is valid.
        assert!(encode_account_achievements_result(&page(vec![valid; 64], true)).is_ok());
    }

    /// FND-02 §22 item 4: semantic round trips and bounds over a deterministic pseudo-random
    /// sample of valid values (xorshift64, fixed seed).
    #[test]
    fn round_trip_property_over_generated_values() {
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let alphabet = ['a', 'Z', ' ', '?', '\u{e9}', '\u{2713}', '\u{1f600}'];
        let text = |next: &mut dyn FnMut() -> u64, maximum: usize| {
            let mut value = String::new();
            let target = (next() as usize) % (maximum + 1);
            loop {
                let character = alphabet[(next() as usize) % alphabet.len()];
                if value.len() + character.len_utf8() > target {
                    return value;
                }
                value.push(character);
            }
        };
        for _ in 0..512 {
            let page = next() as u32;
            let query = AccountAchievementsQuery { page };
            let bytes = encode_account_achievements_query(query);
            assert!(bytes.len() <= MAX_ACCOUNT_ACHIEVEMENTS_QUERY_BYTES);
            assert_eq!(decode_account_achievements_query(&bytes), Ok(query));

            let count = (next() as usize) % (ACCOUNT_ACHIEVEMENTS_PAGE_ROWS + 1);
            let mut rows = Vec::with_capacity(count);
            for _ in 0..count {
                let parts = 1 + (next() as usize) % 4;
                let slug: Vec<String> = (0..parts).map(|_| format!("p{}", next() % 1000)).collect();
                let name = text(&mut next, MAX_ACHIEVEMENT_NAME_BYTES);
                let description = text(&mut next, MAX_ACHIEVEMENT_DESCRIPTION_BYTES);
                rows.push(AccountAchievementRow {
                    key: format!("{KEY_PREFIX}{}", slug.join("_")),
                    name,
                    description,
                    grade: 1 + (next() as u32) % MAX_ACHIEVEMENT_GRADE,
                    points: if next() % 2 == 0 { 0 } else { next() as u32 },
                    earned_at: next() as i64,
                    secret: next() % 2 == 0,
                });
            }
            let value = AccountAchievementsResult {
                total_points: next() as u32,
                fact_count: next() as u32,
                page,
                has_more: count == ACCOUNT_ACHIEVEMENTS_PAGE_ROWS && next() % 2 == 0,
                rows,
            };
            let bytes = encode_account_achievements_result(&value).expect("valid value encodes");
            assert!(bytes.len() <= MAX_ACCOUNT_ACHIEVEMENTS_RESULT_BYTES);
            assert_eq!(decode_account_achievements_result(&bytes), Ok(value));
        }
    }

    #[test]
    fn registry_binds_the_account_achievements_command() {
        let protocol: Value = serde_json::from_str(PROTOCOL_REGISTRY).expect("protocol registry");
        let commands = protocol["command_types"].as_array().expect("command_types");
        let matching: Vec<&Value> = commands
            .iter()
            .filter(|command| command["id"] == COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY)
            .collect();
        assert_eq!(matching.len(), 1, "the ID is registered once");
        let command = matching[0];
        assert_eq!(command["name"], "ACCOUNT_ACHIEVEMENTS_QUERY");
        assert!(
            command["owner_decision"]
                .as_str()
                .is_some_and(|decision| decision.contains("D223-D228"))
        );
        let anchor = "docs/contracts/protocol-oteryn/v1/account_achievements_v1.proto#";
        assert_eq!(
            command["payload_schema"],
            format!("{anchor}AccountAchievementsQuery")
        );
        assert_eq!(
            command["result_schema"],
            format!("{anchor}AccountAchievementsResult")
        );
        assert_eq!(
            command["max_payload_bytes"],
            MAX_ACCOUNT_ACHIEVEMENTS_QUERY_BYTES as u64
        );
        assert_eq!(
            command["max_result_payload_bytes"],
            MAX_ACCOUNT_ACHIEVEMENTS_RESULT_BYTES as u64
        );
        for message in [
            "AccountAchievementsQuery",
            "AccountAchievementRow",
            "AccountAchievementsResult",
        ] {
            assert!(
                SCHEMA.contains(&format!("message {message} {{")),
                "{message}"
            );
        }
        for bound in [
            MAX_ACCOUNT_ACHIEVEMENTS_QUERY_BYTES,
            MAX_ACCOUNT_ACHIEVEMENT_ROW_BYTES,
            MAX_ACCOUNT_ACHIEVEMENTS_RESULT_BYTES,
        ] {
            assert!(
                SCHEMA.contains(&format!("At most {bound} encoded bytes")),
                "{bound}"
            );
        }
    }
}
