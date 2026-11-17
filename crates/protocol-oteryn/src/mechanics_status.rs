//! CL6 read-only observations of existing death, potion, party and Forge owners.
use crate::condition_status::ConditionStatus;
pub const CAPABILITY_MECHANICS_STATUS_V1: u32 = 384;
pub const COMMAND_TYPE_MECHANICS_STATUS_QUERY: u32 = 669;
pub const MAX_REPLY_BYTES: usize = 65_536;
pub const MAX_DEATHS: usize = 5;
pub const MAX_MEMBERS: usize = 50;
pub const MAX_TIMERS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeathConsumption {
    pub at_unix_ms: i64,
    /// Receipt before-minus-after; includes Twist of Fate only when actually consumed.
    pub consumed: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectTimer {
    pub key: String,
    pub remaining_ms: u64,
    pub online_only: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberEffects {
    pub character: [u8; 16],
    pub name: String,
    /// None means no qualified live owner in this Channel, never "no effects".
    pub conditions: Option<ConditionStatus>,
    /// Train, Protect, Enchant, Heal, Enlighten Party; zero means inactive.
    pub party_spell_remaining_ms: [u32; 5],
    /// Damage reduction, auto attack, spell damage, healing (%); then Mantra points.
    pub bonuses: [u32; 5],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MechanicsStatus {
    pub observed_unix_ms: i64,
    pub dust: u32,
    pub dust_limit: u32,
    pub deaths: Vec<DeathConsumption>,
    pub timers: Vec<EffectTimer>,
    pub members: Vec<MemberEffects>,
}
fn text(out: &mut Vec<u8>, value: &str, max: usize) -> Option<()> {
    if value.is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return None;
    }
    out.push(u8::try_from(value.len()).ok()?);
    out.extend_from_slice(value.as_bytes());
    Some(())
}
impl MechanicsStatus {
    pub fn encode(&self) -> Option<Vec<u8>> {
        if self.observed_unix_ms < 0
            || self.dust_limit == 0
            || self.dust > self.dust_limit
            || self.deaths.len() > MAX_DEATHS
            || self.timers.len() > MAX_TIMERS
            || self.members.len() > MAX_MEMBERS
            || self
                .deaths
                .windows(2)
                .any(|v| v[0].at_unix_ms < v[1].at_unix_ms)
            || self.timers.windows(2).any(|v| v[0].key >= v[1].key)
            || self.members.iter().enumerate().any(|(i, m)| {
                self.members[..i]
                    .iter()
                    .any(|prior| prior.character == m.character)
            })
        {
            return None;
        }
        let mut out = Vec::new();
        out.extend_from_slice(&self.observed_unix_ms.to_le_bytes());
        out.extend_from_slice(&self.dust.to_le_bytes());
        out.extend_from_slice(&self.dust_limit.to_le_bytes());
        out.push(u8::try_from(self.deaths.len()).ok()?);
        for death in &self.deaths {
            if death.at_unix_ms < 0
                || death.at_unix_ms > self.observed_unix_ms
                || death.consumed.len() > 32
                || death.consumed.windows(2).any(|v| v[0] >= v[1])
            {
                return None;
            }
            out.extend_from_slice(&death.at_unix_ms.to_le_bytes());
            out.push(u8::try_from(death.consumed.len()).ok()?);
            for key in &death.consumed {
                text(&mut out, key, 128)?;
            }
        }
        out.push(u8::try_from(self.timers.len()).ok()?);
        for timer in &self.timers {
            if timer.remaining_ms == 0 {
                return None;
            }
            text(&mut out, &timer.key, 128)?;
            out.extend_from_slice(&timer.remaining_ms.to_le_bytes());
            out.push(u8::from(timer.online_only));
        }
        out.push(u8::try_from(self.members.len()).ok()?);
        for member in &self.members {
            if member.conditions.is_none()
                && (member.party_spell_remaining_ms != [0; 5] || member.bonuses != [0; 5])
            {
                return None;
            }
            out.extend_from_slice(&member.character);
            text(&mut out, &member.name, 128)?;
            out.push(u8::from(member.conditions.is_some()));
            if let Some(conditions) = &member.conditions {
                let bytes = conditions.encode()?;
                out.extend_from_slice(&u16::try_from(bytes.len()).ok()?.to_le_bytes());
                out.extend_from_slice(&bytes);
            }
            for remaining in member
                .party_spell_remaining_ms
                .into_iter()
                .chain(member.bonuses)
            {
                out.extend_from_slice(&remaining.to_le_bytes());
            }
        }
        (out.len() <= MAX_REPLY_BYTES).then_some(out)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_REPLY_BYTES {
            return None;
        }
        let mut r = Reader(bytes);
        let observed_unix_ms = i64::from_le_bytes(r.take()?);
        let dust = u32::from_le_bytes(r.take()?);
        let dust_limit = u32::from_le_bytes(r.take()?);
        let count = r.count(MAX_DEATHS)?;
        let mut deaths = Vec::with_capacity(count);
        for _ in 0..count {
            let at_unix_ms = i64::from_le_bytes(r.take()?);
            let count = r.count(32)?;
            let mut consumed = Vec::with_capacity(count);
            for _ in 0..count {
                consumed.push(r.text()?);
            }
            deaths.push(DeathConsumption {
                at_unix_ms,
                consumed,
            });
        }
        let count = r.count(MAX_TIMERS)?;
        let mut timers = Vec::with_capacity(count);
        for _ in 0..count {
            timers.push(EffectTimer {
                key: r.text()?,
                remaining_ms: u64::from_le_bytes(r.take()?),
                online_only: r.flag()?,
            });
        }
        let count = r.count(MAX_MEMBERS)?;
        let mut members = Vec::with_capacity(count);
        for _ in 0..count {
            let character = r.take()?;
            let name = r.text()?;
            let conditions = if r.flag()? {
                let size = usize::from(u16::from_le_bytes(r.take()?));
                let bytes = r.bytes(size)?;
                Some(ConditionStatus::decode(bytes)?)
            } else {
                None
            };
            let mut party_spell_remaining_ms = [0; 5];
            for remaining in &mut party_spell_remaining_ms {
                *remaining = u32::from_le_bytes(r.take()?);
            }
            let mut bonuses = [0; 5];
            for bonus in &mut bonuses {
                *bonus = u32::from_le_bytes(r.take()?);
            }
            members.push(MemberEffects {
                character,
                name,
                conditions,
                party_spell_remaining_ms,
                bonuses,
            });
        }
        if !r.0.is_empty() {
            return None;
        }
        let value = Self {
            observed_unix_ms,
            dust,
            dust_limit,
            deaths,
            timers,
            members,
        };
        value.encode()?;
        Some(value)
    }
}
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn bytes(&mut self, size: usize) -> Option<&'a [u8]> {
        let (a, b) = self.0.split_at_checked(size)?;
        self.0 = b;
        Some(a)
    }
    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.bytes(N)?.try_into().ok()
    }
    fn count(&mut self, max: usize) -> Option<usize> {
        let n = usize::from(self.take::<1>()?[0]);
        (n <= max).then_some(n)
    }
    fn flag(&mut self) -> Option<bool> {
        match self.take::<1>()?[0] {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }
    fn text(&mut self) -> Option<String> {
        let n = self.count(128)?;
        Some(std::str::from_utf8(self.bytes(n)?).ok()?.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cl6_status_codec_bounds_and_all_fields() -> Result<(), &'static str> {
        let value = MechanicsStatus {
            observed_unix_ms: 100,
            dust: 73,
            dust_limit: 100,
            deaths: vec![DeathConsumption {
                at_unix_ms: 50,
                consumed: vec!["oteryn:blessing.twist_of_fate".into()],
            }],
            timers: vec![EffectTimer {
                key: "wealth_duplex".into(),
                remaining_ms: 1234,
                online_only: true,
            }],
            members: vec![MemberEffects {
                bonuses: [0; 5],
                party_spell_remaining_ms: [0; 5],
                character: [1; 16],
                name: "Friend".into(),
                conditions: Some(ConditionStatus::default()),
            }],
        };
        let bytes = value.encode().ok_or("encode")?;
        assert_eq!(MechanicsStatus::decode(&bytes), Some(value.clone()));
        for end in 0..bytes.len() {
            assert!(MechanicsStatus::decode(&bytes[..end]).is_none());
        }
        let mut trailing = bytes;
        trailing.push(0);
        assert!(MechanicsStatus::decode(&trailing).is_none());
        let mut bad = value.clone();
        bad.dust = 101;
        assert!(bad.encode().is_none());
        let mut bad = value.clone();
        bad.timers[0].remaining_ms = 0;
        assert!(bad.encode().is_none());
        let mut bad = value.clone();
        bad.members.push(bad.members[0].clone());
        assert!(bad.encode().is_none());
        let mut bad = value.clone();
        bad.members[0].name = "bad\nname".into();
        assert!(bad.encode().is_none());
        let mut bad = value;
        let duplicate = bad.deaths[0].consumed[0].clone();
        bad.deaths[0].consumed.push(duplicate);
        assert!(bad.encode().is_none());
        Ok(())
    }
    #[test]
    fn cl6_status_largest_owner_bounds_fit_one_reply() -> Result<(), &'static str> {
        let conditions = ConditionStatus {
            conditions: (0..64)
                .map(|sub_id| crate::condition_status::ActiveCondition {
                    icon: 20,
                    sub_id,
                    remaining_ms: -1,
                })
                .collect(),
        };
        let keys: Vec<String> = (0..32)
            .map(|n| format!("{n:03}{}", "x".repeat(125)))
            .collect();
        let value = MechanicsStatus {
            observed_unix_ms: 10,
            dust: 100,
            dust_limit: 100,
            deaths: (5..10)
                .rev()
                .map(|at_unix_ms| DeathConsumption {
                    at_unix_ms,
                    consumed: keys.clone(),
                })
                .collect(),
            timers: keys[..MAX_TIMERS]
                .iter()
                .map(|key| EffectTimer {
                    key: key.clone(),
                    remaining_ms: u64::MAX,
                    online_only: false,
                })
                .collect(),
            members: (0..50)
                .map(|n| MemberEffects {
                    bonuses: [0; 5],
                    character: [n; 16],
                    name: "n".repeat(128),
                    conditions: Some(conditions.clone()),
                    party_spell_remaining_ms: [120_000; 5],
                })
                .collect(),
        };
        let bytes = value.encode().ok_or("largest encode")?;
        assert!(bytes.len() > 60_000 && bytes.len() <= MAX_REPLY_BYTES);
        assert_eq!(MechanicsStatus::decode(&bytes), Some(value.clone()));
        let mut excessive = value;
        excessive.members.push(MemberEffects {
            bonuses: [0; 5],
            character: [99; 16],
            name: "Extra".into(),
            conditions: None,
            party_spell_remaining_ms: [0; 5],
        });
        assert!(excessive.encode().is_none());
        Ok(())
    }
}
