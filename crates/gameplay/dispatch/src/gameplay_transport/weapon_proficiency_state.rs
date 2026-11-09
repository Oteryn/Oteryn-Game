//! Bounded current-owner projection for the negotiated proficiency shaping selector.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use crate::durability::character_proficiency::ProficiencyDefinitions;
use oteryn_gameplay_content::content::{
    ImportedShapingEntry, ImportedShapingUnit, import_committed_proficiency_shaping,
};
use oteryn_protocol_oteryn::weapon_proficiency_state::{self as wire, Slot, State};
pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || entry.id != wire::CAPABILITY_WEAPON_PROFICIENCY_STATE_V1)
        .collect()
}
macro_rules! authority_methods {
    () => {
        fn weapon_proficiency_state_available(&self) -> bool {
            false
        }
        fn weapon_proficiency_state_query(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _definition: u32,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn weapon_proficiency_state_available(&self) -> bool {
            self.proficiency_catalogue.is_some()
                && self
                    .active_generation
                    .and_then(|active| active.native_gameplay())
                    .is_some()
        }
        async fn weapon_proficiency_state_query(
            &self,
            admitted: $crate::gameplay_transport::AdmittedSession,
            definition: u32,
        ) -> Option<Vec<u8>> {
            $crate::gameplay_transport::weapon_proficiency_state::execute(
                self, admitted, definition,
            )
            .await
        }
    };
}
pub(super) use composed_methods;
pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    definition: u32,
) -> Option<Vec<u8>> {
    if definition == 0
        || !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_WEAPON_PROFICIENCY_STATE_V1)
    {
        return None;
    }
    let fence = admitted.item_fence?;
    if fence.game_session_id != admitted.game_session_id
        || fence.connection_generation.get() != admitted.continuity.connection_generation
    {
        return None;
    }
    let _permit = owner.spell_lane_permit().await?;
    let catalogue = owner.proficiency_catalogue.as_ref()?;
    let resolved = {
        let runtime = owner.runtime.lock().await;
        if !super::battle_instinct_status::actor_is_current(&runtime, admitted)
            || runtime.binding().runtime_scope() != fence.runtime_scope
            || runtime.binding().scope_generation() != fence.scope_ownership_generation
        {
            return None;
        }
        let index = super::item_ref_admission::pinned_index(&runtime, owner.active_generation)?;
        let (key, _) = index.definition_at(std::num::NonZeroU32::new(definition)?)?;
        catalogue.resolve_weapon(key)?
    };
    let observation = owner
        .root
        .read_weapon_proficiency_state(
            owner.character,
            owner.holder,
            fence,
            &resolved.canonical_item_key,
        )
        .await
        .ok()?;
    {
        let runtime = owner.runtime.lock().await;
        if !super::battle_instinct_status::actor_is_current(&runtime, admitted)
            || runtime.binding().runtime_scope() != fence.runtime_scope
            || runtime.binding().scope_generation() != fence.scope_ownership_generation
        {
            return None;
        }
    }
    let shaping = import_committed_proficiency_shaping().ok()?;
    let pool = shaping
        .pools
        .get(shaping.definition_vocations.get(&resolved.definition_key)?)?;
    let label = |index: u8, rank: u8| shape_label(pool.get(usize::from(index))?, rank);
    let mut slots = [None, None];
    for (target, row) in slots.iter_mut().zip(observation.slots) {
        if let Some(row) = row {
            let suffix = resolved
                .definition_key
                .strip_prefix("oteryn:proficiency.")?;
            if row.shaping_key != format!("oteryn:proficiency-shaping.{suffix}")
                || row.shaping_revision != shaping.revision
            {
                return None;
            }
            let offers = match row.pending_offer {
                Some(offer) => offer
                    .into_iter()
                    .map(|index| label(index, row.rank))
                    .collect::<Option<Vec<_>>>()?,
                None => Vec::new(),
            };
            *target = Some(Slot {
                level: row.level,
                rank: row.rank,
                current: label(row.entry_index, row.rank)?,
                next: label(row.entry_index, row.rank.saturating_add(1).min(10))?,
                maximum: label(row.entry_index, 10)?,
                offers,
            });
        }
    }
    State {
        definition,
        revision: observation.revision,
        slots,
        modify_dust: costs(shaping.modify_dust)?,
        rank_dust: costs(shaping.rank_step_dust)?,
        reshape_dust: u32::try_from(shaping.reshape_offer_dust).ok()?,
        clear_dust: u32::try_from(shaping.clear_dust).ok()?,
        orbs: u32::from(shaping.orb_count),
    }
    .encode()
}
fn costs<const N: usize>(values: [u64; N]) -> Option<[u32; N]> {
    values
        .into_iter()
        .map(u32::try_from)
        .collect::<Result<Vec<_>, _>>()
        .ok()?
        .try_into()
        .ok()
}

fn shape_label(entry: &ImportedShapingEntry, rank: u8) -> Option<String> {
    let value = entry.rank_values.get(usize::from(rank))?;
    let unit = match entry.unit {
        ImportedShapingUnit::Points => "points",
        ImportedShapingUnit::Percent => "%",
    };
    Some(format!(
        "{}: {}/{} {}",
        entry.identity.replace('_', " "),
        value.numerator,
        value.denominator,
        unit
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proficiency_state_real_shaping_catalogue_fits_wire_bound()
    -> Result<(), Box<dyn std::error::Error>> {
        let data = import_committed_proficiency_shaping().map_err(|error| format!("{error:?}"))?;
        let mut longest = String::new();
        for pool in data.pools.values() {
            for entry in pool {
                for rank in 0..=10 {
                    let label = shape_label(entry, rank).ok_or("rank label")?;
                    assert!(!label.contains('\0'));
                    assert!(label.len() <= 256);
                    if label.len() > longest.len() {
                        longest = label;
                    }
                }
            }
        }
        assert!(!longest.is_empty());
        let slot = Slot {
            level: 6,
            rank: 10,
            current: longest.clone(),
            next: longest.clone(),
            maximum: longest.clone(),
            offers: vec![longest; 3],
        };
        let state = State {
            definition: u32::MAX,
            revision: u64::MAX,
            slots: [Some(slot.clone()), Some(slot)],
            modify_dust: costs(data.modify_dust).ok_or("modify costs")?,
            rank_dust: costs(data.rank_step_dust).ok_or("rank costs")?,
            reshape_dust: u32::try_from(data.reshape_offer_dust)?,
            clear_dust: u32::try_from(data.clear_dust)?,
            orbs: u32::from(data.orb_count),
        };
        let bytes = state
            .encode()
            .ok_or("worst-case catalogue projection exceeds wire bound")?;
        assert!(bytes.len() <= wire::MAX_REPLY_BYTES);
        assert_eq!(State::decode(&bytes), Some(state));
        Ok(())
    }
}
