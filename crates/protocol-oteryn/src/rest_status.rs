//! GAP-SV7 C10 authoritative resting-area and food presentation.
pub const CAPABILITY_REST_STATUS_V1: u32 = 277;
pub const COMMAND_TYPE_REST_STATUS_QUERY: u32 = 493;
pub const MAX_REST_STATUS_REQUEST_BYTES: usize = 0;
pub const MAX_REST_STATUS_REPLY_BYTES: usize = 11;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RestStatus {
    pub inside_resting_area: bool,
    pub streak: u32,
    /// Zero means hungry; -1 means an infinite condition; positive values are milliseconds.
    pub food_remaining_ms: i32,
    /// Zero is inactive, one is normal, two is doubled.
    pub health_multiplier: u8,
    pub mana_multiplier: u8,
    pub stamina: bool,
    pub soul: bool,
}

pub fn decode_query(payload: &[u8]) -> Option<()> {
    payload.is_empty().then_some(())
}

impl RestStatus {
    fn valid(self) -> bool {
        self.food_remaining_ms >= -1
            && self.health_multiplier <= 2
            && self.mana_multiplier <= 2
            && (self.inside_resting_area
                || (self.health_multiplier == 0
                    && self.mana_multiplier == 0
                    && !self.stamina
                    && !self.soul))
    }

    pub fn encode(self) -> Option<Vec<u8>> {
        if !self.valid() {
            return None;
        }
        let mut bytes = Vec::with_capacity(MAX_REST_STATUS_REPLY_BYTES);
        bytes.push(
            u8::from(self.inside_resting_area)
                | (u8::from(self.stamina) << 1)
                | (u8::from(self.soul) << 2),
        );
        bytes.extend_from_slice(&self.streak.to_le_bytes());
        bytes.extend_from_slice(&self.food_remaining_ms.to_le_bytes());
        bytes.push(self.health_multiplier);
        bytes.push(self.mana_multiplier);
        Some(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != MAX_REST_STATUS_REPLY_BYTES || bytes[0] & !7 != 0 {
            return None;
        }
        let value = Self {
            inside_resting_area: bytes[0] & 1 != 0,
            stamina: bytes[0] & 2 != 0,
            soul: bytes[0] & 4 != 0,
            streak: u32::from_le_bytes(bytes.get(1..5)?.try_into().ok()?),
            food_remaining_ms: i32::from_le_bytes(bytes.get(5..9)?.try_into().ok()?),
            health_multiplier: bytes[9],
            mana_multiplier: bytes[10],
        };
        value.valid().then_some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rest_status_codec_preserves_authority_and_refuses_malformed_payloads()
    -> Result<(), &'static str> {
        let status = RestStatus {
            inside_resting_area: true,
            streak: 7,
            food_remaining_ms: 180_000,
            health_multiplier: 2,
            mana_multiplier: 2,
            stamina: true,
            soul: true,
        };
        let bytes = status.encode().ok_or("encode")?;
        assert_eq!(RestStatus::decode(&bytes), Some(status));
        assert_eq!(decode_query(&[]), Some(()));
        assert_eq!(decode_query(&[0]), None);
        for end in 0..bytes.len() {
            assert!(RestStatus::decode(&bytes[..end]).is_none());
        }
        let mut invalid = bytes.clone();
        invalid.push(0);
        assert!(RestStatus::decode(&invalid).is_none());
        for (index, value) in [(0, 8), (9, 3), (10, 3)] {
            let mut invalid = bytes.clone();
            invalid[index] = value;
            assert!(RestStatus::decode(&invalid).is_none());
        }
        assert!(
            RestStatus {
                inside_resting_area: false,
                ..status
            }
            .encode()
            .is_none()
        );
        assert!(
            RestStatus {
                food_remaining_ms: -2,
                ..status
            }
            .encode()
            .is_none()
        );
        assert!(
            RestStatus {
                food_remaining_ms: -1,
                ..status
            }
            .encode()
            .is_some()
        );
        Ok(())
    }
}
