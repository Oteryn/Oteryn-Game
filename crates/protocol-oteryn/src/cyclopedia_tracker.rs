//! Cyclopedia Bestiary/Bosstiary tracker commands 383-384.
//!
//! Crystal `ProtocolGame::parseCyclopediaMonsterTracker` (protocolgame.cpp:2782) reads a race
//! and an enabled byte; Oteryn names the creature by its canonical key. An empty payload reads
//! the stored list, as the hazard command does. Every accepted result is the refreshed tracker
//! in the shape of Crystal `refreshCyclopediaMonsterTracker` (protocolgame.cpp:3171): kind,
//! then per entry the kill count, the three stage thresholds and the completed flag.
pub const CAPABILITY_CYCLOPEDIA_TRACKER_V1: u32 = 197;
pub const COMMAND_TYPE_BESTIARY_TRACKER: u32 = 383;
pub const COMMAND_TYPE_BOSSTIARY_TRACKER: u32 = 384;
/// Pushes both trackers on join and one tracker after a kill credits a creature on it.
pub const STATE_DOMAIN_CYCLOPEDIA_TRACKER: u32 = 98;
/// Snapshot: [`encode_domain_snapshot`].
pub const SNAPSHOT_TYPE_CYCLOPEDIA_TRACKER_V1: u32 = 1;
/// Delta: one [`encode_snapshot`] tracker.
pub const DELTA_TYPE_CYCLOPEDIA_TRACKER_V1: u32 = 1;
/// The longest creature key, as the Bestiary race key check allows.
pub const MAX_KEY_BYTES: usize = 128;
/// The Bestiary race list bound (`bestiary::MAX_BESTIARY_VIEW_ENTRIES`); no gameplay cap.
pub const MAX_TRACKED: usize = crate::bestiary::MAX_BESTIARY_VIEW_ENTRIES;
/// Kill count, three thresholds and the completed byte of one entry, after its key.
const ENTRY_FIXED_BYTES: usize = 1 + 4 + 3 * 4 + 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerAction {
    Remove,
    Add,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackerRequest {
    Read,
    Change {
        action: TrackerAction,
        creature_key: String,
    },
}

/// One tracked creature as the refresh shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackerEntry {
    pub creature_key: String,
    pub kill_count: u32,
    pub thresholds: [u32; 3],
    pub completed: bool,
}

/// The refreshed tracker; `boss` is Crystal's `isBoss` byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackerSnapshot {
    pub boss: bool,
    pub entries: Vec<TrackerEntry>,
}

fn valid_key(key: &[u8]) -> bool {
    !key.is_empty()
        && key.len() <= MAX_KEY_BYTES
        && key
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || b":._/-".contains(byte))
}

/// Whether a tracker of `count` entries whose keys total `key_bytes` encodes within one
/// command result. The durable owner refuses an add past it.
#[must_use]
pub const fn snapshot_fits(count: usize, key_bytes: usize) -> bool {
    count <= MAX_TRACKED
        && 3 + count * ENTRY_FIXED_BYTES + key_bytes <= crate::MAX_COMMAND_RESULT_PAYLOAD_BYTES
}

pub fn decode_request(bytes: &[u8]) -> Option<TrackerRequest> {
    let Some((&action, key)) = bytes.split_first() else {
        return Some(TrackerRequest::Read);
    };
    if !valid_key(key) {
        return None;
    }
    let action = match action {
        0 => TrackerAction::Remove,
        1 => TrackerAction::Add,
        _ => return None,
    };
    Some(TrackerRequest::Change {
        action,
        creature_key: std::str::from_utf8(key).ok()?.to_owned(),
    })
}

pub fn encode_request(request: &TrackerRequest) -> Option<Vec<u8>> {
    let TrackerRequest::Change {
        action,
        creature_key,
    } = request
    else {
        return Some(Vec::new());
    };
    if !valid_key(creature_key.as_bytes()) {
        return None;
    }
    let mut out = Vec::with_capacity(creature_key.len() + 1);
    out.push(match action {
        TrackerAction::Remove => 0,
        TrackerAction::Add => 1,
    });
    out.extend_from_slice(creature_key.as_bytes());
    Some(out)
}

pub fn encode_snapshot(snapshot: &TrackerSnapshot) -> Option<Vec<u8>> {
    let entries = &snapshot.entries;
    let key_bytes = entries.iter().map(|entry| entry.creature_key.len()).sum();
    if !snapshot_fits(entries.len(), key_bytes) {
        return None;
    }
    let mut out = Vec::with_capacity(3 + entries.len() * ENTRY_FIXED_BYTES + key_bytes);
    out.push(u8::from(snapshot.boss));
    out.extend_from_slice(&u16::try_from(entries.len()).ok()?.to_be_bytes());
    for entry in entries {
        if !valid_key(entry.creature_key.as_bytes()) {
            return None;
        }
        out.push(u8::try_from(entry.creature_key.len()).ok()?);
        out.extend_from_slice(entry.creature_key.as_bytes());
        out.extend_from_slice(&entry.kill_count.to_be_bytes());
        for threshold in entry.thresholds {
            out.extend_from_slice(&threshold.to_be_bytes());
        }
        out.push(u8::from(entry.completed));
    }
    Some(out)
}

/// The domain snapshot: the Bestiary tracker followed by the Bosstiary tracker.
pub fn encode_domain_snapshot(
    bestiary: &TrackerSnapshot,
    bosstiary: &TrackerSnapshot,
) -> Option<Vec<u8>> {
    if bestiary.boss || !bosstiary.boss {
        return None;
    }
    let mut out = encode_snapshot(bestiary)?;
    out.extend(encode_snapshot(bosstiary)?);
    Some(out)
}

pub fn decode_domain_snapshot(bytes: &[u8]) -> Option<(TrackerSnapshot, TrackerSnapshot)> {
    let mut rest = bytes;
    let bestiary = decode_one(&mut rest)?;
    let bosstiary = decode_one(&mut rest)?;
    (rest.is_empty() && !bestiary.boss && bosstiary.boss).then_some((bestiary, bosstiary))
}

fn take<'a>(bytes: &mut &'a [u8], len: usize) -> Option<&'a [u8]> {
    let (head, tail) = bytes.split_at_checked(len)?;
    *bytes = tail;
    Some(head)
}

fn take_u32(bytes: &mut &[u8]) -> Option<u32> {
    Some(u32::from_be_bytes(take(bytes, 4)?.try_into().ok()?))
}

pub fn decode_snapshot(mut bytes: &[u8]) -> Option<TrackerSnapshot> {
    let snapshot = decode_one(&mut bytes)?;
    bytes.is_empty().then_some(snapshot)
}

fn decode_one(bytes: &mut &[u8]) -> Option<TrackerSnapshot> {
    let boss = match take(bytes, 1)? {
        [0] => false,
        [1] => true,
        _ => return None,
    };
    let count = usize::from(u16::from_be_bytes(take(bytes, 2)?.try_into().ok()?));
    if count > MAX_TRACKED {
        return None;
    }
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let len = usize::from(*take(bytes, 1)?.first()?);
        let key = take(bytes, len)?;
        if !valid_key(key) {
            return None;
        }
        let creature_key = std::str::from_utf8(key).ok()?.to_owned();
        let kill_count = take_u32(bytes)?;
        let thresholds = [take_u32(bytes)?, take_u32(bytes)?, take_u32(bytes)?];
        let completed = match take(bytes, 1)? {
            [0] => false,
            [1] => true,
            _ => return None,
        };
        entries.push(TrackerEntry {
            creature_key,
            kill_count,
            thresholds,
            completed,
        });
    }
    Some(TrackerSnapshot { boss, entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, kill_count: u32, completed: bool) -> TrackerEntry {
        TrackerEntry {
            creature_key: key.to_owned(),
            kill_count,
            thresholds: [5, 20, 60],
            completed,
        }
    }

    #[test]
    fn tracker_request_and_refresh_snapshot_are_canonical() -> Result<(), String> {
        let request = TrackerRequest::Change {
            action: TrackerAction::Add,
            creature_key: "oteryn:creature.rotworm".to_owned(),
        };
        let encoded = encode_request(&request).ok_or("request encoding")?;
        assert_eq!(decode_request(&encoded), Some(request));
        assert_eq!(decode_request(&[]), Some(TrackerRequest::Read));
        assert_eq!(encode_request(&TrackerRequest::Read), Some(Vec::new()));
        let snapshot = TrackerSnapshot {
            boss: true,
            entries: vec![
                entry("oteryn:creature.dragon", 3, false),
                entry("oteryn:creature.rotworm", 60, true),
            ],
        };
        let bytes = encode_snapshot(&snapshot).ok_or("snapshot encoding")?;
        assert_eq!(bytes[..3], [1, 0, 2]);
        assert_eq!(decode_snapshot(&bytes), Some(snapshot));
        Ok(())
    }

    #[test]
    fn domain_snapshot_carries_both_trackers_in_order() -> Result<(), String> {
        let bestiary = TrackerSnapshot {
            boss: false,
            entries: vec![entry("oteryn:creature.rotworm", 7, false)],
        };
        let bosstiary = TrackerSnapshot {
            boss: true,
            entries: Vec::new(),
        };
        let bytes = encode_domain_snapshot(&bestiary, &bosstiary).ok_or("domain snapshot")?;
        assert_eq!(
            decode_domain_snapshot(&bytes),
            Some((bestiary.clone(), bosstiary.clone()))
        );
        assert!(encode_domain_snapshot(&bosstiary, &bestiary).is_none());
        assert!(decode_snapshot(&bytes).is_none());
        Ok(())
    }

    #[test]
    fn tracker_wire_refuses_malformed_input() {
        assert!(decode_request(&[2, b'x']).is_none());
        assert!(decode_request(&[1]).is_none());
        assert!(decode_request(&[1, b' ']).is_none());
        assert!(decode_request(&[1; MAX_KEY_BYTES + 2]).is_none());
        assert!(decode_snapshot(&[2, 0, 0]).is_none());
        assert!(decode_snapshot(&[0, 0, 0, 0]).is_none());
        assert!(decode_snapshot(&[0, 0, 1]).is_none());
        let too_many = TrackerSnapshot {
            boss: false,
            entries: vec![entry("oteryn:creature.a", 0, false); MAX_TRACKED + 1],
        };
        assert!(encode_snapshot(&too_many).is_none());
    }

    #[test]
    fn largest_admitted_tracker_fits_one_command_result() {
        let key = "k".repeat(MAX_KEY_BYTES);
        let mut count = 0;
        while snapshot_fits(count + 1, (count + 1) * key.len()) {
            count += 1;
        }
        let snapshot = TrackerSnapshot {
            boss: false,
            entries: vec![entry(&key, 1, false); count],
        };
        let bytes = encode_snapshot(&snapshot).unwrap_or_default();
        assert!(!bytes.is_empty());
        assert!(bytes.len() <= crate::MAX_COMMAND_RESULT_PAYLOAD_BYTES);
        assert!(!snapshot_fits(MAX_TRACKED + 1, 0));
    }
}
