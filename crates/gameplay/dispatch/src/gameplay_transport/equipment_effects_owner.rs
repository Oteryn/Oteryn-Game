//! EQUIP-0 §3–§4: derive only from the equipment owner and the Channel's pinned content.

use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{ReferenceEquipmentSlot, ReferenceItemField};
use crate::durability::character_equipment::EquipmentSnapshot;
use oteryn_gameplay_combat_rules::attack::AttackConstants;
use oteryn_gameplay_combat_rules::resolution::player_defence::{HandDefence, hand_defence};
use oteryn_gameplay_content::content::item_abilities::item_ability_profile;
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
            if patterns.iter().any(|pattern| pattern.primary_slot == ReferenceItemField::Known(semantic_slot))))
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

/// The 413 equippable production profiles have known zero raw speed. The two potion profiles
/// (i266, i285) have unknown native attributes and are never equippable, so never contribute.
/// Preserve unavailable speed on contributors; the raw value never supplies a sum.
pub(in crate::gameplay_transport) fn speed_term(
    equipment: &EquipmentSnapshot,
    content: &NativeGameplayState,
) -> Option<i32> {
    i32::try_from(equipment_effects_inner(equipment, content, true)?.speed).ok()
}

/// The suppressions, worn armor and hands' defence one admission installs.
pub(in crate::gameplay_transport) type AdmissionEffects =
    (EquipmentEffects, u32, Option<HandDefence>);

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
        Ok(Some((effects, armor, defence)))
    }

    /// The caller holds the spell lane and runtime; lock only spell state here.
    pub(in crate::gameplay_transport) async fn install_admission_suppressions(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        actor: crate::foundation::ExactActorRef,
        session: crate::foundation::GameSessionId,
        effects: Option<AdmissionEffects>,
    ) -> Option<()> {
        if let Some((effects, armor, defence)) = effects {
            let mut states = self.states.spell_states.lock().await;
            let state = states.get_mut(runtime, actor, session)?;
            install_suppressions(state, effects);
            state.install_equipped_armor(armor);
            state.install_equipped_defence(defence);
        }
        Some(())
    }
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
