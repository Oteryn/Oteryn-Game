//! P12c: capture the same equipment and live skills that the production combat path consumes.
//! No observation authorizes combat. Failure to refresh this read model never fails a swing.

#[path = "cyclopedia_augments.rs"]
mod augments;
use super::*;
use crate::durability::cyclopedia_character::combat::{CombatSnapshot, CombatSnapshotReceipt};
use crate::gameplay_transport::equipment_move::equipment_effects_owner as equipment_owner;
use crate::spell::cast::PlayerSpellState;
use oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence;
use oteryn_gameplay_combat_rules::resolution::player_defence::player_defence_bounds;
use oteryn_gameplay_content::content::ReferenceResistanceKind;
use oteryn_gameplay_content::content::spell_data::formula::level_base_damage_healing;
use oteryn_gameplay_progression::wheel_effects::{BattleInstinctBonuses, WheelRuntimeEffects};
use oteryn_protocol_oteryn::cyclopedia_character::combat::{CombatStat as Stat, CombatStats};
use oteryn_protocol_oteryn::cyclopedia_character::{CharacterTab, TabWireError};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn refresh_cyclopedia_combat(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        expected_fence: Option<crate::durability::item_transfer::CurrentCharacterItemFence>,
    ) -> Result<CombatSnapshotReceipt, ()> {
        let _permit = self.spell_lane_permit().await.ok_or(())?;
        let content = self
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or(())?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(self.root.clone())
                .current_session_at(session)
                .await
                .map_err(|_| ())?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or(())?;
        if expected_fence.is_some_and(|expected| expected != fence) {
            return Err(());
        }
        let Some((effects, armor, defence, _, offence, equipment)) =
            self.read_admission_effects(session).await?
        else {
            return Err(());
        };
        let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(content);
        let quiver = if equipment.items.iter().any(|item| {
            matches!(item.slot, 6 | 10)
                && crate::durability::item_loop_move::ItemLoopContent::is_quiver(
                    &weights,
                    &item.definition,
                )
        }) {
            self.root
                .read_quiver_ammunition(
                    self.character,
                    self.holder,
                    &fence,
                    equipment.clone(),
                    &weights,
                )
                .await
                .map_err(|_| ())?
        } else {
            Vec::new()
        };
        let ranged = ranged::loadout(content, &equipment, &quiver).ok_or(())?;
        let weapon = swing_weapon(content, &equipment.items).ok_or(())?;
        let equipment_only = equipment_owner::equipped_offence(&equipment, content).ok_or(())?;
        let mantra = equipment_owner::equipped_mantra(&equipment, content).ok_or(())?;
        let now = self.owner_now();
        let observed_ms = u64::try_from(crate::gameplay_transport::character_info::unix_now_ms())
            .map_err(|_| ())?;
        let mut snapshot = CombatSnapshot {
            observed_owner_micros: now.get(),
            offence: CombatStats::new(
                CharacterTab::Offence,
                observed_ms,
                equipment.character_revision,
                equipment.revision,
            )
            .map_err(|_| ())?,
            defence: CombatStats::new(
                CharacterTab::Defence,
                observed_ms,
                equipment.character_revision,
                equipment.revision,
            )
            .map_err(|_| ())?,
            misc: CombatStats::new(
                CharacterTab::Misc,
                observed_ms,
                equipment.character_revision,
                equipment.revision,
            )
            .map_err(|_| ())?,
            equipment,
        };
        let augments;
        {
            // Stats are best-effort. Never await another lock while holding the runtime lock.
            let runtime = self.runtime.lock().await;
            let lease = runtime
                .bound_attacker_lease(actor, session)
                .map_err(|_| ())?;
            if lease.character_id().as_bytes() != fence.character_id.as_bytes()
                || lease.generation() != fence.character_lease_generation
                || runtime.content_pin().server_artifact_digest()
                    != snapshot.equipment.content_digest
            {
                return Err(());
            }
            let states = self.states.spell_states.try_lock().map_err(|_| ())?;
            let state = states.get(&runtime, actor, session).ok_or(())?;
            // A read racing an equipment installation cannot report a set the actor did not
            // consume. Its next combat refresh or query retries after the normal installation.
            if state.equipped_offence() != offence
                || state.equipped_armor() != armor
                || state.equipped_defence() != defence
            {
                return Err(());
            }
            let attack = self.states.attack.try_lock().map_err(|_| ())?;
            let mode = fight_mode(attack.expert_modes(actor, session).modes.fight_mode);
            let facts = state.character_facts();
            let wheel = self.wheel_stat_bonuses(
                session,
                vocation_key(facts.vocation),
                facts.level,
                state.wheel_promoted(),
                state.hunting_premium_current(now.get()),
            );
            let mut skill_bonuses = [0; 8];
            if let Some(catalogue) = &self.proficiency_catalogue
                && let Some(key) = snapshot
                    .equipment
                    .items
                    .iter()
                    .filter(|item| matches!(item.slot, 5 | 6 | 10))
                    .find_map(|item| {
                        catalogue
                            .resolve_weapon(&item.definition.production_key)
                            .map(|weapon| weapon.canonical_item_key)
                    })
                && let Ok(sessions) = self.states.proficiencies.lock()
                && let Some(track) = sessions.get(&session).and_then(|p| p.track(&key))
                && let Some(perks) = catalogue.cast_perks(&key, track, None)
            {
                skill_bonuses = perks.skills;
            }
            let observation = Observation {
                state,
                now_micros: now.get(),
                mode,
                weapon,
                ranged: ranged.as_deref(),
                equipment_only: &equipment_only,
                offence: &offence,
                wheel: wheel.as_ref(),
                battle_instinct: state
                    .wheel_battle_instinct(&runtime, actor, now.get())
                    .ok_or(())?,
                skill_bonuses,
                mantra,
            };
            observation.offence(&mut snapshot.offence).map_err(|_| ())?;
            observation.defence(&mut snapshot.defence).map_err(|_| ())?;
            observation.misc(&mut snapshot.misc).map_err(|_| ())?;
            let stages = if state.hunting_premium_current(now.get()) {
                self.wheel_stages(
                    session,
                    vocation_key(facts.vocation),
                    facts.level,
                    state.wheel_promoted(),
                )
            } else {
                Default::default()
            };
            augments = self
                .capture_cyclopedia_augments(
                    session,
                    &snapshot.equipment,
                    content,
                    wheel.as_ref(),
                    &stages,
                )
                .ok();

            snapshot
                .misc
                .set(Stat::Amplification, amplification_ppm(offence.boots_tier))
                .map_err(|_| ())?;
            for (kind, key) in [
                (ReferenceResistanceKind::Physical, Stat::AbsorbPhysical),
                (ReferenceResistanceKind::Fire, Stat::AbsorbFire),
                (ReferenceResistanceKind::Earth, Stat::AbsorbEarth),
                (ReferenceResistanceKind::Energy, Stat::AbsorbEnergy),
                (ReferenceResistanceKind::Ice, Stat::AbsorbIce),
                (ReferenceResistanceKind::Holy, Stat::AbsorbHoly),
                (ReferenceResistanceKind::Death, Stat::AbsorbDeath),
            ] {
                // Read the protection actually installed on the combat owner. The native
                // owner's current additive composition is not replaced by a UI-only formula.
                if state.equipped_element_protection(kind)
                    != effects.protections.get(&kind).copied()
                {
                    return Err(());
                }
                let ppm = match state.equipped_element_protection(kind) {
                    Some(ratio) => i128::from(ratio.numerator)
                        .checked_mul(10_000)
                        .and_then(|n| n.checked_div(i128::from(ratio.denominator)))
                        .and_then(|n| i64::try_from(n).ok())
                        .ok_or(())?,
                    None => 0,
                };
                snapshot.defence.set(key, ppm).map_err(|_| ())?;
            }
        }
        self.root
            .record_cyclopedia_combat_with_augments(
                self.character,
                self.holder,
                fence,
                snapshot,
                augments,
            )
            .await
            .map_err(|_| ())
    }
}

struct Observation<'a> {
    state: &'a PlayerSpellState,
    now_micros: u64,
    mode: FightMode,
    weapon: SwingWeapon,
    ranged: Option<&'a ranged::RangedLoadout>,
    equipment_only: &'a EquippedOffence,
    offence: &'a EquippedOffence,
    wheel: Option<&'a WheelRuntimeEffects>,
    battle_instinct: BattleInstinctBonuses,
    skill_bonuses: [u32; 8],
    mantra: u32,
}

impl Observation<'_> {
    fn skill(&self, index: usize) -> Option<u32> {
        self.state
            .live_skill(index, self.now_micros)
            .map(|value| value.saturating_add(self.skill_bonuses[index]))
    }

    fn offence(&self, out: &mut CombatStats) -> Result<(), TabWireError> {
        let (chance, damage) = self.offence.player_critical();
        let (base_chance, base_damage) = EquippedOffence::default().player_critical();
        let mut life = self.offence.life_leech;
        let mut mana = self.offence.mana_leech;
        if let Some(wheel) = self.wheel {
            life = life.saturating_add(
                u16::try_from(wheel.life_leech_hundredths_percent).unwrap_or(u16::MAX),
            );
            mana = mana.saturating_add(
                u16::try_from(wheel.mana_leech_hundredths_percent).unwrap_or(u16::MAX),
            );
            out.set(
                Stat::LifeLeechWheel,
                i64::from(wheel.life_leech_hundredths_percent) * 100,
            )?;
            out.set(
                Stat::ManaLeechWheel,
                i64::from(wheel.mana_leech_hundredths_percent) * 100,
            )?;
        }
        for (key, value) in [
            (Stat::CriticalChance, chance),
            (Stat::CriticalChanceBase, base_chance),
            (Stat::CriticalDamage, damage),
            (Stat::CriticalDamageBase, base_damage),
            (Stat::LifeLeech, life),
            (Stat::ManaLeech, mana),
            (
                Stat::CriticalChanceEquipment,
                self.equipment_only.critical_chance,
            ),
            (
                Stat::CriticalDamageEquipment,
                self.equipment_only.critical_damage,
            ),
            (Stat::LifeLeechEquipment, self.equipment_only.life_leech),
            (Stat::ManaLeechEquipment, self.equipment_only.mana_leech),
        ] {
            out.set(key, i64::from(value) * 100)?;
        }
        for (key, total, equipment) in [
            (
                Stat::CriticalChanceImbuement,
                self.offence.critical_chance,
                self.equipment_only.critical_chance,
            ),
            (
                Stat::CriticalDamageImbuement,
                self.offence.critical_damage,
                self.equipment_only.critical_damage,
            ),
            (
                Stat::LifeLeechImbuement,
                self.offence.life_leech,
                self.equipment_only.life_leech,
            ),
            (
                Stat::ManaLeechImbuement,
                self.offence.mana_leech,
                self.equipment_only.mana_leech,
            ),
        ] {
            out.set(key, (i64::from(total) - i64::from(equipment)) * 100)?;
        }
        let (base, total) = onslaught_ppm(self.offence.weapon_tier, self.offence.boots_tier);
        out.set(Stat::Onslaught, total)?;
        out.set(Stat::OnslaughtEquipment, base)?;
        out.set(Stat::OnslaughtAmplification, total - base)?;
        out.set(
            Stat::Cleave,
            i64::try_from(self.offence.cleave)
                .ok()
                .and_then(|n| n.checked_mul(100))
                .ok_or(TabWireError)?,
        )?;
        for (range, key) in (1_u16..=7).zip([
            Stat::PerfectShot1,
            Stat::PerfectShot2,
            Stat::PerfectShot3,
            Stat::PerfectShot4,
            Stat::PerfectShot5,
            Stat::PerfectShot6,
            Stat::PerfectShot7,
        ]) {
            out.set(
                key,
                self.offence.perfect_shot.get(&range).copied().unwrap_or(0),
            )?;
        }
        out.set(
            Stat::FlatDamageHealing,
            level_base_damage_healing(self.state.character_facts().level),
        )?;
        self.weapon(out)
    }

    fn weapon(&self, out: &mut CombatStats) -> Result<(), TabWireError> {
        let flat = level_base_damage_healing(self.state.character_facts().level);
        if let Some(loadout) = self.ranged {
            match loadout.damage {
                ranged::RangedDamage::Magic { element, .. } => {
                    out.set(Stat::AttackElement, element_id(element))?;
                    // Crystal's wand display reads maxHitChance, not the actual damage
                    // bounds. That value is not admitted as a native wand combat stat.
                }
                ranged::RangedDamage::Distance {
                    attack,
                    elemental_attack,
                    element,
                    cap,
                    chance,
                    launcher_bonus,
                } => {
                    let total = attack.checked_add(elemental_attack).ok_or(TabWireError)?;
                    out.set(Stat::AttackEquipment, i64::from(total))?;
                    out.set(Stat::AttackSkillId, 7)?;
                    out.set(Stat::AttackElement, 0)?;
                    out.set(Stat::ConvertedElement, element_id(element))?;
                    out.set(
                        Stat::ConvertedDamage,
                        if total == 0 {
                            0
                        } else {
                            i64::from(elemental_attack) * 1_000_000 / i64::from(total)
                        },
                    )?;
                    if let Some(skill) = self.skill(4) {
                        // Crystal protocolgame.cpp:5076-5085 reports the donor display formula.
                        let (total_attack, raw_attack) =
                            attack_totals(flat, total, skill, self.mode);
                        out.set(Stat::AttackTotal, total_attack)?;
                        out.set(Stat::AttackFightMode, total_attack - raw_attack)?;
                        out.set(Stat::AttackSkill, attack_skill(total, skill))?;
                        for (range, key) in (1..=7).zip([
                            Stat::Accuracy1,
                            Stat::Accuracy2,
                            Stat::Accuracy3,
                            Stat::Accuracy4,
                            Stat::Accuracy5,
                            Stat::Accuracy6,
                            Stat::Accuracy7,
                        ]) {
                            if range <= u32::from(loadout.range) {
                                let value =
                                    oteryn_gameplay_combat_rules::attack::ranged::shot_hit_chance(
                                        u32::from(cap),
                                        u32::try_from(chance).ok(),
                                        Some(i32::from(launcher_bonus)),
                                        range,
                                        skill,
                                    )
                                    .ok_or(TabWireError)?;
                                out.set(key, i64::from(value) * 10_000)?;
                            }
                        }
                    }
                }
            }
        } else {
            let constants = AttackConstants::checked_in().ok_or(TabWireError)?;
            let (attack, id, skill_index) = match self.weapon {
                SwingWeapon::Fists => (constants.fist.attack, 11, 0),
                SwingWeapon::Melee { attack, skill } => (
                    attack,
                    match skill.index() {
                        1 => 9,
                        2 => 8,
                        3 => 10,
                        _ => 11,
                    },
                    skill.index(),
                ),
            };
            out.set(Stat::AttackEquipment, i64::from(attack))?;
            out.set(Stat::AttackSkillId, id)?;
            out.set(Stat::AttackElement, 0)?;
            if let Some(skill) = self.skill(skill_index) {
                // Crystal protocolgame.cpp:5114-5126,5142-5154 reports these display values.
                let (total_attack, raw_attack) = attack_totals(flat, attack, skill, self.mode);
                out.set(Stat::AttackTotal, total_attack)?;
                out.set(Stat::AttackFightMode, total_attack - raw_attack)?;
                out.set(Stat::AttackSkill, attack_skill(attack, skill))?;
            }
        }
        Ok(())
    }

    fn defence(&self, out: &mut CombatStats) -> Result<(), TabWireError> {
        out.set(
            Stat::PhysicalReflection,
            i64::from(self.offence.reflect_flat),
        )?;
        out.set(Stat::Armor, i64::from(self.state.equipped_armor()))?;
        out.set(Stat::Mantra, i64::from(self.mantra))?;
        out.set(Stat::DefenceSkillId, 6)?;
        if let Some(wheel) = self.wheel {
            // Crystal 201189f player_wheel.cpp:2779 exposes the active Wheel aggregate.
            // `gem_value_milli` is thousandths of percent; the wire uses ppm of 100%.
            let dodge = wheel.gem_value_milli("dodge", "").max(0) * 10;
            out.set(Stat::Dodge, dodge)?;
            out.set(Stat::DodgeWheel, dodge)?;
            out.set(
                Stat::MitigationWheel,
                i64::from(wheel.mitigation_milli_percent) * 10,
            )?;
        }
        if let Some(percent) = self.state.player_mitigation_percent(self.now_micros) {
            let ppm = f64::from(percent) * 10_000.0;
            if ppm.is_finite() && (0.0..=i64::MAX as f64).contains(&ppm) {
                out.set(Stat::Mitigation, ppm.trunc() as i64)?;
            }
        }
        // Incoming defence adds the current Battle Instinct census to the live skill.
        // Attack-only proficiency adjustments must not inflate this read model.
        if let Some(skill) = self
            .state
            .live_skill(5, self.now_micros)
            .and_then(|skill| skill.checked_add(self.battle_instinct.shielding))
        {
            out.set(Stat::ShieldingSkill, i64::from(skill))?;
        }
        if let Some(hand) = self.state.equipped_defence() {
            out.set(Stat::DefenceEquipment, i64::from(hand.value))?;
            let constants = AttackConstants::checked_in().ok_or(TabWireError)?;
            let skill = match hand.skill_index {
                Some(index) => self
                    .state
                    .swing_skills(index, self.now_micros)
                    .map(|skills| skills.map_or(constants.starting_skill, |skills| skills.attack)),
                None => Some(0),
            };
            let bonus = match hand.skill_index {
                Some(1..=3) => self.battle_instinct.melee,
                Some(5) => self.battle_instinct.shielding,
                _ => 0,
            };
            if let Some(skill) = skill.and_then(|skill| skill.checked_add(bonus)) {
                // getDefense(true) reports the mode's post-swing factor (player.cpp:722-740).
                if let Some((_, value)) =
                    player_defence_bounds(constants, hand, skill, self.mode, true)
                {
                    out.set(Stat::DefenceTotal, i64::from(value))?;
                }
            }
        }
        Ok(())
    }

    fn misc(&self, out: &mut CombatStats) -> Result<(), TabWireError> {
        if let Some(wheel) = self.wheel {
            // Crystal 201189f wheel_gems.cpp:325 adds supreme-gem momentum globally.
            let momentum = wheel.gem_value_milli("momentum_chance", "").max(0) * 10;
            out.set(Stat::Momentum, momentum)?;
            out.set(Stat::MomentumWheel, momentum)?;
        }
        Ok(())
    }
}

/// Crystal `item.cpp:194-204,230-240`, `config.lua.dist:219-221,232-234` and
/// `protocolgame.cpp:10441-10474`; exactly the chance the existing `fatal` owner consumes.
fn onslaught_ppm(weapon_tier: u8, boots_tier: u8) -> (i64, i64) {
    if weapon_tier == 0 {
        return (0, 0);
    }
    let tier = i64::from(weapon_tier.min(10));
    let base = (5 * tier * tier + 40 * tier + 5) * 100;
    (
        base,
        base * (1_000_000 + amplification_ppm(boots_tier)) / 1_000_000,
    )
}

/// Crystal 201189f8 player.cpp:542-576: (attackTotal, attackRawTotal).
fn attack_totals(flat: i64, equipment: u32, skill: u32, mode: FightMode) -> (i64, i64) {
    let factor: f32 = match mode {
        FightMode::Offensive => 1.2,
        FightMode::Balanced => 1.0,
        FightMode::Defensive => 0.6,
    };
    let fight = f64::from((factor * equipment as f32).floor());
    let skill_factor = f64::from(skill.saturating_add(4)) / 28.0;
    (
        (flat as f64 + fight * skill_factor) as i64,
        (flat as f64 + f64::from(equipment) * skill_factor) as i64,
    )
}

/// Crystal 201189f8 player.cpp:427-431 and :467-469.
fn attack_skill(weapon_attack: u32, skill: u32) -> i64 {
    let attack = f64::from(weapon_attack);
    (attack * (f64::from(skill.saturating_add(4)) / 28.0) - attack) as i64
}

fn amplification_ppm(boots_tier: u8) -> i64 {
    if boots_tier == 0 {
        return 0;
    }
    let tier = i64::from(boots_tier.min(10));
    (40 * tier * tier + 170 * tier + 40) * 100
}

/// Crystal `utils_definitions.hpp:36-50` (Cipbia element ids).
fn element_id(element: crate::content::ProjectV2RangedElement) -> i64 {
    use crate::content::ProjectV2RangedElement as Element;
    match element {
        Element::Physical => 0,
        Element::Fire => 1,
        Element::Earth => 2,
        Element::Energy => 3,
        Element::Ice => 4,
        Element::Death => 6,
    }
}

fn vocation_key(vocation: crate::spell::Vocation) -> &'static str {
    use crate::spell::Vocation as V;
    match vocation {
        V::None => "none",
        V::Druid => "druid",
        V::ElderDruid => "elder_druid",
        V::Sorcerer => "sorcerer",
        V::MasterSorcerer => "master_sorcerer",
        V::Knight => "knight",
        V::EliteKnight => "elite_knight",
        V::Paladin => "paladin",
        V::RoyalPaladin => "royal_paladin",
        V::Monk => "monk",
        V::ExaltedMonk => "exalted_monk",
    }
}

#[cfg(test)]
#[path = "cyclopedia_tests.rs"]
mod tests;
