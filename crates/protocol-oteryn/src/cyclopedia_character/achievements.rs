//! Opt-in rich achievement pages; legacy character tab payloads remain unchanged.
use crate::account_achievements::{
    AccountAchievementsResult, MAX_ACCOUNT_ACHIEVEMENTS_RESULT_BYTES,
    decode_account_achievements_result, encode_account_achievements_result,
};
use crate::charm_wire::CyclopediaWireError as Error;
pub const SELECTOR: u8 = 128;
pub const MAX_RESULT_BYTES: usize = 19 + 29 + MAX_ACCOUNT_ACHIEVEMENTS_RESULT_BYTES;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AchievementRequest {
    pub page: u8,
    pub target: Option<[u8; 16]>,
}
impl AchievementRequest {
    pub fn encode(self) -> Vec<u8> {
        let mut out = vec![SELECTOR, self.page];
        if let Some(target) = self.target {
            out.extend_from_slice(&target);
        }
        out
    }
}
pub fn decode_request(bytes: &[u8]) -> Result<AchievementRequest, Error> {
    if !matches!(bytes.len(), 2 | 18) || bytes[0] != SELECTOR {
        return Err(Error::Malformed);
    }
    Ok(AchievementRequest {
        page: bytes[1],
        target: if bytes.len() == 18 {
            Some(bytes[2..].try_into().map_err(|_| Error::Malformed)?)
        } else {
            None
        },
    })
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AchievementPage {
    pub character: [u8; 16],
    pub name: String,
    pub page: AccountAchievementsResult,
}
impl AchievementPage {
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        if !(2..=29).contains(&self.name.len())
            || self.name.contains(['\0', '\r', '\n'])
            || self.page.page > 255
            || self.page.fact_count > 16_384
        {
            return Err(Error::Malformed);
        }
        let mut out = vec![SELECTOR];
        out.extend_from_slice(&self.character);
        out.push(self.name.len() as u8);
        out.extend_from_slice(self.name.as_bytes());
        out.extend_from_slice(
            &encode_account_achievements_result(&self.page).map_err(|_| Error::Malformed)?,
        );
        if out.len() > MAX_RESULT_BYTES {
            return Err(Error::LimitExceeded);
        }
        Ok(out)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_RESULT_BYTES {
            return Err(Error::LimitExceeded);
        }
        if bytes.len() < 18 || bytes[0] != SELECTOR {
            return Err(Error::Malformed);
        }
        let end = 18 + usize::from(bytes[17]);
        let value = Self {
            character: bytes[1..17].try_into().map_err(|_| Error::Malformed)?,
            name: std::str::from_utf8(bytes.get(18..end).ok_or(Error::Malformed)?)
                .map_err(|_| Error::Malformed)?
                .into(),
            page: decode_account_achievements_result(bytes.get(end..).ok_or(Error::Malformed)?)
                .map_err(|_| Error::Malformed)?,
        };
        value.encode()?;
        Ok(value)
    }
}
#[cfg(test)]
#[path = "achievements_tests.rs"]
mod tests;
