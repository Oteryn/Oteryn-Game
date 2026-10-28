//! Capability 150, commands 174..=176: bounded house-podium mutation/query and
//! viewer projection. Strings are UTF-8 and length-prefixed with one byte.
pub const CAPABILITY_HOUSE_PODIUM_V1: u32 = 150;
pub const COMMAND_QUERY: u32 = 174;
pub const COMMAND_SET: u32 = 175;
pub const COMMAND_PROJECTION: u32 = 176;
pub const MAX_KEY_BYTES: usize = 160;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetRequest {
    pub item: [u8; 16],
    pub expected_revision: u64,
    pub display_kind: u8,
    pub direction: u8,
    pub key: String,
}
impl SetRequest {
    pub fn encode(&self) -> Option<Vec<u8>> {
        let key = self.key.as_bytes();
        if key.is_empty() || key.len() > MAX_KEY_BYTES {
            return None;
        }
        let mut v = Vec::with_capacity(27 + key.len());
        v.extend(self.item);
        v.extend(self.expected_revision.to_be_bytes());
        v.extend([self.display_kind, self.direction, key.len() as u8]);
        v.extend(key);
        Some(v)
    }
    pub fn decode(v: &[u8]) -> Option<Self> {
        if v.len() < 27 {
            return None;
        }
        let n = v[26] as usize;
        if n == 0 || n > MAX_KEY_BYTES || v.len() != 27 + n {
            return None;
        }
        Some(Self {
            item: v[..16].try_into().ok()?,
            expected_revision: u64::from_be_bytes(v[16..24].try_into().ok()?),
            display_kind: match v[24] {
                1..=3 => v[24],
                _ => return None,
            },
            direction: match v[25] {
                0..=3 => v[25],
                _ => return None,
            },
            key: std::str::from_utf8(&v[27..]).ok()?.to_owned(),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn set_wire_is_canonical() {
        let r = SetRequest {
            item: [7; 16],
            expected_revision: 9,
            display_kind: 1,
            direction: 3,
            key: "oteryn:outfit.citizen".into(),
        };
        let encoded = r.encode().unwrap_or_default();
        assert!(!encoded.is_empty());
        assert_eq!(SetRequest::decode(&encoded), Some(r));
        assert!(SetRequest::decode(&[0; 26]).is_none());
    }
}
