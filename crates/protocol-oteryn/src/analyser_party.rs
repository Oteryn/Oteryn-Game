//! Confirmed current-party accounting, valued with explicit client-supplied unit prices.
pub const COMMAND_TYPE_ANALYSER_PARTY_QUERY: u32 = 735;
pub const MAX_PRICES: usize = 2048;
pub const MAX_MEMBERS: usize = 50;
pub const MAX_BYTES: usize = 65536;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Price {
    pub item: u32,
    pub loot: Option<u64>,
    pub supply: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeededPrice {
    pub item: u32,
    pub loot: bool,
    pub supply: bool,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Member {
    pub handle: u64,
    pub name: String,
    pub available: bool,
    pub incomplete: bool,
    pub elapsed_seconds: u64,
    pub kills: u64,
    pub xp: u64,
    pub raw_xp: u64,
    pub loot_items: u64,
    pub loot_value: u64,
    pub supplies: u64,
    pub supply_cost: u64,
    pub unpriced_loot: u64,
    pub unpriced_supplies: u64,
    pub damage: u64,
    pub healing: u64,
    pub needed_prices: Vec<NeededPrice>,
}
impl Member {
    fn numbers(&self) -> [u64; 12] {
        [
            self.elapsed_seconds,
            self.kills,
            self.xp,
            self.raw_xp,
            self.loot_items,
            self.loot_value,
            self.supplies,
            self.supply_cost,
            self.unpriced_loot,
            self.unpriced_supplies,
            self.damage,
            self.healing,
        ]
    }
}
fn read<const N: usize>(bytes: &[u8], cursor: &mut usize) -> Result<[u8; N], &'static str> {
    let end = cursor.checked_add(N).ok_or("party accounting cursor")?;
    let value = bytes
        .get(*cursor..end)
        .ok_or("short party accounting")?
        .try_into()
        .map_err(|_| "party accounting field")?;
    *cursor = end;
    Ok(value)
}
pub fn encode_request(prices: &[Price]) -> Result<Vec<u8>, &'static str> {
    if prices.len() > MAX_PRICES {
        return Err("too many party prices");
    }
    let mut out = (prices.len() as u16).to_le_bytes().to_vec();
    for (i, price) in prices.iter().enumerate() {
        if price.item == 0
            || prices[..i].iter().any(|p| p.item == price.item)
            || (price.loot.is_none() && price.supply.is_none())
        {
            return Err("invalid party price");
        }
        out.extend_from_slice(&price.item.to_le_bytes());
        out.push(u8::from(price.loot.is_some()) | (u8::from(price.supply.is_some()) << 1));
        if let Some(value) = price.loot {
            out.extend_from_slice(&value.to_le_bytes());
        }
        if let Some(value) = price.supply {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    Ok(out)
}
pub fn decode_request(bytes: &[u8]) -> Result<Vec<Price>, &'static str> {
    if bytes.len() > MAX_BYTES {
        return Err("party price size");
    }
    let mut cursor = 0;
    let count = usize::from(u16::from_le_bytes(read(bytes, &mut cursor)?));
    if count > MAX_PRICES {
        return Err("party price count");
    }
    let mut rows = Vec::with_capacity(count);
    for _ in 0..count {
        let item = u32::from_le_bytes(read(bytes, &mut cursor)?);
        let [flags] = read(bytes, &mut cursor)?;
        if !(1..=3).contains(&flags) {
            return Err("party price flags");
        }
        let loot = if flags & 1 != 0 {
            Some(u64::from_le_bytes(read(bytes, &mut cursor)?))
        } else {
            None
        };
        let supply = if flags & 2 != 0 {
            Some(u64::from_le_bytes(read(bytes, &mut cursor)?))
        } else {
            None
        };
        rows.push(Price { item, loot, supply });
    }
    if cursor != bytes.len() {
        return Err("trailing party prices");
    }
    encode_request(&rows)?;
    Ok(rows)
}
pub fn encode_reply(rows: &[Member]) -> Result<Vec<u8>, &'static str> {
    if rows.len() > MAX_MEMBERS {
        return Err("party member count");
    }
    let mut out = vec![rows.len() as u8];
    for (i, row) in rows.iter().enumerate() {
        if row.handle == 0
            || rows[..i].iter().any(|p| p.handle == row.handle)
            || row.name.is_empty()
            || row.name.len() > 64
            || row.name.chars().any(char::is_control)
            || (!row.available && (row.incomplete || row.numbers().iter().any(|n| *n != 0)))
        {
            return Err("invalid party member accounting");
        }
        if row.needed_prices.len() > 32
            || (!row.available && !row.needed_prices.is_empty())
            || row.needed_prices.iter().enumerate().any(|(i, p)| {
                p.item == 0
                    || (!p.loot && !p.supply)
                    || (p.loot && row.unpriced_loot == 0)
                    || (p.supply && row.unpriced_supplies == 0)
                    || row.needed_prices[..i].iter().any(|q| q.item == p.item)
            })
        {
            return Err("invalid needed party prices");
        }
        out.extend_from_slice(&row.handle.to_le_bytes());
        out.push(row.name.len() as u8);
        out.extend_from_slice(row.name.as_bytes());
        out.push(u8::from(row.available) | (u8::from(row.incomplete) << 1));
        for value in row.numbers() {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out.push(row.needed_prices.len() as u8);
        for price in &row.needed_prices {
            out.extend_from_slice(&price.item.to_le_bytes());
            out.push(u8::from(price.loot) | (u8::from(price.supply) << 1));
        }
    }
    Ok(out)
}
pub fn decode_reply(bytes: &[u8]) -> Result<Vec<Member>, &'static str> {
    if bytes.len() > MAX_BYTES {
        return Err("party accounting size");
    }
    let mut cursor = 0;
    let [count] = read(bytes, &mut cursor)?;
    if usize::from(count) > MAX_MEMBERS {
        return Err("party member count");
    }
    let mut rows = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        let handle = u64::from_le_bytes(read(bytes, &mut cursor)?);
        let [length] = read(bytes, &mut cursor)?;
        let end = cursor + usize::from(length);
        let name = std::str::from_utf8(bytes.get(cursor..end).ok_or("party member name")?)
            .map_err(|_| "party name UTF-8")?
            .to_owned();
        cursor = end;
        let [flags] = read(bytes, &mut cursor)?;
        if flags > 3 {
            return Err("party member flags");
        }
        let mut numbers = [0; 12];
        for value in &mut numbers {
            *value = u64::from_le_bytes(read(bytes, &mut cursor)?);
        }
        let [
            elapsed_seconds,
            kills,
            xp,
            raw_xp,
            loot_items,
            loot_value,
            supplies,
            supply_cost,
            unpriced_loot,
            unpriced_supplies,
            damage,
            healing,
        ] = numbers;
        let [needed_count] = read(bytes, &mut cursor)?;
        if needed_count > 32 {
            return Err("needed party price count");
        }
        let mut needed_prices = Vec::with_capacity(usize::from(needed_count));
        for _ in 0..needed_count {
            let item = u32::from_le_bytes(read(bytes, &mut cursor)?);
            let [flags] = read(bytes, &mut cursor)?;
            if !(1..=3).contains(&flags) {
                return Err("needed party price flags");
            }
            needed_prices.push(NeededPrice {
                item,
                loot: flags & 1 != 0,
                supply: flags & 2 != 0,
            });
        }
        rows.push(Member {
            handle,
            name,
            available: flags & 1 != 0,
            incomplete: flags & 2 != 0,
            elapsed_seconds,
            kills,
            xp,
            raw_xp,
            loot_items,
            loot_value,
            supplies,
            supply_cost,
            unpriced_loot,
            unpriced_supplies,
            damage,
            healing,
            needed_prices,
        });
    }
    if cursor != bytes.len() {
        return Err("trailing party accounting");
    }
    encode_reply(&rows)?;
    Ok(rows)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn party_accounting_declared_wire_bounds_include_needed_prices() -> Result<(), &'static str> {
        let prices: Vec<_> = (1..=MAX_PRICES as u32)
            .map(|item| Price {
                item,
                loot: Some(u64::MAX),
                supply: Some(u64::MAX),
            })
            .collect();
        let request = encode_request(&prices)?;
        assert_eq!(request.len(), 43010);
        assert_eq!(decode_request(&request)?, prices);
        let rows: Vec<_> = (1..=MAX_MEMBERS as u64)
            .map(|handle| Member {
                handle,
                name: "A".repeat(64),
                available: true,
                unpriced_loot: 32,
                needed_prices: (1..=32)
                    .map(|item| NeededPrice {
                        item,
                        loot: true,
                        supply: false,
                    })
                    .collect(),
                ..Default::default()
            })
            .collect();
        let reply = encode_reply(&rows)?;
        assert_eq!(reply.len(), 16551);
        assert_eq!(decode_reply(&reply)?, rows);
        Ok(())
    }
    #[test]
    fn party_accounting_round_trips_prices_members_and_refuses_malformed_bounds()
    -> Result<(), &'static str> {
        let prices = vec![Price {
            item: 7,
            loot: Some(u64::MAX),
            supply: Some(0),
        }];
        let request = encode_request(&prices)?;
        assert_eq!(decode_request(&request)?, prices);
        let rows = vec![Member {
            handle: 9,
            name: "Scout".into(),
            available: true,
            loot_value: 120,
            supply_cost: 30,
            kills: 2,
            ..Default::default()
        }];
        let reply = encode_reply(&rows)?;
        assert_eq!(decode_reply(&reply)?, rows);
        for end in 0..request.len() {
            assert!(decode_request(&request[..end]).is_err());
        }
        for end in 0..reply.len() {
            assert!(decode_reply(&reply[..end]).is_err());
        }
        assert!(encode_request(&[prices[0].clone(), prices[0].clone()]).is_err());
        assert!(encode_reply(&[rows[0].clone(), rows[0].clone()]).is_err());
        let mut trailing = reply;
        trailing.push(0);
        assert!(decode_reply(&trailing).is_err());
        assert!(encode_request(&vec![prices[0].clone(); MAX_PRICES + 1]).is_err());
        Ok(())
    }
}
