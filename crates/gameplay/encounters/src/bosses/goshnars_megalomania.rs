//! Crystal201189f soul_war.lua:588–599,1456–1474 and mechanics.lua:915–940.
//! Source callback clocks are encounter DATA; actor lookup/spawn/HP remain owners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    Purple,
    Green,
    Blue,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    Green,
    LookupBlue,
    ReturnCapturedPurple(u64),
    ReturnIfGreen,
    RespawnAspect(u64),
}
#[derive(Debug)]
struct Timer {
    id: u64,
    due: u64,
    effect: Effect,
}
#[derive(Debug, Default)]
pub struct Megalomania {
    deaths: u8,
    next: u64,
    timers: Vec<Timer>,
    blue: Option<u64>,
    purple: Option<u64>,
}
impl Megalomania {
    fn schedule(&mut self, now: u64, delay: u64, effect: Effect) -> Option<u64> {
        let due = now.checked_add(delay)?;
        let id = self.next.checked_add(1)?;
        self.timers.try_reserve(1).ok()?;
        self.next = id;
        self.timers.push(Timer { id, due, effect });
        Some(id)
    }
    pub fn aspect_died(
        &mut self,
        now: u64,
        form: Option<Form>,
        respawn_position: u64,
    ) -> Vec<Effect> {
        let mut effects = Vec::new();
        if form == Some(Form::Purple) {
            self.deaths += 1;
            if self.deaths == 4 {
                self.deaths = 0;
                effects.push(Effect::Green);
                self.blue = self.schedule(now, 60_000, Effect::LookupBlue);
                self.purple = self.schedule(now, 70_000, Effect::ReturnIfGreen);
            }
        }
        let _ = self.schedule(now, 5000, Effect::RespawnAspect(respawn_position));
        effects
    }
    pub fn blue_executed(&mut self, now: u64, boss: u64) {
        // The donor assigns global changePurpleEvent here, not the stored
        // SoulWarQuest.changePurpleEvent cancelled by the Purple spawn hook.
        let _ = self.schedule(now, 7000, Effect::ReturnCapturedPurple(boss));
    }
    pub fn spawned(&mut self) {
        self.deaths = 0;
        let blue = self.blue.take();
        let purple = self.purple.take();
        self.timers
            .retain(|timer| Some(timer.id) != blue && Some(timer.id) != purple);
    }
    pub fn advance(&mut self, now: u64) -> Vec<Effect> {
        let mut effects = Vec::new();
        loop {
            let next = self
                .timers
                .iter()
                .enumerate()
                .filter(|(_, timer)| timer.due <= now)
                .min_by_key(|(_, timer)| (timer.due, timer.id))
                .map(|(index, _)| index);
            let Some(index) = next else {
                break;
            };
            effects.push(self.timers.remove(index).effect);
        }
        effects
    }
    pub fn next_due_ms(&self) -> Option<u64> {
        self.timers.iter().map(|timer| timer.due).min()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_megalomania_four_purple_aspects_open_green_then_blue60_purple67() {
        let mut owner = Megalomania::default();
        for _ in 0..3 {
            assert!(owner.aspect_died(100, Some(Form::Purple), 42).is_empty());
        }
        assert_eq!(
            owner.aspect_died(100, Some(Form::Purple), 42),
            vec![Effect::Green]
        );
        assert_eq!(owner.advance(5100), vec![Effect::RespawnAspect(42); 4]);
        assert_eq!(owner.advance(60100), vec![Effect::LookupBlue]);
        owner.blue_executed(60100, 7);
        assert_eq!(owner.advance(67100), vec![Effect::ReturnCapturedPurple(7)]);
        assert_eq!(owner.advance(70100), vec![Effect::ReturnIfGreen]);
    }
    #[test]
    fn srx3_megalomania_green_deaths_do_not_count_but_respawn_after_boss_disappears() {
        let mut owner = Megalomania::default();
        for form in [None, Some(Form::Green), Some(Form::Blue)] {
            assert!(owner.aspect_died(0, form, 9).is_empty());
        }
        assert_eq!(owner.advance(5000), vec![Effect::RespawnAspect(9); 3]);
    }
    #[test]
    fn srx3_megalomania_spawn_stops_stored_phase_handles_but_not_unstored_blue_return() {
        let mut owner = Megalomania::default();
        for _ in 0..4 {
            let _ = owner.aspect_died(0, Some(Form::Purple), 1);
        }
        let _ = owner.advance(60000);
        owner.blue_executed(60000, 7);
        owner.spawned();
        assert_eq!(owner.advance(67000), vec![Effect::ReturnCapturedPurple(7)]);
        assert_eq!(owner.advance(70000), Vec::<Effect>::new());
    }
}

/// Lua soul_war.lua1193–1212 shares this counter across all three forms.
#[derive(Debug, Default, Clone)]
pub struct WhiteClock {
    accumulated_ms: u64,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WhitePulse {
    pub warning: bool,
    pub create: bool,
}
impl WhiteClock {
    pub fn think(&mut self, interval_ms: u64) -> Option<WhitePulse> {
        let accumulated = self.accumulated_ms.checked_add(interval_ms)?;
        let pulse = WhitePulse {
            warning: accumulated == 38_000,
            create: accumulated >= 40_000,
        };
        self.accumulated_ms = if pulse.create { 0 } else { accumulated };
        Some(pulse)
    }
}
#[cfg(test)]
mod white_tests {
    use super::*;
    #[test]
    fn srx3_megalomania_white_warns_at_exact38_and_creates_at40_actual_thinks() {
        let mut clock = WhiteClock::default();
        for _ in 0..37 {
            assert_eq!(clock.think(1000), Some(WhitePulse::default()));
        }
        assert_eq!(
            clock.think(1000),
            Some(WhitePulse {
                warning: true,
                create: false
            })
        );
        assert_eq!(clock.think(1000), Some(WhitePulse::default()));
        assert_eq!(
            clock.think(1000),
            Some(WhitePulse {
                warning: false,
                create: true
            })
        );
        assert_eq!(clock.think(1000), Some(WhitePulse::default()));
    }
    #[test]
    fn srx3_megalomania_white_uses_interval_not_elapsed_time_or_synthetic_warning() {
        let mut clock = WhiteClock::default();
        assert_eq!(
            clock.think(40_000),
            Some(WhitePulse {
                warning: false,
                create: true
            })
        );
        assert_eq!(clock.think(39_000), Some(WhitePulse::default()));
        assert_eq!(
            clock.think(1_000),
            Some(WhitePulse {
                warning: false,
                create: true
            })
        );
    }
    #[test]
    fn srx3_megalomania_white_shared_form_counter_does_not_reset_on_set_type() {
        let mut clock = WhiteClock::default();
        for _form in [Form::Purple, Form::Green] {
            assert_eq!(clock.think(15_000), Some(WhitePulse::default()));
        }
        let _form = Form::Blue;
        assert_eq!(
            clock.think(10_000),
            Some(WhitePulse {
                warning: false,
                create: true
            })
        );
    }
}
