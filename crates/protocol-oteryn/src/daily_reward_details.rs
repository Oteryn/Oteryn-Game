//! Read-only reward catalogue and latest thirty durable account/World claims.
pub const COMMAND_TYPE: u32 = 738;
pub const MAX_BYTES: usize = 1104;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tier {
    pub cosmetic_tokens: u8,
    pub prey_wildcards: u8,
    pub temple_scrolls: u8,
    pub xp_percent: u8,
    pub xp_minutes: u8,
    pub bounty_tokens: u8,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Day {
    pub free: Tier,
    pub premium: Tier,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Receipt {
    pub epoch: u64,
    pub reset_day: i64,
    pub lane_day: u8,
    pub streak: u32,
    pub jokers_spent: u8,
    pub own_character: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Details {
    pub epoch: u64,
    pub days: [Day; 30],
    pub history: [Option<Receipt>; 30],
}
impl Details {
    pub fn empty(epoch: u64) -> Self {
        Self {
            epoch,
            days: [Day::default(); 30],
            history: [None; 30],
        }
    }
    pub fn encode(&self) -> Result<Vec<u8>, &'static str> {
        if self.epoch == 0 {
            return Err("invalid epoch");
        }
        let mut out = b"DRD1".to_vec();
        out.extend(self.epoch.to_le_bytes());
        for day in self.days {
            for t in [day.free, day.premium] {
                if t.cosmetic_tokens > 10
                    || t.prey_wildcards > 2
                    || t.temple_scrolls > 1
                    || t.bounty_tokens > 1
                    || !matches!(t.xp_percent, 0 | 10 | 20 | 25 | 50)
                    || !matches!(t.xp_minutes, 0 | 30 | 45 | 60)
                    || (t.xp_percent == 0) != (t.xp_minutes == 0)
                {
                    return Err("invalid tier");
                }
                out.extend([
                    t.cosmetic_tokens,
                    t.prey_wildcards,
                    t.temple_scrolls,
                    t.xp_percent,
                    t.xp_minutes,
                    t.bounty_tokens,
                ]);
            }
        }
        out.push(self.history.iter().flatten().count() as u8);
        let mut previous = self.epoch;
        let mut ended = false;
        for receipt in self.history {
            match receipt {
                None => ended = true,
                Some(r) => {
                    if ended
                        || r.epoch == 0
                        || r.epoch > previous
                        || !(1..=30).contains(&r.lane_day)
                        || r.streak > 100000
                        || r.jokers_spent > 3
                    {
                        return Err("invalid history");
                    }
                    previous = r.epoch;
                    out.extend(r.epoch.to_le_bytes());
                    out.extend(r.reset_day.to_le_bytes());
                    out.push(r.lane_day);
                    out.extend(r.streak.to_le_bytes());
                    out.extend([r.jokers_spent, u8::from(r.own_character)]);
                }
            }
        }
        Ok(out)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > MAX_BYTES || !bytes.starts_with(b"DRD1") {
            return Err("invalid details");
        }
        let mut at = 4;
        fn take<const N: usize>(bytes: &[u8], at: &mut usize) -> Result<[u8; N], &'static str> {
            let result = bytes
                .get(*at..*at + N)
                .ok_or("truncated details")?
                .try_into()
                .map_err(|_| "truncated details")?;
            *at += N;
            Ok(result)
        }
        let mut value = Self::empty(u64::from_le_bytes(take(bytes, &mut at)?));
        for day in &mut value.days {
            for t in [&mut day.free, &mut day.premium] {
                let [
                    cosmetic_tokens,
                    prey_wildcards,
                    temple_scrolls,
                    xp_percent,
                    xp_minutes,
                    bounty_tokens,
                ] = take(bytes, &mut at)?;
                *t = Tier {
                    cosmetic_tokens,
                    prey_wildcards,
                    temple_scrolls,
                    xp_percent,
                    xp_minutes,
                    bounty_tokens,
                };
            }
        }
        let [count] = take(bytes, &mut at)?;
        if count > 30 {
            return Err("too much history");
        }
        for slot in &mut value.history[..usize::from(count)] {
            let epoch = u64::from_le_bytes(take(bytes, &mut at)?);
            let reset_day = i64::from_le_bytes(take(bytes, &mut at)?);
            let [lane_day] = take(bytes, &mut at)?;
            let streak = u32::from_le_bytes(take(bytes, &mut at)?);
            let [jokers_spent, own] = take(bytes, &mut at)?;
            if own > 1 {
                return Err("invalid owner");
            }
            *slot = Some(Receipt {
                epoch,
                reset_day,
                lane_day,
                streak,
                jokers_spent,
                own_character: own == 1,
            });
        }
        if at != bytes.len() {
            return Err("trailing details");
        }
        value.encode()?;
        Ok(value)
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn reward_details_round_trip_rejects_truncation_and_bad_history()
    -> Result<(), Box<dyn std::error::Error>> {
        let details = super::Details::empty(1);
        let bytes = details.encode()?;
        assert_eq!(super::Details::decode(&bytes)?, details);
        assert!(super::Details::decode(&bytes[..bytes.len() - 1]).is_err());
        let mut invalid = details;
        invalid.history[0] = Some(super::Receipt {
            epoch: 2,
            reset_day: 0,
            lane_day: 1,
            streak: 1,
            jokers_spent: 0,
            own_character: true,
        });
        assert!(invalid.encode().is_err());
        invalid = details;
        invalid.days[0].premium.xp_percent = 50;
        assert!(invalid.encode().is_err());
        Ok(())
    }
}
