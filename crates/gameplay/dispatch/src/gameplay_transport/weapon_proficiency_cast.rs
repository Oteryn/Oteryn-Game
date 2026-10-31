//! Selected SpellAugment and SkillBonus rewards follow the currently worn weapon.
use super::*;
use crate::spell::{SpellDefinition, owned_cast_facts::OwnedCastFacts};
use oteryn_gameplay_content::content::item_spell_augments::{
    ItemSpellAugment, ItemSpellAugmentKind as K,
};
use oteryn_gameplay_content::content::proficiency_perk_identity;

#[derive(Default)]
pub(in crate::gameplay_transport) struct CastPerks {
    pub(in crate::gameplay_transport) skills: [u32; 8],
    augments: Vec<ItemSpellAugment>,
}

impl CastPerks {
    fn add(
        &mut self,
        identity: &str,
        value: ProjectV2ExactRatio,
        spell: Option<&SpellDefinition>,
    ) -> Option<()> {
        let scaled = |scale: i64| -> Option<u32> {
            u32::try_from(
                value
                    .numerator
                    .checked_mul(scale)?
                    .checked_div(i64::try_from(value.denominator).ok()?)?,
            )
            .ok()
        };
        if let Some(skill) = identity.strip_prefix("skill_bonus:") {
            let index = match skill {
                "fist" => 0,
                "club" => 1,
                "sword" => 2,
                "axe" => 3,
                "distance" => 4,
                "shielding" => 5,
                "fishing" => 6,
                "magic_level" => 7,
                _ => return None,
            };
            self.skills[index] = self.skills[index].checked_add(scaled(1)?)?;
        } else if let Some(augment) = identity.strip_prefix("spell_augment:") {
            let Some(spell) = spell else {
                return Some(());
            };
            if !matches!(spell.carrier, crate::spell::Carrier::Instant { .. }) {
                return Some(());
            }
            let (kind, id) = augment.rsplit_once(':')?;
            if spell
                .authored
                .as_ref()
                .and_then(|p| p.header.reference_spell_id)
                != Some(id.parse().ok()?)
            {
                return Some(());
            }
            let kind = match kind {
                "base_damage" => K::BaseDamage,
                "healing" => K::BaseHealing,
                "critical_hit_chance" => K::CriticalHitChance,
                "critical_extra_damage" => K::CriticalExtraDamage,
                "life_leech" => K::LifeLeech,
                "mana_leech" => K::ManaLeech,
                "cooldown" => K::Cooldown,
                _ => return None,
            };
            let amount = if kind == K::Cooldown {
                scaled(-1000)?
            } else {
                scaled(10000)?
            };
            self.augments.push(ItemSpellAugment {
                spell: spell.name.clone(),
                kind,
                value: amount,
            });
        }
        Some(())
    }
}

impl ProductionProficiencyCatalogue {
    pub(in crate::gameplay_transport) fn cast_perks(
        &self,
        item_key: &str,
        track: &LiveProficiencyTrack,
        spell: Option<&SpellDefinition>,
    ) -> Option<CastPerks> {
        let resolved = self.resolved(item_key)?;
        if track.shape != resolved.shape {
            return None;
        }
        let definition = self.data.definitions().get(&resolved.definition_key)?;
        let mut result = CastPerks::default();
        let mut replaced = Vec::new();
        for (slot, row) in (1..).zip(&track.modifications) {
            let Some(row) = row else { continue };
            if !proficiency_modification_active(
                resolved.shape,
                track.progress,
                &track.selections,
                slot,
                row,
            ) {
                continue;
            }
            replaced.push(row.level);
            let (identity, unit, mut value) =
                shaping::modified_value(row, &resolved.definition_key)?;
            if unit == ImportedShapingUnit::Percent {
                value.denominator = value.denominator.checked_mul(100)?;
            }
            result.add(identity, value, spell)?;
        }
        for selected in resolved
            .shape
            .active_selections(track.progress, &track.selections)
            .ok()?
        {
            if replaced.contains(&selected.level) {
                continue;
            }
            let perk = definition
                .levels
                .get(usize::from(selected.level))?
                .perks
                .get(usize::from(selected.perk_index))?;
            let value = match perk {
                ProjectV2ProficiencyPerk::SpellAugment { value, .. }
                | ProjectV2ProficiencyPerk::SkillBonus { value, .. } => *value,
                _ => continue,
            };
            result.add(&proficiency_perk_identity(perk)?, value, spell)?;
        }
        Some(result)
    }
}

impl super::super::ComposedFreshAdmission<'_, '_, '_> {
    /// Called under the cast's runtime/spell locks after its current equipment read.
    pub(in crate::gameplay_transport) fn install_cast_proficiency(
        &self,
        spell: &SpellDefinition,
        owned: &mut OwnedCastFacts,
    ) {
        let Some(catalogue) = &self.proficiency_catalogue else {
            return;
        };
        let Some(weapon) = owned.equipment().items.iter().find(|item| item.slot == 5) else {
            return;
        };
        let Ok(sessions) = self.states.proficiencies.lock() else {
            return;
        };
        let Some(track) = sessions
            .get(&owned.binding.session)
            .and_then(|owner| owner.track(&weapon.definition.production_key))
        else {
            return;
        };
        let Some(perks) =
            catalogue.cast_perks(&weapon.definition.production_key, track, Some(spell))
        else {
            return;
        };
        owned.access.proficiency_skills = perks.skills;
        if let Some(magnitude) = &mut owned.access.magnitude {
            magnitude.item_augments.extend(perks.augments);
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "committed catalogue and authored spell fixtures"
)]
mod tests {
    use super::*;

    #[test]
    fn c11r_amber_selected_augments_and_skills_use_global_rewards() {
        let catalogue = ProductionProficiencyCatalogue::embedded().expect("catalogue");
        let key = "oteryn:item.tibia.i50239";
        let mut track = catalogue.empty_track(key).expect("amber track");
        track.progress = track.shape.track_shape().final_progress();
        track.selections = vec![None, Some(0), Some(0), Some(0), Some(1), None, None];
        let data: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
        )).expect("spells");
        let spell = |key: &str| {
            let row = data["bundles"]
                .as_array()
                .expect("bundles")
                .iter()
                .find(|row| {
                    row["bundle"]["spell"]["identity"]["key"] == format!("candidate:spell/{key}")
                })
                .expect("spell row");
            oteryn_gameplay_content::content::spell_data::authoring::spell_from_bundle(
                &row["bundle"],
                &row["dependencies"],
            )
            .expect("authored spell")
        };
        let knockout = spell("devastating_knockout");
        let perks = catalogue
            .cast_perks(key, &track, Some(&knockout))
            .expect("perks");
        assert_eq!(perks.skills, [0, 0, 0, 0, 0, 0, 0, 2]);
        assert_eq!(
            perks
                .augments
                .iter()
                .filter(|p| p.kind == K::BaseDamage)
                .map(|p| p.value)
                .sum::<u32>(),
            500
        );
        assert_eq!(
            perks
                .augments
                .iter()
                .filter(|p| p.kind == K::CriticalExtraDamage)
                .map(|p| p.value)
                .sum::<u32>(),
            1400
        );
        assert!(perks.augments.iter().all(|p| p.kind != K::Cooldown));
        let other = catalogue
            .cast_perks(key, &track, Some(&spell("tiger_clash")))
            .expect("other spell");
        assert!(other.augments.is_empty());
        assert_eq!(other.skills[7], 2);
        track.progress = 0;
        let unearned = catalogue
            .cast_perks(key, &track, Some(&knockout))
            .expect("unearned");
        assert_eq!(unearned.skills, [0; 8]);
        assert!(unearned.augments.is_empty());
    }
}
