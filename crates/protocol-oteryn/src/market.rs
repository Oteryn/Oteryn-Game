//! MARKET_V1: bounded queries and settlement intents share command 80.
use crate::charm_wire::{push_message_field, push_varint_field, read_bytes, read_varint, set_once};
use crate::world_interaction::WorldInteractionWireError as Error;

pub const CAPABILITY_MARKET_V1: u32 = 63;
pub const COMMAND_TYPE_MARKET: u32 = 80;
pub const STATE_DOMAIN_MARKET: u32 = 28;

/// Actions: 1 wares, 2 book, 3 own offers, 4 history, 5 buy,
/// 6 sell, 7 cancel, 8 accept. Authority, Account and World are never wire fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketRequest {
    pub action: u8,
    pub definition: u32,
    pub offer: [u8; 16],
    pub amount: u32,
    pub price: u64,
    pub anonymous: bool,
    pub page: u32,
}

pub fn encode_request(r: MarketRequest) -> Result<Vec<u8>, Error> {
    let zero = [0; 16];
    let valid = match r.action {
        1 | 3 | 4 => {
            r.definition == 0 && r.offer == zero && r.amount == 0 && r.price == 0 && !r.anonymous
        }
        2 => r.definition > 0 && r.offer == zero && r.amount == 0 && r.price == 0 && !r.anonymous,
        5 | 6 => {
            r.definition > 0
                && r.offer == zero
                && (1..=64_000).contains(&r.amount)
                && r.price > 0
                && u128::from(r.price) * u128::from(r.amount) <= 999_999_999_999
                && r.page == 0
        }
        7 | 8 => {
            r.definition == 0
                && r.offer[6] >> 4 == 7
                && r.offer[8] >> 6 == 2
                && r.price == 0
                && !r.anonymous
                && r.page == 0
                && if r.action == 7 {
                    r.amount == 0
                } else {
                    (1..=64_000).contains(&r.amount)
                }
        }
        _ => false,
    };
    if !valid || r.page > 100_000 || (r.action == 3 && r.page > 3) || (r.action == 4 && r.page > 18)
    {
        return Err(Error::Malformed);
    }
    let mut out = Vec::new();
    for (field, n) in [
        (1, u64::from(r.action)),
        (2, u64::from(r.definition)),
        (4, u64::from(r.amount)),
        (5, r.price),
        (6, u64::from(r.anonymous)),
        (7, u64::from(r.page)),
    ] {
        push_varint_field(&mut out, field, n);
    }
    if r.offer != zero {
        push_message_field(&mut out, 3, &r.offer);
    }
    Ok(out)
}

pub fn decode_request(bytes: &[u8]) -> Result<MarketRequest, Error> {
    if bytes.len() > 96 {
        return Err(Error::LimitExceeded);
    }
    let mut fields = [None; 7];
    let mut offer = None;
    let mut at = 0;
    while at < bytes.len() {
        let tag = read_varint(bytes, &mut at)?;
        let field = tag >> 3;
        if field == 3 && tag & 7 == 2 {
            let value: [u8; 16] = read_bytes(bytes, &mut at)?
                .try_into()
                .map_err(|_| Error::Malformed)?;
            set_once(&mut offer, value)?;
        } else if (1..=7).contains(&field) && field != 3 && tag & 7 == 0 {
            set_once(
                &mut fields[field as usize - 1],
                read_varint(bytes, &mut at)?,
            )?;
        } else {
            return Err(Error::Malformed);
        }
    }
    let r = MarketRequest {
        action: u8::try_from(fields[0].ok_or(Error::Malformed)?).map_err(|_| Error::Malformed)?,
        definition: u32::try_from(fields[1].unwrap_or(0)).map_err(|_| Error::Malformed)?,
        offer: offer.unwrap_or([0; 16]),
        amount: u32::try_from(fields[3].unwrap_or(0)).map_err(|_| Error::Malformed)?,
        price: fields[4].unwrap_or(0),
        anonymous: match fields[5].unwrap_or(0) {
            0 => false,
            1 => true,
            _ => return Err(Error::Malformed),
        },
        page: u32::try_from(fields[6].unwrap_or(0)).map_err(|_| Error::Malformed)?,
    };
    encode_request(r)?;
    Ok(r)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketRow {
    pub offer: [u8; 16],
    pub definition: u32,
    pub side: u8,
    pub amount: u32,
    pub price: u64,
    pub state: u8,
    pub anonymous: bool,
}

/// Codes: 1 OK, 2 funds, 3 items, 4 Inbox full, 5 offer limit, 6 gone,
/// 7 same Account, 8 touched-item limit, 9 rejected, 10 balance limit,
/// 11 no locker, 12 not Premium, 13 junior, 14 not marketable, 15 book busy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketReply {
    pub action: u8,
    pub code: u8,
    pub offer: [u8; 16],
    pub rows: Vec<MarketRow>,
}
pub fn encode_reply(r: &MarketReply) -> Result<Vec<u8>, Error> {
    if !(1..=15).contains(&r.code) || r.action > 8 || r.rows.len() > 32 {
        return Err(Error::LimitExceeded);
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, u64::from(r.code));
    if r.offer != [0; 16] {
        push_message_field(&mut out, 2, &r.offer);
    }
    push_varint_field(&mut out, 4, u64::from(r.action));
    for row in &r.rows {
        let mut bytes = Vec::new();
        push_message_field(&mut bytes, 1, &row.offer);
        for (field, n) in [
            (2, u64::from(row.definition)),
            (3, u64::from(row.side)),
            (4, u64::from(row.amount)),
            (5, row.price),
            (6, u64::from(row.state)),
            (7, u64::from(row.anonymous)),
        ] {
            push_varint_field(&mut bytes, field, n);
        }
        push_message_field(&mut out, 3, &bytes);
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn market_wire_rejects_ambiguous_or_unbounded_requests() -> Result<(), Error> {
        let r = MarketRequest {
            action: 6,
            definition: 1,
            offer: [0; 16],
            amount: 100,
            price: 25,
            anonymous: true,
            page: 0,
        };
        assert_eq!(decode_request(&encode_request(r)?)?, r);
        let mut bytes = encode_request(r)?;
        bytes.extend([8, 6]);
        assert!(decode_request(&bytes).is_err());
        for invalid in [
            MarketRequest { action: 0, ..r },
            MarketRequest { amount: 0, ..r },
            MarketRequest {
                price: u64::MAX,
                ..r
            },
            MarketRequest { page: 1, ..r },
        ] {
            assert!(encode_request(invalid).is_err());
        }
        Ok(())
    }
}
