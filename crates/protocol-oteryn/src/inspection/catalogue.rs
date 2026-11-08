//! Public-template browsing and inverse loot-source pages within command 153.
use super::{Result, WireError};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Query {
    Items {
        after: u32,
        category: u8,
        search: [u8; 64],
        len: u8,
    },
    Drops {
        reference: u32,
        after: u16,
    },
}
pub fn items(after: u32, category: u8, search: &str) -> Result<Query> {
    if search.len() > 64 || search.contains('\0') || category > 64 {
        return Err(WireError::LimitExceeded);
    }
    let mut bytes = [0; 64];
    bytes[..search.len()].copy_from_slice(search.as_bytes());
    Ok(Query::Items {
        after,
        category,
        search: bytes,
        len: search.len() as u8,
    })
}
pub fn search(query: Query) -> Result<String> {
    let Query::Items { search, len, .. } = query else {
        return Err(WireError::Malformed);
    };
    let len = usize::from(len);
    if len > search.len() || search[len..].iter().any(|b| *b != 0) {
        return Err(WireError::Malformed);
    }
    let value = std::str::from_utf8(&search[..len]).map_err(|_| WireError::Malformed)?;
    if value.contains('\0') {
        return Err(WireError::Malformed);
    }
    Ok(value.into())
}
pub fn encode(query: Query) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    match query {
        Query::Items {
            after, category, ..
        } => {
            if category > 64 {
                return Err(WireError::LimitExceeded);
            }
            out.push(1);
            out.extend(after.to_le_bytes());
            out.push(category);
            out.extend(search(query)?.as_bytes());
        }
        Query::Drops { reference, after } if reference > 0 && after <= 1024 => {
            out.push(2);
            out.extend(reference.to_le_bytes());
            out.extend(after.to_le_bytes());
        }
        _ => return Err(WireError::Malformed),
    }
    Ok(out)
}
pub fn decode(bytes: &[u8]) -> Result<Query> {
    if bytes.len() > 70 || bytes.len() < 6 {
        return Err(WireError::Malformed);
    }
    let value = u32::from_le_bytes(bytes[1..5].try_into().map_err(|_| WireError::Malformed)?);
    let query = match bytes[0] {
        1 => items(
            value,
            bytes[5],
            std::str::from_utf8(&bytes[6..]).map_err(|_| WireError::Malformed)?,
        )?,
        2 if bytes.len() == 7 => Query::Drops {
            reference: value,
            after: u16::from_le_bytes([bytes[5], bytes[6]]),
        },
        _ => return Err(WireError::Malformed),
    };
    encode(query)?;
    Ok(query)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn item_catalogue_queries_are_bounded_and_round_trip() -> Result<()> {
        for query in [
            items(10, 2, "silver")?,
            Query::Drops {
                reference: 7,
                after: 10,
            },
        ] {
            assert_eq!(decode(&encode(query)?)?, query);
        }
        assert!(items(0, 0, &"x".repeat(65)).is_err());
        assert!(items(0, 65, "").is_err());
        assert!(decode(&[2, 0, 0, 0, 0, 0, 0]).is_err());
        assert!(decode(&[1, 0, 0, 0, 0, 0, 255]).is_err());
        Ok(())
    }
}
