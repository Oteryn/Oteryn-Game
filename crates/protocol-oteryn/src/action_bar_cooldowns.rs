//! Read-only own-actor item exhaustion for up to three multiaction alternatives.
pub const CAPABILITY_ACTION_BAR_COOLDOWNS_V1: u32 = 463;
pub const COMMAND_TYPE_ACTION_BAR_COOLDOWNS_QUERY: u32 = 6147;
pub const MAX_ITEMS: usize = 3;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemCooldown {
    pub definition: u32,
    pub remaining_ms: u64,
}
fn valid_ids(ids: &[u32]) -> bool {
    ids.len() <= MAX_ITEMS
        && ids.iter().all(|id| *id > 0 && *id < 1 << 28)
        && ids.windows(2).all(|pair| pair[0] < pair[1])
}
pub fn encode_query(ids: &[u32]) -> Option<Vec<u8>> {
    if !valid_ids(ids) {
        return None;
    }
    let mut bytes = vec![u8::try_from(ids.len()).ok()?];
    for id in ids {
        bytes.extend(id.to_le_bytes());
    }
    Some(bytes)
}
pub fn decode_query(bytes: &[u8]) -> Option<Vec<u32>> {
    let count = usize::from(*bytes.first()?);
    if count > MAX_ITEMS || bytes.len() != 1 + count * 4 {
        return None;
    }
    let ids = bytes[1..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| u32::from_le_bytes(*chunk))
        .collect::<Vec<_>>();
    valid_ids(&ids).then_some(ids)
}
pub fn encode_reply(items: &[ItemCooldown]) -> Option<Vec<u8>> {
    let ids = items.iter().map(|item| item.definition).collect::<Vec<_>>();
    if !valid_ids(&ids) {
        return None;
    }
    let mut bytes = vec![u8::try_from(items.len()).ok()?];
    for item in items {
        bytes.extend(item.definition.to_le_bytes());
        bytes.extend(item.remaining_ms.to_le_bytes());
    }
    Some(bytes)
}
pub fn decode_reply(bytes: &[u8]) -> Option<Vec<ItemCooldown>> {
    let count = usize::from(*bytes.first()?);
    if count > MAX_ITEMS || bytes.len() != 1 + count * 12 {
        return None;
    }
    let items = bytes[1..]
        .as_chunks::<12>()
        .0
        .iter()
        .map(|chunk| {
            Some(ItemCooldown {
                definition: u32::from_le_bytes(chunk[..4].try_into().ok()?),
                remaining_ms: u64::from_le_bytes(chunk[4..].try_into().ok()?),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    valid_ids(&items.iter().map(|item| item.definition).collect::<Vec<_>>()).then_some(items)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_item_cooldown_codec_rejects_duplicates_truncation_and_trailing_bytes()
    -> Result<(), &'static str> {
        let ids = [1, 4, 9];
        let query = encode_query(&ids).ok_or("query")?;
        assert_eq!(decode_query(&query), Some(ids.to_vec()));
        assert!(encode_query(&[1, 1]).is_none());
        assert!(encode_query(&[1, 2, 3, 4]).is_none());
        let reply = encode_reply(&[ItemCooldown {
            definition: 1,
            remaining_ms: 1000,
        }])
        .ok_or("reply")?;
        for end in 0..reply.len() {
            assert!(decode_reply(&reply[..end]).is_none());
        }
        let mut trailing = reply;
        trailing.push(0);
        assert!(decode_reply(&trailing).is_none());
        Ok(())
    }
}
