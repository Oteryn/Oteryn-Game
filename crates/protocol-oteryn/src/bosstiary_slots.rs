//! WB2/P02: two personal boss slots under existing BOSS_ENCOUNTER_V1.
pub const COMMAND_TYPE_BOSSTIARY_SLOTS: u32 = 491;
pub const MAX_REQUEST_BYTES: usize = 131;
pub const MAX_REPLY_BYTES: usize = 440;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BossSlotRequest {
    Query,
    Detailed(Box<Self>),
    Equip { slot: u8, boss: String },
    Remove { slot: u8 },
}
impl BossSlotRequest {
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_REQUEST_BYTES {
            return None;
        }
        if let Some(inner) = bytes.strip_prefix(&[3]) {
            if inner.first() == Some(&3) {
                return None;
            }
            return Self::decode(inner).map(|r| Self::Detailed(Box::new(r)));
        }
        match bytes {
            [0] => Some(Self::Query),
            [2, slot @ 1..=2] => Some(Self::Remove { slot: *slot }),
            [1, slot @ 1..=2, key @ ..] if !key.is_empty() && key.len() <= 128 => {
                let boss = std::str::from_utf8(key).ok()?;
                if !boss.starts_with("oteryn:creature.")
                    || !boss
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b":._".contains(&c))
                {
                    return None;
                }
                Some(Self::Equip {
                    slot: *slot,
                    boss: boss.into(),
                })
            }
            _ => None,
        }
    }
    pub fn encode(&self) -> Option<Vec<u8>> {
        let bytes = match self {
            Self::Query => vec![0],
            Self::Detailed(inner) => {
                if matches!(inner.as_ref(), Self::Detailed(_)) {
                    return None;
                }
                let mut v = vec![3];
                v.extend(inner.encode()?);
                v
            }
            Self::Remove { slot } => vec![2, *slot],
            Self::Equip { slot, boss } => {
                let mut v = vec![1, *slot];
                v.extend_from_slice(boss.as_bytes());
                v
            }
        };
        Self::decode(&bytes).map(|_| bytes)
    }
}
/// Optional read-only explanation requested by prefix 3. Old receipts stay byte-identical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotDetails {
    pub base_bonus: u16,
    pub bonuses: [u16; 2],
    pub boosted: String,
    pub now: u64,
}
/// Status: 0=query/success, 1=locked/ineligible, 2=occupied/duplicate,
/// 3=cooldown, 4=insufficient money, 5=change cannot fit. Rejections write nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BossSlotReply {
    pub details: Option<SlotDetails>,
    pub status: u8,
    pub unlocked: u8,
    pub bonus: u16,
    pub points: u32,
    pub remove_fee: u64,
    pub ready_at_ms: u64,
    pub bosses: [String; 2],
}
impl BossSlotReply {
    pub fn encode(&self) -> Option<Vec<u8>> {
        if self.status > 5 || self.unlocked > 3 || self.bosses.iter().any(|s| s.len() > 128) {
            return None;
        }
        let mut v = vec![self.status, self.unlocked];
        v.extend(self.bonus.to_be_bytes());
        v.extend(self.points.to_be_bytes());
        v.extend(self.remove_fee.to_be_bytes());
        v.extend(self.ready_at_ms.to_be_bytes());
        for boss in &self.bosses {
            v.push(boss.len() as u8);
            v.extend(boss.as_bytes());
        }
        if let Some(d) = &self.details {
            if d.boosted.len() > 128
                || (!d.boosted.is_empty()
                    && !matches!(
                        BossSlotRequest::decode(
                            &[vec![1, 1], d.boosted.as_bytes().to_vec()].concat()
                        ),
                        Some(BossSlotRequest::Equip { .. })
                    ))
                || d.now > i64::MAX as u64
            {
                return None;
            }
            v.extend(b"BD1");
            v.extend(d.base_bonus.to_be_bytes());
            for bonus in d.bonuses {
                v.extend(bonus.to_be_bytes());
            }
            v.extend(d.now.to_be_bytes());
            v.push(d.boosted.len() as u8);
            v.extend(d.boosted.as_bytes());
        }
        Some(v)
    }
    pub fn decode(v: &[u8]) -> Option<Self> {
        if !(26..=MAX_REPLY_BYTES).contains(&v.len()) {
            return None;
        }
        let mut offset = 24;
        let mut bosses = [String::new(), String::new()];
        for boss in &mut bosses {
            let n = usize::from(*v.get(offset)?);
            offset += 1;
            *boss = std::str::from_utf8(v.get(offset..offset + n)?).ok()?.into();
            offset += n;
        }
        let details = if offset == v.len() {
            None
        } else {
            if v.get(offset..offset + 3)? != b"BD1" {
                return None;
            }
            offset += 3;
            let base_bonus = u16::from_be_bytes(v.get(offset..offset + 2)?.try_into().ok()?);
            offset += 2;
            let mut bonuses = [0; 2];
            for bonus in &mut bonuses {
                *bonus = u16::from_be_bytes(v.get(offset..offset + 2)?.try_into().ok()?);
                offset += 2;
            }
            let now = u64::from_be_bytes(v.get(offset..offset + 8)?.try_into().ok()?);
            offset += 8;
            let n = usize::from(*v.get(offset)?);
            offset += 1;
            let boosted = std::str::from_utf8(v.get(offset..offset + n)?).ok()?.into();
            offset += n;
            Some(SlotDetails {
                base_bonus,
                bonuses,
                boosted,
                now,
            })
        };
        let reply = Self {
            details,
            status: v[0],
            unlocked: v[1],
            bonus: u16::from_be_bytes(v[2..4].try_into().ok()?),
            points: u32::from_be_bytes(v[4..8].try_into().ok()?),
            remove_fee: u64::from_be_bytes(v[8..16].try_into().ok()?),
            ready_at_ms: u64::from_be_bytes(v[16..24].try_into().ok()?),
            bosses,
        };
        (offset == v.len() && reply.encode().as_deref() == Some(v)).then_some(reply)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boss_slot_wire_strict_roundtrip() -> Result<(), &'static str> {
        for r in [
            BossSlotRequest::Query,
            BossSlotRequest::Remove { slot: 2 },
            BossSlotRequest::Equip {
                slot: 1,
                boss: "oteryn:creature.ferumbras".into(),
            },
        ] {
            assert_eq!(
                BossSlotRequest::decode(&r.encode().ok_or("encode")?),
                Some(r)
            );
        }
        for bytes in [&[][..], &[0, 0], &[2, 0], &[2, 3], &[1, 1], &[1, 1, 255]] {
            assert!(BossSlotRequest::decode(bytes).is_none());
        }
        let r = BossSlotReply {
            details: None,
            status: 0,
            unlocked: 3,
            bonus: 50,
            points: 250,
            remove_fee: 100000,
            ready_at_ms: 0,
            bosses: [String::new(), "oteryn:creature.ferumbras".into()],
        };
        assert_eq!(BossSlotReply::decode(&r.encode().ok_or("reply")?), Some(r));
        Ok(())
    }
}

#[cfg(test)]
#[path = "bosstiary_slots/details_tests.rs"]
mod details_tests;
