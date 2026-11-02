//! WB2/P02: two personal boss slots under existing BOSS_ENCOUNTER_V1.
pub const COMMAND_TYPE_BOSSTIARY_SLOTS: u32 = 491;
pub const MAX_REQUEST_BYTES: usize = 130;
pub const MAX_REPLY_BYTES: usize = 284;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BossSlotRequest {
    Query,
    Equip { slot: u8, boss: String },
    Remove { slot: u8 },
}
impl BossSlotRequest {
    pub fn decode(bytes: &[u8]) -> Option<Self> {
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
/// Status: 0=query/success, 1=locked/ineligible, 2=occupied/duplicate,
/// 3=cooldown, 4=insufficient money, 5=change cannot fit. Rejections write nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BossSlotReply {
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
        let reply = Self {
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
