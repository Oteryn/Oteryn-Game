//! EQUIP-0 §3–§4: derive only from the equipment owner and the Channel's pinned content.

use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{ReferenceEquipmentSlot, ReferenceItemField};
use crate::durability::character_equipment::EquipmentSnapshot;
use oteryn_gameplay_combat_rules::attack::AttackConstants;
use oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence;
use oteryn_gameplay_combat_rules::resolution::player_defence::{HandDefence, hand_defence};
use oteryn_gameplay_content::content::item_abilities::{
    item_ability_profile, item_offence_modifiers,
};
use oteryn_gameplay_domain::domain::death::EquipmentSlot;
use oteryn_gameplay_items::equipment_effects::{
    EquipmentEffectInput, EquipmentEffects, derive_equipment_effects,
};

/// Durable slot codes are neither wire discriminants nor death draw indexes.
fn canonical_slot(slot: u8) -> Option<(EquipmentSlot, ReferenceEquipmentSlot)> {
    Some(match slot {
        1 => (EquipmentSlot::Head, ReferenceEquipmentSlot::Head),
        7 => (EquipmentSlot::Necklace, ReferenceEquipmentSlot::Amulet),
        9 => (EquipmentSlot::Container, ReferenceEquipmentSlot::Container),
        2 => (EquipmentSlot::Armor, ReferenceEquipmentSlot::Torso),
        5 => (EquipmentSlot::RightHand, ReferenceEquipmentSlot::Weapon),
        6 => (EquipmentSlot::LeftHand, ReferenceEquipmentSlot::Shield),
        3 => (EquipmentSlot::Legs, ReferenceEquipmentSlot::Legs),
        4 => (EquipmentSlot::Feet, ReferenceEquipmentSlot::Feet),
        8 => (EquipmentSlot::Ring, ReferenceEquipmentSlot::Ring),
        10 => (EquipmentSlot::Ammo, ReferenceEquipmentSlot::Extra),
        _ => return None,
    })
}

pub(in crate::gameplay_transport) fn equipment_effects(
    equipment: &EquipmentSnapshot,
    content: &NativeGameplayState,
) -> Option<EquipmentEffects> {
    equipment_effects_inner(equipment, content, false)
}

fn equipment_effects_inner(
    equipment: &EquipmentSnapshot,
    content: &NativeGameplayState,
    require_known_speed: bool,
) -> Option<EquipmentEffects> {
    if equipment.content_digest != content.source_digest() {
        return None;
    }
    let inputs = equipment
        .items
        .iter()
        .map(|item| {
            if item.definition.family != "Item" {
                return None;
            }
            let (slot, semantic_slot) = canonical_slot(item.slot)?;
            let policy = content.item_policy(
                &item.definition.production_key,
                &item.definition.revision_ref,
            )?;
            let semantics = &policy.record().semantics;
            let in_own_slot = in_own_slot(semantics, semantic_slot);
            let input = EquipmentEffectInput {
                slot,
                definition_key: (
                    &policy.record().production_definition.production_key,
                    &policy.record().production_definition.revision_ref,
                ),
                in_own_slot,
                profile: item_ability_profile(semantics).ok()?,
            };
            if require_known_speed && input.contributes_speed() {
                policy.record().attributes.speed_bonus?;
            }
            Some(input)
        })
        .collect::<Option<Vec<_>>>()?;
    derive_equipment_effects(inputs)
}

fn in_own_slot(
    semantics: &crate::content::ReferenceItemSemantics,
    semantic_slot: ReferenceEquipmentSlot,
) -> bool {
    matches!(&semantics.equipment, ReferenceItemField::Known(group)
    if matches!(&group.patterns, ReferenceItemField::Known(patterns)
        if patterns.iter().any(|pattern| match pattern.primary_slot {
            ReferenceItemField::Known(primary) => primary == semantic_slot
                || matches!((primary, semantic_slot), (ReferenceEquipmentSlot::Weapon, ReferenceEquipmentSlot::Shield) | (ReferenceEquipmentSlot::Shield, ReferenceEquipmentSlot::Weapon)),
            _ => false,
        })))
}

/// ENERGY-RING-1: only a mana-shield marker on the active form of a worn ring counts.
/// Backpack custody, another slot and inactive forms never grant the contribution.
pub(in crate::gameplay_transport) fn equipped_mana_shield(
    equipment: &EquipmentSnapshot,
    content: &NativeGameplayState,
) -> Option<bool> {
    use crate::content::{ReferenceModifierParameter, ReferenceSkillModifierKind};
    if equipment.content_digest != content.source_digest() {
        return None;
    }
    let Some(item) = equipment.items.iter().find(|item| item.slot == 8) else {
        return Some(false);
    };
    if item.definition.family != "Item" {
        return None;
    }
    let active = super::super::timed_item_content::equipped_definition(content, &item.definition)?;
    let policy = content.item_policy(&active.production_key, &active.revision_ref)?;
    let semantics = &policy.record().semantics;
    if !in_own_slot(semantics, ReferenceEquipmentSlot::Ring) {
        return Some(false);
    }
    Some(
        matches!(&semantics.skill_modifiers, ReferenceItemField::Known(group)
        if matches!(&group.modifiers, ReferenceItemField::Known(modifiers)
            if modifiers.iter().any(|modifier| modifier.kind == ReferenceSkillModifierKind::ManaShield
                && modifier.parameter == ReferenceItemField::Known(ReferenceModifierParameter::Boolean(true))))),
    )
}

/// ARMOR-MITIGATION-1 (`Player::getArmor`): the armor of the items worn in their own head,
/// necklace, torso, legs, feet, ring and ammo slots; the hands and the container never count.
/// An item whose armor is not known contributes nothing; a negative armor value contributes
/// nothing. `None` for the same unreadable equipment [`equipment_effects`] refuses.
pub(in crate::gameplay_transport) fn equipped_armor(
    equipment: &EquipmentSnapshot,
    content: &NativeGameplayState,
) -> Option<u32> {
    if equipment.content_digest != content.source_digest() {
        return None;
    }
    equipment.items.iter().try_fold(0_u32, |total, item| {
        if item.definition.family != "Item" {
            return None;
        }
        let (slot, semantic_slot) = canonical_slot(item.slot)?;
        let policy = content.item_policy(
            &item.definition.production_key,
            &item.definition.revision_ref,
        )?;
        let semantics = &policy.record().semantics;
        let worn = !matches!(
            slot,
            EquipmentSlot::RightHand | EquipmentSlot::LeftHand | EquipmentSlot::Container
        ) && in_own_slot(semantics, semantic_slot);
        let armor = match &semantics.protection {
            ReferenceItemField::Known(protection) if worn => match protection.armor {
                ReferenceItemField::Known(points) => u32::try_from(points.0).unwrap_or(0),
                _ => 0,
            },
            _ => 0,
        };
        Some(total.saturating_add(armor))
    })
}

/// SHIELD-DEFENCE-1 (`Player::getShieldAndWeapon`, `Player::getDefense`): what the hands
/// defend with. `None` for the same unreadable equipment [`equipment_effects`] refuses;
/// `Some(None)` while a held item's weapon facts are not known, so the hands defend with nothing.
pub(in crate::gameplay_transport) fn equipped_defence(
    equipment: &EquipmentSnapshot,
    content: &NativeGameplayState,
) -> Option<Option<HandDefence>> {
    if equipment.content_digest != content.source_digest() {
        return None;
    }
    let constants = AttackConstants::checked_in()?;
    Some(hand_defence(content, constants, |slot| {
        equipment
            .items
            .iter()
            .find(|item| item.slot == slot && item.definition.family == "Item")
            .map(|item| {
                (
                    item.definition.production_key.as_str(),
                    item.definition.revision_ref.as_str(),
                )
            })
    }))
}

/// PARITY-EQUIP-MODIFIERS-1 (`Player::getSkillLevel` over `abilities.skills`): the critical hit
/// and leech skills of the items in their own slot, summed per skill and clamped to the skill
/// range. A timed item and the ammo slot contribute nothing, as for every EQUIP-0 ability. `None`
/// for the same unreadable equipment [`equipment_effects`] refuses.
pub(in crate::gameplay_transport) fn equipped_offence(
    equipment: &EquipmentSnapshot,
    content: &NativeGameplayState,
) -> Option<EquippedOffence> {
    if equipment.content_digest != content.source_digest() {
        return None;
    }
    let mut tiers = (0, 0);
    let mut sums = [0_i64; 7];
    let mut shots = std::collections::BTreeMap::<u16, i64>::new();
    for item in &equipment.items {
        if item.definition.family != "Item" {
            return None;
        }
        let (slot, semantic_slot) = canonical_slot(item.slot)?;
        let policy = content.item_policy(
            &item.definition.production_key,
            &item.definition.revision_ref,
        )?;
        let semantics = &policy.record().semantics;
        if item.tier > 0 {
            let facts = oteryn_gameplay_items::item_ref_admission::generation_item_facts(
                content,
                &item.definition,
            )
            .ok()?;
            crate::domain::forge::ForgeItemShape::from_content(
                &facts,
                policy.record().authoring.forge,
                crate::domain::forge::ItemTier::new(item.tier)?,
            )
            .ok()?;
        }

        if item.slot == 4 && in_own_slot(semantics, semantic_slot) {
            tiers.1 = item.tier;
        }
        if item.slot == 6
            && matches!(&semantics.weapon, ReferenceItemField::Known(weapon)
            if matches!(weapon.weapon_type, ReferenceItemField::Known(kind) if !matches!(kind, crate::content::ReferenceWeaponType::Shield)))
        {
            tiers.0 = item.tier;
        }
        if item_ability_profile(semantics).ok()?.timed
            || slot == EquipmentSlot::Ammo
            || !in_own_slot(semantics, semantic_slot)
        {
            continue;
        }
        let offence = item_offence_modifiers(semantics);
        for (sum, value) in sums.iter_mut().zip([
            offence.critical_chance,
            offence.critical_damage,
            offence.life_leech,
            offence.mana_leech,
            offence.cleave,
            offence.reflect_percent,
            offence.reflect_flat,
        ]) {
            *sum = sum.saturating_add(value);
        }
        for (range, damage) in offence.perfect_shot {
            let sum = shots.entry(range).or_default();
            *sum = sum.saturating_add(damage);
        }
    }
    let mut offence = EquippedOffence::from_sums(sums[0], sums[1], sums[2], sums[3]);
    offence.cleave = u64::try_from(sums[4].max(0)).ok()?;
    offence.reflect_percent = u64::try_from(sums[5].max(0)).ok()?;
    offence.reflect_flat = u32::try_from(sums[6].clamp(0, i64::from(u32::MAX))).ok()?;
    offence.perfect_shot = shots;
    offence.weapon_tier = tiers.0;
    offence.boots_tier = tiers.1;
    Some(offence)
}

/// The 413 equippable production profiles have known zero raw speed. The two potion profiles
/// (i266, i285) have unknown native attributes and are never equippable, so never contribute.
/// Preserve unavailable speed on contributors; the raw value never supplies a sum.
pub(in crate::gameplay_transport) fn speed_term(
    equipment: &EquipmentSnapshot,
    content: &NativeGameplayState,
) -> Option<i32> {
    i32::try_from(equipment_effects_inner(equipment, content, true)?.speed).ok()
}

/// The suppressions, worn armor, hands' defence, mana shield and offence one admission installs.
pub(in crate::gameplay_transport) type AdmissionEffects = (
    EquipmentEffects,
    u32,
    Option<HandDefence>,
    bool,
    EquippedOffence,
);

/// EQUIP-0 §4: read under the caller's spell lane, before its runtime lock.
impl super::super::ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn read_admission_effects(
        &self,
        session: crate::foundation::GameSessionId,
    ) -> Result<Option<AdmissionEffects>, ()> {
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return Ok(None);
        };
        let equipment = super::read_current_equipment(self, session)
            .await
            .ok_or(())?;
        let effects = equipment_effects(&equipment, native).ok_or(())?;
        let armor = equipped_armor(&equipment, native).ok_or(())?;
        let defence = equipped_defence(&equipment, native).ok_or(())?;
        let mana_shield = equipped_mana_shield(&equipment, native).ok_or(())?;
        let offence = equipped_offence(&equipment, native).ok_or(())?;
        Ok(Some((effects, armor, defence, mana_shield, offence)))
    }

    /// The caller holds the spell lane and runtime; lock only spell state here.
    pub(in crate::gameplay_transport) async fn install_admission_suppressions(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        actor: crate::foundation::ExactActorRef,
        session: crate::foundation::GameSessionId,
        effects: Option<AdmissionEffects>,
    ) -> Option<()> {
        if let Some((effects, armor, defence, mana_shield, offence)) = effects {
            let mut states = self.states.spell_states.lock().await;
            let state = states.get_mut(runtime, actor, session)?;
            state.install_equipped_physical_protection(physical_protection(&effects));
            install_suppressions(state, effects);
            state.install_equipped_armor(armor);
            state.install_equipped_defence(defence);
            state.install_equipped_mana_shield(mana_shield);
            state.install_equipped_offence(offence);
        }
        Some(())
    }
}

/// EQUIP-PROTECTION-1: the summed physical protection of `effects`, installed on admission and
/// committed moves; field-only protections never meet a physical hit.
pub(in crate::gameplay_transport) fn physical_protection(
    effects: &EquipmentEffects,
) -> Option<crate::content::ReferenceRationalPercent> {
    effects
        .protections
        .get(&crate::content::ReferenceResistanceKind::Physical)
        .copied()
}

/// EQUIP-0 §4: share the same suppression mapping on admission and committed moves.
pub(in crate::gameplay_transport) fn install_suppressions(
    state: &mut crate::spell::cast::PlayerSpellState,
    effects: EquipmentEffects,
) {
    use oteryn_gameplay_content::content::item_abilities::AbilitySuppression;
    use oteryn_gameplay_domain::domain::condition::{ConditionType, DotElement, StatusKind};
    state.conditions.set_suppressed(
        effects
            .suppressions
            .into_iter()
            .map(|kind| match kind {
                AbilitySuppression::Drown => ConditionType::DamageOverTime(DotElement::Drown),
                AbilitySuppression::Drunk => ConditionType::Status(StatusKind::Drunk),
            })
            .collect(),
    );
}

#[cfg(test)]
#[path = "equipment_effects_owner_tests.rs"]
mod tests;
