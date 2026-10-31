//! Wheel contributions remain projections; durable Character skills are never rewritten.

use super::PlayerSpellState;
use oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects;

/// Crystal 201189f player_wheel.cpp:4090-4115 reads the two actual hand items,
/// not the selected block-defence summary. Oteryn slots 5/6 are weapon/shield.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WheelHand {
    pub defense: i32,
    pub extra_defense: i32,
    pub real_shield: bool,
    pub secondary_shield: bool,
    pub two_handed: bool,
    /// Crystal Player::getSkillLevel grants Battle Instinct to Club/Sword/Axe only.
    pub battle_melee: bool,
    pub ammunition: Option<oteryn_gameplay_content::content::ReferenceAmmoType>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WheelEquipment {
    pub weapon_hand: Option<WheelHand>,
    pub shield_hand: Option<WheelHand>,
    pub proficiency_defense: i32,
    pub proficiency_shield_modifier: i32,
}
impl WheelEquipment {
    pub fn real_shield(self) -> bool {
        [self.weapon_hand, self.shield_hand]
            .into_iter()
            .flatten()
            .any(|hand| hand.real_shield)
    }
    pub fn battle_melee(self) -> bool {
        [self.weapon_hand, self.shield_hand]
            .into_iter()
            .flatten()
            .any(|hand| hand.battle_melee)
    }
    pub fn two_handed(self) -> bool {
        [self.weapon_hand, self.shield_hand]
            .into_iter()
            .flatten()
            .any(|hand| hand.two_handed)
    }
    pub fn ammunition(self) -> Option<oteryn_gameplay_content::content::ReferenceAmmoType> {
        self.weapon_hand
            .and_then(|hand| hand.ammunition)
            .or_else(|| self.shield_hand.and_then(|hand| hand.ammunition))
    }
}

/// Current monster census for the two adjacency skills; player-owned summons are excluded
/// exactly as Crystal player_wheel.cpp:2985,3009. No timer or cached count grants a skill.
pub fn wheel_adjacent_monsters(
    runtime: &crate::foundation::ChannelRuntimeV1,
    actor: crate::foundation::ExactActorRef,
) -> Option<u32> {
    let at = runtime.read_actor_position(actor).ok()?.position();
    wheel_adjacent_monsters_at(runtime, actor, at)
}

/// Initial field contact is staged before the physical step commits; its movement owner
/// supplies the independently qualified destination from the existing step proof.
pub fn wheel_adjacent_monsters_at(
    runtime: &crate::foundation::ChannelRuntimeV1,
    actor: crate::foundation::ExactActorRef,
    at: crate::foundation::MovementLocalPosition,
) -> Option<u32> {
    if !runtime.borrow_exact_actor_lookup().contains(actor) {
        return None;
    }
    let mut count = 0_u32;
    for (candidate, position, session) in runtime.positioned_actor_census().ok()? {
        let position = position.position();
        if candidate != actor
            && session.is_none()
            && position.floor == at.floor
            && position.x.abs_diff(at.x) <= 1
            && position.y.abs_diff(at.y) <= 1
            && runtime
                .companion_snapshot(candidate)
                .ok()?
                .state
                .master
                .is_none()
        {
            count = count.checked_add(1)?;
        }
    }
    Some(count)
}

/// Actual target adjacency, including player targets, for Sanctuary's separate 10% bonus.
pub fn wheel_adjacent_target(
    runtime: &crate::foundation::ChannelRuntimeV1,
    actor: crate::foundation::ExactActorRef,
    target: crate::foundation::ExactActorRef,
) -> Option<bool> {
    if actor == target {
        return Some(false);
    }
    let from = runtime.read_actor_position(actor).ok()?.position();
    let to = runtime.read_actor_position(target).ok()?.position();
    Some(from.floor == to.floor && from.x.abs_diff(to.x) <= 1 && from.y.abs_diff(to.y) <= 1)
}

impl PlayerSpellState {
    pub fn install_wheel_equipment(&mut self, equipment: Option<WheelEquipment>) {
        self.wheel_equipment = equipment;
    }
    pub fn wheel_equipment(&self) -> Option<WheelEquipment> {
        self.wheel_equipment
    }

    pub fn install_wheel_proficiency(
        &mut self,
        weapon: Option<&str>,
        defense: i32,
        shield_modifier: i32,
    ) {
        self.wheel_proficiency_weapon = weapon.map(str::to_owned);
        if let Some(hands) = &mut self.wheel_equipment {
            hands.proficiency_defense = defense;
            hands.proficiency_shield_modifier = shield_modifier;
        }
    }
    /// Selection/modification refresh is restricted to the currently equipped weapon.
    pub fn refresh_wheel_proficiency(&mut self, weapon: &str, defense: i32, shield_modifier: i32) {
        if self.wheel_proficiency_weapon.as_deref() == Some(weapon)
            && let Some(hands) = &mut self.wheel_equipment
        {
            hands.proficiency_defense = defense;
            hands.proficiency_shield_modifier = shield_modifier;
        }
    }

    /// This read is made under the same physical owner borrow as the hit or block.
    /// Neither a previous attack's census nor durable allocation evidence grants shielding.
    pub fn wheel_battle_instinct(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        actor: crate::foundation::ExactActorRef,
        now_us: u64,
    ) -> Option<oteryn_gameplay_progression::wheel_effects::BattleInstinctBonuses> {
        if !self
            .wheel_effects_at(now_us)
            .is_some_and(|effects| effects.active_unique_perks.contains("battle_instinct"))
        {
            return Some(Default::default());
        }
        let position = runtime.read_actor_position(actor).ok()?.position();
        self.wheel_battle_instinct_at(runtime, actor, position, now_us)
    }

    pub fn wheel_battle_instinct_at(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        actor: crate::foundation::ExactActorRef,
        position: crate::foundation::MovementLocalPosition,
        now_us: u64,
    ) -> Option<oteryn_gameplay_progression::wheel_effects::BattleInstinctBonuses> {
        if !self
            .wheel_effects_at(now_us)
            .is_some_and(|effects| effects.active_unique_perks.contains("battle_instinct"))
        {
            return Some(Default::default());
        }
        let count = wheel_adjacent_monsters_at(runtime, actor, position)?;
        let data = oteryn_gameplay_progression::wheel_gem_data::WheelGemData::embedded().ok()?;
        oteryn_gameplay_progression::wheel_effects::battle_instinct_bonuses(data.wheel(), count)
    }

    /// Crystal 201189f player_wheel.cpp:4079-4121; vocation factors are the pinned
    /// data/XML/vocations.xml:12,25,38,51,64,77,93,109,125,141,153 values.
    /// Baseline mitigation requires current equipment and owned shielding, independently
    /// of Wheel eligibility. Only the Wheel multiplier requires current Premium/promotion.
    pub fn player_mitigation_percent(&self, now_us: u64) -> Option<f32> {
        self.player_mitigation_with_shielding(now_us, 0)
    }

    fn player_mitigation_with_shielding(&self, now_us: u64, shielding: u32) -> Option<f32> {
        use super::super::Vocation;
        let hands = self.wheel_equipment?;
        let skill = self.live_skill(5, now_us)?.checked_add(shielding)? as f32;
        let (factor, primary, secondary) = match self.facts.vocation {
            Vocation::Sorcerer
            | Vocation::MasterSorcerer
            | Vocation::Druid
            | Vocation::ElderDruid => (1.26_f32, 2.0_f32, 1.2_f32),
            Vocation::Paladin | Vocation::RoyalPaladin => (1.28, 2.08, 1.2),
            Vocation::Monk | Vocation::ExaltedMonk => (1.3, 2.10, 1.30),
            Vocation::None | Vocation::Knight | Vocation::EliteKnight => (1.3, 2.05, 1.25),
        };
        let mut defense = 0_i32;
        let mut shield_factor = 1.0_f32;
        let mut distance_factor = 1.0_f32;
        if let Some(shield) = hands.shield_hand {
            if shield.secondary_shield {
                distance_factor = secondary;
            } else {
                shield_factor = primary;
            }
            defense = shield.defense;
            // Crystal:3090-3101 clears Combat Mastery's retired flat defence stat.
        }
        if let Some(weapon) = hands.weapon_hand {
            if weapon.ammunition.is_some() {
                distance_factor = secondary;
            } else if weapon.two_handed {
                defense = weapon
                    .defense
                    .checked_add(hands.proficiency_defense)?
                    .checked_add(weapon.extra_defense)?
                    .checked_add(hands.proficiency_shield_modifier)?;
                shield_factor = secondary;
            } else {
                defense = defense
                    .checked_add(weapon.defense)?
                    .checked_add(weapon.extra_defense)?
                    .checked_add(hands.proficiency_shield_modifier)?;
                shield_factor = primary;
            }
        }
        // Match the donor's float evaluation and hundredth-percent ceiling.
        let mut mitigation =
            (((skill * factor + shield_factor * defense as f32) / 100.0) * distance_factor * 100.0)
                .ceil()
                / 100.0;
        if let Some(effects) = self.wheel_effects_at(now_us) {
            mitigation += mitigation * (effects.mitigation_milli_percent as f32 / 1_000.0) / 100.0;
        }
        (mitigation.is_finite() && mitigation >= 0.0).then_some(mitigation)
    }

    pub(crate) fn after_player_mitigation(&self, magnitude: u32, now_us: u64) -> Option<u32> {
        self.after_player_mitigation_with_shielding(magnitude, now_us, 0)
    }

    pub(crate) fn after_player_mitigation_with_shielding(
        &self,
        magnitude: u32,
        now_us: u64,
        shielding: u32,
    ) -> Option<u32> {
        let Some(mitigation) = self.player_mitigation_with_shielding(now_us, shielding) else {
            return Some(magnitude);
        };
        // Creature.cpp:842-845 uses float multiplication, then compound integer truncation.
        let remaining = (f64::from(magnitude) - f64::from(magnitude as f32 * mitigation) / 100.0)
            .max(0.0)
            .trunc();
        (remaining.is_finite() && remaining <= f64::from(u32::MAX)).then_some(remaining as u32)
    }

    /// Crystal wheel:4070-4076 applies only net positive resistance, removes ceil,
    /// then Avatar is applied by the existing incoming condition owner. Projection sums
    /// positive and negative gem contributions before this check; a penalty alone never
    /// amplifies the hit. Units here are source-normalized thousandths percent.
    pub fn after_wheel_resistance(
        &self,
        magnitude: u32,
        kind: crate::player_lethal::PlayerHitKind,
        now_us: u64,
    ) -> u32 {
        let Some(effects) = self.wheel_effects_at(now_us) else {
            return magnitude;
        };
        let Some(key) = kind.resistance_key() else {
            return magnitude;
        };
        let resistance = effects.gem_value_milli(key, "");
        if resistance <= 0 {
            return magnitude;
        }
        let removed = (u128::from(magnitude) * resistance as u128).div_ceil(100_000);
        magnitude - removed.min(u128::from(magnitude)) as u32
    }

    /// Mana drain uses its existing MP owner. Crystal game.cpp:8782-8783 caps to current
    /// mana before blockHit; baseline mitigation is skipped, but gem resistance and Avatar
    /// still apply in that order (player_wheel.cpp:4070-4076).
    pub fn after_wheel_mana_drain(&self, magnitude: u32, now_us: u64) -> Option<u32> {
        let magnitude = self.after_wheel_resistance(
            magnitude.min(self.mana),
            crate::player_lethal::PlayerHitKind::ManaDrain,
            now_us,
        );
        self.after_avatar_reduction(magnitude, now_us)
    }

    fn after_avatar_reduction(&self, magnitude: u32, now_us: u64) -> Option<u32> {
        if let Some(crate::domain::condition::ConditionValues::Attributes {
            incoming_reduction_percent,
            ..
        }) = self.conditions.attributes_at(now_us)
        {
            if incoming_reduction_percent > 100 {
                return None;
            }
            let removed =
                (u64::from(magnitude) * u64::from(incoming_reduction_percent)).div_ceil(100);
            return magnitude.checked_sub(u32::try_from(removed).ok()?);
        }
        Some(magnitude)
    }

    /// The source block stage precedes the attacker's Wheel flat/multiplier stage.
    /// The committed HP/mana owner remains separate and applies Combat Mastery, Virtue,
    /// mana shield and Gift after that attacker stage (Crystal game.cpp:7945-8060).
    pub fn after_player_block(
        &self,
        magnitude: u32,
        kind: crate::player_lethal::PlayerHitKind,
        shielding: u32,
        now_us: u64,
    ) -> Option<u32> {
        let magnitude = self.after_player_baseline(magnitude, kind, shielding, now_us)?;
        self.after_wheel_block(magnitude, kind, now_us)
    }

    pub(crate) fn after_player_baseline(
        &self,
        magnitude: u32,
        kind: crate::player_lethal::PlayerHitKind,
        shielding: u32,
        now_us: u64,
    ) -> Option<u32> {
        use crate::player_lethal::PlayerHitKind;
        match kind {
            PlayerHitKind::ManaDrain => None,
            PlayerHitKind::LifeDrain => Some(magnitude),
            _ => self.after_player_mitigation_with_shielding(magnitude, now_us, shielding),
        }
    }

    pub(crate) fn after_wheel_block(
        &self,
        magnitude: u32,
        kind: crate::player_lethal::PlayerHitKind,
        now_us: u64,
    ) -> Option<u32> {
        self.after_avatar_reduction(self.after_wheel_resistance(magnitude, kind, now_us), now_us)
    }

    pub(crate) fn combat_mastery_damage_reduction(&self, magnitude: u32, now_us: u64) -> u32 {
        let Some(effects) = self.wheel_effects_at(now_us) else {
            return magnitude;
        };
        let shield = self
            .wheel_equipment
            .is_some_and(WheelEquipment::real_shield);
        let percent = effects
            .combat_mastery_percent(
                u64::from(self.health),
                u64::from(self.facts.max_health),
                shield,
            )
            .min(50);
        // game.cpp:8045-8047 truncates the whole remaining magnitude, not the removed part.
        (u64::from(magnitude) * u64::from(100 - percent) / 100) as u32
    }

    /// Auto-attacks use the same currently eligible projection as spells. The caller
    /// has already resolved each target's block, mitigation and element response.
    pub fn finish_wheel_attack_components(
        &self,
        primary: i64,
        secondary: i64,
        health: u64,
        maximum: u64,
        now_us: u64,
        adjacent: bool,
    ) -> Option<(i64, i64)> {
        let Some(effects) = self.wheel_effects_at(now_us) else {
            return Some((primary, secondary));
        };
        let (mut primary, mut secondary) = effects.finish_damage_pair(
            primary,
            secondary,
            health,
            maximum,
            self.wheel_equipment.is_some_and(WheelEquipment::two_handed),
        )?;
        if primary == 0 && secondary == 0 {
            return Some((0, 0));
        }
        // Crystal game.cpp:7708-7715 adds Ascetic after the normal Wheel flat bonus,
        // not to the raw roll before critical or armor.
        let ascetic = i64::from(
            self.ascetic_attack_bonus(
                effects
                    .revelation_stages
                    .get("ascetic")
                    .copied()
                    .unwrap_or(0),
            ),
        );
        primary = primary.checked_add(ascetic)?;
        if secondary != 0 {
            secondary = secondary.checked_add(ascetic)?;
        }
        if effects.active_unique_perks.contains("sanctuary") {
            let factor = self.wheel_sanctuary_percent(None, now_us);
            let finish = |value: i64| -> Option<i64> {
                let mut value = (value as f64 * f64::from(factor) / 100.0).round();
                if adjacent {
                    value = (value * 1.10).round();
                }
                (value.is_finite() && value >= 0.0 && value <= f64::from(i32::MAX))
                    .then_some(value as i64)
            };
            primary = finish(primary)?;
            secondary = finish(secondary)?;
        }
        Some((primary, secondary))
    }

    /// Crystal tools.cpp:2216-2223 is the closed Sanctuary trigger set.
    fn sanctuary_trigger(spell: &str) -> bool {
        matches!(
            spell,
            "Devastating Knockout"
                | "Greater Tiger Clash"
                | "Spiritual Outburst"
                | "Sweeping Takedown"
                | "Tiger Clash"
        )
    }

    pub(super) fn prepare_wheel_spell_state(
        &mut self,
        spell: &super::super::SpellDefinition,
        now_us: u64,
    ) -> Option<()> {
        if self
            .wheel_effects_at(now_us)
            .is_some_and(|effects| effects.active_unique_perks.contains("sanctuary"))
            && Self::sanctuary_trigger(&spell.name)
            && self.wheel_sanctuary.is_none_or(|(until, _)| until < now_us)
        {
            // Capture Harmony before the existing spender consumes it; active windows do not refresh.
            self.wheel_sanctuary = Some((
                now_us.checked_add(5_000_000)?,
                self.monk.as_ref()?.harmony(),
            ));
        }
        Some(())
    }

    /// The numerical owner can preview the same cast window without mutating its predecessor.
    pub fn wheel_sanctuary_percent(&self, selected_spell: Option<&str>, now_us: u64) -> u32 {
        if !self
            .wheel_effects_at(now_us)
            .is_some_and(|effects| effects.active_unique_perks.contains("sanctuary"))
        {
            return 100;
        }
        if let Some((_, harmony)) = self.wheel_sanctuary.filter(|(until, _)| *until >= now_us) {
            return 100 + u32::from(harmony) * 2;
        }
        if selected_spell.is_some_and(Self::sanctuary_trigger) {
            return 100
                + self
                    .monk
                    .as_ref()
                    .map_or(0, |monk| u32::from(monk.harmony()) * 2);
        }
        100
    }

    /// Crystal wheel:3346-3351 consumes the inclusive old window for one real Combat.
    pub fn wheel_focus_damage_percent(&self, now_us: u64) -> u32 {
        if self
            .wheel_effects_at(now_us)
            .is_some_and(|effects| effects.active_unique_perks.contains("focus_mastery"))
            && self.wheel_focus_until.is_some_and(|until| until >= now_us)
        {
            35
        } else {
            0
        }
    }

    /// Called on the paid successor only after the actual Combat magnitude owner qualified.
    /// Healing consumes the old damage bonus too; callbacks without Combat never call this.
    pub fn complete_wheel_focus_cast(&mut self, secondary_focus: bool, now_us: u64) -> Option<()> {
        if self
            .wheel_effects_at(now_us)
            .is_some_and(|effects| effects.active_unique_perks.contains("focus_mastery"))
        {
            self.wheel_focus_until = if secondary_focus {
                Some(now_us.checked_add(12_000_000)?)
            } else {
                None
            };
        }
        Some(())
    }

    pub fn install_wheel_effects(&mut self, effects: WheelRuntimeEffects) {
        if !effects.active_unique_perks.contains("sanctuary") {
            self.wheel_sanctuary = None;
        }
        if !effects.active_unique_perks.contains("focus_mastery") {
            self.wheel_focus_until = None;
        }
        self.wheel_effects = effects;
    }

    /// Use the independently installed promotion and conservative Premium deadline at
    /// every consumer. Reset/reallocation replaces the projection rather than accumulating it.
    pub fn wheel_effects_at(&self, now_us: u64) -> Option<&WheelRuntimeEffects> {
        (self.facts.level > oteryn_gameplay_progression::wheel::WHEEL_LEVEL_OFFSET
            && self.stored_promotion
            && self.hunting_premium_current(now_us))
        .then_some(&self.wheel_effects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell::{Vocation, cast::CharacterCastFacts};

    fn state() -> Result<PlayerSpellState, String> {
        PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::Knight,
                level: 500,
                magic_level: 20,
                max_health: 1_000,
                max_mana: 500,
                max_soul: 200,
            },
            0,
            0,
        )
        .ok_or_else(|| "fixture".to_owned())
    }

    #[test]
    fn wheel_skills_are_live_additions_and_reset_never_changes_character_base() -> Result<(), String>
    {
        let mut state = state()?;
        state.owned_skills = Some([10, 11, 12, 13, 14, 15, 16]);
        state
            .apply_owner_promotion_transition(true, true, Some(100))
            .map_err(|error| format!("{error:?}"))?;
        let mut effects = WheelRuntimeEffects::default();
        effects.skill_bonuses.extend([
            ("weapon_skill".to_owned(), 4),
            ("distance_skill".to_owned(), 3),
            ("fist_fighting_skill".to_owned(), 2),
            ("magic_skill".to_owned(), 1),
        ]);
        state.install_wheel_effects(effects);
        for (index, expected) in [12, 15, 16, 17, 17, 15, 16].into_iter().enumerate() {
            assert_eq!(state.live_skill(index, 99), Some(expected));
        }
        assert_eq!(
            state
                .owned_effective_magic_level(99)
                .map_err(|e| format!("{e:?}"))?,
            21
        );
        assert_eq!(state.character_facts().magic_level, 20);
        assert_eq!(state.owned_skills, Some([10, 11, 12, 13, 14, 15, 16]));
        state.install_wheel_effects(WheelRuntimeEffects::default());
        assert_eq!(state.live_skill(2, 99), Some(12));
        assert_eq!(
            state
                .owned_effective_magic_level(99)
                .map_err(|e| format!("{e:?}"))?,
            20
        );
        Ok(())
    }

    #[test]
    fn wheel_consumers_close_at_premium_expiry_and_without_promotion() -> Result<(), String> {
        let mut state = state()?;
        state.owned_skills = Some([10; 7]);
        let mut effects = WheelRuntimeEffects::default();
        effects.skill_bonuses.insert("weapon_skill".to_owned(), 4);
        state.install_wheel_effects(effects);
        assert!(state.wheel_effects_at(0).is_none());
        state
            .apply_owner_promotion_transition(true, true, Some(100))
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(state.live_skill(2, 99), Some(14));
        assert_eq!(state.live_skill(2, 100), Some(10));
        state
            .apply_owner_promotion_transition(false, true, Some(200))
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(state.live_skill(2, 100), Some(10));
        Ok(())
    }
}

#[cfg(test)]
mod mitigation_tests {
    use super::*;
    use crate::spell::{Vocation, cast::CharacterCastFacts};
    fn player() -> Result<PlayerSpellState, String> {
        let mut state = PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::Knight,
                level: 500,
                magic_level: 20,
                max_health: 1_000,
                max_mana: 500,
                max_soul: 200,
            },
            0,
            0,
        )
        .ok_or_else(|| "fixture".to_owned())?;
        state.owned_skills = Some([100; 7]);
        Ok(state)
    }
    #[test]
    fn wheel_mitigation_uses_qualified_equipment_and_keeps_baseline_without_premium()
    -> Result<(), String> {
        let mut state = player()?;
        assert_eq!(state.player_mitigation_percent(0), None);
        state.install_wheel_equipment(Some(WheelEquipment {
            shield_hand: Some(WheelHand {
                defense: 40,
                real_shield: true,
                ..Default::default()
            }),
            weapon_hand: Some(WheelHand {
                defense: 20,
                extra_defense: 3,
                ..Default::default()
            }),
            ..Default::default()
        }));
        assert_eq!(state.player_mitigation_percent(0), Some(2.6));
        assert_eq!(state.after_player_mitigation(1_000, 0), Some(974));
        let mut effects = WheelRuntimeEffects::default();
        effects.mitigation_milli_percent = 100_000;
        state.install_wheel_effects(effects);
        assert_eq!(
            state.player_mitigation_percent(0),
            Some(2.6),
            "Wheel closed, baseline remains"
        );
        state
            .apply_owner_promotion_transition(true, true, Some(100))
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(state.player_mitigation_percent(99), Some(5.2));
        assert_eq!(state.after_player_mitigation(1_000, 99), Some(948));
        assert_eq!(state.player_mitigation_percent(100), Some(2.6));
        state.install_wheel_equipment(Some(WheelEquipment::default()));
        assert_eq!(
            state.player_mitigation_percent(100),
            Some(1.3),
            "removal replaces hand facts"
        );
        Ok(())
    }
    #[test]
    fn wheel_mitigation_uses_vocation_ranged_and_two_hand_proficiency_factors() -> Result<(), String>
    {
        let mut state = player()?;
        for (vocation, expected) in [
            (Vocation::Knight, 1.3),
            (Vocation::EliteKnight, 1.3),
            (Vocation::Paladin, 1.28),
            (Vocation::RoyalPaladin, 1.28),
            (Vocation::Sorcerer, 1.26),
            (Vocation::Druid, 1.26),
            (Vocation::Monk, 1.3),
            (Vocation::ExaltedMonk, 1.3),
        ] {
            state.facts.vocation = vocation;
            state.install_wheel_equipment(Some(WheelEquipment::default()));
            assert_eq!(
                state.player_mitigation_percent(0),
                Some(expected),
                "{vocation:?}"
            );
        }
        state.facts.vocation = Vocation::Knight;
        state.install_wheel_equipment(Some(WheelEquipment {
            weapon_hand: Some(WheelHand {
                defense: 42,
                extra_defense: 2,
                two_handed: true,
                ..Default::default()
            }),
            ..Default::default()
        }));
        state.install_wheel_proficiency(Some("weapon:first"), 3, 4);
        assert_eq!(state.player_mitigation_percent(0), Some(1.94));
        state.refresh_wheel_proficiency("weapon:other", 99, 99);
        assert_eq!(
            state.player_mitigation_percent(0),
            Some(1.94),
            "another weapon selection cannot alter equipped defense"
        );
        state.refresh_wheel_proficiency("weapon:first", 0, 0);
        assert_eq!(
            state.player_mitigation_percent(0),
            Some(1.85),
            "cleared selection applies immediately"
        );
        state.facts.vocation = Vocation::Paladin;
        state.install_wheel_equipment(Some(WheelEquipment {
            weapon_hand: Some(WheelHand {
                ammunition: Some(oteryn_gameplay_content::content::ReferenceAmmoType::Arrow),
                ..Default::default()
            }),
            shield_hand: Some(WheelHand {
                secondary_shield: true,
                ..Default::default()
            }),
            ..Default::default()
        }));
        assert_eq!(state.player_mitigation_percent(0), Some(1.54));
        state.install_wheel_equipment(None);
        state.install_wheel_proficiency(None, 0, 0);
        assert_eq!(state.player_mitigation_percent(0), None);
        Ok(())
    }
    #[test]
    fn battle_shielding_enters_the_live_mitigation_formula_and_fist_is_excluded()
    -> Result<(), String> {
        let mut state = player()?;
        state.install_wheel_equipment(Some(WheelEquipment::default()));
        assert_eq!(state.player_mitigation_with_shielding(0, 24), Some(1.62));
        assert_eq!(
            state.after_player_mitigation_with_shielding(10_000, 0, 24),
            Some(9_838)
        );
        assert_eq!(state.after_player_mitigation(10_000, 0), Some(9_870));
        assert!(!WheelEquipment::default().battle_melee());
        assert!(
            !WheelEquipment {
                weapon_hand: Some(WheelHand::default()),
                ..Default::default()
            }
            .battle_melee()
        );
        assert!(
            WheelEquipment {
                weapon_hand: Some(WheelHand {
                    battle_melee: true,
                    ..Default::default()
                }),
                ..Default::default()
            }
            .battle_melee()
        );
        Ok(())
    }

    #[test]
    fn life_drain_skips_only_base_mitigation_while_agony_skips_the_incoming_pipeline()
    -> Result<(), String> {
        let mut state = player()?;
        state.install_wheel_equipment(Some(WheelEquipment::default()));
        state.install_equipped_mana_shield(true);
        let (normal, hit) = crate::spell::actor_conditions::stage_creature_hit(&state, 900, 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            (normal.health, normal.vitals().mana, hit.applied),
            (612, 0, 388)
        );
        let (drain, hit) = crate::spell::actor_conditions::stage_life_drain_hit(&state, 900, 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            (drain.health, drain.vitals().mana, hit.applied),
            (600, 0, 400)
        );
        let (agony, hit) = crate::spell::actor_conditions::stage_agony_hit(&state, 900, 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            (agony.health, agony.vitals().mana, hit.applied),
            (100, 500, 900)
        );
        assert_eq!(normal.revision(), state.revision() + 1);
        assert_eq!(drain.revision(), state.revision() + 1);
        Ok(())
    }
    #[test]
    fn incoming_combat_mastery_uses_own_health_real_shield_and_current_premium()
    -> Result<(), String> {
        let mut state = player()?;
        state.health = 500;
        state.owned_skills = Some([0; 7]);
        state
            .apply_owner_promotion_transition(true, true, Some(100))
            .map_err(|e| format!("{e:?}"))?;
        let mut effects = WheelRuntimeEffects::default();
        effects.revelation_stages.insert("combat_mastery".into(), 3);
        state.install_wheel_effects(effects);
        for (real_shield, expected) in [(true, 88), (false, 94)] {
            state.install_wheel_equipment(Some(WheelEquipment {
                shield_hand: Some(WheelHand {
                    real_shield,
                    secondary_shield: !real_shield,
                    ..Default::default()
                }),
                ..Default::default()
            }));
            let (_, damage) = crate::spell::actor_conditions::stage_creature_hit(&state, 100, 99)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(damage.applied, expected);
        }
        let (_, damage) = crate::spell::actor_conditions::stage_creature_hit(&state, 100, 100)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(damage.applied, 100);
        Ok(())
    }
}

#[cfg(test)]
mod sanctuary_tests {
    use super::*;
    use crate::spell::{Vocation, cast::CharacterCastFacts};

    #[test]
    fn sanctuary_captures_harmony_keeps_inclusive_window_and_never_refreshes_it()
    -> Result<(), String> {
        let mut state = PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::ExaltedMonk,
                level: 500,
                magic_level: 20,
                max_health: 1000,
                max_mana: 500,
                max_soul: 200,
            },
            5,
            0,
        )
        .ok_or_else(|| "monk".to_owned())?;
        state
            .apply_owner_promotion_transition(true, true, Some(20_000_000))
            .map_err(|e| format!("{e:?}"))?;
        let mut effects = WheelRuntimeEffects::default();
        effects.active_unique_perks.insert("sanctuary".into());
        state.install_wheel_effects(effects);
        let mut spell = super::super::v1_spell_book()?
            .spells
            .into_iter()
            .next()
            .ok_or_else(|| "spell".to_owned())?;
        spell.name = "Mass Spirit Mend".into();
        state
            .prepare_wheel_spell_state(&spell, 1_000_000)
            .ok_or_else(|| "prepare".to_owned())?;
        assert_eq!(
            state.wheel_sanctuary, None,
            "Mass Spirit Mend is absent from the donor trigger set"
        );
        spell.name = "Tiger Clash".into();
        assert_eq!(
            state.wheel_sanctuary_percent(Some(&spell.name), 1_000_000),
            110
        );
        assert_eq!(
            state.wheel_sanctuary, None,
            "numerical preview is read-only"
        );
        state
            .prepare_wheel_spell_state(&spell, 1_000_000)
            .ok_or_else(|| "prepare".to_owned())?;
        assert_eq!(state.wheel_sanctuary, Some((6_000_000, 5)));
        state
            .prepare_wheel_spell_state(&spell, 5_000_000)
            .ok_or_else(|| "prepare".to_owned())?;
        assert_eq!(
            state.wheel_sanctuary,
            Some((6_000_000, 5)),
            "the second spender cannot refresh an active window"
        );
        assert_eq!(state.wheel_sanctuary_percent(None, 6_000_000), 110);
        assert_eq!(state.wheel_sanctuary_percent(None, 6_000_001), 100);
        assert_eq!(
            state.wheel_sanctuary_percent(Some(&spell.name), 20_000_000),
            100,
            "Premium expiry closes the perk"
        );
        state.install_wheel_effects(WheelRuntimeEffects::default());
        assert_eq!(
            state.wheel_sanctuary, None,
            "removing the perk clears its transient state"
        );
        Ok(())
    }
}

#[cfg(test)]
mod focus_tests {
    use super::*;
    use crate::spell::{Vocation, cast::CharacterCastFacts};
    #[test]
    fn focus_window_is_read_only_until_combat_commit_and_consumes_before_rearming()
    -> Result<(), String> {
        let mut state = PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::MasterSorcerer,
                level: 500,
                magic_level: 100,
                max_health: 1000,
                max_mana: 5000,
                max_soul: 200,
            },
            0,
            0,
        )
        .ok_or_else(|| "sorcerer".to_owned())?;
        state
            .apply_owner_promotion_transition(true, true, Some(50_000_000))
            .map_err(|e| format!("{e:?}"))?;
        let mut effects = WheelRuntimeEffects::default();
        effects.active_unique_perks.insert("focus_mastery".into());
        state.install_wheel_effects(effects);
        assert_eq!(state.wheel_focus_damage_percent(1_000_000), 0);
        state
            .complete_wheel_focus_cast(true, 1_000_000)
            .ok_or_else(|| "window".to_owned())?;
        let before = state.clone();
        assert_eq!(state.wheel_focus_damage_percent(13_000_000), 35);
        assert_eq!(state.wheel_focus_damage_percent(13_000_001), 0);
        assert_eq!(state, before, "preview does not consume the old window");
        let mut healed = state.clone();
        healed
            .complete_wheel_focus_cast(false, 2_000_000)
            .ok_or_else(|| "healing combat".to_owned())?;
        assert_eq!(
            healed.wheel_focus_damage_percent(2_000_000),
            0,
            "a healing Combat consumes without applying damage bonus"
        );
        state
            .complete_wheel_focus_cast(true, 2_000_000)
            .ok_or_else(|| "rearm".to_owned())?;
        assert_eq!(state.wheel_focus_until, Some(14_000_000));
        state.install_wheel_effects(WheelRuntimeEffects::default());
        assert_eq!(state.wheel_focus_until, None);
        Ok(())
    }
    #[test]
    fn eligibility_loss_clears_windows_before_a_new_grant() -> Result<(), String> {
        let mut state = PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::MasterSorcerer,
                level: 500,
                magic_level: 100,
                max_health: 1000,
                max_mana: 5000,
                max_soul: 200,
            },
            0,
            0,
        )
        .ok_or("sorcerer")?;
        state
            .apply_owner_promotion_transition(true, true, Some(1_000))
            .map_err(|error| format!("{error:?}"))?;
        let mut effects = WheelRuntimeEffects::default();
        effects
            .active_unique_perks
            .extend(["focus_mastery".into(), "sanctuary".into()]);
        state.install_wheel_effects(effects);
        state
            .complete_wheel_focus_cast(true, 1)
            .ok_or("focus window")?;
        state.wheel_sanctuary = Some((5_000_001, 5));
        assert_eq!(state.wheel_focus_damage_percent(999), 35);
        assert!(
            state
                .expire_owner_premium(1_000)
                .map_err(|error| format!("{error:?}"))?
        );
        assert_eq!(
            (state.wheel_focus_until, state.wheel_sanctuary),
            (None, None)
        );
        state
            .apply_owner_promotion_transition(true, true, Some(20_000_000))
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            state.wheel_focus_damage_percent(1_001),
            0,
            "renewal does not revive the old window"
        );
        state
            .complete_wheel_focus_cast(true, 1_002)
            .ok_or("new focus window")?;
        state.wheel_sanctuary = Some((5_001_002, 5));
        let before = state.clone();
        assert!(
            state
                .apply_owner_promotion_transition(false, true, None)
                .is_err()
        );
        assert_eq!(
            state, before,
            "invalid entitlement never partially clears a timer"
        );
        state
            .apply_owner_promotion_transition(false, true, Some(20_000_000))
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            (state.wheel_focus_until, state.wheel_sanctuary),
            (None, None)
        );
        Ok(())
    }
}

#[cfg(test)]
mod gem_resistance_tests {
    use super::*;
    use crate::player_lethal::PlayerHitKind;
    use crate::spell::{
        Vocation,
        cast::CharacterCastFacts,
        combat_batch::{AvatarState, OwnerCombatChange},
    };

    fn player() -> Result<PlayerSpellState, String> {
        let mut state = PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::Knight,
                level: 500,
                magic_level: 0,
                max_health: 1000,
                max_mana: 500,
                max_soul: 200,
            },
            0,
            0,
        )
        .ok_or("player")?;
        state
            .apply_owner_promotion_transition(true, true, Some(100_000))
            .map_err(|e| format!("{e:?}"))?;
        Ok(state)
    }

    #[test]
    fn only_net_positive_gem_resistance_removes_ceil_and_current_eligibility_is_required()
    -> Result<(), String> {
        let mut state = player()?;
        let mut effects = WheelRuntimeEffects::default();
        effects.sockets.gem_values_milli.extend([
            (("Fire Resistance".into(), "".into()), 1_010),
            (("Energy Resistance".into(), "".into()), -1_000),
            (("Physical Resistance".into(), "".into()), 500),
        ]);
        state.install_wheel_effects(effects);
        assert_eq!(
            state.after_wheel_resistance(101, PlayerHitKind::Fire, 99_999),
            99
        );
        assert_eq!(
            state.after_wheel_resistance(101, PlayerHitKind::Energy, 99_999),
            101,
            "negative net resistance does not amplify"
        );
        assert_eq!(
            state.after_wheel_resistance(101, PlayerHitKind::Physical, 99_999),
            100
        );
        assert_eq!(
            state.after_wheel_resistance(101, PlayerHitKind::Holy, 99_999),
            101,
            "no other element's bonus"
        );
        assert_eq!(
            state.after_wheel_resistance(101, PlayerHitKind::Mitigated, 99_999),
            101,
            "unknown element grants nothing"
        );
        assert_eq!(
            state.after_wheel_resistance(101, PlayerHitKind::Fire, 100_000),
            101
        );
        state
            .apply_owner_promotion_transition(false, true, Some(200_000))
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            state.after_wheel_resistance(101, PlayerHitKind::Fire, 99_999),
            101
        );
        Ok(())
    }

    #[test]
    fn mana_and_life_drain_keep_resistance_and_avatar_in_source_order() -> Result<(), String> {
        let mut state = player()?;
        let mut effects = WheelRuntimeEffects::default();
        effects.sockets.gem_values_milli.extend([
            (("Mana Drain Resistance".into(), "".into()), 1_010),
            (("Life Drain Resistance".into(), "".into()), 1_010),
        ]);
        state.install_wheel_effects(effects);
        crate::spell::actor_conditions::apply_combat_change(
            &mut state,
            &OwnerCombatChange::Avatar(AvatarState {
                expires_ms: 100,
                outfit_look_type: 1595,
                incoming_reduction_percent: 15,
                critical_chance_percent: 100,
                critical_extra_percentage_points: 15,
            }),
            0,
        )
        .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            state.after_wheel_mana_drain(101, 1),
            Some(84),
            "101 - ceil(1.0201)=99; 99-ceil(14.85)=84"
        );
        assert_eq!(
            state.after_wheel_mana_drain(1000, 1),
            Some(419),
            "cap to500 before1.01% resistance andAvatar"
        );
        let (_, damage) = crate::spell::actor_conditions::stage_life_drain_hit(&state, 101, 1)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(damage.applied, 84);
        Ok(())
    }
    #[test]
    fn virtue_follows_attacker_wheel_and_target_combat_mastery() -> Result<(), String> {
        let mut state = player()?;
        state.health = 500;
        state.owned_skills = Some([100; 7]);
        state.install_wheel_equipment(Some(WheelEquipment::default()));
        state
            .install_virtue_party_bonuses(
                crate::spell::harmony::VirtuePartyBonuses {
                    damage_reduction: 3,
                    ..Default::default()
                },
                0,
            )
            .map_err(|error| format!("{error:?}"))?;
        let mut effects = WheelRuntimeEffects::default();
        effects.revelation_stages.insert("combat_mastery".into(), 3);
        effects
            .sockets
            .gem_values_milli
            .insert(("Physical Resistance".into(), "".into()), 1_010);
        state.install_wheel_effects(effects);
        crate::spell::actor_conditions::apply_combat_change(
            &mut state,
            &OwnerCombatChange::Avatar(AvatarState {
                expires_ms: 100,
                outfit_look_type: 1595,
                incoming_reduction_percent: 15,
                critical_chance_percent: 100,
                critical_extra_percentage_points: 15,
            }),
            0,
        )
        .map_err(|error| format!("{error:?}"))?;
        let block = state
            .after_player_block(100, PlayerHitKind::Physical, 0, 1)
            .ok_or("block")?;
        assert_eq!(
            block, 82,
            "baseline98, resistance97, Avatar82; Virtue belongs to the later HP stage"
        );
        let (next, damage) =
            crate::spell::actor_conditions::stage_preprocessed_player_hit(&state, block + 20, 1)
                .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            damage.applied, 92,
            "attacker flat20 gives102, Combat Mastery6% gives95, Virtue3% gives92"
        );
        assert_eq!(next.health, 408);
        let (_, damage) = crate::spell::actor_conditions::stage_agony_hit(&state, 102, 1)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            damage.applied, 102,
            "the existing Agony bypass remains intact"
        );
        Ok(())
    }
}
