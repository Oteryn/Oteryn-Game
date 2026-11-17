//! Bounded item-selection messages of the retired daily item picker. WU26-05 withdrew
//! commands 6122/6123 from capability 182 and the registry; the numbers stay retired.
pub const QUERY: u32 = 6122;
pub const CLAIM: u32 = 6123;
pub const MAX_PICKS: usize = 19;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pick {
    pub item: u32,
    pub count: u8,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    pub epoch: u64,
    pub day: u8,
    pub picks: Vec<Pick>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Offer {
    pub epoch: u64,
    pub day: u8,
    pub premium: bool,
    pub maximum: u8,
    pub items: [u32; 32],
}
impl Claim {
    pub fn encode(&self) -> Result<Vec<u8>, &'static str> {
        if self.epoch == 0
            || !(1..=30).contains(&self.day)
            || self.picks.is_empty()
            || self.picks.len() > MAX_PICKS
            || self
                .picks
                .iter()
                .any(|p| p.item == 0 || p.count == 0 || p.count > 20)
            || self.picks.windows(2).any(|p| p[0].item >= p[1].item)
        {
            return Err("invalid selection");
        }
        let mut b = b"DRC1".to_vec();
        b.extend(self.epoch.to_le_bytes());
        b.extend([self.day, self.picks.len() as u8]);
        for p in &self.picks {
            b.extend(p.item.to_le_bytes());
            b.push(p.count);
        }
        Ok(b)
    }
    pub fn decode(b: &[u8]) -> Result<Self, &'static str> {
        if b.len() < 14 || &b[..4] != b"DRC1" || b.len() != 14 + usize::from(b[13]) * 5 {
            return Err("invalid selection");
        }
        let value = Self {
            epoch: u64::from_le_bytes(b[4..12].try_into().map_err(|_| "epoch")?),
            day: b[12],
            picks: b[14..]
                .as_chunks::<5>()
                .0
                .iter()
                .map(|p| {
                    Ok(Pick {
                        item: u32::from_le_bytes(p[..4].try_into().map_err(|_| "item")?),
                        count: p[4],
                    })
                })
                .collect::<Result<_, &str>>()?,
        };
        value.encode()?;
        Ok(value)
    }
}
impl Offer {
    pub fn encode(&self) -> Result<Vec<u8>, &'static str> {
        if self.epoch == 0 || !(1..=30).contains(&self.day) || self.maximum > 20 {
            return Err("invalid offer");
        }
        let mut b = b"DRP1".to_vec();
        b.extend(self.epoch.to_le_bytes());
        b.extend([self.day, u8::from(self.premium), self.maximum]);
        for id in self.items {
            b.extend(id.to_le_bytes());
        }
        Ok(b)
    }
    pub fn decode(b: &[u8]) -> Result<Self, &'static str> {
        if b.len() != 143 || &b[..4] != b"DRP1" || b[13] > 1 {
            return Err("invalid offer");
        }
        let mut items = [0; 32];
        for (slot, id) in items.iter_mut().zip(b[15..].as_chunks::<4>().0.iter()) {
            *slot = u32::from_le_bytes(*id);
        }
        let v = Self {
            epoch: u64::from_le_bytes(b[4..12].try_into().map_err(|_| "epoch")?),
            day: b[12],
            premium: b[13] != 0,
            maximum: b[14],
            items,
        };
        v.encode()?;
        Ok(v)
    }
}
