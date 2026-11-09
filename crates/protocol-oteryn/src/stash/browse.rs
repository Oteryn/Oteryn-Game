//! Additive view criteria. Conversion receipt identity remains the original intent.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StashFilter {
    pub search: String,
    pub category: String,
    pub trader: String,
    pub stock_order: bool,
}

pub fn encode_stash_query(
    intent: StashIntent,
    filter: Option<&StashFilter>,
) -> Result<Vec<u8>, Error> {
    let mut out = encode_stash_intent(intent)?;
    if let Some(filter) = filter {
        let mut fields = Vec::new();
        for (tag, value) in [
            (1, &filter.search),
            (2, &filter.category),
            (3, &filter.trader),
        ] {
            if value.len() > 128 || value.chars().any(char::is_control) {
                return Err(Error::Malformed);
            }
            push_message_field(&mut fields, tag, value.as_bytes());
        }
        push_varint_field(&mut fields, 4, u64::from(filter.stock_order));
        push_message_field(&mut out, 6, &fields);
    }
    Ok(out)
}

pub fn decode_stash_query(bytes: &[u8]) -> Result<(StashIntent, Option<StashFilter>), Error> {
    if bytes.len() > 460 {
        return Err(Error::LimitExceeded);
    }
    let mut base = Vec::new();
    let mut filter = None;
    let mut at = 0;
    while at < bytes.len() {
        let start = at;
        let tag = read_varint(bytes, &mut at)?;
        match tag {
            8 | 16 | 24 | 40 => {
                read_varint(bytes, &mut at)?;
            }
            34 => {
                read_bytes(bytes, &mut at)?;
            }
            50 => {
                let fields = read_bytes(bytes, &mut at)?;
                let (mut search, mut category, mut trader, mut stock) = (None, None, None, None);
                let mut pos = 0;
                while pos < fields.len() {
                    match read_varint(fields, &mut pos)? {
                        10 => set_once(&mut search, read_text(fields, &mut pos)?)?,
                        18 => set_once(&mut category, read_text(fields, &mut pos)?)?,
                        26 => set_once(&mut trader, read_text(fields, &mut pos)?)?,
                        32 => set_once(&mut stock, read_varint(fields, &mut pos)?)?,
                        _ => return Err(Error::Malformed),
                    }
                }
                set_once(
                    &mut filter,
                    StashFilter {
                        search: search.ok_or(Error::Malformed)?,
                        category: category.ok_or(Error::Malformed)?,
                        trader: trader.ok_or(Error::Malformed)?,
                        stock_order: match stock {
                            Some(0) => false,
                            Some(1) => true,
                            _ => return Err(Error::Malformed),
                        },
                    },
                )?;
                continue;
            }
            _ => return Err(Error::Malformed),
        }
        base.extend_from_slice(&bytes[start..at]);
    }
    let intent = decode_stash_intent(&base)?;
    encode_stash_query(intent, filter.as_ref())?;
    Ok((intent, filter))
}

fn read_text(bytes: &[u8], at: &mut usize) -> Result<String, Error> {
    Ok(std::str::from_utf8(read_bytes(bytes, at)?)
        .map_err(|_| Error::Malformed)?
        .to_owned())
}
