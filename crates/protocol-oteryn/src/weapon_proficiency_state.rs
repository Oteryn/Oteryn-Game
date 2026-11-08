//! Negotiated, bounded read-only shaping projection. Legacy proficiency bytes stay unchanged.
pub const CAPABILITY_WEAPON_PROFICIENCY_STATE_V1: u32 = 448;
pub const COMMAND_TYPE_WEAPON_PROFICIENCY_STATE: u32 = 732;
pub const MAX_REPLY_BYTES: usize = 4096;
const MAX_TEXT: usize = 256;
pub fn encode_request(definition: u32) -> Option<[u8; 4]> {
    (definition != 0).then_some(definition.to_be_bytes())
}
pub fn decode_request(bytes: &[u8]) -> Option<u32> {
    let definition = u32::from_be_bytes(bytes.try_into().ok()?);
    (definition != 0).then_some(definition)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    pub level: u8,
    pub rank: u8,
    pub current: String,
    pub next: String,
    pub maximum: String,
    pub offers: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub definition: u32,
    /// Zero means no committed track; it never authorizes a modification.
    pub revision: u64,
    pub slots: [Option<Slot>; 2],
    pub modify_dust: [u32; 2],
    pub rank_dust: [u32; 10],
    pub reshape_dust: u32,
    pub clear_dust: u32,
    pub orbs: u32,
}
impl State {
    pub fn encode(&self) -> Option<Vec<u8>> {
        if self.definition == 0 || (self.revision == 0 && self.slots.iter().any(Option::is_some)) {
            return None;
        }
        let mut bytes = self.definition.to_be_bytes().to_vec();
        bytes.extend(self.revision.to_be_bytes());
        for slot in &self.slots {
            bytes.push(u8::from(slot.is_some()));
            if let Some(slot) = slot {
                if slot.level >= 7 || slot.rank > 10 || !matches!(slot.offers.len(), 0 | 3) {
                    return None;
                }
                bytes.extend([slot.level, slot.rank, slot.offers.len() as u8]);
                for text in [&slot.current, &slot.next, &slot.maximum]
                    .into_iter()
                    .chain(slot.offers.iter())
                {
                    if text.is_empty() || text.len() > MAX_TEXT || text.contains('\0') {
                        return None;
                    }
                    bytes.extend((text.len() as u16).to_be_bytes());
                    bytes.extend(text.as_bytes());
                }
            }
        }
        for amount in self.modify_dust.into_iter().chain(self.rank_dust).chain([
            self.reshape_dust,
            self.clear_dust,
            self.orbs,
        ]) {
            bytes.extend(amount.to_be_bytes());
        }
        (bytes.len() <= MAX_REPLY_BYTES).then_some(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_REPLY_BYTES {
            return None;
        }
        let mut reader = Reader(bytes);
        let definition = reader.u32()?;
        let revision = u64::from_be_bytes(reader.take(8)?.try_into().ok()?);
        let mut slots = [None, None];
        for slot in &mut slots {
            match reader.byte()? {
                0 => {}
                1 => {
                    let level = reader.byte()?;
                    let rank = reader.byte()?;
                    let count = reader.byte()?;
                    if !matches!(count, 0 | 3) {
                        return None;
                    }
                    let current = reader.text()?;
                    let next = reader.text()?;
                    let maximum = reader.text()?;
                    let offers = (0..count)
                        .map(|_| reader.text())
                        .collect::<Option<Vec<_>>>()?;
                    *slot = Some(Slot {
                        level,
                        rank,
                        current,
                        next,
                        maximum,
                        offers,
                    });
                }
                _ => return None,
            }
        }
        let mut modify_dust = [0; 2];
        let mut rank_dust = [0; 10];
        for amount in modify_dust.iter_mut().chain(rank_dust.iter_mut()) {
            *amount = reader.u32()?;
        }
        let value = Self {
            definition,
            revision,
            slots,
            modify_dust,
            rank_dust,
            reshape_dust: reader.u32()?,
            clear_dust: reader.u32()?,
            orbs: reader.u32()?,
        };
        (reader.0.is_empty() && value.encode()?.as_slice() == bytes).then_some(value)
    }
}
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let (head, tail) = self.0.split_at_checked(count)?;
        self.0 = tail;
        Some(head)
    }
    fn byte(&mut self) -> Option<u8> {
        self.take(1)?.first().copied()
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_be_bytes(self.take(4)?.try_into().ok()?))
    }
    fn text(&mut self) -> Option<String> {
        let len = usize::from(u16::from_be_bytes(self.take(2)?.try_into().ok()?));
        if len > MAX_TEXT {
            return None;
        }
        Some(std::str::from_utf8(self.take(len)?).ok()?.into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proficiency_state_bounds_revision_slots_and_truncation() -> Result<(), &'static str> {
        assert_eq!(
            decode_request(&encode_request(258).ok_or("request")?),
            Some(258)
        );
        assert!(decode_request(&[0; 4]).is_none());
        assert!(decode_request(&[1; 5]).is_none());
        assert!(encode_request(0).is_none());
        let value = State {
            definition: 258,
            revision: 123,
            slots: [
                Some(Slot {
                    level: 1,
                    rank: 2,
                    current: "Attack +2".into(),
                    next: "Attack +3".into(),
                    maximum: "Attack +10".into(),
                    offers: vec!["Defence +2".into(), "Skill +2".into(), "Health +2".into()],
                }),
                None,
            ],
            modify_dust: [1, 2],
            rank_dust: [3; 10],
            reshape_dust: 4,
            clear_dust: 5,
            orbs: 1,
        };
        let bytes = value.encode().ok_or("encode")?;
        assert_eq!(State::decode(&bytes), Some(value.clone()));
        for end in 0..bytes.len() {
            assert!(State::decode(&bytes[..end]).is_none());
        }
        let mut trailing = bytes;
        trailing.push(0);
        assert!(State::decode(&trailing).is_none());
        let mut invalid = value;
        invalid.revision = 0;
        assert!(invalid.encode().is_none());
        invalid.slots = [None, None];
        assert!(invalid.encode().is_some());
        Ok(())
    }
}
