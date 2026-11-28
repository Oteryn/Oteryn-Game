//! Charge-only resistance for the direct swing owner. Timed item expiry owns custody.
use super::{canonical_slot, in_own_slot};
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{ReferenceRationalPercent, ReferenceResistanceKind as Kind};
use crate::durability::character_equipment::EquipmentSnapshot;
use crate::durability::item_timed_runtime::{TimedOperation, TimedRequest};
use crate::durability::item_timed_state::TimedDefinitionFacts;
use crate::player_lethal::PlayerHitKind;
use oteryn_gameplay_content::content::item_abilities::{ItemAbility, item_ability_profile};
use oteryn_gameplay_items::equipment_effects::{EquipmentEffectInput, derive_equipment_effects};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub(in crate::gameplay_transport) struct CombatCharges {
    items: BTreeMap<[u8; 16], (TimedDefinitionFacts, BTreeSet<Kind>)>,
    protections: BTreeMap<Kind, ReferenceRationalPercent>,
}

impl CombatCharges {
    /// The PvP configuration only admits the newly supported charge-based protection.
    /// An unrelated equipped item still needs the separate PvP equipment qualification.
    pub(in crate::gameplay_transport) fn qualify(
        equipment: &EquipmentSnapshot,
        content: &NativeGameplayState,
    ) -> Option<Self> {
        if equipment.content_digest != content.source_digest() {
            return None;
        }
        let mut items = BTreeMap::new();
        let mut inputs = Vec::new();
        for item in &equipment.items {
            let (slot, semantic_slot) = canonical_slot(item.slot)?;
            if !matches!(item.slot, 7 | 8) || item.tier != 0 || item.definition.family != "Item" {
                return None;
            }
            let policy = content.item_policy(
                &item.definition.production_key,
                &item.definition.revision_ref,
            )?;
            let semantics = &policy.record().semantics;
            let (definition, target) =
                crate::gameplay_transport::timed_item_content::facts(content, &item.definition)?;
            if definition.full.charges().is_none()
                || definition.full.remaining_ms().is_some()
                || target.is_some()
                || !in_own_slot(semantics, semantic_slot)
            {
                return None;
            }
            let mut profile = item_ability_profile(semantics).ok()?;
            let mut kinds = BTreeSet::new();
            for ability in &profile.abilities {
                let ItemAbility::Protection {
                    element,
                    percent,
                    field_only,
                } = ability
                else {
                    return None;
                };
                if *field_only {
                    continue;
                }
                if percent.numerator <= 0 {
                    return None;
                }
                kinds.insert(*element);
            }
            if kinds.is_empty() {
                return None;
            }
            items.insert(item.item_instance_id, (definition, kinds));
            profile.timed = false;
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
        Some(Self {
            items,
            protections: derive_equipment_effects(inputs)?.protections,
        })
    }

    pub(in crate::gameplay_transport) fn mitigate(
        &self,
        spends: &mut BTreeSet<[u8; 16]>,
        magnitude: u32,
        kind: PlayerHitKind,
    ) -> Option<u32> {
        let element = match kind {
            PlayerHitKind::Physical => Kind::Physical,
            PlayerHitKind::Death => Kind::Death,
            PlayerHitKind::Earth => Kind::Earth,
            PlayerHitKind::Energy => Kind::Energy,
            PlayerHitKind::Fire => Kind::Fire,
            PlayerHitKind::Ice => Kind::Ice,
            PlayerHitKind::Holy => Kind::Holy,
            _ => return Some(magnitude),
        };
        let Some(percent) = self.protections.get(&element).filter(|_| magnitude > 0) else {
            return Some(magnitude);
        };
        spends.extend(
            self.items
                .iter()
                .filter_map(|(item, (_, kinds))| kinds.contains(&element).then_some(*item)),
        );
        // Global protective amulets/rings round the reduction up to an entire hit point.
        let numerator = u128::try_from(percent.numerator).ok()?;
        let denominator = u128::from(percent.denominator) * 100;
        let absorbed = (u128::from(magnitude) * numerator).div_ceil(denominator);
        magnitude.checked_sub(u32::try_from(absorbed).ok()?)
    }

    pub(in crate::gameplay_transport) async fn spend(
        &self,
        tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
        authority: &crate::durability::character_equipment::EquipmentAuthority,
        hit: (crate::foundation::CommandRef, u16, [u8; 32]),
        content_digest: [u8; 32],
        spends: &BTreeSet<[u8; 16]>,
    ) -> Result<bool, crate::durability::DurabilityError> {
        use crate::durability::{DurabilityError, item_timed_runtime, item_timed_state};
        let mut retired = false;
        for item in spends {
            let (definition, _) = self.items.get(item).ok_or(DurabilityError::Unavailable)?;
            let state = item_timed_state::load_timed_state(tx, item)
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
            let (transaction, event, occurred): (Vec<u8>, Vec<u8>, i64) = sqlx::query_as(
                "SELECT uuid_send(game_character_uuid_v7()),uuid_send(game_character_uuid_v7()),floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
            )
            .fetch_one(&mut ***tx).await?;
            let request = TimedRequest {
                transaction_id: transaction
                    .try_into()
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                event_id: event
                    .try_into()
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                content_digest,
                binding: hit.2,
                item: *item,
                expected_revision: state.map_or(0, |state| state.revision),
                definition: definition.clone(),
                target: None,
                operation: TimedOperation::Use { elapsed_ms: 0 },
                occurred_at_unix_ms: occurred,
            };
            retired |= item_timed_runtime::consume_combat_protection_in_transaction(
                tx, authority, hit.0, hit.1, &request,
            )
            .await
            .map_err(|_| DurabilityError::Unavailable)?;
        }
        Ok(retired)
    }
}
