//! Greed's current placement read preserves Unknown separately from Lua false.
use super::*;

#[allow(
    clippy::too_many_arguments,
    reason = "existing exact spawn source and deterministic callback input remain separate from current placement facts"
)]
pub(super) async fn apply(
    runtime: &mut ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    source: &oteryn_gameplay_entities::carrier::soul_war::SoulWarSpawnSource,
    boss: ExactActorRef,
    child: &str,
    center: MovementLocalPosition,
    forced: bool,
    sampled_at: u64,
    event: u64,
) -> Result<Option<ExactActorRef>, ()> {
    if forced != (child == greed_model::SOUL_SPHERE) {
        return Err(());
    }
    if !map.source_tile_exists(runtime, center).ok_or(())? {
        return Ok(None);
    }
    let center_admits = map.read(runtime, center).await.ok_or(())?;
    if (center_admits || forced) && (forced || runtime.native_summon_cell_free(center)) {
        if !map.validate_transaction(runtime).await {
            return Err(());
        }
        return Ok(runtime
            .spawn_soul_war_child_from_source(source, child, center, |r, p| {
                map.view()
                    .matches(r)
                    .then(|| forced || map.view().admits(p))
            })
            .ok());
    }
    let cells = room.movement_cells();
    let pz = |at: MovementLocalPosition| {
        cells
            .spell_tiles()
            .lookup(
                cells.scope(),
                crate::content::LogicalCell {
                    x: at.x,
                    y: at.y,
                    z: i32::from(at.floor),
                },
            )
            .ok()
            .map(|tile| tile.flags().protection_zone)
    };
    let center_pz = pz(center).ok_or(())?;
    let root = GameplayDecisionRoot::from_bytes(
        Sha256::new()
            .chain_update(b"srx3:greed:extended-placement:v1")
            .chain_update(native.source_digest())
            .chain_update(boss.placement_identity())
            .chain_update(child.as_bytes())
            .finalize()
            .into(),
    );
    let mut occurrence = [0; 16];
    occurrence[..8].copy_from_slice(&sampled_at.to_be_bytes());
    occurrence[8..].copy_from_slice(&event.to_be_bytes());
    let occurrence = DecisionOccurrenceId::from_bytes(occurrence);
    let mut ordinal = 0;
    let offsets = greed_model::extended_spawn_offsets(|bound| {
        let limit = u64::MAX - u64::MAX % u64::from(bound);
        for _ in 0..64 {
            let draw =
                deterministic_decision_u64(&root, occurrence, "greed_spawn_placement", ordinal);
            ordinal += 1;
            if let Ok(value) = draw
                && value < limit
            {
                return (value % u64::from(bound)) as u32;
            }
        }
        bound
    })
    .map_err(|_| ())?;
    for (dx, dy) in offsets {
        let destination = MovementLocalPosition {
            x: center.x.checked_add(dx).ok_or(())?,
            y: center.y.checked_add(dy).ok_or(())?,
            floor: center.floor,
        };
        if (center_pz && pz(destination) != Some(true))
            || crate::map::floor::is_teleport(
                cells,
                crate::content::LogicalCell {
                    x: destination.x,
                    y: destination.y,
                    z: i32::from(destination.floor),
                },
            )
            || !map.source_tile_exists(runtime, destination).ok_or(())?
        {
            continue;
        }
        if !map.read(runtime, destination).await.ok_or(())?
            || !runtime.native_summon_cell_free(destination)
        {
            continue;
        }
        let Ok(steps) = crate::native_world_read::source_sight_steps(center, destination) else {
            continue;
        };
        for (at, exempt) in steps {
            if !exempt {
                map.read(runtime, at).await.ok_or(())?;
            }
        }
        if !map.view().matches(runtime) {
            return Err(());
        }
        if !map.view().sight(center, destination) {
            continue;
        }
        if !map.validate_transaction(runtime).await {
            return Err(());
        }
        if let Ok(actor) =
            runtime.spawn_soul_war_child_from_source(source, child, destination, |r, p| {
                map.view().matches(r).then(|| map.view().admits(p))
            })
        {
            return Ok(Some(actor));
        }
    }
    Ok(None)
}
