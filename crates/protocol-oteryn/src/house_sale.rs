//! House disposition and ownership offers. Payloads select; the server owns all terms.
use crate::charm_wire::CyclopediaWireError as Error;
mod codec;
mod reply;
pub use reply::{HouseSaleReply, HouseSaleRow, decode_reply, encode_reply};
pub const CAPABILITY: u32 = crate::house::CAPABILITY_HOUSE_V1;
pub const COMMAND: u32 = 6129;
pub const PAGE_SIZE: usize = 20;
pub const MAX_GOLD: u64 = 999_999_999_999;
/// 0 catalogue, 1 schedule move-out, 2 cancel move-out, 3 offer transfer,
/// 4 cancel transfer, 5 accept transfer, 6 reject transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseSaleRequest {
    pub action: u8,
    pub house: String,
    pub expected_revision: u64,
    pub due_at_unix_ms: i64,
    pub recipient: String,
    pub price: u64,
    /// Catalogue cursor; only action 0 may carry it.
    pub after_house: String,
}
impl HouseSaleRequest {
    pub fn catalogue(after_house: String) -> Self {
        Self {
            action: 0,
            house: String::new(),
            expected_revision: 0,
            due_at_unix_ms: 0,
            recipient: String::new(),
            price: 0,
            after_house,
        }
    }
    pub fn valid(&self) -> bool {
        if self.action == 0 {
            return self.house.is_empty()
                && self.expected_revision == 0
                && self.due_at_unix_ms == 0
                && self.recipient.is_empty()
                && self.price == 0
                && (self.after_house.is_empty() || valid_house(&self.after_house));
        }
        if self.action > 6
            || !valid_house(&self.house)
            || self.expected_revision == 0
            || !self.after_house.is_empty()
            || self.price > MAX_GOLD
        {
            return false;
        }
        match self.action {
            1 => self.due_at_unix_ms > 0 && self.recipient.is_empty() && self.price == 0,
            3 => {
                self.due_at_unix_ms > 0
                    && (2..=29).contains(&self.recipient.len())
                    && self.recipient.trim() == self.recipient
                    && self
                        .recipient
                        .bytes()
                        .all(|b| b.is_ascii_alphabetic() || b == b' ')
            }
            _ => self.due_at_unix_ms == 0 && self.recipient.is_empty() && self.price == 0,
        }
    }
}
pub(super) fn valid_house(value: &str) -> bool {
    value.len() <= 512
        && value
            .strip_prefix("oteryn:content.house.")
            .is_some_and(|tail| {
                !tail.is_empty()
                    && tail
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            })
}
pub fn encode_request(r: &HouseSaleRequest) -> Result<Vec<u8>, Error> {
    if !r.valid() {
        return Err(Error::Malformed);
    }
    let mut out = vec![1, r.action];
    codec::text(&mut out, &r.house)?;
    out.extend_from_slice(&r.expected_revision.to_be_bytes());
    out.extend_from_slice(&r.due_at_unix_ms.to_be_bytes());
    codec::text(&mut out, &r.recipient)?;
    out.extend_from_slice(&r.price.to_be_bytes());
    codec::text(&mut out, &r.after_house)?;
    Ok(out)
}
pub fn decode_request(bytes: &[u8]) -> Result<HouseSaleRequest, Error> {
    if bytes.len() > 1088 {
        return Err(Error::LimitExceeded);
    }
    let mut input = codec::Reader::new(bytes);
    if input.byte()? != 1 {
        return Err(Error::Malformed);
    }
    let request = HouseSaleRequest {
        action: input.byte()?,
        house: input.text(512)?,
        expected_revision: input.number()?,
        due_at_unix_ms: input.signed()?,
        recipient: input.text(29)?,
        price: input.number()?,
        after_house: input.text(512)?,
    };
    if !input.finished() || !request.valid() {
        return Err(Error::Malformed);
    }
    Ok(request)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn house_sale_exact_terms_roundtrip_and_malformed_refusal() -> Result<(), Error> {
        let mut r = HouseSaleRequest {
            action: 3,
            house: "oteryn:content.house.admiral_s_avenue_1".into(),
            expected_revision: 9,
            due_at_unix_ms: 1_900_000_000_000,
            recipient: "Elara Vale".into(),
            price: 42,
            after_house: String::new(),
        };
        let encoded = encode_request(&r)?;
        assert_eq!(decode_request(&encoded)?, r);
        for end in 0..encoded.len() {
            assert!(decode_request(&encoded[..end]).is_err());
        }
        let mut trailing = encoded.clone();
        trailing.push(0);
        assert!(decode_request(&trailing).is_err());
        r.expected_revision = 0;
        assert!(encode_request(&r).is_err());
        r.expected_revision = 9;
        r.price = MAX_GOLD + 1;
        assert!(encode_request(&r).is_err());
        assert!(encode_request(&HouseSaleRequest::catalogue(String::new())).is_ok());
        Ok(())
    }
}
