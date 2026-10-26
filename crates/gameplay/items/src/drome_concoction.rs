//! Global Tibia rules for the Drome Cube and its twenty concoctions.
//!
//! This module deliberately only produces owner changes.  The item authority must commit a
//! cube's retirement and the returned reward in one transaction; callers must never retire the
//! cube and subsequently attempt a fallible mint.

/// Tibia item id of the Drome Cube.
pub const DROME_CUBE_ITEM_ID: u32 = 36_827;
/// A Global cube costs this many Drome points at the existing reward owner.
pub const DROME_CUBE_PRICE: u32 = 2_500;
pub const EFFECT_DURATION_SECONDS: u64 = 60 * 60;
pub const INDIVIDUAL_COOLDOWN_SECONDS: u64 = 24 * 60 * 60;
/// Global's maximum hunting stamina (42 hours).
pub const STAMINA_CAP_MINUTES: u16 = 42 * 60;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
pub enum Concoction {
    KooldownAid,
    StrikeEnhancement,
    StaminaExtension,
    CharmUpgrade,
    WealthDuplex,
    BestiaryBetterment,
    FireResilience,
    IceResilience,
    EarthResilience,
    EnergyResilience,
    HolyResilience,
    DeathResilience,
    PhysicalResilience,
    FireAmplification,
    IceAmplification,
    EarthAmplification,
    EnergyAmplification,
    HolyAmplification,
    DeathAmplification,
    PhysicalAmplification,
}

pub const ALL_CONCOCTIONS: [Concoction; 20] = [
    Concoction::KooldownAid,
    Concoction::StrikeEnhancement,
    Concoction::StaminaExtension,
    Concoction::CharmUpgrade,
    Concoction::WealthDuplex,
    Concoction::BestiaryBetterment,
    Concoction::FireResilience,
    Concoction::IceResilience,
    Concoction::EarthResilience,
    Concoction::EnergyResilience,
    Concoction::HolyResilience,
    Concoction::DeathResilience,
    Concoction::PhysicalResilience,
    Concoction::FireAmplification,
    Concoction::IceAmplification,
    Concoction::EarthAmplification,
    Concoction::EnergyAmplification,
    Concoction::HolyAmplification,
    Concoction::DeathAmplification,
    Concoction::PhysicalAmplification,
];

impl Concoction {
    /// Definition selected by a cube. These are the twenty Global/Crystal catalogue ids.
    pub const fn item_id(self) -> u32 {
        36_723 + self as u32
    }

    pub const fn from_item_id(id: u32) -> Option<Self> {
        let index = match id.checked_sub(36_723) {
            Some(v) if v < 20 => v,
            _ => return None,
        };
        Some(ALL_CONCOCTIONS[index as usize])
    }

    pub const fn effect(self) -> Effect {
        use Concoction::*;
        match self {
            KooldownAid => Effect::ResetSpellCooldowns,
            StaminaExtension => Effect::AddStaminaMinutes(60),
            StrikeEnhancement => Effect::CriticalChancePercentagePoints(5),
            CharmUpgrade => Effect::OffensiveCharmTriggerPercentagePoints(5),
            WealthDuplex => Effect::RegularLootMultiplier(2),
            BestiaryBetterment => Effect::BestiaryProgressMultiplier(2),
            FireResilience => Effect::Resilience(Element::Fire, 8),
            IceResilience => Effect::Resilience(Element::Ice, 8),
            EarthResilience => Effect::Resilience(Element::Earth, 8),
            EnergyResilience => Effect::Resilience(Element::Energy, 8),
            HolyResilience => Effect::Resilience(Element::Holy, 8),
            DeathResilience => Effect::Resilience(Element::Death, 8),
            PhysicalResilience => Effect::Resilience(Element::Physical, 8),
            FireAmplification => Effect::Pierce(Element::Fire, 16),
            IceAmplification => Effect::Pierce(Element::Ice, 16),
            EarthAmplification => Effect::Pierce(Element::Earth, 16),
            EnergyAmplification => Effect::Pierce(Element::Energy, 16),
            HolyAmplification => Effect::Pierce(Element::Holy, 16),
            DeathAmplification => Effect::Pierce(Element::Death, 16),
            PhysicalAmplification => Effect::Pierce(Element::Physical, 16),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Element {
    Fire,
    Ice,
    Earth,
    Energy,
    Holy,
    Death,
    Physical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effect {
    ResetSpellCooldowns,
    AddStaminaMinutes(u16),
    CriticalChancePercentagePoints(u8),
    OffensiveCharmTriggerPercentagePoints(u8),
    RegularLootMultiplier(u8),
    BestiaryProgressMultiplier(u8),
    Resilience(Element, u8),
    Pierce(Element, u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimedEffect {
    pub concoction: Concoction,
    pub active_until: u64,
    pub ready_at: u64,
}

/// Durable, per-character view consumed by the existing reward owners. Timestamps are absolute
/// Unix seconds: effect time and cooldown time therefore continue while the character is offline
/// and survive a process restart. Different concoctions coexist; only another activation of the
/// same concoction replaces its row.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConcoctionEffects(Vec<TimedEffect>);

impl ConcoctionEffects {
    pub fn from_durable(effects: impl IntoIterator<Item = TimedEffect>) -> Self {
        let mut value = Self::default();
        for effect in effects {
            value.upsert(effect);
        }
        value
    }

    pub fn ready_at(&self, concoction: Concoction) -> Option<u64> {
        self.0
            .iter()
            .find(|effect| effect.concoction == concoction)
            .map(|effect| effect.ready_at)
    }

    pub fn activate(
        &mut self,
        concoction: Concoction,
        now: u64,
    ) -> Result<TimedEffect, ActivationError> {
        let effect = activate(concoction, now, self.ready_at(concoction))?;
        self.upsert(effect);
        Ok(effect)
    }

    pub fn is_active(&self, concoction: Concoction, now: u64) -> bool {
        self.0
            .iter()
            .any(|effect| effect.concoction == concoction && now < effect.active_until)
    }

    pub fn active(&self, now: u64) -> impl Iterator<Item = TimedEffect> + '_ {
        self.0
            .iter()
            .copied()
            .filter(move |effect| now < effect.active_until)
    }

    fn upsert(&mut self, effect: TimedEffect) {
        if let Some(old) = self
            .0
            .iter_mut()
            .find(|old| old.concoction == effect.concoction)
        {
            *old = effect;
        } else {
            self.0.push(effect);
            self.0.sort_unstable_by_key(|effect| effect.concoction);
        }
    }
}

/// Adds Stamina Extension through the existing stamina owner's units and cap.
pub fn extend_stamina(current_minutes: u16) -> u16 {
    current_minutes.saturating_add(60).min(STAMINA_CAP_MINUTES)
}

/// Strike Enhancement is five percentage points, expressed in Oteryn's permyriad unit.
pub fn strike_critical_permyriad(base: u16, active: bool) -> u16 {
    base.saturating_add(if active { 500 } else { 0 })
}

/// The charm owner calls this only for an offensive charm which permits trigger modifiers.
pub fn offensive_charm_chance_percent(base: u8, active: bool, permits_upgrade: bool) -> u8 {
    base.saturating_add(if active && permits_upgrade { 5 } else { 0 })
}

pub const fn regular_loot_multiplier(active: bool) -> u8 {
    if active { 2 } else { 1 }
}

pub const fn bestiary_progress_multiplier(active: bool) -> u8 {
    if active { 2 } else { 1 }
}

/// Resilience reduces incoming damage of its element by eight percent.
pub fn resilient_damage(damage: u64, active: bool) -> u64 {
    if active {
        damage.saturating_mul(92) / 100
    } else {
        damage
    }
}

/// Pierce belongs to monster sensitivity resolution. Player targets and charm damage bypass it.
pub fn pierce_sensitivity(
    original_percent: u16,
    active: bool,
    target_is_monster: bool,
    damage_is_charm: bool,
) -> u16 {
    if active && target_is_monster && !damage_is_charm {
        pierced_monster_sensitivity(original_percent, 16)
    } else {
        original_percent
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationError {
    Cooldown { ready_at: u64 },
    ClockOverflow,
}

/// Prepares the durable timestamps. Cooldowns are per concoction, not shared. Reusing an active
/// concoction after its cooldown elapsed replaces (rather than stacks) its one-hour expiry.
pub fn activate(
    concoction: Concoction,
    now: u64,
    previous_ready_at: Option<u64>,
) -> Result<TimedEffect, ActivationError> {
    if let Some(ready_at) = previous_ready_at.filter(|ready| now < *ready) {
        return Err(ActivationError::Cooldown { ready_at });
    }
    Ok(TimedEffect {
        concoction,
        active_until: now
            .checked_add(EFFECT_DURATION_SECONDS)
            .ok_or(ActivationError::ClockOverflow)?,
        ready_at: now
            .checked_add(INDIVIDUAL_COOLDOWN_SECONDS)
            .ok_or(ActivationError::ClockOverflow)?,
    })
}

/// Applies Elemental Pierce to a monster's sensitivity percentage. Immunity is preserved, the
/// increase above 100% is halved, and sensitivity is capped at twice its original value.
pub fn pierced_monster_sensitivity(original_percent: u16, pierce_points: u8) -> u16 {
    if original_percent == 0 {
        return 0;
    }
    let room = original_percent.saturating_mul(2);
    let below = 100u16
        .saturating_sub(original_percent)
        .min(u16::from(pierce_points));
    let remainder = u16::from(pierce_points) - below;
    original_percent
        .saturating_add(below)
        .saturating_add(remainder / 2)
        .min(room)
}

/// Maps an unbiased decision in the owner's `[0, u64::MAX]` decision space to the catalogue.
/// Rejection avoids modulo bias; `None` asks the existing decision owner for another draw.
pub fn cube_reward(draw: u64) -> Option<Concoction> {
    const N: u64 = ALL_CONCOCTIONS.len() as u64;
    const ACCEPT: u64 = u64::MAX - (u64::MAX % N);
    (draw < ACCEPT).then(|| ALL_CONCOCTIONS[(draw % N) as usize])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_is_complete_and_stable() {
        for (index, value) in ALL_CONCOCTIONS.into_iter().enumerate() {
            assert_eq!(Concoction::from_item_id(36_723 + index as u32), Some(value));
        }
        assert_eq!(Concoction::from_item_id(36_722), None);
        assert_eq!(Concoction::from_item_id(36_743), None);
    }

    #[test]
    fn effects_use_global_numbers_and_individual_day_cooldown() -> Result<(), ActivationError> {
        assert_eq!(
            Concoction::FireAmplification.effect(),
            Effect::Pierce(Element::Fire, 16)
        );
        let fire = activate(Concoction::FireAmplification, 10, None)?;
        assert_eq!(fire.active_until, 3_610);
        assert_eq!(fire.ready_at, 86_410);
        assert_eq!(
            activate(Concoction::IceAmplification, 11, None)?.active_until,
            3_611
        );
        assert_eq!(
            activate(Concoction::FireAmplification, 11, Some(fire.ready_at)),
            Err(ActivationError::Cooldown {
                ready_at: fire.ready_at
            })
        );
        Ok(())
    }

    #[test]
    fn pierce_preserves_immunity_halves_above_hundred_and_caps_at_double() {
        assert_eq!(pierced_monster_sensitivity(0, 16), 0);
        assert_eq!(pierced_monster_sensitivity(80, 16), 96);
        assert_eq!(pierced_monster_sensitivity(95, 16), 105);
        assert_eq!(pierced_monster_sensitivity(110, 16), 118);
        assert_eq!(pierced_monster_sensitivity(5, 16), 10);
    }

    #[test]
    fn cube_selection_covers_each_reward_and_rejects_tail() {
        for index in 0..20 {
            assert_eq!(cube_reward(index), Some(ALL_CONCOCTIONS[index as usize]));
        }
        assert_eq!(cube_reward(u64::MAX), None);
    }

    #[test]
    fn durable_effects_stack_by_kind_tick_offline_and_reload() -> Result<(), ActivationError> {
        let mut effects = ConcoctionEffects::default();
        let fire = effects.activate(Concoction::FireResilience, 1_000)?;
        effects.activate(Concoction::StrikeEnhancement, 1_001)?;
        assert_eq!(effects.active(1_002).count(), 2);

        let reloaded = ConcoctionEffects::from_durable(effects.0.clone());
        assert!(reloaded.is_active(Concoction::FireResilience, fire.active_until - 1));
        assert!(!reloaded.is_active(Concoction::FireResilience, fire.active_until));
        assert_eq!(
            reloaded.ready_at(Concoction::FireResilience),
            Some(fire.ready_at)
        );
        Ok(())
    }

    #[test]
    fn owner_adapters_apply_caps_exceptions_and_units() {
        assert_eq!(extend_stamina(2_500), STAMINA_CAP_MINUTES);
        assert_eq!(strike_critical_permyriad(250, true), 750);
        assert_eq!(offensive_charm_chance_percent(10, true, false), 10);
        assert_eq!(offensive_charm_chance_percent(10, true, true), 15);
        assert_eq!(regular_loot_multiplier(true), 2);
        assert_eq!(bestiary_progress_multiplier(true), 2);
        assert_eq!(resilient_damage(100, true), 92);
        assert_eq!(pierce_sensitivity(80, true, true, false), 96);
        assert_eq!(pierce_sensitivity(80, true, false, false), 80);
        assert_eq!(pierce_sensitivity(80, true, true, true), 80);
    }
}
