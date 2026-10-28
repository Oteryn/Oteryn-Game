//! ANIMUS-1 bounded query wire values.

pub const CAPABILITY_ANIMUS_MASTERY_V1: u32 = 168;
pub const COMMAND_TYPE_ANIMUS_MASTERY_QUERY: u32 = 232;
pub const MAX_ANIMUS_QUERY_BYTES: usize = 4;
pub const MAX_ANIMUS_REPLY_BYTES: usize = 8_192;
/// 50 keys of at most 160 bytes plus their length bytes fit the 8192-byte reply bound.
pub const MAX_ANIMUS_PAGE_KEYS: u8 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimusPage {
    pub offset: u16,
    pub limit: u8,
}

pub fn decode_query(payload: &[u8]) -> Option<AnimusPage> {
    let [0, low, high, limit] = payload else {
        return None;
    };
    (1..=MAX_ANIMUS_PAGE_KEYS)
        .contains(limit)
        .then_some(AnimusPage {
            offset: u16::from_le_bytes([*low, *high]),
            limit: *limit,
        })
}

pub fn encode_page(total: u16, keys: &[String]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(4 + keys.iter().map(|key| key.len() + 1).sum::<usize>());
    out.extend_from_slice(&total.to_le_bytes());
    out.push(u8::try_from(keys.len()).ok()?);
    for key in keys {
        out.push(u8::try_from(key.len()).ok()?);
        out.extend_from_slice(key.as_bytes());
    }
    (out.len() <= MAX_ANIMUS_REPLY_BYTES).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_and_page_are_bounded() {
        assert_eq!(
            decode_query(&[0, 2, 0, 25]),
            Some(AnimusPage {
                offset: 2,
                limit: 25
            })
        );
        assert!(decode_query(&[0, 0, 0, 0]).is_none());
        assert!(decode_query(&[0, 0, 0, 51]).is_none());
        let widest = vec!["a".repeat(160); usize::from(MAX_ANIMUS_PAGE_KEYS)];
        assert!(encode_page(u16::from(MAX_ANIMUS_PAGE_KEYS), &widest).is_some());
        let encoded = encode_page(1, &["oteryn:creature.dragon".into()]);
        assert!(encoded.as_ref().is_some_and(|bytes| bytes[2] == 1));
    }
}
