//! ANIMUS-1 bounded query wire values.

pub const CAPABILITY_ANIMUS_MASTERY_V1: u32 = 168;
pub const COMMAND_TYPE_ANIMUS_MASTERY_QUERY: u32 = 232;
pub const MAX_ANIMUS_QUERY_BYTES: usize = 4;
pub const MAX_ANIMUS_REPLY_BYTES: usize = 8_192;
/// 50 keys of at most 160 bytes plus their length bytes fit the 8192-byte reply bound.
pub const MAX_ANIMUS_PAGE_KEYS: u8 = 50;
pub const MAX_ANIMUS_KEY_BYTES: usize = 160;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimusPage {
    pub offset: u16,
    pub limit: u8,
}

impl AnimusPage {
    pub fn encode(self) -> Option<[u8; MAX_ANIMUS_QUERY_BYTES]> {
        let [low, high] = self.offset.to_le_bytes();
        let bytes = [0, low, high, self.limit];
        decode_query(&bytes).map(|_| bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimusMasteries {
    pub total: u16,
    pub keys: Vec<String>,
}

fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= MAX_ANIMUS_KEY_BYTES
        && key.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b':' | b'_' | b'.' | b'-')
        })
}

/// Decode one bounded page without trusting lengths, text or ordering from the wire.
pub fn decode_page(payload: &[u8]) -> Option<AnimusMasteries> {
    if payload.len() > MAX_ANIMUS_REPLY_BYTES {
        return None;
    }
    let [low, high, count, tail @ ..] = payload else {
        return None;
    };
    let mut rest = tail;
    let total = u16::from_le_bytes([*low, *high]);
    if *count > MAX_ANIMUS_PAGE_KEYS || u16::from(*count) > total {
        return None;
    }
    let mut keys = Vec::<String>::with_capacity(usize::from(*count));
    for _ in 0..*count {
        let (&len, tail) = rest.split_first()?;
        let bytes = tail.get(..usize::from(len))?;
        let key = std::str::from_utf8(bytes).ok()?;
        if !valid_key(key) || keys.last().is_some_and(|previous| previous.as_str() >= key) {
            return None;
        }
        keys.push(key.to_owned());
        rest = &tail[usize::from(len)..];
    }
    rest.is_empty().then_some(AnimusMasteries { total, keys })
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
    if keys.len() > usize::from(MAX_ANIMUS_PAGE_KEYS)
        || keys.len() > usize::from(total)
        || keys.iter().any(|key| !valid_key(key))
        || keys.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return None;
    }
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
        // The playable page contains unique, byte-sorted durable species keys.
        let widest = (0..MAX_ANIMUS_PAGE_KEYS)
            .map(|index| format!("{index:02}{}", "a".repeat(158)))
            .collect::<Vec<_>>();
        assert!(encode_page(u16::from(MAX_ANIMUS_PAGE_KEYS), &widest).is_some());
        let encoded = encode_page(1, &["oteryn:creature.dragon".into()]);
        assert!(encoded.as_ref().is_some_and(|bytes| bytes[2] == 1));
    }

    #[test]
    fn animus_page_roundtrip_preserves_server_count_and_sorted_species() -> Result<(), &'static str>
    {
        let keys = vec![
            "oteryn:creature.dragon".into(),
            "oteryn:creature.rat".into(),
        ];
        let bytes = encode_page(23, &keys).ok_or("page encode")?;
        assert_eq!(
            decode_page(&bytes),
            Some(AnimusMasteries { total: 23, keys })
        );
        let query = AnimusPage {
            offset: u16::MAX,
            limit: MAX_ANIMUS_PAGE_KEYS,
        };
        assert_eq!(
            decode_query(&query.encode().ok_or("query encode")?),
            Some(query)
        );
        assert!(
            AnimusPage {
                offset: 0,
                limit: 0
            }
            .encode()
            .is_none()
        );
        assert!(
            AnimusPage {
                offset: 0,
                limit: 51
            }
            .encode()
            .is_none()
        );
        assert_eq!(
            decode_page(&[0, 0, 0]),
            Some(AnimusMasteries {
                total: 0,
                keys: vec![]
            })
        );
        Ok(())
    }

    #[test]
    fn animus_page_rejects_truncation_bad_keys_counts_order_and_trailing_bytes()
    -> Result<(), &'static str> {
        let bytes = encode_page(2, &["a".into(), "b".into()]).ok_or("page encode")?;
        for end in 0..bytes.len() {
            assert!(decode_page(&bytes[..end]).is_none());
        }
        let mut trailing = bytes;
        trailing.push(0);
        assert!(decode_page(&trailing).is_none());
        for invalid in [
            vec![1, 0, 2, 1, b'a', 1, b'b'],
            vec![2, 0, 2, 1, b'b', 1, b'a'],
            vec![2, 0, 2, 1, b'a', 1, b'a'],
            vec![1, 0, 1, 0],
            vec![1, 0, 1, 1, 255],
            vec![1, 0, 1, 1, b'A'],
            vec![1, 0, 1, 1, b'/'],
            vec![51, 0, 51],
        ] {
            assert!(decode_page(&invalid).is_none());
        }
        assert!(encode_page(1, &["a".repeat(161)]).is_none());
        assert!(encode_page(0, &["a".into()]).is_none());
        assert!(encode_page(2, &["a".into(), "a".into()]).is_none());
        assert!(decode_page(&vec![0; MAX_ANIMUS_REPLY_BYTES + 1]).is_none());
        Ok(())
    }
}
