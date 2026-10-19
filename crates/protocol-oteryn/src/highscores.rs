//! HS-WIRE-1 payload codecs under HIGHSCORES0 §6 (capability 9, command type 14).
//!
//! The query contains category, base-vocation filter, page and snapshot token only. World and
//! asking Character come from the admitted session, never from this payload. The result carries
//! one persisted page and an optional own row; hidden progress and source identities stay private.
//! Rows preserve the durable vocation key, including promotions and `none` for account rankings.
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/highscores_v1.proto`. RANK-1 (CP D1155) registers
//! capability 9 (offered) and command type 14, and the game server's session handler answers it.
//! NOT_SUPPORTED (1200) and SNAPSHOT_EXPIRED (1201) are FND-02 error results without a success
//! payload ([`crate::FoundationProtocolError`]), not a second status field inside this message.
//!
//! As in the existing native payload codecs, unknown or repeated singular fields, wrong wire
//! types, invalid UTF-8, invalid semantic defaults and values above a bound fail closed. Encoding
//! validates the same rules before returning any bytes. This code never computes ranks or treats
//! protobuf bytes as command identity.

pub use crate::charm_wire::CyclopediaWireError as HighscoresWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_nonzero_varint_field, read_bytes, read_uint32,
    read_varint, set_once,
};

pub const CAPABILITY_HIGHSCORES_V1: u32 = 9;
pub const COMMAND_TYPE_HIGHSCORES_QUERY_V1: u32 = 14;
/// HIGHSCORES0-RL-02: each filter has its own capped list; global ranks can exceed this bound.
pub const MAX_HIGHSCORES_LIST_ROWS: u32 = 1000;
/// HIGHSCORES0-RL-03.
pub const MAX_HIGHSCORES_PAGE_ROWS: usize = 50;
/// CHAR-NAME-1 policy revision 1: 2..=29 ASCII letters with single internal spaces.
pub const MAX_HIGHSCORE_NAME_BYTES: usize = 29;
/// The current durable build vocation-key bound (`character_progression::valid_revision`).
pub const MAX_HIGHSCORE_VOCATION_BYTES: usize = 128;
/// Three uint32 fields (6, 2, 6 bytes) and one uint64 field (11 bytes).
pub const MAX_HIGHSCORES_QUERY_BYTES: usize = 25;
/// Rank (11), name (31), vocation (131), level (6) and primary value (11).
pub const MAX_HIGHSCORE_ROW_BYTES: usize = 190;
/// Scalar upper bounds (35), 50 page rows and one own row, each with a tag and two-byte length.
pub const MAX_HIGHSCORES_RESULT_BYTES: usize = 35 + 51 * (3 + MAX_HIGHSCORE_ROW_BYTES);

const MAX_LIST_PAGE: u32 = MAX_HIGHSCORES_LIST_ROWS / MAX_HIGHSCORES_PAGE_ROWS as u32;
const _: () = assert!(MAX_HIGHSCORES_QUERY_BYTES <= crate::MAX_COMMAND_PAYLOAD_BYTES);
const _: () = assert!(MAX_HIGHSCORES_RESULT_BYTES <= crate::MAX_COMMAND_RESULT_PAYLOAD_BYTES);

/// Query tags 1..=4, all unsigned varints. Unsupported nonzero categories reach the owning
/// query handler so it can return NOT_SUPPORTED; the codec does not infer available sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighscoresQuery {
    pub category: u32,
    /// 0 all, 1 knight, 2 paladin, 3 sorcerer, 4 druid, 5 monk (base vocation).
    pub vocation: u32,
    pub page: u32,
    /// Zero only for page 1; subsequent pages echo the returned opaque token.
    pub snapshot_id: u64,
}

/// Row tags 1..=5: uint64, string, string, uint32, uint64.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighscoreRow {
    pub rank: u64,
    pub name: String,
    pub vocation: String,
    pub level: u32,
    /// The primary value only; skill/magic progress breaks ties inside the snapshot store.
    pub value: u64,
}

/// Result tags 1..=7: uint32, uint64, int64, uint32, bool, repeated row, optional row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighscoresResult {
    pub category: u32,
    pub snapshot_id: u64,
    /// Unix milliseconds from the persisted snapshot cut, not client time or authority.
    pub computed_at: i64,
    pub page: u32,
    pub has_more: bool,
    pub rows: Vec<HighscoreRow>,
    /// May be outside this page, including on a page beyond the end of the selected list.
    pub own_row: Option<HighscoreRow>,
}

fn malformed<T>() -> WireResult<T> {
    Err(HighscoresWireError::Malformed)
}

fn check_query(query: HighscoresQuery) -> WireResult<()> {
    if query.category == 0
        || query.vocation > 5
        || query.page == 0
        || (query.page > 1 && query.snapshot_id == 0)
    {
        return malformed();
    }
    Ok(())
}

fn check_row(row: &HighscoreRow) -> WireResult<()> {
    if row.name.len() > MAX_HIGHSCORE_NAME_BYTES
        || row.vocation.len() > MAX_HIGHSCORE_VOCATION_BYTES
    {
        return Err(HighscoresWireError::LimitExceeded);
    }
    let mut vocation = row.vocation.bytes();
    if row.rank == 0
        || row.level == 0
        || row.name.len() < 2
        || !row
            .name
            .bytes()
            .all(|byte| byte.is_ascii_alphabetic() || byte == b' ')
        || row.name.split(' ').any(str::is_empty)
        || !vocation
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        || !vocation.all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
    {
        return malformed();
    }
    Ok(())
}

fn check_result(result: &HighscoresResult) -> WireResult<()> {
    if result.rows.len() > MAX_HIGHSCORES_PAGE_ROWS {
        return Err(HighscoresWireError::LimitExceeded);
    }
    if result.category == 0
        || result.snapshot_id == 0
        || result.computed_at < 0
        || result.page == 0
        || (result.page > MAX_LIST_PAGE && !result.rows.is_empty())
        || (result.has_more
            && (result.rows.len() != MAX_HIGHSCORES_PAGE_ROWS || result.page >= MAX_LIST_PAGE))
    {
        return malformed();
    }
    result.rows.iter().try_for_each(check_row)?;
    if let Some(own_row) = &result.own_row {
        check_row(own_row)?;
    }
    Ok(())
}

pub fn encode_highscores_query(query: HighscoresQuery) -> WireResult<Vec<u8>> {
    check_query(query)?;
    let mut output = Vec::with_capacity(MAX_HIGHSCORES_QUERY_BYTES);
    push_nonzero_varint_field(&mut output, 1, u64::from(query.category));
    push_nonzero_varint_field(&mut output, 2, u64::from(query.vocation));
    push_nonzero_varint_field(&mut output, 3, u64::from(query.page));
    push_nonzero_varint_field(&mut output, 4, query.snapshot_id);
    Ok(output)
}

pub fn decode_highscores_query(payload: &[u8]) -> WireResult<HighscoresQuery> {
    if payload.len() > MAX_HIGHSCORES_QUERY_BYTES {
        return Err(HighscoresWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let (mut category, mut vocation, mut page, mut snapshot_id) = (None, None, None, None);
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x08 => set_once(&mut category, read_uint32(payload, &mut cursor)?)?,
            0x10 => set_once(&mut vocation, read_uint32(payload, &mut cursor)?)?,
            0x18 => set_once(&mut page, read_uint32(payload, &mut cursor)?)?,
            0x20 => set_once(&mut snapshot_id, read_varint(payload, &mut cursor)?)?,
            _ => return malformed(),
        }
    }
    let query = HighscoresQuery {
        category: category.unwrap_or(0),
        vocation: vocation.unwrap_or(0),
        page: page.unwrap_or(0),
        snapshot_id: snapshot_id.unwrap_or(0),
    };
    check_query(query)?;
    Ok(query)
}

fn encode_row(output: &mut Vec<u8>, field: u64, row: &HighscoreRow) {
    let mut body = Vec::with_capacity(MAX_HIGHSCORE_ROW_BYTES);
    push_nonzero_varint_field(&mut body, 1, row.rank);
    push_message_field(&mut body, 2, row.name.as_bytes());
    push_message_field(&mut body, 3, row.vocation.as_bytes());
    push_nonzero_varint_field(&mut body, 4, u64::from(row.level));
    push_nonzero_varint_field(&mut body, 5, row.value);
    push_message_field(output, field, &body);
}

pub fn encode_highscores_result(result: &HighscoresResult) -> WireResult<Vec<u8>> {
    check_result(result)?;
    let mut output = Vec::new();
    push_nonzero_varint_field(&mut output, 1, u64::from(result.category));
    push_nonzero_varint_field(&mut output, 2, result.snapshot_id);
    push_nonzero_varint_field(&mut output, 3, result.computed_at as u64);
    push_nonzero_varint_field(&mut output, 4, u64::from(result.page));
    push_nonzero_varint_field(&mut output, 5, u64::from(result.has_more));
    for row in &result.rows {
        encode_row(&mut output, 6, row);
    }
    if let Some(own_row) = &result.own_row {
        encode_row(&mut output, 7, own_row);
    }
    Ok(output)
}

fn read_text(input: &[u8], cursor: &mut usize, maximum: usize) -> WireResult<String> {
    let bytes = read_bytes(input, cursor)?;
    if bytes.len() > maximum {
        return Err(HighscoresWireError::LimitExceeded);
    }
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| HighscoresWireError::Malformed)
}

fn decode_row(input: &[u8]) -> WireResult<HighscoreRow> {
    if input.len() > MAX_HIGHSCORE_ROW_BYTES {
        return Err(HighscoresWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let (mut rank, mut name, mut vocation, mut level, mut value) = (None, None, None, None, None);
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            0x08 => set_once(&mut rank, read_varint(input, &mut cursor)?)?,
            0x12 => set_once(
                &mut name,
                read_text(input, &mut cursor, MAX_HIGHSCORE_NAME_BYTES)?,
            )?,
            0x1a => set_once(
                &mut vocation,
                read_text(input, &mut cursor, MAX_HIGHSCORE_VOCATION_BYTES)?,
            )?,
            0x20 => set_once(&mut level, read_uint32(input, &mut cursor)?)?,
            0x28 => set_once(&mut value, read_varint(input, &mut cursor)?)?,
            _ => return malformed(),
        }
    }
    let row = HighscoreRow {
        rank: rank.unwrap_or(0),
        name: name.unwrap_or_default(),
        vocation: vocation.unwrap_or_default(),
        level: level.unwrap_or(0),
        value: value.unwrap_or(0),
    };
    check_row(&row)?;
    Ok(row)
}

pub fn decode_highscores_result(payload: &[u8]) -> WireResult<HighscoresResult> {
    if payload.len() > MAX_HIGHSCORES_RESULT_BYTES {
        return Err(HighscoresWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let (mut category, mut snapshot_id, mut computed_at, mut page, mut has_more) =
        (None, None, None, None, None);
    let mut rows = Vec::new();
    let mut own_row = None;
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x08 => set_once(&mut category, read_uint32(payload, &mut cursor)?)?,
            0x10 => set_once(&mut snapshot_id, read_varint(payload, &mut cursor)?)?,
            0x18 => set_once(&mut computed_at, read_varint(payload, &mut cursor)? as i64)?,
            0x20 => set_once(&mut page, read_uint32(payload, &mut cursor)?)?,
            0x28 => {
                let value = match read_varint(payload, &mut cursor)? {
                    0 => false,
                    1 => true,
                    _ => return malformed(),
                };
                set_once(&mut has_more, value)?;
            }
            0x32 => {
                if rows.len() == MAX_HIGHSCORES_PAGE_ROWS {
                    return Err(HighscoresWireError::LimitExceeded);
                }
                rows.push(decode_row(read_bytes(payload, &mut cursor)?)?);
            }
            0x3a => set_once(&mut own_row, decode_row(read_bytes(payload, &mut cursor)?)?)?,
            _ => return malformed(),
        }
    }
    let result = HighscoresResult {
        category: category.unwrap_or(0),
        snapshot_id: snapshot_id.unwrap_or(0),
        computed_at: computed_at.unwrap_or(0),
        page: page.unwrap_or(0),
        has_more: has_more.unwrap_or(false),
        rows,
        own_row,
    };
    check_result(&result)?;
    Ok(result)
}

#[cfg(test)]
#[path = "highscores_tests.rs"]
mod tests;
