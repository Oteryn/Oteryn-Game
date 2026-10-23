//! EQUIP-0 §3–§4: derive only from the equipment owner and the Channel's pinned content.

use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{ReferenceEquipmentSlot, ReferenceItemField};
use crate::durability::character_equipment::EquipmentSnapshot;
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
    let inputs = equipment.items.iter().map(|item| {
        if item.definition.family != "Item" {
            return None;
        }
        let (slot, semantic_slot) = canonical_slot(item.slot)?;
        let policy = content.item_policy(
            &item.definition.production_key,
            &item.definition.revision_ref,
        )?;
        let semantics = &policy.record().semantics;
        let in_own_slot = matches!(&semantics.equipment, ReferenceItemField::Known(group)
            if matches!(&group.patterns, ReferenceItemField::Known(patterns)
                if patterns.iter().any(|pattern| pattern.primary_slot == ReferenceItemField::Known(semantic_slot))));
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
    }).collect::<Option<Vec<_>>>()?;
    derive_equipment_effects(inputs)
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

/// EQUIP-0 §4: read under the caller's spell lane, before its runtime lock.
impl super::super::ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn read_admission_effects(
        &self,
        session: crate::foundation::GameSessionId,
    ) -> Result<Option<EquipmentEffects>, ()> {
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return Ok(None);
        };
        let equipment = super::read_current_equipment(self, session)
            .await
            .ok_or(())?;
        equipment_effects(&equipment, native).map(Some).ok_or(())
    }

    /// The caller holds the spell lane and runtime; lock only spell state here.
    pub(in crate::gameplay_transport) async fn install_admission_suppressions(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        actor: crate::foundation::ExactActorRef,
        session: crate::foundation::GameSessionId,
        effects: Option<EquipmentEffects>,
    ) -> Option<()> {
        if let Some(effects) = effects {
            let mut states = self.states.spell_states.lock().await;
            let state = states.get_mut(runtime, actor, session)?;
            install_suppressions(state, effects);
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
