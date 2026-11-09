//! Crystal201189f soul_war_mechanics.lua364-480; lib/quests/soul_war.lua602-643.
//! Map items, durable player cooldowns and HP remain owned by their canonical callers.
pub const FIRE_ITEM: u32 = 33877;
pub const WEEPING_CORPSE: u32 = 33876;
pub const FIRE_POSITIONS: [(u16, u16, u8); 4] = [
    (33743, 31628, 14),
    (33736, 31632, 14),
    (33750, 31632, 14),
    (33742, 31637, 14),
];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorpseStep {
    pub consume: bool,
    pub outfit: Option<(u16, u64)>,
    pub heal: u64,
}
pub fn corpse_step(has_outfit: bool, boss_max: Option<u64>, draw: u8) -> Option<CorpseStep> {
    if !(1..=100).contains(&draw) {
        return None;
    }
    if has_outfit {
        return Some(CorpseStep {
            consume: false,
            outfit: None,
            heal: 0,
        });
    }
    Some(CorpseStep {
        consume: true,
        outfit: Some((286, 14_000)),
        heal: if draw <= 10 {
            boss_max.unwrap_or(0) / 10
        } else {
            0
        },
    })
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireStep {
    Missing,
    Recovering { seconds: u64 },
    Stomp { saved_second: u64, recovery_at: u64 },
}
pub fn fire_step(present: bool, saved_second: Option<u64>, now_second: u64) -> Option<FireStep> {
    use oteryn_gameplay_quests::quest::soul_war::source_items::{
        Kind, SPITE_FIRE_COOLDOWN_SECONDS, SourceItemState,
    };
    if !present {
        return Some(FireStep::Missing);
    }
    let plan = SourceItemState {
        spite_fire_second: saved_second.unwrap_or(0),
        ..Default::default()
    }
    .apply(Kind::SpiteFire, now_second, true)?;
    if !plan.consume {
        return Some(FireStep::Recovering {
            seconds: plan.cooldown_remaining,
        });
    }
    Some(FireStep::Stomp {
        saved_second: plan.after.spite_fire_second,
        recovery_at: now_second.checked_add(SPITE_FIRE_COOLDOWN_SECONDS)?,
    })
}
pub fn fire_expired(item_still_present: bool, boss_present: bool) -> (bool, u32) {
    (
        item_still_present,
        if item_still_present && boss_present {
            10
        } else {
            0
        },
    )
}
/// One accepted private Instance registers the source GlobalEvent independently
/// of its boss and lever. Delayed callbacks are scheduled only after real MINT.
#[derive(Debug, Clone)]
pub struct FireClock {
    next_tick: u64,
    delayed: Vec<(u8, u64)>,
}
impl FireClock {
    pub fn at_registration(now_ms: u64) -> Option<Self> {
        Some(Self {
            next_tick: now_ms.checked_add(14_000)?,
            delayed: Vec::new(),
        })
    }
    pub fn next_due(&self) -> Option<u64> {
        self.delayed
            .iter()
            .map(|(_, due)| *due)
            .chain(Some(self.next_tick))
            .min()
    }
    pub fn claim_tick(&mut self, now_ms: u64) -> bool {
        if now_ms < self.next_tick {
            return false;
        }
        let Some(next) = now_ms.checked_add(14_000) else {
            return false;
        };
        self.next_tick = next;
        true
    }
    pub fn created(&mut self, index: u8, now_ms: u64) -> Option<()> {
        if index >= 4 || self.delayed.len() >= 4 {
            return None;
        }
        self.delayed.push((index, now_ms.checked_add(5_000)?));
        Some(())
    }
    pub fn expiries(&mut self, now_ms: u64) -> Vec<u8> {
        let due = self
            .delayed
            .iter()
            .filter(|(_, at)| *at <= now_ms)
            .map(|(index, _)| *index)
            .collect();
        self.delayed.retain(|(_, at)| *at > now_ms);
        due
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_spite_weeping_corpse_outfit_guard_and_exact_ten_percent() {
        assert_eq!(
            corpse_step(true, Some(300_000), 1),
            Some(CorpseStep {
                consume: false,
                outfit: None,
                heal: 0
            })
        );
        assert_eq!(
            corpse_step(false, Some(300_009), 10),
            Some(CorpseStep {
                consume: true,
                outfit: Some((286, 14_000)),
                heal: 30_000
            })
        );
        assert_eq!(
            corpse_step(false, Some(300_009), 11).map(|step| step.heal),
            Some(0)
        );
        assert_eq!(
            corpse_step(false, None, 1),
            Some(CorpseStep {
                consume: true,
                outfit: Some((286, 14_000)),
                heal: 0
            })
        );
        assert_eq!(corpse_step(false, Some(300_000), 0), None);
        assert_eq!(corpse_step(false, Some(300_000), 101), None);
    }
    #[test]
    fn srx3_spite_fire_stomp_preserves_durable_os_time_boundary() {
        assert_eq!(fire_step(false, None, 1_000), Some(FireStep::Missing));
        assert_eq!(
            fire_step(true, Some(1_000), 1_055),
            Some(FireStep::Recovering { seconds: 1 })
        );
        assert_eq!(
            fire_step(true, Some(1_000), 1_056),
            Some(FireStep::Stomp {
                saved_second: 1_056,
                recovery_at: 1_112
            })
        );
        assert_eq!(
            fire_step(true, None, 1_000),
            Some(FireStep::Stomp {
                saved_second: 1_000,
                recovery_at: 1_056
            })
        );
        // Source absent KV defaults to zero, including its first 56 wall seconds.
        assert_eq!(
            fire_step(true, None, 55),
            Some(FireStep::Recovering { seconds: 1 })
        );
    }
    #[test]
    fn srx3_spite_delayed_fire_lookup_only_empowers_when_item_remains() {
        assert_eq!(fire_expired(true, true), (true, 10));
        assert_eq!(fire_expired(true, false), (true, 0));
        assert_eq!(fire_expired(false, true), (false, 0));
    }
    #[test]
    fn srx3_spite_global_fire_clock_is_not_lever_or_boss_driven() -> Result<(), &'static str> {
        let mut clock = FireClock::at_registration(1000).ok_or("registration clock")?;
        assert_eq!(clock.next_due(), Some(15000));
        assert!(!clock.claim_tick(14999));
        assert!(clock.claim_tick(15000));
        assert!(!clock.claim_tick(15000));
        clock
            .created(2, 15000)
            .ok_or("real successful fire creation")?;
        assert_eq!(clock.next_due(), Some(20000));
        assert!(clock.expiries(19999).is_empty());
        assert_eq!(clock.expiries(20000), vec![2]);
        assert!(clock.expiries(20000).is_empty());
        assert!(
            clock.claim_tick(60000),
            "a late real GlobalEvent fires once"
        );
        assert_eq!(
            clock.next_due(),
            Some(74000),
            "no invented historical catch-up fires"
        );
        assert!(!clock.claim_tick(60000));
        Ok(())
    }
}
