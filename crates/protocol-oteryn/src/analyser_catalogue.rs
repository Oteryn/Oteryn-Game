//! Read-only names and currency identity for active-generation analyser item references.
pub const COMMAND_TYPE_ANALYSER_ITEM_QUERY: u32 = 734;
pub const MAX_ITEMS: usize = 16;
pub const MAX_NAME_BYTES: usize = 120;
pub const MAX_REPLY_BYTES: usize = 1 + MAX_ITEMS * (4 + 2 + MAX_NAME_BYTES + 8);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemInfo {
    pub item_type: u32,
    pub name: String,
    /// Zero denotes a non-currency item; nonzero is its authoritative gold denomination.
    pub coin_worth: u64,
}
fn valid_ids(ids: &[u32]) -> bool {
    !ids.is_empty()
        && ids.len() <= MAX_ITEMS
        && ids
            .iter()
            .enumerate()
            .all(|(i, id)| *id != 0 && !ids[..i].contains(id))
}
pub fn encode_request(ids: &[u32]) -> Result<Vec<u8>, &'static str> {
    if !valid_ids(ids) {
        return Err("invalid analyser item query");
    }
    let mut out = vec![ids.len() as u8];
    for id in ids {
        out.extend_from_slice(&id.to_le_bytes());
    }
    Ok(out)
}
pub fn decode_request(bytes: &[u8]) -> Result<Vec<u32>, &'static str> {
    let count = usize::from(*bytes.first().ok_or("missing count")?);
    if count == 0 || count > MAX_ITEMS || bytes.len() != 1 + count * 4 {
        return Err("invalid analyser item query size");
    }
    let mut cursor = 1;
    let mut ids = Vec::with_capacity(count);
    for _ in 0..count {
        ids.push(u32::from_le_bytes(read(bytes, &mut cursor)?));
    }
    if !valid_ids(&ids) {
        return Err("invalid analyser item references");
    }
    Ok(ids)
}
pub fn encode_reply(rows: &[ItemInfo]) -> Result<Vec<u8>, &'static str> {
    if rows.is_empty()
        || rows.len() > MAX_ITEMS
        || rows.iter().enumerate().any(|(index, row)| {
            row.item_type == 0
                || rows[..index]
                    .iter()
                    .any(|previous| previous.item_type == row.item_type)
        })
    {
        return Err("invalid analyser item reply");
    }
    let mut out = vec![rows.len() as u8];
    for row in rows {
        if row.name.len() > MAX_NAME_BYTES
            || row.name.chars().any(char::is_control)
            || !matches!(row.coin_worth, 0 | 1 | 100 | 10000)
        {
            return Err("invalid analyser item metadata");
        }
        out.extend_from_slice(&row.item_type.to_le_bytes());
        out.extend_from_slice(&(row.name.len() as u16).to_le_bytes());
        out.extend_from_slice(row.name.as_bytes());
        out.extend_from_slice(&row.coin_worth.to_le_bytes());
    }
    Ok(out)
}
pub fn decode_reply(bytes: &[u8]) -> Result<Vec<ItemInfo>, &'static str> {
    let count = usize::from(*bytes.first().ok_or("missing count")?);
    if count == 0 || count > MAX_ITEMS || bytes.len() > MAX_REPLY_BYTES {
        return Err("invalid analyser item reply size");
    }
    let mut cursor = 1;
    let mut rows = Vec::with_capacity(count);
    for _ in 0..count {
        let item_type = u32::from_le_bytes(read(bytes, &mut cursor)?);
        let length = usize::from(u16::from_le_bytes(read(bytes, &mut cursor)?));
        if length > MAX_NAME_BYTES {
            return Err("invalid analyser item name length");
        }
        let end = cursor
            .checked_add(length)
            .ok_or("invalid analyser item name")?;
        let name = std::str::from_utf8(
            bytes
                .get(cursor..end)
                .ok_or("truncated analyser item name")?,
        )
        .map_err(|_| "invalid analyser item name")?
        .to_owned();
        cursor = end;
        let coin_worth = u64::from_le_bytes(read(bytes, &mut cursor)?);
        rows.push(ItemInfo {
            item_type,
            name,
            coin_worth,
        });
    }
    if cursor != bytes.len() {
        return Err("trailing analyser item bytes");
    }
    encode_reply(&rows)?;
    Ok(rows)
}
fn read<const N: usize>(bytes: &[u8], cursor: &mut usize) -> Result<[u8; N], &'static str> {
    let end = cursor
        .checked_add(N)
        .ok_or("invalid analyser item offset")?;
    let value = bytes
        .get(*cursor..end)
        .ok_or("truncated analyser item value")?
        .try_into()
        .map_err(|_| "invalid analyser item value")?;
    *cursor = end;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn analyser_catalogue_roundtrips_and_refuses_ambiguous_or_unbounded_values()
    -> Result<(), &'static str> {
        let rows = vec![
            ItemInfo {
                item_type: 7,
                name: "Gold piece".into(),
                coin_worth: 1,
            },
            ItemInfo {
                item_type: 19,
                name: "Potion".into(),
                coin_worth: 0,
            },
        ];
        let bytes = encode_reply(&rows)?;
        assert_eq!(decode_reply(&bytes)?, rows);
        assert_eq!(decode_request(&encode_request(&[7, 19])?)?, vec![7, 19]);
        for end in 0..bytes.len() {
            assert!(decode_reply(&bytes[..end]).is_err());
        }
        assert!(encode_request(&[0]).is_err());
        assert!(encode_request(&[7, 7]).is_err());
        assert!(encode_request(&[1; MAX_ITEMS + 1]).is_err());
        assert!(
            encode_reply(&[ItemInfo {
                item_type: 7,
                name: "x".into(),
                coin_worth: 2
            }])
            .is_err()
        );
        Ok(())
    }
}
