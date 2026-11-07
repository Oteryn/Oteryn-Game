//! Source stations join the existing craft debit, terrain CAS and World contribution owner.
use super::*;
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use oteryn_gameplay_world_change::variants::{self, Definition, UseStation};
use sha2::{Digest, Sha256};

pub(super) const RECIPE: &str = "oteryn:world-change.contribution";

pub(super) const REFILL: &str = "oteryn:world-change.refill";
pub const FILLED_BUCKET: u16 = 12722;
pub const EMPTY_BUCKET: u16 = 12819;
pub const BOG: u16 = 12721;
pub const FILLED_LIFETIME_MS: u64 = 3_600_000;
pub const WOLF: &str = "oteryn:creature.thornfire_wolf";
pub const WOLF_PADS: [crate::foundation::MovementLocalPosition; 4] = [
    crate::foundation::MovementLocalPosition {
        x: 33091,
        y: 32149,
        floor: 7,
    },
    crate::foundation::MovementLocalPosition {
        x: 33079,
        y: 32143,
        floor: 7,
    },
    crate::foundation::MovementLocalPosition {
        x: 33104,
        y: 32160,
        floor: 7,
    },
    crate::foundation::MovementLocalPosition {
        x: 33095,
        y: 32158,
        floor: 7,
    },
];
const RESTORE_OFFSETS: [u32; 2] = [28_800_000, 57_600_000];
const FLAMES: [u16; 13] = [
    232, 5061, 5062, 5063, 5064, 5065, 5066, 5067, 6288, 6289, 7803, 19096, 32021,
];

/// Existing private carried-craft entropy, with bounded rejection to keep100
/// source outcomes unbiased. Exhaustion fails the whole use before any debit.
pub fn source_station_draw(
    session: [u8; 16],
    command: u64,
    source: [u8; 16],
    purpose: u8,
    upper: u64,
) -> Option<u64> {
    if upper == 0 {
        return None;
    }
    let seed = reward_bag_draw(session, command, source);
    let ceiling = u64::MAX - (u64::MAX % upper);
    for nonce in 0..16_u8 {
        let digest = Sha256::new()
            .chain_update(b"oteryn.world-change.station-draw.v1\0")
            .chain_update(seed.to_be_bytes())
            .chain_update([purpose, nonce])
            .finalize();
        let value = u64::from_be_bytes(digest[..8].try_into().ok()?);
        if value < ceiling {
            return Some(value % upper);
        }
    }
    None
}
pub fn source_station_wolves(request: &ItemCraftRequest) -> Option<bool> {
    Some(
        source_station_draw(
            *request.command.game_session_id().as_bytes(),
            request.command.command_id().get(),
            request.source,
            0,
            100,
        )? < 10,
    )
}
pub fn source_station_actor_binding(request: &ItemCraftRequest) -> Option<[u8; 32]> {
    let gathering = request.gathering.as_ref()?;
    Some(
        Sha256::new()
            .chain_update(b"oteryn.world-change.station-actors.v1\0")
            .chain_update(request.command.game_session_id().as_bytes())
            .chain_update(request.command.command_id().get().to_be_bytes())
            .chain_update(request.source)
            .chain_update(&request.intent)
            .chain_update(gathering.mutation.before.to_be_bytes())
            .chain_update(gathering.mutation.after.to_be_bytes())
            .chain_update(gathering.mutation.expected_revision.to_be_bytes())
            .chain_update(&gathering.mutation.placement)
            .chain_update(gathering.scope.reset_epoch.to_be_bytes())
            .chain_update(gathering.scope.content_generation)
            .finalize()
            .into(),
    )
}

pub(super) struct StationUse {
    pub(super) definition: &'static Definition,
    pub(super) station: &'static UseStation,
}

impl StationUse {
    pub(super) fn refill(&self) -> bool {
        self.station.resource.is_none()
    }
    pub(super) fn extinguish(&self) -> bool {
        self.station.resource
            == Some(oteryn_gameplay_world_change::owner::ContributionKind::ThornfireExtinguished)
    }
    pub(super) fn recipe(&self) -> &'static str {
        if self.refill() { REFILL } else { RECIPE }
    }
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
        || gathering.mutation.decay
            != station
                .restore_after_save_ms
                .first()
                .map(|offset| vec![(u32::from(station.appearance), i64::from(*offset))])
                .unwrap_or_default()
    {
        return None;
    }
    let use_station = StationUse {
        definition,
        station,
    };
    let valid = if use_station.refill() {
        definition.key == "thornfire"
            && station.appearance == BOG
            && station.material == "oteryn:item.tibia.i12819"
            && station.after == Some(BOG)
            && station.yield_item == Some(FILLED_BUCKET)
            && station.restore_after_save_ms.is_empty()
    } else if use_station.extinguish() {
        definition.key == "thornfire"
            && FLAMES.contains(&station.appearance)
            && station.material == "oteryn:item.tibia.i12722"
            && station.after.is_none()
            && station.yield_item == Some(EMPTY_BUCKET)
            && station.restore_after_save_ms == RESTORE_OFFSETS
    } else {
        station.restore_after_save_ms.is_empty()
    };
    valid.then_some(use_station)
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
    let ordinal = usize::from(gathering.mutation.placement[39]);
    if !selected.is_some_and(|variant| {
        variant.tiles.iter().any(|tile| {
            tile.position == station.station.position
                && tile
                    .entries
                    .get(ordinal)
                    .is_some_and(|entry| entry.0 == station.station.appearance)
        })
    }) {
        return Ok(false);
    }
    if station.refill() {
        return Ok(true);
    }
    let Some(resource) = station.station.resource else {
        return Ok(false);
    };
    let result = super::super::world_event::changes::contribute(
        tx,
        recovery,
        world,
        epoch,
        operation_id(request.command),
        resource,
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

#[cfg(test)]
fn variant_contains_station(variant: &variants::Variant, station: &variants::UseStation) -> bool {
    variant.tiles.iter().any(|tile| {
        tile.position == station.position
            && tile
                .entries
                .iter()
                .any(|entry| entry.0 == station.appearance)
    })
}

pub(super) async fn prepare(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ItemCraftRequest,
    station: &StationUse,
    native: &NativeGameplayState,
    actors: Option<&oteryn_gameplay_entities::carrier::native_actor_batch::NativeActorBatchProof>,
) -> Result<bool, DurabilityError> {
    if !station.extinguish() {
        return Ok(actors.is_none());
    }
    let Some(wolves) = source_station_wolves(request) else {
        return Ok(false);
    };
    if wolves {
        let Some(proof) = actors else {
            return Ok(false);
        };
        let Some(binding) = source_station_actor_binding(request) else {
            return Ok(false);
        };
        if !proof.authenticates(
            *fence.character_id.as_bytes(),
            fence.game_session_id,
            fence.character_lease_generation,
            fence.runtime_scope,
            fence.scope_ownership_generation,
            native.source_digest(),
            binding,
            WOLF,
            2,
            &WOLF_PADS,
        ) {
            return Ok(false);
        }
    } else if actors.is_some() {
        return Ok(false);
    }
    let live:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules p WHERE p.item_instance_id=encode($1,'hex')::uuid AND p.definition_family='Item' AND p.definition_production_key='oteryn:item.tibia.i12722' AND p.definition_revision='definition-r1' AND p.duration_millis=3600000 AND p.target_family='Item' AND p.target_production_key='oteryn:item.tibia.i12819' AND p.target_revision='definition-r1' AND p.content_digest=$2 AND p.expires_at_unix_ms>floor(extract(epoch FROM statement_timestamp())*1000)::bigint AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts r WHERE r.item_instance_id=p.item_instance_id))")
        .bind(request.source.as_slice()).bind(native.source_digest().as_slice()).fetch_one(&mut ***tx).await?;
    Ok(live)
}

pub(super) fn decorate_output(output: &mut Value, station: Option<&StationUse>, digest: [u8; 32]) {
    if station.is_some_and(StationUse::refill) {
        output["decay"] = json!({"duration_millis":FILLED_LIFETIME_MS,"target":{"family":"Item","key":"oteryn:item.tibia.i12819","revision":"definition-r1"}});
        output["content_digest"] = Value::String(format!(
            "sha256:{}",
            digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ));
    }
}

pub(super) async fn anchor_restore(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ItemCraftRequest,
    station: &StationUse,
    occurred: i64,
) -> Result<(), DurabilityError> {
    if !station.extinguish() {
        return Ok(());
    }
    let gathering = request
        .gathering
        .as_ref()
        .ok_or(DurabilityError::InvalidStoredState)?;
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let save:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM activated_at)*1000)::bigint FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(gathering.scope.reset_epoch.to_string()).fetch_one(&mut ***tx).await?;
    let future = station
        .station
        .restore_after_save_ms
        .iter()
        .filter_map(|offset| save.checked_add(i64::from(*offset)))
        .find(|deadline| *deadline > occurred);
    if let Some(deadline) = future {
        sqlx::query("UPDATE game_terrain_tool_transforms SET decay_at_unix_ms=$1 WHERE world_id=encode($2,'hex')::uuid AND channel_id=encode($3,'hex')::uuid AND placement=$4 AND reset_epoch=$5")
            .bind(deadline).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&gathering.mutation.placement).bind(gathering.scope.reset_epoch).execute(&mut ***tx).await?;
    } else {
        sqlx::query("UPDATE game_terrain_tool_transforms SET decay_states='{}'::bigint[],decay_delays='{}'::bigint[],decay_step=0,decay_at_unix_ms=NULL WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND placement=$3 AND reset_epoch=$4")
            .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&gathering.mutation.placement).bind(gathering.scope.reset_epoch).execute(&mut ***tx).await?;
    }
    Ok(())
}

pub(super) async fn achievement(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ItemCraftRequest,
    station: Option<&StationUse>,
    event: &[u8],
) -> Result<(), DurabilityError> {
    if station.is_some_and(StationUse::extinguish) {
        let wolves = source_station_wolves(request).ok_or(DurabilityError::InvalidStoredState)?;
        super::super::account_achievement::add_progress_conditionally(
            tx,
            fence.character_id,
            "oteryn:achievement/firefighter",
            500,
            "oteryn:world-change.thornfire",
            event,
            wolves,
        )
        .await?;
    }
    Ok(())
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
