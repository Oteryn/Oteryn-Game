//! Crystal 201189f soul_war.lua:1477–1493; goshnars_cruelty.lua:138–145.
//! Mutable encounter DATA; HP, defense and Item writers remain canonical owners.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Cruelty {
    thought_ms: u64,
    last_maw_seconds: u64,
    drain: Option<u64>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefenseIncrease {
    pub defense: u16,
    pub drain: u64,
}
impl Cruelty {
    pub fn think(&mut self, now_seconds: u64, interval_ms: u64) -> Option<DefenseIncrease> {
        self.thought_ms = self.thought_ms.checked_add(interval_ms)?;
        if self.thought_ms < 15_000 || now_seconds < self.last_maw_seconds.checked_add(15)? {
            return None;
        }
        // Lua seeds an absent drain KV at the defense change (2), then adds 1.
        let drain = self.drain.unwrap_or(2).checked_add(1)?;
        self.drain = Some(drain);
        self.last_maw_seconds = now_seconds;
        Some(DefenseIncrease { defense: 2, drain })
    }
    pub fn mortal_essence_used(&mut self, now_seconds: u64) -> Option<()> {
        self.record_mortal_essence_time(now_seconds)?;
        self.drain = self.drain.map(|value| value.saturating_sub(1));
        Some(())
    }

    /// The source stores its shared timestamp even when no named boss has a type.
    pub fn record_mortal_essence_time(&mut self, now_seconds: u64) -> Option<()> {
        self.last_maw_seconds = now_seconds.checked_add(15)?;
        Some(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_cruelty_first_native_fifteen_seconds_use_literal_drain_seed_two_then_increment() {
        let mut owner = Cruelty::default();
        for tick in 1..15 {
            assert_eq!(owner.think(1_800_000_000 + tick, 1000), None);
        }
        assert_eq!(
            owner.think(1_800_000_015, 1000),
            Some(DefenseIncrease {
                defense: 2,
                drain: 3
            })
        );
        assert_eq!(owner.think(1_800_000_029, 1000), None);
        assert_eq!(
            owner.think(1_800_000_030, 1000),
            Some(DefenseIncrease {
                defense: 2,
                drain: 4
            })
        );
    }
    #[test]
    fn srx3_cruelty_maw_future_timestamp_delays_next_increase_thirty_seconds() -> Result<(), String>
    {
        let mut owner = Cruelty::default();
        for tick in 1..=15 {
            let _ = owner.think(1_800_000_000 + tick, 1000);
        }
        owner
            .mortal_essence_used(1_800_000_020)
            .ok_or("source timestamp")?;
        assert_eq!(owner.think(1_800_000_049, 1000), None);
        assert_eq!(
            owner.think(1_800_000_050, 1000),
            Some(DefenseIncrease {
                defense: 2,
                drain: 3
            })
        );
        Ok(())
    }
}
