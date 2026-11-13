//! Crystal201189f lib/quests/soul_war.lua1398-1450,1545-1579.
//! Values only: HP, icons and persistent Remorse remain with their existing owners.
pub const MAX_TORMENT: u8 = 36;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Warning {
    Begins,
    Unbearable,
    Tearing,
    Killing,
    Lethal,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconChange {
    Keep,
    Set(u16),
    Remove,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pulse {
    pub damage: u32,
    pub warning: Option<Warning>,
    pub icon: IconChange,
    pub effect: bool,
}
impl Default for Pulse {
    fn default() -> Self {
        Self {
            damage: 0,
            warning: None,
            icon: IconChange::Keep,
            effect: false,
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerCounter {
    value: u8,
}
impl PlayerCounter {
    pub fn from_current(value: u8) -> Option<Self> {
        (value <= MAX_TORMENT).then_some(Self { value })
    }
    pub const fn value(self) -> u8 {
        self.value
    }
    pub fn pulse(&mut self, maximum: u8, megalomania_present: bool) -> Pulse {
        if !megalomania_present {
            return Pulse {
                icon: self.reset(),
                ..Pulse::default()
            };
        }
        let old = self.value;
        let warning = match old {
            5 => Some(Warning::Begins),
            15 => Some(Warning::Unbearable),
            24 => Some(Warning::Tearing),
            30 => Some(Warning::Killing),
            36 => Some(Warning::Lethal),
            _ => None,
        };
        if old > maximum {
            return Pulse {
                warning,
                ..Pulse::default()
            };
        }
        self.value = old.saturating_add(1).min(maximum).min(MAX_TORMENT);
        let damage = match old {
            0 => 0,
            1..=23 => u32::from(old) * 35,
            24..=36 => [
                1400, 1600, 1800, 2200, 2400, 2600, 3000, 3400, 3800, 4200, 4800, 5200, 5600,
            ][usize::from(old - 24)],
            _ => 0,
        };
        Pulse {
            damage,
            warning,
            icon: IconChange::Set(u16::from(self.value)),
            effect: damage > 0,
        }
    }
    pub fn remove(&mut self, count: u8) -> IconChange {
        self.value = self.value.saturating_sub(count);
        if self.value == 0 {
            IconChange::Remove
        } else {
            IconChange::Set(u16::from(self.value))
        }
    }
    pub fn reset(&mut self) -> IconChange {
        self.value = 0;
        IconChange::Remove
    }
}
/// Source one shared closure gate, not one timer per symbol/boss.
#[derive(Debug, Clone, Copy, Default)]
pub struct Cadence {
    last_ms: Option<u64>,
}
impl Cadence {
    pub fn claim(&mut self, now_ms: u64, interval_ms: u64) -> bool {
        let now_ms = now_ms / 1_000 * 1_000;
        if self
            .last_ms
            .is_some_and(|last| now_ms.saturating_sub(last) < interval_ms)
        {
            return false;
        }
        self.last_ms = Some(now_ms);
        true
    }
    pub const fn last_ms(self) -> Option<u64> {
        self.last_ms
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_torment_source_uses_old_count_for_damage_and_warnings_then_clamps()
    -> Result<(), &'static str> {
        let mut counter = PlayerCounter::default();
        let first = counter.pulse(36, true);
        assert_eq!(counter.value(), 1);
        assert_eq!(
            (first.damage, first.effect, first.icon),
            (0, false, IconChange::Set(1))
        );
        for (old, damage, warning) in [
            (1, 35, None),
            (5, 175, Some(Warning::Begins)),
            (15, 525, Some(Warning::Unbearable)),
            (24, 1400, Some(Warning::Tearing)),
            (30, 3000, Some(Warning::Killing)),
            (36, 5600, Some(Warning::Lethal)),
        ] {
            let mut counter = PlayerCounter::from_current(old).ok_or("source counter")?;
            let pulse = counter.pulse(36, true);
            assert_eq!(
                (pulse.damage, pulse.warning, pulse.effect),
                (damage, warning, true)
            );
            assert_eq!(counter.value(), old.saturating_add(1).min(36));
            assert_eq!(pulse.icon, IconChange::Set(u16::from(counter.value())));
        }
        Ok(())
    }
    #[test]
    fn srx3_torment_source_ignored_hatred_name_requires_literal_megalomania()
    -> Result<(), &'static str> {
        // Source signature has4args; Symbol's fifth "Goshnar's Hatred" is ignored.
        let mut counter = PlayerCounter::from_current(20).ok_or("source counter")?;
        let pulse = counter.pulse(30, false);
        assert_eq!(counter.value(), 0);
        assert_eq!(
            (pulse.damage, pulse.icon, pulse.warning),
            (0, IconChange::Remove, None)
        );
        let mut counter = PlayerCounter::from_current(36).ok_or("source counter")?;
        let pulse = counter.pulse(30, true);
        assert_eq!(counter.value(), 36);
        assert_eq!(
            (pulse.damage, pulse.icon, pulse.warning),
            (0, IconChange::Keep, Some(Warning::Lethal))
        );
        Ok(())
    }
    #[test]
    fn srx3_torment_removal_and_login_reset_remove_icon_at_zero() -> Result<(), &'static str> {
        let mut counter = PlayerCounter::from_current(12).ok_or("source counter")?;
        assert_eq!(counter.remove(5), IconChange::Set(7));
        assert_eq!(counter.value(), 7);
        assert_eq!(counter.reset(), IconChange::Remove);
        assert_eq!(counter.value(), 0);
        let mut counter = PlayerCounter::from_current(5).ok_or("source counter")?;
        assert_eq!(counter.remove(5), IconChange::Remove);
        assert_eq!(counter.value(), 0);
        Ok(())
    }
    #[test]
    fn srx3_torment_shared_wall_second_gate_is_not_elapsed_think_time() {
        let mut clock = Cadence::default();
        assert!(
            clock.claim(100, 3000),
            "source UTC makes first invocation eligible"
        );
        assert!(!clock.claim(2999, 3000));
        assert!(clock.claim(3000, 3000));
        assert!(!clock.claim(12_999, 10_000));
        assert!(
            clock.claim(13_000, 10_000),
            "Mego's10s period shares Symbol's gate"
        );
        assert!(!clock.claim(15_999, 3000));
        assert!(clock.claim(16_000, 3000));
        assert!(!clock.claim(25_999, 10_000));
        assert!(clock.claim(26_000, 10_000));
        assert_eq!(clock.last_ms(), Some(26_000));
    }
}
