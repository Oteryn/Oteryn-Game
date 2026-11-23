//! BOOSTED-1 bounded client query for the World server-save day's selected creature and boss.

pub const CAPABILITY_DAILY_BOOST_V1: u32 = 151;
pub const COMMAND_TYPE_DAILY_BOOST_QUERY: u32 = 177;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyBoostCodecError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyBoostView {
    pub reset_epoch: u64,
    pub creature_key: String,
    pub boss_key: String,
}

pub fn decode_query(bytes: &[u8]) -> Result<(), DailyBoostCodecError> {
    bytes.is_empty().then_some(()).ok_or(DailyBoostCodecError)
}

pub fn encode_view(view: &DailyBoostView) -> Result<Vec<u8>, DailyBoostCodecError> {
    fn put(output: &mut Vec<u8>, value: &str) -> Result<(), DailyBoostCodecError> {
        let len = u8::try_from(value.len()).map_err(|_| DailyBoostCodecError)?;
        if value.is_empty() {
            return Err(DailyBoostCodecError);
        }
        output.push(len);
        output.extend_from_slice(value.as_bytes());
        Ok(())
    }
    let mut output = Vec::with_capacity(10 + view.creature_key.len() + view.boss_key.len());
    output.extend_from_slice(&view.reset_epoch.to_be_bytes());
    put(&mut output, &view.creature_key)?;
    put(&mut output, &view.boss_key)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_and_view_match_the_documented_wire_layout() -> Result<(), DailyBoostCodecError> {
        assert!(decode_query(&[]).is_ok());
        assert!(decode_query(&[0]).is_err());
        let view = DailyBoostView {
            reset_epoch: 9,
            creature_key: "rat".into(),
            boss_key: "ferumbras".into(),
        };
        let encoded = encode_view(&view)?;
        // daily-boost-v1.md: big-endian u64, then two UTF-8 keys with u8 byte lengths.
        const EXPECTED: &[u8] = &[
            0, 0, 0, 0, 0, 0, 0, 9, 3, b'r', b'a', b't', 9, b'f', b'e', b'r', b'u', b'm', b'b',
            b'r', b'a', b's',
        ];
        assert_eq!(encoded.as_slice(), EXPECTED);
        Ok(())
    }
}
