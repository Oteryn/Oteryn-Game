//! Source stations join the existing craft debit, terrain CAS and World contribution owner.
use super::*;
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use oteryn_gameplay_world_change::variants::{self, Definition, UseStation};
use sha2::{Digest, Sha256};

pub(super) const RECIPE: &str = "oteryn:world-change.contribution";

pub(super) struct StationUse {
    pub(super) definition: &'static Definition,
    pub(super) station: &'static UseStation,
}

pub(super) fn map_prefix_matches(placement: &[u8], revision: &str) -> bool {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let Some(hex) = revision.strip_prefix("sha256:") else {
        return false;
    };
    placement.len() == 40
        && hex.len() == 64
        && placement[..32]
            .iter()
            .zip(hex.as_bytes().as_chunks::<2>().0)
            .all(|(byte, pair)| *pair == [HEX[usize::from(byte >> 4)], HEX[usize::from(byte & 15)]])
}

pub(super) fn resolve(
    request: &ItemCraftRequest,
    source: &TypedDefinitionRef,
) -> Option<StationUse> {
    let gathering = request.gathering.as_ref()?;
    let position = variants::station_position(&gathering.mutation.placement)?;
    let (definition, station) = variants::station_at(
        position,
        u16::try_from(gathering.mutation.before).ok()?,
        &source.production_key,
    )?;
    if request.tool.is_some()
        || request.ingredient.is_some()
        || request.enchanting.is_some()
        || request.station.is_some()
        || source.family != "Item"
        || gathering.mutation.tool != request.source
        || gathering.mutation.tool_key != source.production_key
        || gathering.mutation.tool_revision != source.revision_ref
        || !map_prefix_matches(&gathering.mutation.placement, &gathering.scope.map_revision)
        || !gathering.attempt.consumes_bait
        || gathering.attempt.after != station.after
        || gathering.attempt.yield_item != station.yield_item
        || gathering.mutation.after != station.after.map_or(0, u32::from)
        || !gathering.mutation.decay.is_empty()
    {
        return None;
    }
    Some(StationUse {
        definition,
        station,
    })
}

/// Shared with migration1101: UUID bytes followed by canonical decimal command bytes.
fn operation_id(command: CommandRef) -> [u8; 16] {
    let digest = Sha256::new()
        .chain_update(b"oteryn.world-change.station.v1\0")
        .chain_update(command.game_session_id().as_bytes())
        .chain_update(command.command_id().get().to_string().as_bytes())
        .finalize();
    let mut operation = [0; 16];
    operation.copy_from_slice(&digest[..16]);
    operation
}

/// Called after the terrain CAS and before the craft record, in the SAME transaction.
pub(super) async fn contribute(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    recovery: &CharacterRecoveryFenceV1,
    fence: CurrentCharacterItemFence,
    request: &ItemCraftRequest,
    station: &StationUse,
) -> Result<bool, DurabilityError> {
    let Some(gathering) = request.gathering.as_ref() else {
        return Ok(false);
    };
    let Ok(epoch) = u64::try_from(gathering.scope.reset_epoch) else {
        return Ok(false);
    };
    let (world, _) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let Some(day) = super::super::world_event::changes::load(tx, world, epoch).await? else {
        return Ok(false);
    };
    let selected = day
        .variants
        .variant(&station.definition.key)
        .and_then(|index| station.definition.variants.get(index));
    if !selected.is_some_and(|variant| variant_contains_station(variant, station.station)) {
        return Ok(false);
    }
    let result = super::super::world_event::changes::contribute(
        tx,
        recovery,
        world,
        epoch,
        operation_id(request.command),
        station.station.resource,
        1,
    )
    .await?;
    // An exact craft replay returned before live custody/CAS. An orphan contribution
    // replay here must not authorize another debit; the whole transaction rolls back.
    Ok(matches!(
        result,
        super::super::world_event::changes::ContributionWrite::Applied(_)
    ))
}

fn variant_contains_station(variant: &variants::Variant, station: &variants::UseStation) -> bool {
    variant.tiles.iter().any(|tile| {
        tile.position == station.position
            && tile
                .entries
                .iter()
                .any(|entry| entry.0 == station.appearance)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn station_requires_exact_current_layer_membership() -> Result<(), Box<dyn std::error::Error>> {
        let definition = variants::definitions()?
            .iter()
            .find(|d| d.key == "awash")
            .ok_or("Awash")?;
        let station = definition.stations.first().ok_or("source pump")?;
        for variant in &definition.variants {
            assert!(variant_contains_station(variant, station));
        }
        let mut absent: variants::Variant = serde_json::from_value(
            serde_json::json!({"key":"absent","weight":1,"layers":[],"tiles":[],"tags":[],"spawns":[],"npcs":[],"portals":[]}),
        )?;
        assert!(!variant_contains_station(&absent, station));
        absent.tiles.push(serde_json::from_value(
            serde_json::json!({"position":station.position,"entries":[[station.appearance+1,0,0]]}),
        )?);
        assert!(!variant_contains_station(&absent, station));
        Ok(())
    }

    #[test]
    fn station_map_prefix_is_exact_and_canonical() {
        let placement = vec![0; 40];
        assert!(map_prefix_matches(
            &placement,
            &format!("sha256:{}", "00".repeat(32))
        ));
        assert!(!map_prefix_matches(
            &placement[..39],
            &format!("sha256:{}", "00".repeat(32))
        ));
        assert!(!map_prefix_matches(
            &placement,
            &format!("sha256:{}", "01".repeat(32))
        ));
        assert!(!map_prefix_matches(&placement, &"00".repeat(32)));
    }
}
