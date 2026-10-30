//! Cyclopedia Bestiary/Bosstiary tracker commands 383-384.
pub const CAPABILITY_CYCLOPEDIA_TRACKER_V1: u32 = 197;
pub const COMMAND_TYPE_BESTIARY_TRACKER: u32 = 383;
pub const COMMAND_TYPE_BOSSTIARY_TRACKER: u32 = 384;
const MAX_KEY_BYTES: usize = 128;
const MAX_TRACKED: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerAction {
    Remove,
    Add,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackerRequest {
    pub action: TrackerAction,
    pub creature_key: String,
}

pub fn decode_request(bytes: &[u8]) -> Option<TrackerRequest> {
    let (&action, key) = bytes.split_first()?;
    if key.is_empty() || key.len() > MAX_KEY_BYTES {
        return None;
    }
    let creature_key = std::str::from_utf8(key).ok()?;
    if !creature_key
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b":._/-".contains(&byte))
    {
        return None;
    }
    Some(TrackerRequest {
        action: match action {
            0 => TrackerAction::Remove,
            1 => TrackerAction::Add,
            _ => return None,
        },
        creature_key: creature_key.to_owned(),
    })
}

pub fn encode_request(request: &TrackerRequest) -> Option<Vec<u8>> {
    if request.creature_key.is_empty() || request.creature_key.len() > MAX_KEY_BYTES {
        return None;
    }
    let mut out = Vec::with_capacity(request.creature_key.len() + 1);
    out.push(match request.action {
        TrackerAction::Remove => 0,
        TrackerAction::Add => 1,
    });
    out.extend_from_slice(request.creature_key.as_bytes());
    decode_request(&out).map(|_| out)
}

pub fn encode_snapshot(keys: &[String]) -> Option<Vec<u8>> {
    if keys.len() > MAX_TRACKED {
        return None;
    }
    let mut out = Vec::new();
    out.extend_from_slice(&u16::try_from(keys.len()).ok()?.to_be_bytes());
    for key in keys {
        let len = u8::try_from(key.len()).ok()?;
        out.push(len);
        out.extend_from_slice(key.as_bytes());
    }
    Some(out)
}

pub fn decode_snapshot(mut bytes: &[u8]) -> Option<Vec<String>> {
    let count = usize::from(u16::from_be_bytes(bytes.get(..2)?.try_into().ok()?));
    if count > MAX_TRACKED {
        return None;
    }
    bytes = &bytes[2..];
    let mut keys = Vec::with_capacity(count);
    for _ in 0..count {
        let (&len, tail) = bytes.split_first()?;
        let len = usize::from(len);
        let key = std::str::from_utf8(tail.get(..len)?).ok()?.to_owned();
        keys.push(key);
        bytes = &tail[len..];
    }
    if bytes.is_empty() { Some(keys) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_request_and_refresh_snapshot_are_canonical() -> Result<(), String> {
        let request = TrackerRequest {
            action: TrackerAction::Add,
            creature_key: "oteryn:creature.rotworm".to_owned(),
        };
        let encoded = encode_request(&request).ok_or("request encoding")?;
        assert_eq!(decode_request(&encoded), Some(request));
        let keys = vec![
            "oteryn:creature.dragon".to_owned(),
            "oteryn:creature.rotworm".to_owned(),
        ];
        assert_eq!(
            decode_snapshot(&encode_snapshot(&keys).ok_or("snapshot encoding")?),
            Some(keys)
        );
        assert!(decode_request(&[2, b'x']).is_none());
        Ok(())
    }
}
