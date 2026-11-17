//! C08: own active condition catalogue and authoritative remaining milliseconds.
pub const CAPABILITY_CONDITION_STATUS_V1: u32 = 276;
pub const COMMAND_TYPE_CONDITION_STATUS_QUERY: u32 = 492;
pub const MAX_CONDITION_STATUS_COUNT: usize = 64;
pub const MAX_CONDITION_STATUS_REPLY_BYTES: usize = 1 + 10 * MAX_CONDITION_STATUS_COUNT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveCondition {
    /// Closed catalogue in the condition-status-v1 contract (1..=27).
    pub icon: u8,
    pub sub_id: u8,
    /// -1 is permanent; zero and values below -1 are rejected.
    pub remaining_ms: i64,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConditionStatus {
    pub conditions: Vec<ActiveCondition>,
}
pub fn decode_query(payload: &[u8]) -> Option<()> {
    payload.is_empty().then_some(())
}
impl ConditionStatus {
    fn valid(&self) -> bool {
        self.conditions.len() <= MAX_CONDITION_STATUS_COUNT
            && self.conditions.iter().all(|c| {
                (1..=27).contains(&c.icon)
                    && (c.remaining_ms == -1 || c.remaining_ms > 0)
                    && (matches!(c.icon, 20 | 21) || c.sub_id == 0)
            })
            && self
                .conditions
                .windows(2)
                .all(|pair| (pair[0].icon, pair[0].sub_id) < (pair[1].icon, pair[1].sub_id))
    }
    pub fn encode(&self) -> Option<Vec<u8>> {
        if !self.valid() {
            return None;
        }
        let mut bytes = Vec::with_capacity(1 + 10 * self.conditions.len());
        bytes.push(u8::try_from(self.conditions.len()).ok()?);
        for condition in &self.conditions {
            bytes.extend_from_slice(&[condition.icon, condition.sub_id]);
            bytes.extend_from_slice(&condition.remaining_ms.to_le_bytes());
        }
        Some(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let count = usize::from(*bytes.first()?);
        if count > MAX_CONDITION_STATUS_COUNT || bytes.len() != 1 + count * 10 {
            return None;
        }
        let conditions = bytes[1..]
            .as_chunks::<10>()
            .0
            .iter()
            .map(|entry| {
                Some(ActiveCondition {
                    icon: entry[0],
                    sub_id: entry[1],
                    remaining_ms: i64::from_le_bytes(entry[2..].try_into().ok()?),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let status = Self { conditions };
        status.valid().then_some(status)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn condition_status_codec_refuses_malformed_catalogue_and_durations() -> Result<(), &'static str>
    {
        let value = ConditionStatus {
            conditions: vec![ActiveCondition {
                icon: 1,
                sub_id: 0,
                remaining_ms: 5_000,
            }],
        };
        let bytes = value.encode().ok_or("encode")?;
        assert_eq!(ConditionStatus::decode(&bytes), Some(value.clone()));
        for end in 0..bytes.len() {
            assert!(ConditionStatus::decode(&bytes[..end]).is_none());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(ConditionStatus::decode(&trailing).is_none());
        for (icon, sub_id, remaining_ms) in
            [(0, 0, 1), (30, 0, 1), (1, 1, 1), (1, 0, 0), (1, 0, -2)]
        {
            assert!(
                ConditionStatus {
                    conditions: vec![ActiveCondition {
                        icon,
                        sub_id,
                        remaining_ms
                    }]
                }
                .encode()
                .is_none()
            );
        }
        let mut duplicate = value.clone();
        duplicate.conditions.extend_from_slice(&value.conditions);
        assert!(duplicate.encode().is_none());
        assert_eq!(
            ConditionStatus::decode(&[0]),
            Some(ConditionStatus::default())
        );
        assert_eq!(decode_query(&[]), Some(()));
        assert_eq!(decode_query(&[0]), None);
        Ok(())
    }
    #[test]
    fn condition_status_codec_accepts_both_owner_bounds_and_permanent_only_as_minus_one()
    -> Result<(), &'static str> {
        let value = ConditionStatus {
            conditions: (0..64)
                .map(|sub_id| ActiveCondition {
                    icon: 20,
                    sub_id,
                    remaining_ms: if sub_id == 0 { -1 } else { i64::MAX },
                })
                .collect(),
        };
        let bytes = value.encode().ok_or("bounded encode")?;
        assert_eq!(bytes.len(), MAX_CONDITION_STATUS_REPLY_BYTES);
        assert_eq!(ConditionStatus::decode(&bytes), Some(value.clone()));
        let mut excessive = value;
        excessive.conditions.push(ActiveCondition {
            icon: 20,
            sub_id: 64,
            remaining_ms: 1,
        });
        assert!(excessive.encode().is_none());
        assert!(ConditionStatus::decode(&[65]).is_none());
        Ok(())
    }
}
