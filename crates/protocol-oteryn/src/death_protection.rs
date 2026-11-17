//! C20c: immutable protection consumed by one committed death, retained across login.
pub const CAPABILITY_DEATH_PROTECTION_V1: u32 = 382;
pub const COMMAND_TYPE_DEATH_PROTECTION_QUERY: u32 = 664;
pub const MAX_DEATH_PROTECTION_REPLY_BYTES: usize = 4146;
const MAX_BLESSINGS: usize = 32;
const MAX_KEY_BYTES: usize = 128;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeathProtection {
    pub occurrence: [u8; 16],
    /// Exact immutable receipt difference; an empty list means this death consumed none.
    pub consumed: Vec<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeathProtectionReply(pub Option<DeathProtection>);
impl DeathProtectionReply {
    pub fn encode(&self) -> Option<Vec<u8>> {
        let Some(value) = &self.0 else {
            return Some(vec![0]);
        };
        if value.occurrence == [0; 16] || value.consumed.len() > MAX_BLESSINGS {
            return None;
        }
        let mut bytes = vec![1];
        bytes.extend_from_slice(&value.occurrence);
        bytes.push(u8::try_from(value.consumed.len()).ok()?);
        let mut previous = "";
        for key in &value.consumed {
            if key.is_empty()
                || key.len() > MAX_KEY_BYTES
                || key.as_str() <= previous
                || !key.starts_with("oteryn:blessing.")
                || !key
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || matches!(byte, b':' | b'.' | b'_'))
            {
                return None;
            }
            previous = key;
            bytes.push(u8::try_from(key.len()).ok()?);
            bytes.extend_from_slice(key.as_bytes());
        }
        Some(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes == [0] {
            return Some(Self(None));
        }
        if bytes.len() < 18 || bytes.len() > MAX_DEATH_PROTECTION_REPLY_BYTES || bytes[0] != 1 {
            return None;
        }
        let occurrence = bytes[1..17].try_into().ok()?;
        let mut consumed = Vec::with_capacity(usize::from(bytes[17]));
        let mut remaining = &bytes[18..];
        for _ in 0..bytes[17] {
            let (&len, tail) = remaining.split_first()?;
            let (key, tail) = tail.split_at_checked(usize::from(len))?;
            consumed.push(std::str::from_utf8(key).ok()?.to_owned());
            remaining = tail;
        }
        if !remaining.is_empty() {
            return None;
        }
        let value = Self(Some(DeathProtection {
            occurrence,
            consumed,
        }));
        value.encode().filter(|canonical| canonical == bytes)?;
        Some(value)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn death_protection_codec_retains_empty_consumption_and_rejects_malformed()
    -> Result<(), &'static str> {
        for consumed in [vec![], vec!["oteryn:blessing.twist_of_fate".into()]] {
            let value = DeathProtectionReply(Some(DeathProtection {
                occurrence: [7; 16],
                consumed,
            }));
            let bytes = value.encode().ok_or("encode")?;
            assert_eq!(DeathProtectionReply::decode(&bytes), Some(value));
            for end in 0..bytes.len() {
                assert!(DeathProtectionReply::decode(&bytes[..end]).is_none());
            }
            let mut extra = bytes;
            extra.push(0);
            assert!(DeathProtectionReply::decode(&extra).is_none());
        }
        assert_eq!(
            DeathProtectionReply::decode(&[0]),
            Some(DeathProtectionReply(None))
        );
        assert!(DeathProtectionReply::decode(&[0, 1]).is_none());
        Ok(())
    }
}
