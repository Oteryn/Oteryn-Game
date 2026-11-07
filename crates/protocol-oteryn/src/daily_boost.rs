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
fn decode_view(mut bytes: &[u8]) -> Result<DailyBoostView, DailyBoostCodecError> {
    fn take(bytes: &mut &[u8]) -> Result<String, DailyBoostCodecError> {
        let (&len, rest) = bytes.split_first().ok_or(DailyBoostCodecError)?;
        let (value, rest) = rest
            .split_at_checked(usize::from(len))
            .ok_or(DailyBoostCodecError)?;
        *bytes = rest;
        if value.is_empty() {
            return Err(DailyBoostCodecError);
        }
        String::from_utf8(value.to_vec()).map_err(|_| DailyBoostCodecError)
    }
    let (epoch, rest) = bytes.split_at_checked(8).ok_or(DailyBoostCodecError)?;
    bytes = rest;
    let reset_epoch = u64::from_be_bytes(epoch.try_into().map_err(|_| DailyBoostCodecError)?);
    let creature_key = take(&mut bytes)?;
    let boss_key = take(&mut bytes)?;
    if !bytes.is_empty() {
        return Err(DailyBoostCodecError);
    }
    Ok(DailyBoostView {
        reset_epoch,
        creature_key,
        boss_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_and_view_round_trip() -> Result<(), DailyBoostCodecError> {
        assert!(decode_query(&[]).is_ok());
        assert!(decode_query(&[0]).is_err());
        let view = DailyBoostView {
            reset_epoch: 9,
            creature_key: "rat".into(),
            boss_key: "ferumbras".into(),
        };
        let encoded = encode_view(&view)?;
        assert_eq!(decode_view(&encoded), Ok(view));
        Ok(())
    }
}
