//! Coin market intents. Account and World identity always come from the session.
use crate::world_interaction::WorldInteractionWireError as Error;
pub const CAPABILITY: u32 = 63;
pub const COMMAND: u32 = 6128;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub action: u8,
    pub amount: u64,
    pub price: u64,
    pub offer: [u8; 16],
    pub operation: [u8; 16],
    pub cursor: String,
}
impl Default for Request {
    fn default() -> Self {
        Self {
            action: 1,
            amount: 0,
            price: 0,
            offer: [0; 16],
            operation: [0; 16],
            cursor: String::new(),
        }
    }
}
fn id(v: &[u8; 16]) -> bool {
    v[6] >> 4 == 7 && v[8] >> 6 == 2
}
pub fn encode(r: &Request) -> Result<Vec<u8>, Error> {
    use crate::charm_wire::{push_message_field as b, push_varint_field as n};
    let mut out = Vec::new();
    n(&mut out, 1, u64::from(r.action));
    if r.amount != 0 {
        n(&mut out, 2, r.amount);
    }
    if r.price != 0 {
        n(&mut out, 3, r.price);
    }
    if r.offer != [0; 16] {
        b(&mut out, 4, &r.offer);
    }
    if r.operation != [0; 16] {
        b(&mut out, 5, &r.operation);
    }
    if !r.cursor.is_empty() {
        b(&mut out, 6, r.cursor.as_bytes());
    }
    decode(&out)?;
    Ok(out)
}
pub fn decode(bytes: &[u8]) -> Result<Request, Error> {
    use crate::charm_wire::{read_bytes as b, read_varint as n, set_once};
    if bytes.len() > 256 {
        return Err(Error::LimitExceeded);
    }
    let (mut action, mut amount, mut price, mut offer, mut operation, mut cursor) =
        (None, None, None, None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match n(bytes, &mut at)? {
            8 => set_once(
                &mut action,
                u8::try_from(n(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            16 => set_once(&mut amount, n(bytes, &mut at)?)?,
            24 => set_once(&mut price, n(bytes, &mut at)?)?,
            34 => set_once(
                &mut offer,
                <[u8; 16]>::try_from(b(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            42 => set_once(
                &mut operation,
                <[u8; 16]>::try_from(b(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            50 => set_once(
                &mut cursor,
                std::str::from_utf8(b(bytes, &mut at)?)
                    .map_err(|_| Error::Malformed)?
                    .to_owned(),
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    let r = Request {
        action: action.ok_or(Error::Malformed)?,
        amount: amount.unwrap_or(0),
        price: price.unwrap_or(0),
        offer: offer.unwrap_or([0; 16]),
        operation: operation.unwrap_or([0; 16]),
        cursor: cursor.unwrap_or_default(),
    };
    if matches!(r.action, 3 | 4)
        && r.amount
            .checked_mul(r.price)
            .is_none_or(|total| total > 999_999_999_999)
    {
        return Err(Error::Malformed);
    }
    let mutation = matches!(r.action, 3..=6);
    let needs_offer = matches!(r.action, 5 | 6 | 8);
    if !(1..=9).contains(&r.action)
        || (mutation != id(&r.operation))
        || (!mutation && r.operation != [0; 16])
        || (needs_offer != id(&r.offer))
        || (!needs_offer && r.offer != [0; 16])
        || (matches!(r.action, 3..=5) != (r.amount > 0))
        || r.amount > 64000
        || (matches!(r.action, 3 | 4) != (r.price > 0))
        || r.price > 999_999_999_999
        || (!matches!(r.action, 1 | 2 | 7) && !r.cursor.is_empty())
        || r.cursor.len() > 36
        || (!r.cursor.is_empty()
            && if matches!(r.action, 1 | 2) {
                !r.cursor
                    .parse::<u16>()
                    .is_ok_and(|n| n <= 13 && n.to_string() == r.cursor)
            } else {
                !cursor_id(&r.cursor)
            })
    {
        return Err(Error::Malformed);
    }
    Ok(r)
}
pub fn encode_reply(view: &str) -> Result<Vec<u8>, Error> {
    if view.len() > 65500 {
        return Err(Error::LimitExceeded);
    }
    let mut out = Vec::new();
    crate::charm_wire::push_message_field(&mut out, 1, view.as_bytes());
    Ok(out)
}
pub fn decode_reply(bytes: &[u8]) -> Result<String, Error> {
    if bytes.len() > 65536 {
        return Err(Error::LimitExceeded);
    }
    let mut at = 0;
    if crate::charm_wire::read_varint(bytes, &mut at)? != 10 {
        return Err(Error::Malformed);
    }
    let s = std::str::from_utf8(crate::charm_wire::read_bytes(bytes, &mut at)?)
        .map_err(|_| Error::Malformed)?;
    if at != bytes.len() {
        return Err(Error::Malformed);
    }
    Ok(s.into())
}
fn cursor_id(s: &str) -> bool {
    s.len() == 36
        && [8, 13, 18, 23].iter().all(|i| s.as_bytes()[*i] == b'-')
        && s.as_bytes()[14] == b'7'
        && b"89ab".contains(&s.as_bytes()[19])
        && s.bytes()
            .all(|b| b.is_ascii_digit() || b"abcdef-".contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coin_market_codec_binds_confirmed_operation_and_refuses_unbounded_intents()
    -> Result<(), Error> {
        let mut r = Request {
            action: 3,
            amount: 25,
            price: 100,
            operation: [1; 16],
            ..Default::default()
        };
        assert!(encode(&r).is_err());
        r.operation[6] = 0x70;
        r.operation[8] = 0x80;
        assert_eq!(decode(&encode(&r)?)?, r);
        r.amount = 64001;
        assert!(encode(&r).is_err());
        Ok(())
    }
}
