//! Field incoming effects use current equipment and the ordinary timed Item writer.
use super::{canonical_slot, in_own_slot};
use crate::content::{
    ReferenceItemField as F, ReferenceModifierParameter, ReferenceResistanceKind,
    ReferenceSkillModifierKind,
};
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::spell_item_transaction::StandingTileRead;
use crate::durability::{
    DurabilityError,
    item_timed_runtime::{TimedOperation, TimedRequest},
    item_timed_state,
};
use oteryn_gameplay_content::content::item_abilities::{ItemAbility, item_ability_profile};
use oteryn_gameplay_domain::domain::{condition::DotElement, death::EquipmentSlot};
use oteryn_gameplay_items::equipment_effects::{
    EquipmentEffectInput, apply_protection, derive_equipment_effects,
};

impl crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_> {
    /// Called only after harm legality, before mana shield and HP, in the field's transaction.
    pub(in crate::gameplay_transport) async fn equipped_field_damage(
        &self,
        tx: &mut Type2Transaction<'_>,
        read: &StandingTileRead,
        element: DotElement,
        damage: u32,
        initial: bool,
        cycle: [u8; 32],
    ) -> Result<u32, DurabilityError> {
        let unavailable = || DurabilityError::Unavailable;
        let content = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .ok_or_else(unavailable)?;
        let equipment =
            crate::durability::character_equipment::read_field_equipment_in_transaction(
                tx,
                read,
                self.root,
                self.character,
                self.holder,
            )
            .await
            .map_err(|_| unavailable())?;
        let element = match element {
            DotElement::Fire => ReferenceResistanceKind::Fire,
            DotElement::Energy => ReferenceResistanceKind::Energy,
            DotElement::Poison => ReferenceResistanceKind::Earth,
            _ => return Err(unavailable()),
        };
        let mut inputs = Vec::new();
        let mut charged = Vec::new();
        let mut mantra = 0_i64;
        let mut worn = equipment.items.iter().collect::<Vec<_>>();
        worn.sort_by_key(|item| canonical_slot(item.slot).map(|(slot, _)| slot));
        for item in worn {
            let (slot, semantic_slot) = canonical_slot(item.slot).ok_or_else(unavailable)?;
            let definition = crate::gameplay_transport::timed_item_content::equipped_definition(
                content,
                &item.definition,
            )
            .ok_or_else(unavailable)?;
            let policy = content
                .item_policy(&definition.production_key, &definition.revision_ref)
                .ok_or_else(unavailable)?;
            let semantics = &policy.record().semantics;
            // Mantra reads the seven body/Extra slots, independently of the
            // six basic EQUIP abilities (Canary Player::getMantra).
            if matches!(
                slot,
                EquipmentSlot::Head
                    | EquipmentSlot::Necklace
                    | EquipmentSlot::Armor
                    | EquipmentSlot::Legs
                    | EquipmentSlot::Feet
                    | EquipmentSlot::Ring
                    | EquipmentSlot::Ammo
            ) && let F::Known(group) = &semantics.skill_modifiers
                && let F::Known(modifiers) = &group.modifiers
            {
                for modifier in modifiers {
                    if modifier.kind == ReferenceSkillModifierKind::Mantra
                        && let F::Known(ReferenceModifierParameter::SignedPoints(points)) =
                            modifier.parameter
                    {
                        mantra = mantra
                            .checked_add(i64::from(points.0))
                            .ok_or_else(unavailable)?;
                    }
                }
            }
            if slot == EquipmentSlot::Ammo || !in_own_slot(semantics, semantic_slot) {
                continue;
            }
            let mut profile = item_ability_profile(semantics).map_err(|_| unavailable())?;
            let timed = if profile.timed {
                let (facts, target) =
                    crate::gameplay_transport::timed_item_content::facts(content, &item.definition)
                        .ok_or_else(unavailable)?;
                let state = item_timed_state::load_timed_state(tx, &item.item_instance_id)
                    .await
                    .map_err(|_| unavailable())?;
                let values = state.map_or(Some(facts.full), |state| state.values);
                let Some(values) = values else { continue };
                if state.is_some_and(|state| state.deadline_at.is_some()) {
                    return Err(unavailable());
                }
                if let Some(remaining) = values.remaining_ms() {
                    let clocks = self.states.timed_items.sessions.lock().await;
                    let lane = clocks
                        .get(&read.fence().game_session_id)
                        .and_then(|clock| clock.items.get(&item.item_instance_id));
                    let elapsed = lane.map_or(0, |lane| {
                        u64::try_from(self.clock_origin.elapsed().as_millis())
                            .unwrap_or(u64::MAX)
                            .saturating_sub(lane.at_ms)
                    });
                    if lane.is_some_and(|lane| lane.pending.is_some()) {
                        return Err(unavailable());
                    }
                    if elapsed >= remaining {
                        continue;
                    }
                }
                profile.timed = false;
                values
                    .charges()
                    .map(|_| (facts, target, state.map_or(0, |state| state.revision)))
            } else {
                None
            };
            // Both general and field-only protection join the same element sum.
            profile.abilities.retain_mut(|ability| match ability {
                ItemAbility::Protection {
                    element: kind,
                    field_only,
                    ..
                } if *kind == element => {
                    *field_only = false;
                    true
                }
                _ => false,
            });
            if let Some((facts, target, revision)) = timed {
                let own = derive_equipment_effects(vec![EquipmentEffectInput {
                    slot,
                    definition_key: (&definition.production_key, &definition.revision_ref),
                    in_own_slot: true,
                    profile: profile.clone(),
                }])
                .ok_or_else(unavailable)?;
                if let Some(percent) = own.protections.get(&element) {
                    charged.push((item.item_instance_id, facts, target, revision, *percent));
                }
            }
            inputs.push(EquipmentEffectInput {
                slot,
                definition_key: (
                    &item.definition.production_key,
                    &item.definition.revision_ref,
                ),
                in_own_slot: true,
                profile,
            });
        }
        // Canary combatBlockHit applies Mantra to the initial hit, before blockHit's
        // equipment absorption; subsequent condition pulses bypass Mantra.
        let damage = i64::from(damage)
            .saturating_sub(if initial { mantra.max(0) } else { 0 })
            .max(0);
        let effects = derive_equipment_effects(inputs).ok_or_else(unavailable)?;
        let reduced = effects
            .protections
            .get(&element)
            .map_or(Some(damage), |percent| apply_protection(damage, *percent))
            .ok_or_else(unavailable)?;
        for (item, definition, target, expected_revision, percent) in charged {
            if apply_protection(damage, percent).is_none_or(|after| after >= damage) {
                continue;
            }
            let (transaction, event, occurred): (Vec<u8>, Vec<u8>, i64) = sqlx::query_as("SELECT uuid_send(game_character_uuid_v7()),uuid_send(game_character_uuid_v7()),floor(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut ***tx).await?;
            let request = TimedRequest {
                transaction_id: transaction.try_into().map_err(|_| unavailable())?,
                event_id: event.try_into().map_err(|_| unavailable())?,
                content_digest: read.content_digest(),
                binding: [0; 32],
                item,
                expected_revision,
                definition,
                target,
                operation: TimedOperation::Use { elapsed_ms: 0 },
                occurred_at_unix_ms: occurred,
            };
            let result =
                crate::durability::item_timed_runtime::consume_field_protection_in_transaction(
                    tx,
                    read,
                    equipment.character_revision,
                    cycle,
                    &request,
                )
                .await
                .map_err(|_| unavailable())?;
            if result.disposition != oteryn_protocol_oteryn::timed_item::TimedDisposition::Ok {
                return Err(unavailable());
            }
        }
        u32::try_from(reduced).map_err(|_| unavailable())
    }
}
