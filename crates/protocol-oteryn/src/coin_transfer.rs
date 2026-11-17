//! Reviewed coin gifts; account identity is derived exclusively by Game.
use crate::world_interaction::WorldInteractionWireError as Error;
pub const CAPABILITY: u32 = 154;
pub const COMMAND: u32 = 6127;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    pub action: u8,
    pub recipient: String,
    pub amount: u64,
    pub quote: [u8; 16],
    pub operation: [u8; 16],
}
fn uuid(v: &[u8; 16]) -> bool {
    v[6] >> 4 == 7 && v[8] >> 6 == 2
}
pub fn encode(r: &Request) -> Result<Vec<u8>, Error> {
    use crate::charm_wire::{push_message_field as b, push_varint_field as n};
    let mut v = Vec::new();
    n(&mut v, 1, r.action.into());
    if !r.recipient.is_empty() {
        b(&mut v, 2, r.recipient.as_bytes());
    }
    if r.amount != 0 {
        n(&mut v, 3, r.amount);
    }
    if r.quote != [0; 16] {
        b(&mut v, 4, &r.quote);
    }
    if r.operation != [0; 16] {
        b(&mut v, 5, &r.operation);
    }
    decode(&v)?;
    Ok(v)
}
pub fn decode(bytes: &[u8]) -> Result<Request, Error> {
    use crate::charm_wire::{read_bytes as b, read_varint as n, set_once};
    if bytes.len() > 160 {
        return Err(Error::LimitExceeded);
    }
    let (mut action, mut recipient, mut amount, mut quote, mut operation) =
        (None, None, None, None, None);
    let mut at = 0;
    while at < bytes.len() {
        match n(bytes, &mut at)? {
            8 => set_once(
                &mut action,
                u8::try_from(n(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            18 => set_once(
                &mut recipient,
                std::str::from_utf8(b(bytes, &mut at)?)
                    .map_err(|_| Error::Malformed)?
                    .to_owned(),
            )?,
            24 => set_once(&mut amount, n(bytes, &mut at)?)?,
            34 => set_once(
                &mut quote,
                <[u8; 16]>::try_from(b(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            42 => set_once(
                &mut operation,
                <[u8; 16]>::try_from(b(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    let r = Request {
        action: action.ok_or(Error::Malformed)?,
        recipient: recipient.unwrap_or_default(),
        amount: amount.unwrap_or(0),
        quote: quote.unwrap_or([0; 16]),
        operation: operation.unwrap_or([0; 16]),
    };
    if matches!(r.action, 1 | 2) {
        if r.recipient.is_empty()
            || r.recipient.len() > 64
            || r.recipient.trim() != r.recipient
            || r.recipient.chars().any(char::is_control)
            || r.amount == 0
            || r.amount > i64::MAX as u64
            || !r.amount.is_multiple_of(25)
        {
            return Err(Error::Malformed);
        }
    } else if r.action != 3 || !r.recipient.is_empty() || r.amount != 0 {
        return Err(Error::Malformed);
    }
    if (r.action == 2) != uuid(&r.quote)
        || (r.action != 1) != uuid(&r.operation)
        || (r.action != 2 && r.quote != [0; 16])
        || (r.action == 1 && r.operation != [0; 16])
    {
        return Err(Error::Malformed);
    }
    Ok(r)
}
pub use crate::coin_market::{decode_reply, encode_reply};
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gifts_require_twenty_five_step_and_explicit_quote() -> Result<(), Error> {
        let mut r = Request {
            action: 1,
            recipient: "Alice".into(),
            amount: 25,
            ..Default::default()
        };
        assert_eq!(decode(&encode(&r)?)?, r);
        r.amount = 26;
        assert!(encode(&r).is_err());
        r.amount = 25;
        r.action = 2;
        assert!(encode(&r).is_err());
        r.quote[6] = 0x70;
        r.quote[8] = 0x80;
        r.operation = r.quote;
        assert!(encode(&r).is_ok());
        Ok(())
    }
}
