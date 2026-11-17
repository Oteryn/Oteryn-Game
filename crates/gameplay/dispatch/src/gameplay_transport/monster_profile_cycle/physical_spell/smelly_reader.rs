//! Exact saved-VAR Smelly area. Ordinary movement/solid admission is irrelevant.
use super::*;
use oteryn_gameplay_encounters::bosses::smelly_cheese::smelly_encounter_dispatch::SourceSmelly;
#[allow(clippy::too_many_arguments)]
pub(in crate::gameplay_transport) async fn prepare_smelly_reader<'native>(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    attack: &mut ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &'native NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    source: &SourceSmelly,
    proposal: &ProfileAbilityProposal,
    centre: MovementLocalPosition,
    now: u64,
    stamp: RuntimeWorkStamp,
) -> Option<CurrentTarget<'native>> {
    if !source.qualifies_native_runtime(runtime, proposal.issuer)
        || source.native_callback()?.content() != native.source_digest()
    {
        return None;
    }
    let caster = runtime
        .read_actor_position(proposal.issuer)
        .ok()?
        .position();
    let sight_origin = area_sight_origin(caster, centre)?;
    let mut tiles = BTreeMap::new();
    for dy in -1..=1 {
        for dx in -1..=1 {
            let at = MovementLocalPosition {
                x: centre.x.checked_add(dx)?,
                y: centre.y.checked_add(dy)?,
                floor: centre.floor,
            };
            map.read(runtime, at).await?;
            let tile = map.view().source_combat_tile_admits(at)?;
            map.read_sight(runtime, sight_origin, at).await;
            let mut clear = true;
            for (cell, exempt) in
                crate::durability::native_world_read::source_sight_steps(sight_origin, at).ok()?
            {
                if exempt {
                    continue;
                }
                match map.source_tile_exists(runtime, cell)? {
                    false => {}
                    true => {
                        clear &= !map.view().blocks_projectile(cell)?;
                    }
                }
            }
            tiles.insert(
                (at.x, at.y, at.floor),
                Some(caster.floor == at.floor && tile && clear),
            );
        }
    }
    if !map.validate_transaction(runtime).await {
        return None;
    }
    let census = runtime.positioned_actor_census().ok()?;
    let mut players = Vec::new();
    let mut facts = Vec::new();
    let mut source_damage = Vec::new();
    for (&(x, y, floor), &allowed) in &tiles {
        if allowed != Some(true) {
            continue;
        }
        let Some(target) = runtime
            .current_top_actor_on_cell(MovementLocalPosition { x, y, floor })
            .ok()?
        else {
            continue;
        };
        let Some((_, _, Some(session))) = census.iter().find(|(actor, _, _)| *actor == target)
        else {
            continue;
        };
        if states.is_dead(target) || states.has_pending_spell_commit(target, *session) {
            continue;
        }
        let (current, policy) = current_player_combat_facts(
            runtime,
            states,
            attack,
            room,
            proposal.issuer,
            target,
            *session,
            now,
            Some("earth"),
            true,
            None,
        )?;
        facts.push(current);
        source_damage.push((target, policy));
        players.push((target, *session));
    }
    Some(CurrentTarget {
        native,
        source_draw: None,
        retained_caster: None,
        stamp,
        players,
        issuer: proposal.issuer,
        element: Some("earth".into()),
        facing: Facing::North,
        facts,
        tiles,
        source_damage,
    })
}
