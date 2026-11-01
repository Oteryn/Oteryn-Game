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

/// Crystal201189f player.cpp:12772-12787 sums positive item Mantra on six body slots.
/// Ammo, hands and the container do not contribute to Ascetic's mantra total.
fn equipped_mantra(equipment: &EquipmentSnapshot, content: &NativeGameplayState) -> Option<u32> {
    use crate::content::{ReferenceModifierParameter, ReferenceSkillModifierKind};
    let mut total = 0_i64;
    for item in &equipment.items {
        if !matches!(item.slot, 1 | 2 | 3 | 4 | 7 | 8) {
            continue;
        }
        let policy = content.item_policy(
            &item.definition.production_key,
            &item.definition.revision_ref,
        )?;
        if let ReferenceItemField::Known(group) = &policy.record().semantics.skill_modifiers
            && let ReferenceItemField::Known(modifiers) = &group.modifiers
        {
            for modifier in modifiers {
                if modifier.kind == ReferenceSkillModifierKind::Mantra
                    && let ReferenceItemField::Known(ReferenceModifierParameter::SignedPoints(
                        points,
                    )) = modifier.parameter
                    && points.0 > 0
                {
                    total = total.checked_add(i64::from(points.0))?;
                }
            }
        }
    }
    u32::try_from(total.max(0)).ok()
}

pub(in crate::gameplay_transport) fn equipment_effects(
    equipment: &EquipmentSnapshot,
    content: &NativeGameplayState,
) -> Option<EquipmentEffects> {
    equipment_effects_inner(equipment, content, false, &[])
}

fn equipment_effects_inner(
    equipment: &EquipmentSnapshot,
    content: &NativeGameplayState,
    require_known_speed: bool,
    imbuements: &[crate::durability::imbuement::EquippedImbuement],
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
            let mut input = EquipmentEffectInput {
                slot,
                definition_key: (
                    &policy.record().production_definition.production_key,
                    &policy.record().production_definition.revision_ref,
                ),
                in_own_slot,
                profile: item_ability_profile(semantics).ok()?,
            };
            // I05: custody keeps a timed ring's inactive form; the worn view's speed and
            // suppression apply (time ring 3053 -> 3090 speed 30, Crystal items.xml:7381-7385;
            // dwarven ring 3097 -> 3099 suppressdrunk, items.xml:7477-7489).
            let worn =
                super::super::timed_item_content::equipped_definition(content, &item.definition)?;
            if worn != item.definition {
                use oteryn_gameplay_content::content::item_abilities::ItemAbility;
                let active = content.item_policy(&worn.production_key, &worn.revision_ref)?;
                let active = item_ability_profile(&active.record().semantics).ok()?;
                input
                    .profile
                    .abilities
                    .extend(active.abilities.into_iter().filter(|ability| {
                        matches!(ability, ItemAbility::Speed(_) | ItemAbility::Suppress(_))
                    }));
            }
            for installed in imbuements
                .iter()
                .filter(|b| b.item == item.item_instance_id && b.remaining_ms > 0)
            {
                use crate::content::ReferenceResistanceKind as Kind;
                use oteryn_gameplay_content::content::item_abilities::ItemAbility;
                // Durable categories are the native catalogue family minus one, not Canary's
                // XML category ids. Only the six admitted protection families contribute.
                let element = match installed.category {
                    2 => Kind::Death,
                    3 => Kind::Earth,
                    4 => Kind::Energy,
                    5 => Kind::Fire,
                    6 => Kind::Holy,
                    7 => Kind::Ice,
                    _ => continue,
                };
                let tier = usize::from(installed.tier.checked_sub(1)?);
                let amounts = if element == Kind::Death {
                    [2, 5, 10]
                } else {
                    [3, 8, 15]
                };
                input.profile.abilities.push(ItemAbility::Protection {
                    element,
                    percent: crate::content::ReferenceRationalPercent {
                        numerator: *amounts.get(tier)?,
                        denominator: 1,
                    },
                    field_only: false,
                });
            }
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

/// Read hand identities before the block-defence owner discards real-shield and
/// two-hand distinctions (Crystal player.cpp:3868-3879, player_wheel.cpp:4090-4115).
fn equipped_wheel_hands(
    equipment: &EquipmentSnapshot,
    content: &NativeGameplayState,
) -> Option<crate::spell::cast::WheelEquipment> {
    use crate::content::ReferenceWeaponType;
    use crate::spell::cast::{WheelEquipment, WheelHand};
    if equipment.content_digest != content.source_digest() {
        return None;
    }
    let hand = |slot| -> Option<Option<WheelHand>> {
        let Some(item) = equipment.items.iter().find(|item| item.slot == slot) else {
            return Some(None);
        };
        if item.definition.family != "Item" {
            return None;
        }
        let policy = content.item_policy(
            &item.definition.production_key,
            &item.definition.revision_ref,
        )?;
        let semantics = &policy.record().semantics;
        let quiver = crate::durability::item_loop_move::ItemLoopContent::is_quiver(
            &super::super::item_ref_admission::NativeItemWeights(content),
            &item.definition,
        );
        let weapon = match &semantics.weapon {
            ReferenceItemField::Known(weapon) => weapon,
            ReferenceItemField::NotApplicable => {
                return Some(Some(WheelHand {
                    secondary_shield: quiver,
                    ..Default::default()
                }));
            }
            _ if quiver => {
                return Some(Some(WheelHand {
                    secondary_shield: true,
                    ..Default::default()
                }));
            }
            _ => return None,
        };
        let (
            ReferenceItemField::Known(kind),
            ReferenceItemField::Known(defense),
            ReferenceItemField::Known(extra_defense),
        ) = (&weapon.weapon_type, &weapon.defense, &weapon.extra_defense)
        else {
            return None;
        };
        let two_handed = match &semantics.equipment {
            ReferenceItemField::Known(equipment) => match &equipment.patterns {
                ReferenceItemField::Known(patterns) => {
                    let mut two_handed = false;
                    for pattern in patterns {
                        match &pattern.additional_reserved_slots {
                            ReferenceItemField::Known(slots) => {
                                two_handed |= slots.iter().any(|slot| {
                                    matches!(
                                        slot,
                                        ReferenceEquipmentSlot::Weapon
                                            | ReferenceEquipmentSlot::Shield
                                    )
                                })
                            }
                            ReferenceItemField::NotApplicable => {}
                            _ => return None,
                        }
                    }
                    two_handed
                }
                _ => return None,
            },
            _ => return None,
        };
        let ammunition = match &weapon.ammunition {
            ReferenceItemField::Known(ammo) => Some(*ammo),
            ReferenceItemField::NotApplicable => None,
            _ if !matches!(kind, ReferenceWeaponType::Distance) => None,
            _ => return None,
        };
        Some(Some(WheelHand {
            defense: defense.0,
            extra_defense: extra_defense.0,
            real_shield: *kind == ReferenceWeaponType::Shield,
            secondary_shield: quiver || *kind == ReferenceWeaponType::Spellbook,
            two_handed,
            battle_melee: matches!(
                kind,
                ReferenceWeaponType::Club | ReferenceWeaponType::Sword | ReferenceWeaponType::Axe
            ),
            ammunition,
        }))
    };
    Some(WheelEquipment {
        weapon_hand: hand(5)?,
        shield_hand: hand(6)?,
        ..Default::default()
    })
}

/// PARITY-EQUIP-MODIFIERS-1 (`Player::getSkillLevel` over `abilities.skills`): the critical hit
/// and leech skills of the items in their own slot, summed per skill and clamped to the skill
/// range. A timed item and the ammo slot contribute nothing, as for every EQUIP-0 ability. `None`
/// for the same unreadable equipment [`equipment_effects`] refuses.
pub(in crate::gameplay_transport) fn equipped_offence(
    equipment: &EquipmentSnapshot,
    content: &NativeGameplayState,
) -> Option<EquippedOffence> {
    equipped_offence_inner(equipment, content, &[])
}

fn equipped_offence_inner(
    equipment: &EquipmentSnapshot,
    content: &NativeGameplayState,
    imbuements: &[crate::durability::imbuement::EquippedImbuement],
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
        for installed in imbuements
            .iter()
            .filter(|b| b.item == item.item_instance_id && b.remaining_ms > 0)
        {
            let tier = usize::from(installed.tier.checked_sub(1)?);
            let values = match installed.category {
                0 => [500, *[500, 1500, 4000].get(tier)?, 0, 0],
                10 => [0, 0, *[500, 1000, 2500].get(tier)?, 0],
                11 => [0, 0, 0, *[300, 500, 800].get(tier)?],
                _ => continue,
            };
            for (sum, value) in sums.iter_mut().zip(values) {
                *sum = sum.saturating_add(value);
            }
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
    i32::try_from(equipment_effects_inner(equipment, content, true, &[])?.speed).ok()
}

/// Equipment effects and the owner snapshot one admission installs.
pub(in crate::gameplay_transport) type AdmissionEffects = (
    EquipmentEffects,
    u32,
    Option<HandDefence>,
    bool,
    EquippedOffence,
    EquipmentSnapshot,
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
        let fence = self
            .current_quest_fence(session)
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        self.equipment_effects_from_snapshot(session, equipment, fence, native, None)
            .await
            .map(Some)
    }

    pub(in crate::gameplay_transport) async fn read_house_admission_effects(
        &self,
        actor: crate::foundation::ExactActorRef,
        session: crate::foundation::GameSessionId,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    ) -> Result<Option<AdmissionEffects>, ()> {
        let Some(native) = self.active_generation.and_then(|a| a.native_gameplay()) else {
            return Ok(None);
        };
        let equipment = super::read_current_house_equipment(self, actor, session, fence)
            .await
            .ok_or(())?;
        let fence = self
            .current_quest_fence(session)
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        self.equipment_effects_from_snapshot(
            session,
            equipment,
            fence,
            native,
            Some(*self.channel_id.as_bytes()),
        )
        .await
        .map(Some)
    }

    /// Only the retained native Death return calls this lifecycle reader.
    /// Its exact Channel successor may be Active or Reconnectable; the durable
    /// equipment owner independently re-proves its session, lease and scope.
    pub(in crate::gameplay_transport) async fn read_activity_death_effects(
        &self,
        expected: crate::durability::item_transfer::CurrentCharacterItemFence,
    ) -> Result<Option<AdmissionEffects>, ()> {
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return Ok(None);
        };
        if !matches!(
            expected.runtime_scope,
            crate::foundation::RuntimeScopeRefV1::Channel { .. }
        ) {
            return Err(());
        }
        let fence = self
            .current_quest_fence(expected.game_session_id)
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        if fence.item_fence() != expected {
            return Err(());
        }
        {
            let runtime = self.runtime.lock().await;
            runtime.owner_fence().map_err(|_| ())?;
            if runtime.binding().runtime_scope() != expected.runtime_scope
                || runtime.binding().scope_generation() != expected.scope_ownership_generation
                || runtime.content_pin().server_artifact_digest() != native.source_digest()
            {
                return Err(());
            }
        }
        let read = self
            .root
            .read_movement_equipment(
                self.character,
                self.holder,
                &expected,
                native.source_digest(),
            )
            .await
            .map_err(|_| ())?;
        if read.fence() != &expected || read.equipment().content_digest != native.source_digest() {
            return Err(());
        }
        self.equipment_effects_from_snapshot(
            expected.game_session_id,
            read.equipment().clone(),
            fence,
            native,
            None,
        )
        .await
        .map(Some)
    }

    async fn equipment_effects_from_snapshot(
        &self,
        session: crate::foundation::GameSessionId,
        equipment: EquipmentSnapshot,
        fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
        native: &NativeGameplayState,
        house_origin: Option<[u8; 16]>,
    ) -> Result<AdmissionEffects, ()> {
        self.load_proficiency_equipment(session, &equipment).await?;
        let mut imbuements = match house_origin {
            Some(origin) => {
                self.root
                    .read_house_worn_imbuements(self.character, self.holder, fence, origin)
                    .await
            }
            None => {
                self.root
                    .read_worn_imbuements(self.character, self.holder, fence)
                    .await
            }
        }
        .map_err(|_| ())?;
        if self
            .states
            .timed_items
            .imbuements
            .lock()
            .await
            .get(&session)
            .is_some_and(|clock| clock.pending.is_some())
        {
            imbuements.clear();
        }
        let effects = equipment_effects_inner(&equipment, native, false, &imbuements).ok_or(())?;
        let armor = equipped_armor(&equipment, native).ok_or(())?;
        let defence = equipped_defence(&equipment, native).ok_or(())?;
        let mana_shield = equipped_mana_shield(&equipment, native).ok_or(())?;
        let offence = equipped_offence_inner(&equipment, native, &imbuements).ok_or(())?;
        Ok((effects, armor, defence, mana_shield, offence, equipment))
    }

    /// The caller holds the spell lane and runtime; publish equipment before its effects.
    pub(in crate::gameplay_transport) async fn install_admission_suppressions(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        actor: crate::foundation::ExactActorRef,
        session: crate::foundation::GameSessionId,
        effects: Option<AdmissionEffects>,
    ) -> Option<()> {
        if let Some((effects, armor, defence, mana_shield, offence, equipment)) = effects {
            self.install_proficiency_equipment(runtime, actor, session, &equipment)
                .await?;
            let mantra = equipped_mantra(&equipment, self.active_generation?.native_gameplay()?)?;
            let wheel_hands =
                equipped_wheel_hands(&equipment, self.active_generation?.native_gameplay()?);
            let (proficiency_weapon, proficiency_defense, proficiency_shield_modifier) =
                self.equipped_proficiency_defense(session, &equipment)?;
            let mut states = self.states.spell_states.lock().await;
            let state = states.get_mut(runtime, actor, session)?;
            state.install_wheel_equipment(wheel_hands);
            state.install_wheel_proficiency(
                proficiency_weapon.as_deref(),
                proficiency_defense,
                proficiency_shield_modifier,
            );
            install_prepared_effects(state, effects, armor, defence, mana_shield, offence, mantra);
        }
        Some(())
    }
    /// House admission reuses the exact equipment interpretation after explicit
    /// current custody proof. Generic Channel projection remains closed.
    pub(in crate::gameplay_transport) async fn install_house_admission_suppressions(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        actor: crate::foundation::ExactActorRef,
        session: crate::foundation::GameSessionId,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
        effects: Option<AdmissionEffects>,
    ) -> Option<()> {
        if let Some((effects, armor, defence, mana_shield, offence, equipment)) = effects {
            self.install_house_proficiency_equipment(runtime, actor, session, fence, &equipment)
                .await?;
            let mantra = equipped_mantra(&equipment, self.active_generation?.native_gameplay()?)?;
            let wheel_hands =
                equipped_wheel_hands(&equipment, self.active_generation?.native_gameplay()?);
            let (proficiency_weapon, proficiency_defense, proficiency_shield_modifier) =
                self.equipped_proficiency_defense(session, &equipment)?;
            let mut states = self.states.spell_states.lock().await;
            let state = states.house_state_mut(runtime, actor, session, fence)?;
            state.install_wheel_equipment(wheel_hands);
            state.install_wheel_proficiency(
                proficiency_weapon.as_deref(),
                proficiency_defense,
                proficiency_shield_modifier,
            );
            install_prepared_effects(state, effects, armor, defence, mana_shield, offence, mantra);
        }
        Some(())
    }
}

/// One native equipment projection shared by both admitted scopes.
fn install_prepared_effects(
    state: &mut crate::spell::cast::PlayerSpellState,
    effects: EquipmentEffects,
    armor: u32,
    defence: Option<HandDefence>,
    mana_shield: bool,
    offence: EquippedOffence,
    mantra: u32,
) {
    state.install_equipped_mantra(mantra);
    state.install_equipped_physical_protection(physical_protection(&effects));
    use crate::content::ReferenceResistanceKind as Kind;
    for kind in [
        Kind::Ice,
        Kind::Energy,
        Kind::Fire,
        Kind::Holy,
        Kind::Earth,
        Kind::Death,
    ] {
        state.install_equipped_element_protection(kind, effects.protections.get(&kind).copied());
    }
    install_suppressions(state, effects);
    state.install_equipped_armor(armor);
    state.install_equipped_defence(defence);
    state.install_equipped_mana_shield(mana_shield);
    state.install_equipped_offence(offence);
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

#[path = "equipment_field.rs"]
mod field;
