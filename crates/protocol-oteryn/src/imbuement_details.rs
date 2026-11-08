//! Negotiated read-only material quotes for the existing imbuement shrine.
//! IMBUE_V1 stays byte-identical; item, family and tier bind these details to its window.
use crate::charm_wire::{
    CyclopediaWireError as Error, push_message_field, push_varint_field, read_bytes, read_varint,
    set_once,
};
use crate::item_view::ItemHandle;
pub const CAPABILITY_IMBUEMENT_DETAILS_V1: u32 = 447;
pub const COMMAND_TYPE_IMBUEMENT_DETAILS: u32 = 731;
pub const MAX_REQUEST_BYTES: usize = 11;
pub const MAX_REPLY_BYTES: usize = 32768;
pub const MAX_QUOTES: usize = 60;
pub const MAX_MATERIALS: usize = 3;
pub const MAX_KEY_BYTES: usize = 128;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Material {
    pub definition_key: String,
    pub quantity: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quote {
    pub family: u8,
    pub tier: u8,
    pub gold: u32,
    pub requires_premium: bool,
    pub recipe_available: bool,
    pub success_percent: u8,
    pub materials: Vec<Material>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub item: ItemHandle,
    pub quotes: Vec<Quote>,
}
pub fn encode_query(item: ItemHandle) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_varint_field(&mut bytes, 1, item.get());
    bytes
}
pub fn decode_query(bytes: &[u8]) -> Result<ItemHandle, Error> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut at = 0;
    if read_varint(bytes, &mut at)? != 8 {
        return Err(Error::Malformed);
    }
    let item = ItemHandle::new(read_varint(bytes, &mut at)?).ok_or(Error::Malformed)?;
    if at != bytes.len() {
        return Err(Error::Malformed);
    }
    Ok(item)
}
fn valid_material(material: &Material) -> bool {
    !material.definition_key.is_empty()
        && material.definition_key.len() <= MAX_KEY_BYTES
        && material.definition_key.starts_with("oteryn:item.")
        && material.definition_key.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b':' | b'.' | b'_')
        })
        && material.quantity > 0
}
fn valid_quote(quote: &Quote) -> bool {
    (1..=20).contains(&quote.family)
        && (1..=3).contains(&quote.tier)
        && quote.success_percent == 100
        && quote.materials.len() <= MAX_MATERIALS
        && quote.recipe_available != quote.materials.is_empty()
        && quote.materials.iter().all(valid_material)
        && quote.materials.iter().enumerate().all(|(i, m)| {
            !quote.materials[..i]
                .iter()
                .any(|old| old.definition_key == m.definition_key)
        })
}
fn valid_reply(reply: &Reply) -> bool {
    reply.quotes.len() <= MAX_QUOTES
        && reply.quotes.iter().all(valid_quote)
        && reply
            .quotes
            .windows(2)
            .all(|rows| (rows[0].family, rows[0].tier) < (rows[1].family, rows[1].tier))
}
pub fn encode_reply(reply: &Reply) -> Result<Vec<u8>, Error> {
    if !valid_reply(reply) {
        return Err(Error::Malformed);
    }
    let mut out = encode_query(reply.item);
    for quote in &reply.quotes {
        let mut row = Vec::new();
        for (field, value) in [
            (1, u64::from(quote.family)),
            (2, u64::from(quote.tier)),
            (3, u64::from(quote.gold)),
            (4, u64::from(quote.requires_premium)),
            (5, u64::from(quote.success_percent)),
            (6, u64::from(quote.recipe_available)),
        ] {
            push_varint_field(&mut row, field, value);
        }
        for material in &quote.materials {
            let mut entry = Vec::new();
            push_message_field(&mut entry, 1, material.definition_key.as_bytes());
            push_varint_field(&mut entry, 2, u64::from(material.quantity));
            push_message_field(&mut row, 7, &entry);
        }
        push_message_field(&mut out, 2, &row);
    }
    if out.len() > MAX_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    Ok(out)
}
fn material(bytes: &[u8]) -> Result<Material, Error> {
    let (mut at, mut key, mut quantity) = (0, None, None);
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            10 => {
                let raw = read_bytes(bytes, &mut at)?;
                if raw.len() > MAX_KEY_BYTES {
                    return Err(Error::LimitExceeded);
                }
                set_once(
                    &mut key,
                    std::str::from_utf8(raw)
                        .map_err(|_| Error::Malformed)?
                        .to_owned(),
                )?;
            }
            16 => set_once(
                &mut quantity,
                u32::try_from(read_varint(bytes, &mut at)?).map_err(|_| Error::Malformed)?,
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    let row = Material {
        definition_key: key.ok_or(Error::Malformed)?,
        quantity: quantity.ok_or(Error::Malformed)?,
    };
    if !valid_material(&row) {
        return Err(Error::Malformed);
    }
    Ok(row)
}
fn quote(bytes: &[u8]) -> Result<Quote, Error> {
    let (mut at, mut fields, mut materials) = (0, [None; 6], Vec::new());
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            58 if materials.len() < MAX_MATERIALS => {
                materials.push(material(read_bytes(bytes, &mut at)?)?)
            }
            tag if tag & 7 == 0 && (1..=6).contains(&(tag >> 3)) => set_once(
                &mut fields[(tag >> 3) as usize - 1],
                read_varint(bytes, &mut at)?,
            )?,
            _ => return Err(Error::Malformed),
        }
    }
    let field = |index: usize| fields[index].ok_or(Error::Malformed);
    let boolean = |index| match field(index)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::Malformed),
    };
    let row = Quote {
        family: u8::try_from(field(0)?).map_err(|_| Error::Malformed)?,
        tier: u8::try_from(field(1)?).map_err(|_| Error::Malformed)?,
        gold: u32::try_from(field(2)?).map_err(|_| Error::Malformed)?,
        requires_premium: boolean(3)?,
        success_percent: u8::try_from(field(4)?).map_err(|_| Error::Malformed)?,
        recipe_available: boolean(5)?,
        materials,
    };
    if !valid_quote(&row) {
        return Err(Error::Malformed);
    }
    Ok(row)
}
pub fn decode_reply(bytes: &[u8]) -> Result<Reply, Error> {
    if bytes.len() > MAX_REPLY_BYTES {
        return Err(Error::LimitExceeded);
    }
    let (mut at, mut item, mut quotes) = (0, None, Vec::new());
    while at < bytes.len() {
        match read_varint(bytes, &mut at)? {
            8 => set_once(
                &mut item,
                ItemHandle::new(read_varint(bytes, &mut at)?).ok_or(Error::Malformed)?,
            )?,
            18 if quotes.len() < MAX_QUOTES => quotes.push(quote(read_bytes(bytes, &mut at)?)?),
            _ => return Err(Error::Malformed),
        }
    }
    let reply = Reply {
        item: item.ok_or(Error::Malformed)?,
        quotes,
    };
    if !valid_reply(&reply) {
        return Err(Error::Malformed);
    }
    Ok(reply)
}
#[cfg(test)]
#[path = "imbuement_details_tests.rs"]
mod tests;
