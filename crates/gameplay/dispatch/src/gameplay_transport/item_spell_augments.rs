//! Numerical Item and Wheel contributions from the same fenced cast snapshot.
use crate::spell::{SpellDefinition, cast::PlayerSpellState, owned_cast_facts::OwnedCastFacts};
use oteryn_gameplay_content::content::item_spell_augments::{
    ItemSpellAugment, ItemSpellAugmentKind as K,
};
use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition;
use oteryn_simulation_determinism::SemanticTimeMicros;

fn selected<'a>(
    owned: &'a OwnedCastFacts,
    spell: &'a SpellDefinition,
) -> impl Iterator<Item = &'a ItemSpellAugment> {
    owned
        .magnitude()
        .into_iter()
        .flat_map(|a| &a.item_augments)
        .filter(move |a| {
            matches!(spell.carrier, crate::spell::Carrier::Instant { .. })
                && a.spell.eq_ignore_ascii_case(&spell.name)
        })
}

pub(super) fn cooldown(
    owned: &OwnedCastFacts,
    spell: &SpellDefinition,
    next: &mut PlayerSpellState,
    anchor: &mut crate::spell::combat_batch::SpellAnchor,
    now: SemanticTimeMicros,
) -> Result<(), SpellCastDisposition> {
    let item_reduction = selected(owned, spell)
        .filter(|a| a.kind == K::Cooldown)
        .try_fold(0u64, |sum, a| sum.checked_add(u64::from(a.value) * 1000))
        .ok_or(SpellCastDisposition::Rejected)?;
    // News 8944: Mystic Repulse I and Thousand Fist Blows II each remove
    // four seconds. Read the current allocation at cast time, including its
    // binding/expiry checks; an unavailable projection grants no bonus.
    let wheel_reduction = if matches!(spell.carrier, crate::spell::Carrier::Instant { .. })
        && owned.has_current_wheel(now.get())
    {
        let required_stage = match spell.name.as_str() {
            "Mystic Repulse" => Some(1),
            "Thousand Fist Blows" => Some(2),
            _ => None,
        };
        if let Some(required_stage) = required_stage {
            let stage = owned
                .wheel_stage(&spell.name, now.get())
                .map_err(|_| SpellCastDisposition::Rejected)?;
            if stage >= required_stage {
                4_000_000
            } else {
                0
            }
        } else {
            0
        }
    } else {
        0
    };
    // Crystal spells.cpp:776,785-786: gems reduce only the named individual
    // timer; the existing half-base floor applies after all individual reductions.
    let gem_reduction =
        u64::try_from(owned.current_wheel_effects(now.get()).map_or(0, |effects| {
            effects.gem_value_milli("cooldown_reduction", &spell.name)
        }))
        .ok()
        .and_then(|millis| millis.checked_mul(1_000))
        .ok_or(SpellCastDisposition::Rejected)?;
    let reduction = item_reduction
        .checked_add(wheel_reduction)
        .and_then(|value| value.checked_add(gem_reduction))
        .ok_or(SpellCastDisposition::Rejected)?;
    if reduction > 0 {
        let current = next
            .cooldowns
            .spell_ready_at(&spell.key)
            .ok_or(SpellCastDisposition::Rejected)?;
        let duration = current
            .get()
            .saturating_sub(now.get())
            .saturating_sub(reduction)
            .max(spell.cooldown_micros / 2);
        next.cooldowns.spells.insert(
            spell.key.clone(),
            now.checked_add(duration)
                .map_err(|_| SpellCastDisposition::Rejected)?,
        );
        anchor.cooldown_deadlines = next.cooldowns.canonical_deadlines();
    }
    // Crystal201189f spells.cpp:805-820: Focus is a secondary-group reduction;
    // unlike individual cooldowns it has no half-base floor.
    if let Some(group) = spell.groups.get(1).filter(|group| group.key == "focus")
        && owned
            .current_wheel_effects(now.get())
            .is_some_and(|effects| effects.active_unique_perks.contains("focus_mastery"))
        && let Some(current) = next.cooldowns.group_ready_at(&group.key)
    {
        let duration = current
            .get()
            .saturating_sub(now.get())
            .saturating_sub(2_000_000);
        if duration == 0 {
            next.cooldowns.groups.remove(&group.key);
        } else {
            next.cooldowns.groups.insert(
                group.key.clone(),
                now.checked_add(duration)
                    .map_err(|_| SpellCastDisposition::Rejected)?,
            );
        }
        anchor.cooldown_deadlines = next.cooldowns.canonical_deadlines();
    }
    Ok(())
}

pub(super) fn leech(
    owned: &OwnedCastFacts,
    spell: &SpellDefinition,
    before: &PlayerSpellState,
    now: SemanticTimeMicros,
) -> (u32, u32) {
    // Equipment leeches on ordinary damage spells
    // even when this spell has no spell-specific leech augment.
    let base = before.equipped_offence();
    let mut rates = (u32::from(base.life_leech), u32::from(base.mana_leech));
    if let Some(effects) = owned.current_wheel_effects(now.get()) {
        rates.0 = rates
            .0
            .saturating_add(effects.life_leech_hundredths_percent);
        rates.1 = rates
            .1
            .saturating_add(effects.mana_leech_hundredths_percent);
    }
    for a in selected(owned, spell) {
        match a.kind {
            K::LifeLeech => rates.0 = rates.0.saturating_add(a.value),
            K::ManaLeech => rates.1 = rates.1.saturating_add(a.value),
            _ => {}
        }
    }
    rates
}

pub(super) fn alters_base(owned: &OwnedCastFacts, spell: &SpellDefinition) -> bool {
    selected(owned, spell).any(|a| a.value > 0 && matches!(a.kind, K::BaseDamage | K::BaseHealing))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::spell::owned_cast_facts::{AccessProjections, CastFactsBinding, CurrentProjection};
    use std::collections::BTreeMap;

    #[test]
    fn news_8944_amber_kusarigama_import_keeps_mastery_rewards() {
        use crate::content::{
            ProjectV2ExactRatio, ProjectV2ProficiencyPerk, ProjectV2ProficiencySkill,
            ProjectV2ProficiencySpellAugment, import_committed_weapon_proficiency_data,
        };
        let data = import_committed_weapon_proficiency_data().unwrap();
        let binding = &data.bindings()["oteryn:item.tibia.i50239"];
        let definition = &data.definitions()[&binding.profile_binding.key];
        assert_eq!(definition.identity.key, "oteryn:proficiency.tibia.p43");
        assert_eq!(
            definition.levels[2].perks[0],
            ProjectV2ProficiencyPerk::SpellAugment {
                spell_client_id: 293,
                augment: ProjectV2ProficiencySpellAugment::BaseDamage,
                value: ProjectV2ExactRatio {
                    numerator: 1,
                    denominator: 20
                },
            }
        );
        assert!(
            definition.levels[4]
                .perks
                .contains(&ProjectV2ProficiencyPerk::SkillBonus {
                    skill: ProjectV2ProficiencySkill::MagicLevel,
                    value: ProjectV2ExactRatio {
                        numerator: 2,
                        denominator: 1
                    },
                })
        );
    }

    #[test]
    fn news_8944_monk_wheel_cooldowns_use_current_stage_and_preserve_groups() {
        let (_, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(88);
        let binding = CastFactsBinding {
            actor,
            session,
            character: [7; 16],
            character_revision: 4,
            lease_generation: 1,
            connection_generation: 1,
            player_revision: 1,
            content_digest: [9; 32],
            equipment_revision: 2,
        };
        let equipment = crate::durability::character_equipment::EquipmentSnapshot {
            character: binding.character,
            content_digest: binding.content_digest,
            character_revision: binding.character_revision,
            revision: binding.equipment_revision,
            combat_mode: None,
            items: vec![],
        };
        let mut owned = OwnedCastFacts::from_owner_reads(
            binding.clone(),
            crate::durability::character_build::DurableBuildState::new(
                "exalted_monk",
                (0, 0),
                [(10, 0); 7],
            )
            .unwrap(),
            200,
            equipment,
            AccessProjections {
                wheel: Some(CurrentProjection {
                    binding,
                    authority_revision: 4,
                    valid_until_micros: 10_000_000,
                    value: BTreeMap::new(),
                }),
                ..AccessProjections::default()
            },
        )
        .unwrap();
        let catalogue: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
        )).unwrap();
        for (key, minimum_stage) in [("mystic_repulse", 1), ("thousand_fist_blows", 2)] {
            let row = catalogue["bundles"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| {
                    row["bundle"]["spell"]["identity"]["key"] == format!("candidate:spell/{key}")
                })
                .unwrap();
            let spell = oteryn_gameplay_content::content::spell_data::authoring::spell_from_bundle(
                &row["bundle"],
                &row["dependencies"],
            )
            .unwrap();
            for stage in 0..=2 {
                for expired in [false, true] {
                    owned.access.wheel.as_mut().unwrap().value =
                        BTreeMap::from([(spell.name.clone(), stage)]);
                    let now = SemanticTimeMicros::from_micros(if expired { 10_000_000 } else { 1 });
                    let mut next = PlayerSpellState::new(
                        crate::spell::cast::CharacterCastFacts {
                            vocation: crate::spell::Vocation::ExaltedMonk,
                            level: 200,
                            magic_level: 0,
                            max_health: 2000,
                            max_mana: 2000,
                            max_soul: 100,
                        },
                        0,
                        0,
                    )
                    .unwrap();
                    next.cooldowns.rearm(&spell, now).unwrap();
                    let groups = next.cooldowns.groups.clone();
                    let mut anchor = crate::spell::combat_batch::SpellAnchor {
                        expected_revision: 1,
                        next_revision: 2,
                        paid_mana: 0,
                        paid_soul: 0,
                        cooldown_deadlines: next.cooldowns.canonical_deadlines(),
                    };
                    cooldown(&owned, &spell, &mut next, &mut anchor, now).unwrap();
                    let duration = if stage >= minimum_stage && !expired {
                        4_000_000
                    } else {
                        8_000_000
                    };
                    assert_eq!(
                        next.cooldowns.spell_ready_at(&spell.key).unwrap().get(),
                        now.get() + duration
                    );
                    assert_eq!(next.cooldowns.groups, groups);
                    assert_eq!(
                        anchor.cooldown_deadlines,
                        next.cooldowns.canonical_deadlines()
                    );
                }
            }
        }
    }
    #[test]
    fn focus_mastery_reduces_only_secondary_group_without_individual_floor() {
        let (_, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(89);
        let binding = CastFactsBinding {
            actor,
            session,
            character: [7; 16],
            character_revision: 4,
            lease_generation: 1,
            connection_generation: 1,
            player_revision: 1,
            content_digest: [9; 32],
            equipment_revision: 2,
        };
        let equipment = crate::durability::character_equipment::EquipmentSnapshot {
            character: binding.character,
            content_digest: binding.content_digest,
            character_revision: 4,
            revision: 2,
            combat_mode: None,
            items: vec![],
        };
        let mut effects =
            oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects::default();
        effects.active_unique_perks.insert("focus_mastery".into());
        let mut owned = OwnedCastFacts::from_owner_reads(
            binding.clone(),
            crate::durability::character_build::DurableBuildState::new(
                "sorcerer",
                (0, 0),
                [(10, 0); 7],
            )
            .unwrap(),
            200,
            equipment,
            AccessProjections {
                premium: Some(CurrentProjection {
                    binding: binding.clone(),
                    authority_revision: 1,
                    valid_until_micros: 10_000_000,
                    value: true,
                }),
                wheel_promoted: true,
                wheel_effects: Some(CurrentProjection {
                    binding: binding.clone(),
                    authority_revision: 4,
                    valid_until_micros: 10_000_000,
                    value: effects,
                }),
                ..Default::default()
            },
        )
        .unwrap();
        let mut spell = crate::spell::cast::v1_spell_book()
            .unwrap()
            .spells
            .remove(0);
        spell.name = "Hell's Core".into();
        spell.groups = vec![
            crate::spell::CooldownGroup {
                key: "attack".into(),
                cooldown_micros: 2_000_000,
            },
            crate::spell::CooldownGroup {
                key: "focus".into(),
                cooldown_micros: 3_000_000,
            },
        ];
        let now = SemanticTimeMicros::from_micros(1);
        for (duration, promoted, expected) in [
            (3_000_000, true, Some(1_000_001)),
            (1_000_000, true, None),
            (3_000_000, false, Some(3_000_001)),
        ] {
            spell.groups[1].cooldown_micros = duration;
            owned.access.wheel_promoted = promoted;
            let mut next = PlayerSpellState::new(
                crate::spell::cast::CharacterCastFacts {
                    vocation: crate::spell::Vocation::Sorcerer,
                    level: 200,
                    magic_level: 0,
                    max_health: 2000,
                    max_mana: 2000,
                    max_soul: 100,
                },
                0,
                0,
            )
            .unwrap();
            next.cooldowns.rearm(&spell, now).unwrap();
            let original = next.cooldowns.spell_ready_at(&spell.key);
            let mut anchor = crate::spell::combat_batch::SpellAnchor {
                expected_revision: 1,
                next_revision: 2,
                paid_mana: 0,
                paid_soul: 0,
                cooldown_deadlines: next.cooldowns.canonical_deadlines(),
            };
            cooldown(&owned, &spell, &mut next, &mut anchor, now).unwrap();
            assert_eq!(
                next.cooldowns
                    .group_ready_at("focus")
                    .map(|time| time.get()),
                expected
            );
            assert_eq!(
                next.cooldowns
                    .group_ready_at("attack")
                    .map(|time| time.get()),
                Some(2_000_001)
            );
            assert_eq!(next.cooldowns.spell_ready_at(&spell.key), original);
            assert_eq!(
                anchor.cooldown_deadlines,
                next.cooldowns.canonical_deadlines()
            );
        }
    }

    #[test]
    fn active_gem_cooldown_and_spell_leech_use_same_current_build_and_expiry() {
        let (_, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(89);
        let binding = CastFactsBinding {
            actor,
            session,
            character: [7; 16],
            character_revision: 4,
            lease_generation: 1,
            connection_generation: 1,
            player_revision: 1,
            content_digest: [9; 32],
            equipment_revision: 2,
        };
        let equipment = crate::durability::character_equipment::EquipmentSnapshot {
            character: binding.character,
            content_digest: binding.content_digest,
            character_revision: 4,
            revision: 2,
            combat_mode: None,
            items: vec![],
        };
        let mut spell = crate::spell::cast::v1_spell_book()
            .unwrap()
            .spells
            .remove(0);
        spell.name = "Executioner's Throw".into();
        spell.cooldown_micros = 10_000_000;
        let mut effects =
            oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects::default();
        effects
            .sockets
            .gem_values_milli
            .insert(("cooldown_reduction".into(), spell.name.clone()), 8_000);
        effects.life_leech_hundredths_percent = 350;
        effects.mana_leech_hundredths_percent = 200;
        let mut owned = OwnedCastFacts::from_owner_reads(
            binding.clone(),
            crate::durability::character_build::DurableBuildState::new(
                "elite_knight",
                (0, 0),
                [(10, 0); 7],
            )
            .unwrap(),
            500,
            equipment,
            AccessProjections {
                premium: Some(CurrentProjection {
                    binding: binding.clone(),
                    authority_revision: 1,
                    valid_until_micros: 10_000_000,
                    value: true,
                }),
                wheel_promoted: true,
                wheel_effects: Some(CurrentProjection {
                    binding: binding.clone(),
                    authority_revision: 4,
                    valid_until_micros: 10_000_000,
                    value: effects,
                }),
                ..Default::default()
            },
        )
        .unwrap();
        for (at, promoted, expected, rates) in [
            (1, true, 5_000_000, (350, 200)),
            (1, false, 10_000_000, (0, 0)),
            (10_000_000, true, 10_000_000, (0, 0)),
        ] {
            owned.access.wheel_promoted = promoted;
            let now = SemanticTimeMicros::from_micros(at);
            let mut next = PlayerSpellState::new(
                crate::spell::cast::CharacterCastFacts {
                    vocation: crate::spell::Vocation::Knight,
                    level: 500,
                    magic_level: 0,
                    max_health: 2000,
                    max_mana: 1000,
                    max_soul: 100,
                },
                0,
                0,
            )
            .unwrap();
            next.cooldowns.rearm(&spell, now).unwrap();
            let groups = next.cooldowns.groups.clone();
            let mut anchor = crate::spell::combat_batch::SpellAnchor {
                expected_revision: 1,
                next_revision: 2,
                paid_mana: 0,
                paid_soul: 0,
                cooldown_deadlines: next.cooldowns.canonical_deadlines(),
            };
            cooldown(&owned, &spell, &mut next, &mut anchor, now).unwrap();
            assert_eq!(
                next.cooldowns.spell_ready_at(&spell.key).unwrap().get(),
                at + expected
            );
            assert_eq!(
                next.cooldowns.groups, groups,
                "gem reduces the individual timer only"
            );
            assert_eq!(leech(&owned, &spell, &next, now), rates);
            assert_eq!(
                anchor.cooldown_deadlines,
                next.cooldowns.canonical_deadlines()
            );
        }
    }
}
