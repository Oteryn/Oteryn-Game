//! WEAPPROF3 bounded Weapon Proficiency command wire contract.

pub const CAPABILITY_WEAPON_PROFICIENCY_V1: u32 = 188;
pub const COMMAND_TYPE_WEAPON_PROFICIENCY_QUERY: u32 = 480;
pub const COMMAND_TYPE_WEAPON_PROFICIENCY_SELECT: u32 = 481;
pub const COMMAND_TYPE_WEAPON_PROFICIENCY_REFINE: u32 = 482;
pub const COMMAND_TYPE_WEAPON_PROFICIENCY_RESHAPE: u32 = 483;
pub const MAX_WEAPON_PROFICIENCY_PAYLOAD_BYTES: usize = 192;
pub const MAX_WEAPON_PROFICIENCY_REPLY_BYTES: usize = 8192;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub item_key: String,
    pub revision: u64,
    pub progress: u64,
    pub selections: Vec<Option<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Query,
    Select {
        occurrence: [u8; 16],
        item_key: String,
        track_revision: u64,
        level: u8,
        perk: u8,
    },
    Refine {
        occurrence: [u8; 16],
        item_key: String,
        track_revision: u64,
        slot: u8,
    },
    Reshape {
        occurrence: [u8; 16],
        item_key: String,
        track_revision: u64,
        slot: u8,
        choice: Option<u8>,
    },
}

fn decode_key(input: &mut &[u8]) -> Option<String> {
    let (&len, tail) = input.split_first()?;
    let (key, rest) = tail.split_at_checked(usize::from(len))?;
    *input = rest;
    let key = std::str::from_utf8(key).ok()?;
    ((1..=128).contains(&key.len())
        && key.starts_with("oteryn:item.")
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-')))
    .then(|| key.to_owned())
}

pub fn decode_request(command: u32, payload: &[u8]) -> Option<Request> {
    if payload.len() > MAX_WEAPON_PROFICIENCY_PAYLOAD_BYTES {
        return None;
    }
    if command == COMMAND_TYPE_WEAPON_PROFICIENCY_QUERY {
        return payload.is_empty().then_some(Request::Query);
    }
    let (occurrence, mut rest) = payload.split_at_checked(16)?;
    let occurrence: [u8; 16] = occurrence.try_into().ok()?;
    let item_key = decode_key(&mut rest)?;
    let (revision, tail) = rest.split_at_checked(8)?;
    let track_revision = u64::from_le_bytes(revision.try_into().ok()?);
    match (command, tail) {
        (COMMAND_TYPE_WEAPON_PROFICIENCY_SELECT, &[level, perk]) if level < 7 && perk < 3 => {
            Some(Request::Select {
                occurrence,
                item_key,
                track_revision,
                level,
                perk,
            })
        }
        (COMMAND_TYPE_WEAPON_PROFICIENCY_REFINE, &[slot]) if (1..=2).contains(&slot) => {
            Some(Request::Refine {
                occurrence,
                item_key,
                track_revision,
                slot,
            })
        }
        (COMMAND_TYPE_WEAPON_PROFICIENCY_RESHAPE, &[slot, choice])
            if (1..=2).contains(&slot) && choice <= 3 =>
        {
            Some(Request::Reshape {
                occurrence,
                item_key,
                track_revision,
                slot,
                choice: (choice != 3).then_some(choice),
            })
        }
        _ => None,
    }
}

pub fn encode_result(status: u8, revision: u64, payload: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(9 + payload.len());
    out.push(status);
    out.extend_from_slice(&revision.to_le_bytes());
    out.extend_from_slice(payload);
    (out.len() <= MAX_WEAPON_PROFICIENCY_REPLY_BYTES).then_some(out)
}

/// Encode the bounded query body. Integers are little endian and selections use `0xff` for an
/// empty level; keeping this codec here prevents the server and native client from inventing
/// parallel representations.
pub fn encode_tracks(tracks: &[Track]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(&u16::try_from(tracks.len()).ok()?.to_le_bytes());
    for track in tracks {
        let key = track.item_key.as_bytes();
        out.push(u8::try_from(key.len()).ok()?);
        out.extend_from_slice(key);
        out.extend_from_slice(&track.revision.to_le_bytes());
        out.extend_from_slice(&track.progress.to_le_bytes());
        out.push(u8::try_from(track.selections.len()).ok()?);
        out.extend(
            track
                .selections
                .iter()
                .map(|choice| choice.unwrap_or(u8::MAX)),
        );
    }
    (out.len() + 9 <= MAX_WEAPON_PROFICIENCY_REPLY_BYTES).then_some(out)
}

pub fn decode_tracks(mut input: &[u8]) -> Option<Vec<Track>> {
    let (count, tail) = input.split_at_checked(2)?;
    input = tail;
    let count = u16::from_le_bytes(count.try_into().ok()?);
    let mut tracks = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        let item_key = decode_key(&mut input)?;
        let (revision, tail) = input.split_at_checked(8)?;
        let revision = u64::from_le_bytes(revision.try_into().ok()?);
        let (progress, tail) = tail.split_at_checked(8)?;
        let progress = u64::from_le_bytes(progress.try_into().ok()?);
        let (&selection_count, tail) = tail.split_first()?;
        let (selections, tail) = tail.split_at_checked(usize::from(selection_count))?;
        input = tail;
        let selections = selections
            .iter()
            .copied()
            .map(|choice| (choice != u8::MAX).then_some(choice))
            .collect();
        tracks.push(Track {
            item_key,
            revision,
            progress,
            selections,
        });
    }
    input.is_empty().then_some(tracks)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mutation(command: u32, suffix: &[u8]) -> Option<Request> {
        let mut bytes = vec![0; 16];
        let key = b"oteryn:item.tibia.i3295";
        bytes.push(key.len() as u8);
        bytes.extend(key);
        bytes.extend(9_u64.to_le_bytes());
        bytes.extend(suffix);
        decode_request(command, &bytes)
    }
    #[test]
    fn bounded_commands_round_trip_and_reject_trailing_data() {
        assert_eq!(decode_request(480, &[]), Some(Request::Query));
        assert!(matches!(
            mutation(481, &[6, 2]),
            Some(Request::Select {
                level: 6,
                perk: 2,
                ..
            })
        ));
        assert!(matches!(
            mutation(482, &[1]),
            Some(Request::Refine { slot: 1, .. })
        ));
        assert!(matches!(
            mutation(483, &[2, 3]),
            Some(Request::Reshape { choice: None, .. })
        ));
        assert!(decode_request(480, &[0]).is_none());
        assert!(encode_result(0, 7, &[0; 8184]).is_none());
        let tracks = vec![Track {
            item_key: "oteryn:item.tibia.i3295".into(),
            revision: 9,
            progress: 12_345,
            selections: vec![Some(2), None],
        }];
        let encoded = encode_tracks(&tracks);
        assert!(encoded.is_some());
        let Some(encoded) = encoded else { return };
        assert_eq!(decode_tracks(&encoded), Some(tracks));
    }
}
