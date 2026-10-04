//! Quest log codecs (QUEST-LOG-WIRE-1).
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/quest_log_v1.proto`, from QUEST-GATE-0 §7.
//! Capability 16 `QUEST_LOG_V1` gates state domain 16 `QUEST_LOG` and command type 22
//! `QUEST_LOG_QUERY`. The server does not offer the capability before the quest log content and
//! the production session copy are composed.
//!
//! Quests and missions are canonical uint32 indexes per content generation. Only the indexes, the
//! completed and done flags, mission names and journal texts (with template counters filled in)
//! leave the server: never a track key, a track value, a transition, a gate or a claim.
//!
//! Decoding is strict, and encoding refuses the same values before any byte is emitted: list
//! entries out of ascending order, missions out of ascending order, a quest index repeated where it
//! must be unique, an empty mission name, invalid UTF-8, an empty or repeated oneof, packed repeated
//! scalars, unknown or repeated fields and any count or byte size over its bound all fail closed.

// The client-side codecs (domain decode, query encode) are exercised by the tests; the server
// composes its own direction in `gameplay_transport`.
#![cfg_attr(not(test), allow(dead_code))]

use std::collections::BTreeSet;

pub use crate::charm_wire::CyclopediaWireError as QuestLogWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_nonzero_varint_field, push_varint, push_varint_field,
    read_bytes, read_uint32, read_varint, set_once,
};

/// Registered capability `QUEST_LOG_V1`: domain 16 and command type 22.
pub const CAPABILITY_QUEST_LOG_V1: u32 = 16;
pub const STATE_DOMAIN_QUEST_LOG: u32 = 16;
pub const SNAPSHOT_TYPE_QUEST_LOG_V1: u32 = 1;
pub const DELTA_TYPE_QUEST_LOG_V1: u32 = 1;
pub const COMMAND_TYPE_QUEST_LOG_QUERY: u32 = 22;

/// `QUESTGATE0-RL-06`: tracked quests of one session (`PARITY_PENDING`).
pub const MAX_TRACKED_QUESTS: usize = 10;
/// `QUESTGATE0-RL-07`: quests per list projection (as `QUESTSTATE0-RL-05`).
pub const MAX_LISTED_QUESTS: usize = 1024;
/// `QUESTGATE0-RL-08`: missions per quest line, measured over the catalogue (87, Killing in the
/// Name Of).
pub const MAX_QUEST_LINE_MISSIONS: usize = 128;
/// `QUESTGATE0-RL-08-NAME`: UTF-8 bytes of one mission name (measured 49).
pub const MAX_MISSION_NAME_BYTES: usize = 128;
/// `QUESTGATE0-RL-08-TEXT`: UTF-8 bytes of one journal text, template counters filled in
/// (measured 463).
pub const MAX_JOURNAL_TEXT_BYTES: usize = 1024;
/// `QUESTGATE0-RL-08-BYTES`: one encoded `QuestLineV1` (measured at most 13,057 over the
/// catalogue). A quest whose worst-case line is larger does not load.
pub const MAX_QUEST_LINE_BYTES: usize = 16_384;
/// `QUESTGATE0-RL-09`: quest log queries per second of one session.
pub const MAX_QUEST_LOG_QUERIES_PER_SECOND: usize = 2;

/// One `QuestLineV1` as an element of a message field: its tag, a 3-byte length and the line.
const MAX_QUEST_LINE_ELEMENT_BYTES: usize = 4 + MAX_QUEST_LINE_BYTES;
/// One `QuestListEntryV1`: the index 1 + 5 and `completed` 1 + 1.
const MAX_LIST_ENTRY_BYTES: usize = 8;
/// One list entry element: its tag, a 1-byte length and the entry.
const MAX_LIST_ENTRY_ELEMENT_BYTES: usize = 2 + MAX_LIST_ENTRY_BYTES;
/// One `QuestListV1`: 1,024 entry elements, 10,240 bytes, within the line bound.
pub const MAX_QUEST_LIST_BYTES: usize = MAX_LISTED_QUESTS * MAX_LIST_ENTRY_ELEMENT_BYTES;
/// `QUESTGATE0-RL-08-DOMAIN-BYTES`: domain 16 snapshot and delta. The view (the list or one
/// line, each within [`MAX_QUEST_LINE_BYTES`]) and 10 tracked lines: 11 x 16,388 = 180,268,
/// within the FND-02 delta and snapshot chunk bounds.
pub const MAX_QUEST_LOG_BYTES: usize = (1 + MAX_TRACKED_QUESTS) * MAX_QUEST_LINE_ELEMENT_BYTES;
/// `QuestLogQueryV1`: the largest query, `track` 1 + 1 + 10 x (1 + 5).
pub const MAX_QUEST_LOG_QUERY_BYTES: usize = 2 + MAX_TRACKED_QUESTS * 6;
/// Command type 22 has no result payload: an accepted query is followed by its domain 16 delta,
/// and a refused one is `REJECTED`.
pub const MAX_QUEST_LOG_QUERY_RESULT_BYTES: usize = 0;

/// `QuestListEntryV1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestListEntry {
    pub quest_index: u32,
    pub completed: bool,
}

/// `QuestMissionV1`: one visible mission of a quest line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestMission {
    /// The mission's index within the revision its quest resolves against.
    pub mission_index: u32,
    /// Non-empty, at most [`MAX_MISSION_NAME_BYTES`].
    pub name: String,
    /// The journal entry for the current value; empty when the mission has none.
    pub text: String,
    /// The mission's track is at its end value.
    pub done: bool,
}

/// `QuestLineV1`: one quest's visible missions in ascending mission index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestLine {
    pub quest_index: u32,
    pub completed: bool,
    pub missions: Vec<QuestMission>,
}

/// The `view` oneof of `QuestLogV1`: the last requested list or quest line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestLogView {
    /// Listed quests in ascending quest index.
    List(Vec<QuestListEntry>),
    Line(QuestLine),
}

/// `QuestLogV1`: domain 16. The default is no view and no tracked quest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestLog {
    pub view: Option<QuestLogView>,
    /// The tracked quests' lines, in the order the client tracked them.
    pub tracked: Vec<QuestLine>,
}

/// `QuestLogQueryV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestLogQuery {
    List,
    Quest {
        quest_index: u32,
    },
    /// Replace the tracked set; at most [`MAX_TRACKED_QUESTS`] distinct indexes, empty for none.
    Track {
        quest_indexes: Vec<u32>,
    },
}

fn malformed<T>() -> WireResult<T> {
    Err(QuestLogWireError::Malformed)
}

fn validate_text(text: &str, maximum: usize) -> WireResult<()> {
    if text.len() > maximum {
        return Err(QuestLogWireError::LimitExceeded);
    }
    Ok(())
}

fn validate_line(line: &QuestLine) -> WireResult<()> {
    if line.missions.len() > MAX_QUEST_LINE_MISSIONS {
        return Err(QuestLogWireError::LimitExceeded);
    }
    for (index, mission) in line.missions.iter().enumerate() {
        validate_text(&mission.name, MAX_MISSION_NAME_BYTES)?;
        validate_text(&mission.text, MAX_JOURNAL_TEXT_BYTES)?;
        if mission.name.is_empty()
            || (index > 0 && line.missions[index - 1].mission_index >= mission.mission_index)
        {
            return malformed();
        }
    }
    Ok(())
}

fn validate_list(entries: &[QuestListEntry]) -> WireResult<()> {
    if entries.len() > MAX_LISTED_QUESTS {
        return Err(QuestLogWireError::LimitExceeded);
    }
    if entries
        .windows(2)
        .any(|pair| pair[0].quest_index >= pair[1].quest_index)
    {
        return malformed();
    }
    Ok(())
}

fn validate_tracked_indexes(indexes: impl Iterator<Item = u32>) -> WireResult<()> {
    let mut seen = BTreeSet::new();
    for index in indexes {
        if seen.len() == MAX_TRACKED_QUESTS {
            return Err(QuestLogWireError::LimitExceeded);
        }
        if !seen.insert(index) {
            return malformed();
        }
    }
    Ok(())
}

fn validate_quest_log(log: &QuestLog) -> WireResult<()> {
    match &log.view {
        Some(QuestLogView::List(entries)) => validate_list(entries)?,
        Some(QuestLogView::Line(line)) => validate_line(line)?,
        None => {}
    }
    validate_tracked_indexes(log.tracked.iter().map(|line| line.quest_index))?;
    log.tracked.iter().try_for_each(validate_line)
}

fn push_string_field(output: &mut Vec<u8>, field: u64, value: &str) {
    if !value.is_empty() {
        push_message_field(output, field, value.as_bytes());
    }
}

/// One `QuestLineV1` body; a line over [`MAX_QUEST_LINE_BYTES`] is refused.
fn encode_line(line: &QuestLine) -> WireResult<Vec<u8>> {
    let mut body = Vec::new();
    push_nonzero_varint_field(&mut body, 1, u64::from(line.quest_index));
    push_nonzero_varint_field(&mut body, 2, u64::from(line.completed));
    for mission in &line.missions {
        let mut element = Vec::new();
        push_nonzero_varint_field(&mut element, 1, u64::from(mission.mission_index));
        push_string_field(&mut element, 2, &mission.name);
        push_string_field(&mut element, 3, &mission.text);
        push_nonzero_varint_field(&mut element, 4, u64::from(mission.done));
        push_message_field(&mut body, 3, &element);
    }
    if body.len() > MAX_QUEST_LINE_BYTES {
        return Err(QuestLogWireError::LimitExceeded);
    }
    Ok(body)
}

fn read_bool(input: &[u8], cursor: &mut usize) -> WireResult<bool> {
    match read_varint(input, cursor)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => malformed(),
    }
}

fn read_string(input: &[u8], cursor: &mut usize, maximum: usize) -> WireResult<String> {
    let bytes = read_bytes(input, cursor)?;
    if bytes.len() > maximum {
        return Err(QuestLogWireError::LimitExceeded);
    }
    String::from_utf8(bytes.to_vec()).map_err(|_| QuestLogWireError::Malformed)
}

fn decode_mission(input: &[u8]) -> WireResult<QuestMission> {
    let mut cursor = 0;
    let (mut index, mut name, mut text, mut done) = (None, None, None, None);
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            0x08 => set_once(&mut index, read_uint32(input, &mut cursor)?)?,
            0x12 => set_once(
                &mut name,
                read_string(input, &mut cursor, MAX_MISSION_NAME_BYTES)?,
            )?,
            0x1a => set_once(
                &mut text,
                read_string(input, &mut cursor, MAX_JOURNAL_TEXT_BYTES)?,
            )?,
            0x20 => set_once(&mut done, read_bool(input, &mut cursor)?)?,
            _ => return malformed(),
        }
    }
    Ok(QuestMission {
        mission_index: index.unwrap_or(0),
        name: name.unwrap_or_default(),
        text: text.unwrap_or_default(),
        done: done.unwrap_or(false),
    })
}

fn decode_line(input: &[u8]) -> WireResult<QuestLine> {
    if input.len() > MAX_QUEST_LINE_BYTES {
        return Err(QuestLogWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let (mut index, mut completed) = (None, None);
    let mut missions = Vec::new();
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            0x08 => set_once(&mut index, read_uint32(input, &mut cursor)?)?,
            0x10 => set_once(&mut completed, read_bool(input, &mut cursor)?)?,
            0x1a => {
                if missions.len() == MAX_QUEST_LINE_MISSIONS {
                    return Err(QuestLogWireError::LimitExceeded);
                }
                missions.push(decode_mission(read_bytes(input, &mut cursor)?)?);
            }
            _ => return malformed(),
        }
    }
    let line = QuestLine {
        quest_index: index.unwrap_or(0),
        completed: completed.unwrap_or(false),
        missions,
    };
    validate_line(&line)?;
    Ok(line)
}

fn decode_list(input: &[u8]) -> WireResult<Vec<QuestListEntry>> {
    if input.len() > MAX_QUEST_LIST_BYTES {
        return Err(QuestLogWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut entries = Vec::new();
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            0x0a => {
                if entries.len() == MAX_LISTED_QUESTS {
                    return Err(QuestLogWireError::LimitExceeded);
                }
                let body = read_bytes(input, &mut cursor)?;
                let mut inner = 0;
                let (mut index, mut completed) = (None, None);
                while inner < body.len() {
                    match read_varint(body, &mut inner)? {
                        0x08 => set_once(&mut index, read_uint32(body, &mut inner)?)?,
                        0x10 => set_once(&mut completed, read_bool(body, &mut inner)?)?,
                        _ => return malformed(),
                    }
                }
                entries.push(QuestListEntry {
                    quest_index: index.unwrap_or(0),
                    completed: completed.unwrap_or(false),
                });
            }
            _ => return malformed(),
        }
    }
    validate_list(&entries)?;
    Ok(entries)
}

/// Domain 16, snapshot type 1 and delta type 1 (the whole domain).
pub fn encode_quest_log(log: &QuestLog) -> WireResult<Vec<u8>> {
    validate_quest_log(log)?;
    let mut output = Vec::new();
    match &log.view {
        Some(QuestLogView::List(entries)) => {
            let mut body = Vec::with_capacity(entries.len() * MAX_LIST_ENTRY_ELEMENT_BYTES);
            for entry in entries {
                let mut element = Vec::with_capacity(MAX_LIST_ENTRY_BYTES);
                push_nonzero_varint_field(&mut element, 1, u64::from(entry.quest_index));
                push_nonzero_varint_field(&mut element, 2, u64::from(entry.completed));
                push_message_field(&mut body, 1, &element);
            }
            push_message_field(&mut output, 1, &body);
        }
        Some(QuestLogView::Line(line)) => push_message_field(&mut output, 2, &encode_line(line)?),
        None => {}
    }
    for line in &log.tracked {
        push_message_field(&mut output, 3, &encode_line(line)?);
    }
    Ok(output)
}

pub fn decode_quest_log(payload: &[u8]) -> WireResult<QuestLog> {
    if payload.len() > MAX_QUEST_LOG_BYTES {
        return Err(QuestLogWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut log = QuestLog::default();
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x0a => set_once(
                &mut log.view,
                QuestLogView::List(decode_list(read_bytes(payload, &mut cursor)?)?),
            )?,
            0x12 => set_once(
                &mut log.view,
                QuestLogView::Line(decode_line(read_bytes(payload, &mut cursor)?)?),
            )?,
            0x1a => {
                if log.tracked.len() == MAX_TRACKED_QUESTS {
                    return Err(QuestLogWireError::LimitExceeded);
                }
                log.tracked
                    .push(decode_line(read_bytes(payload, &mut cursor)?)?);
            }
            _ => return malformed(),
        }
    }
    validate_quest_log(&log)?;
    Ok(log)
}

/// `ClientCommand.payload` of command type 22. More than 10 or repeated track indexes are
/// refused.
pub fn encode_quest_log_query(query: &QuestLogQuery) -> WireResult<Vec<u8>> {
    let mut output = Vec::with_capacity(MAX_QUEST_LOG_QUERY_BYTES);
    match query {
        QuestLogQuery::List => push_message_field(&mut output, 1, &[]),
        QuestLogQuery::Quest { quest_index } => {
            let mut body = Vec::with_capacity(6);
            push_nonzero_varint_field(&mut body, 1, u64::from(*quest_index));
            push_message_field(&mut output, 2, &body);
        }
        QuestLogQuery::Track { quest_indexes } => {
            validate_tracked_indexes(quest_indexes.iter().copied())?;
            // Unpacked: the strict subset refuses packed repeated scalars.
            let mut body = Vec::with_capacity(MAX_TRACKED_QUESTS * 6);
            for index in quest_indexes {
                push_varint_field(&mut body, 1, u64::from(*index));
            }
            push_message_field(&mut output, 3, &body);
        }
    }
    Ok(output)
}

fn decode_track(input: &[u8]) -> WireResult<QuestLogQuery> {
    let mut cursor = 0;
    let mut quest_indexes = Vec::new();
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            0x08 => {
                if quest_indexes.len() == MAX_TRACKED_QUESTS {
                    return Err(QuestLogWireError::LimitExceeded);
                }
                quest_indexes.push(read_uint32(input, &mut cursor)?);
            }
            _ => return malformed(),
        }
    }
    validate_tracked_indexes(quest_indexes.iter().copied())?;
    Ok(QuestLogQuery::Track { quest_indexes })
}

/// An empty or repeated oneof, a packed or repeated index and any unknown field fail closed.
pub fn decode_quest_log_query(payload: &[u8]) -> WireResult<QuestLogQuery> {
    if payload.len() > MAX_QUEST_LOG_QUERY_BYTES {
        return Err(QuestLogWireError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut query = None;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let body = match key {
            0x0a | 0x12 | 0x1a => read_bytes(payload, &mut cursor)?,
            _ => return malformed(),
        };
        let decoded = match key {
            0x0a if body.is_empty() => QuestLogQuery::List,
            0x0a => return malformed(),
            0x12 => {
                let mut inner = 0;
                let mut index = None;
                while inner < body.len() {
                    match read_varint(body, &mut inner)? {
                        0x08 => set_once(&mut index, read_uint32(body, &mut inner)?)?,
                        _ => return malformed(),
                    }
                }
                QuestLogQuery::Quest {
                    quest_index: index.unwrap_or(0),
                }
            }
            _ => decode_track(body)?,
        };
        set_once(&mut query, decoded)?;
    }
    query.ok_or(QuestLogWireError::Malformed)
}

/// The length a `QuestLineV1` of these parts encodes to, at most: the index and flag, and per
/// mission its element with an index of up to 5 bytes. The catalogue bounds each quest's worst
/// case by [`MAX_QUEST_LINE_BYTES`] with it.
#[must_use]
pub fn worst_case_line_bytes(missions: impl Iterator<Item = (usize, usize)>) -> usize {
    let mut total = 6 + 2;
    for (name, text) in missions {
        let mut element = 6 + string_field_bytes(name) + string_field_bytes(text) + 2;
        element += 1 + varint_len(element);
        total += element;
    }
    total
}

fn string_field_bytes(len: usize) -> usize {
    if len == 0 {
        0
    } else {
        1 + varint_len(len) + len
    }
}

fn varint_len(value: usize) -> usize {
    let mut bytes = Vec::with_capacity(10);
    push_varint(&mut bytes, value as u64);
    bytes.len()
}

#[cfg(test)]
#[path = "quest_log_tests.rs"]
mod tests;
