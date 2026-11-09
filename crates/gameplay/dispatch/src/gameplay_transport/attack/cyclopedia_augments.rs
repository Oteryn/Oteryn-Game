//! Read-only spell modifiers from the same owners used by a current cast.
use crate::durability::character_equipment::EquipmentSnapshot;
use crate::foundation::GameSessionId;
use crate::gameplay_transport::ComposedFreshAdmission;
use oteryn_gameplay_content::content::{
    item_spell_augments::{ItemSpellAugment, ItemSpellAugmentKind as K},
    native_gameplay::NativeGameplayState,
};
use oteryn_gameplay_progression::{wheel::WheelStages, wheel_effects::WheelRuntimeEffects};
use oteryn_protocol_oteryn::cyclopedia_character::offence::{
    Augment, Effect, MAX_ROWS, MAX_SOURCES, Snapshot, Source, SourceKind,
};
use std::collections::BTreeMap;
#[derive(Default)]
struct Capture {
    data: Snapshot,
    rows: BTreeMap<(u8, String, Effect), i64>,
}
impl Capture {
    fn add(
        &mut self,
        kind: SourceKind,
        slot: u8,
        key: &str,
        spell: &str,
        effect: Effect,
        value: i64,
    ) -> Result<(), ()> {
        if value == 0 {
            return Ok(());
        }
        if value < 0 {
            return Err(());
        }
        let source = Source {
            kind,
            slot,
            key: key.into(),
        };
        let index = if let Some(i) = self.data.sources.iter().position(|s| s == &source) {
            i
        } else {
            if self.data.sources.len() >= MAX_SOURCES {
                return Err(());
            }
            self.data.sources.push(source);
            self.data.sources.len() - 1
        };
        let entry = self
            .rows
            .entry((index as u8, spell.into(), effect))
            .or_default();
        *entry = entry.checked_add(value).ok_or(())?;
        if self.rows.len() > MAX_ROWS {
            return Err(());
        }
        Ok(())
    }
    fn native(
        &mut self,
        kind: SourceKind,
        slot: u8,
        key: &str,
        spell: &str,
        a: &ItemSpellAugment,
    ) -> Result<(), ()> {
        let effect = match a.kind {
            K::BaseDamage => Effect::BaseDamage,
            K::BaseHealing => Effect::BaseHealing,
            K::CriticalHitChance => Effect::CriticalChance,
            K::CriticalExtraDamage => Effect::CriticalDamage,
            K::ManaLeech => Effect::ManaLeech,
            K::LifeLeech => Effect::LifeLeech,
            K::Cooldown => Effect::Cooldown,
        };
        self.add(kind, slot, key, spell, effect, i64::from(a.value))
    }
    fn finish(mut self) -> Result<Snapshot, ()> {
        self.data.rows = self
            .rows
            .into_iter()
            .map(|((source, spell, effect), value)| Augment {
                source,
                spell,
                effect,
                value,
            })
            .collect();
        self.data.encode().map_err(|_| ())?;
        Ok(self.data)
    }
}
// These conversions are the exact quantization the magnitude/cooldown owners consume.
fn gem(kind: &str, value: i64) -> Result<Option<(Effect, i64)>, ()> {
    let (effect, value) = match kind {
        "base_damage_bonus" => (
            Effect::BaseDamage,
            (value / 1000).checked_mul(100).ok_or(())?,
        ),
        "base_healing_bonus" => (
            Effect::BaseHealing,
            (value / 1000).checked_mul(100).ok_or(())?,
        ),
        "critical_extra_damage" => (Effect::CriticalDamage, value / 10),
        "cooldown_reduction" => (Effect::Cooldown, value),
        _ => return Ok(None),
    };
    if value < 0 {
        return Err(());
    }
    Ok(Some((effect, value)))
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn capture_cyclopedia_augments(
        &self,
        session: GameSessionId,
        equipment: &EquipmentSnapshot,
        native: &NativeGameplayState,
        wheel: Option<&WheelRuntimeEffects>,
        stages: &WheelStages,
    ) -> Result<Snapshot, ()> {
        let mut out = Capture::default();
        out.data.known_sources = SourceKind::Equipment.bit();
        let canonical = |name: &str| {
            self.spells
                .spells
                .iter()
                .find(|spell| spell.name.eq_ignore_ascii_case(name))
                .map_or_else(|| name.to_owned(), |spell| spell.name.clone())
        };
        for item in &equipment.items {
            let policy = native
                .item_policy(
                    &item.definition.production_key,
                    &item.definition.revision_ref,
                )
                .ok_or(())?;
            for augment in &policy.record().attributes.spell_augments {
                out.native(
                    SourceKind::Equipment,
                    item.slot,
                    &item.definition.production_key,
                    &canonical(&augment.spell),
                    augment,
                )?;
            }
        }
        if let Some(catalogue) = &self.proficiency_catalogue
            && let Ok(sessions) = self.states.proficiencies.lock()
        {
            out.data.known_sources |= SourceKind::Proficiency.bit();
            if let Some(weapon) = equipment.items.iter().find(|item| item.slot == 5)
                && let Some(track) = sessions
                    .get(&session)
                    .and_then(|owner| owner.track(&weapon.definition.production_key))
            {
                for spell in &self.spells.spells {
                    if let Some(perks) =
                        catalogue.cast_perks(&weapon.definition.production_key, track, Some(spell))
                    {
                        for augment in perks.spell_augments() {
                            out.native(
                                SourceKind::Proficiency,
                                5,
                                &weapon.definition.production_key,
                                &canonical(&augment.spell),
                                augment,
                            )?;
                        }
                    }
                }
            }
        }
        if let Some(wheel) = wheel {
            out.data.known_sources |= SourceKind::Wheel.bit();
            let key = "Current Wheel allocation and gems";
            for ((kind, spell), value) in &wheel.sockets.gem_values_milli {
                if let Some((effect, value)) = gem(kind, *value)? {
                    let name = if spell == "Critical Extra Damage" {
                        "All critical spell hits".into()
                    } else {
                        canonical(spell)
                    };
                    out.add(SourceKind::Wheel, 0, key, &name, effect, value)?;
                }
            }
            for spell in self
                .spells
                .spells
                .iter()
                .filter(|spell| matches!(spell.carrier, crate::spell::Carrier::Instant { .. }))
            {
                let stage = stages.spell_augment_stage(&spell.name);
                if stage > 0 {
                    out.add(
                        SourceKind::Wheel,
                        0,
                        key,
                        &spell.name,
                        Effect::WheelStage,
                        i64::from(stage),
                    )?;
                }
                let cooldown = match spell.name.as_str() {
                    "Mystic Repulse" if stage >= 1 => 4000,
                    "Thousand Fist Blows" if stage >= 2 => 4000,
                    _ => 0,
                };
                out.add(
                    SourceKind::Wheel,
                    0,
                    key,
                    &spell.name,
                    Effect::Cooldown,
                    cooldown,
                )?;
                if spell
                    .groups
                    .get(1)
                    .is_some_and(|group| group.key == "focus")
                    && wheel.active_unique_perks.contains("focus_mastery")
                {
                    out.add(
                        SourceKind::Wheel,
                        0,
                        key,
                        &spell.name,
                        Effect::GroupCooldown,
                        2000,
                    )?;
                }
            }
        }
        out.finish()
    }
}
#[cfg(test)]
#[path = "cyclopedia_augments_tests.rs"]
mod tests;
