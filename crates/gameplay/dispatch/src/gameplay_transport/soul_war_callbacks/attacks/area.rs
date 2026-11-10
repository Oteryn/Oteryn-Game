//! One accepted source area keeps its exact tiles, top Players and completed
//! callback prefix. Map preparation publishes no HP or presentation effects.
use super::*;

#[derive(Clone, Copy, Debug)]
pub(super) struct Geometry {
    pub(super) caster: MovementLocalPosition,
    pub(super) target: MovementLocalPosition,
}

#[derive(Debug)]
struct Target {
    actor: ExactActorRef,
    session: GameSessionId,
    at: MovementLocalPosition,
    blue_ground: bool,
}

#[derive(Debug)]
struct Tile {
    at: MovementLocalPosition,
    heal: Option<u32>,
}

/// Private construction joins the actual native proposal with the current Map
/// transaction and captured top-Player cohort. This is not a fresh cast port.
#[derive(Debug)]
pub(in crate::gameplay_transport) struct PreparedSoulWarArea {
    binding: AttackOwnerBinding,
    stamp: crate::foundation::RuntimeWorkStamp,
    kind: Kind,
    occurrence: String,
    targets: Vec<Target>,
    tiles: Vec<Tile>,
    tile_cursor: usize,
    target_cursor: usize,
}

impl PreparedSoulWarArea {
    fn current(&self, runtime: &ChannelRuntimeV1, native: &NativeGameplayState) -> bool {
        self.binding == binding(runtime)
            && native.source_digest() == runtime.content_pin().server_artifact_digest()
            && runtime
                .owner_fence()
                .is_ok_and(|fence| fence.accepts_stamp(self.stamp))
    }

    pub(in crate::gameplay_transport) fn permits_blue_target(
        &self,
        runtime: &ChannelRuntimeV1,
        native: &NativeGameplayState,
        target: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        self.kind == Kind::MegalomaniaBlue
            && self.current(runtime, native)
            && self.targets.iter().any(|captured| {
                captured.actor == target && captured.session == session && captured.blue_ground
            })
            && runtime.player_control_facts(target, session).is_ok()
    }
}

pub(super) async fn prepare(
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    source: &RetainedSoulWarAttack,
    geometry: Geometry,
) -> Result<PreparedSoulWarArea, ()> {
    if !source.current(runtime, native) || scene::source_map(room).is_none() {
        return Err(());
    }
    let Geometry { caster, target } = geometry;
    if caster.floor != target.floor {
        return Err(());
    }
    let blue = source.kind == Kind::MegalomaniaBlue;
    // AreaCombat::getList uses one step from the target toward the caster as
    // its sight origin, independently of the source area's own centre marker.
    let dx = caster.x.checked_sub(target.x).ok_or(())?;
    let dy = caster.y.checked_sub(target.y).ok_or(())?;
    let (sx, sy) = if dx.unsigned_abs() > dy.unsigned_abs() {
        (dx.signum(), 0)
    } else if dy.unsigned_abs() > dx.unsigned_abs() {
        (0, dy.signum())
    } else {
        (dx.signum(), dy.signum())
    };
    let sight = MovementLocalPosition {
        x: target.x.checked_add(sx).ok_or(())?,
        y: target.y.checked_add(sy).ok_or(())?,
        ..target
    };
    let mut tiles = Vec::new();
    for (dx, dy) in shape(blue) {
        let at = MovementLocalPosition {
            x: target.x.checked_add(dx).ok_or(())?,
            y: target.y.checked_add(dy).ok_or(())?,
            ..target
        };
        // Known blocked/absent cells are source no-ops. An unavailable read
        // retains the whole callback and cannot silently shorten its area.
        map.read(runtime, at).await.ok_or(())?;
        for (step, exempt) in
            crate::durability::native_world_read::source_sight_steps(sight, at).map_err(|_| ())?
        {
            if !exempt {
                map.read(runtime, step).await.ok_or(())?;
            }
        }
        let cells = room.movement_cells();
        let cell = crate::content::LogicalCell {
            x: at.x,
            y: at.y,
            z: i32::from(at.floor),
        };
        let tile = match cells.spell_tiles().lookup(cells.scope(), cell) {
            Ok(tile) => tile,
            Err(crate::content::SpellTileLookupError::Absent) => continue,
            Err(_) => return Err(()),
        };
        let projectile = map.view().blocks_projectile(at).ok_or(())?;
        if tile.flags().floor_change
            || tile.flags().protection_zone
            || projectile
            || crate::map::floor::is_teleport(cells, cell)
            || !map.view().sight(sight, at)
        {
            continue;
        }
        scene::wire_position(at).ok_or(())?;
        let heal = if blue {
            None
        } else {
            Some(
                2300 + draw(
                    native.source_digest(),
                    source.issuer,
                    &source.occurrence,
                    u64::try_from(tiles.len()).map_err(|_| ())? + 1,
                    701,
                )
                .ok_or(())?,
            )
        };
        tiles.push(Tile { at, heal });
    }
    if !map.validate_transaction(runtime).await || !source.current(runtime, native) {
        return Err(());
    }
    // Lua queues death. Capturing each original top Player prevents a retry
    // from exposing a lower stack after a previous callback became lethal.
    let mut targets = Vec::new();
    for (actor, position, session) in runtime.positioned_melee_census().map_err(|_| ())? {
        let Some(session) = session else {
            continue;
        };
        let at = position.position();
        if (blue && !in_zone(source.kind, at))
            || runtime.current_top_actor_on_cell(at).map_err(|_| ())? != Some(actor)
        {
            continue;
        }
        runtime
            .player_control_facts(actor, session)
            .map_err(|_| ())?;
        targets.push(Target {
            actor,
            session,
            at,
            blue_ground: !blue
                || scene::current_ground(room, at)
                    .is_some_and(|ground| ground.appearance_id != 409),
        });
    }
    Ok(PreparedSoulWarArea {
        binding: source.binding,
        stamp: source.stamp,
        kind: source.kind,
        occurrence: source.occurrence.clone(),
        targets,
        tiles,
        tile_cursor: 0,
        target_cursor: 0,
    })
}

pub(super) fn finish(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    pending: &mut PreparedSoulWarArea,
    now: u64,
) -> Result<(), ()> {
    if !pending.current(runtime, native) || scene::source_map(room).is_none() {
        return Err(());
    }
    let blue = pending.kind == Kind::MegalomaniaBlue;
    let semantic = crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now);
    while let Some(tile) = pending.tiles.get(pending.tile_cursor) {
        while let Some(target) = pending.targets.get(pending.target_cursor) {
            if (blue && !target.blue_ground)
                || (!blue && target.at != tile.at)
                || runtime
                    .player_control_facts(target.actor, target.session)
                    .is_err()
            {
                pending.target_cursor += 1;
                continue;
            }
            let (health, _, _) = states
                .source_player_health_including_dead(runtime, target.actor, target.session)
                .ok_or(())?;
            if health > 0 {
                if blue {
                    states
                        .apply_retained_soul_war_blue_damage(
                            runtime,
                            native,
                            pending,
                            target.actor,
                            target.session,
                            semantic,
                        )
                        .ok_or(())?;
                } else {
                    states
                        .apply_source_player_heal(
                            runtime,
                            target.actor,
                            target.session,
                            tile.heal.ok_or(())?,
                            &format!("{}:tile:{}", pending.occurrence, pending.tile_cursor),
                            semantic,
                        )
                        .ok_or(())?;
                }
            }
            // No await separates the canonical HP receipt and this cursor.
            pending.target_cursor += 1;
            #[cfg(test)]
            if health > 0 && states.soul_war_callbacks.attacks.pause_after_player_commit {
                states.soul_war_callbacks.attacks.pause_after_player_commit = false;
                return Err(());
            }
        }
        if !scene::emit_effect(room, tile.at, if blue { 158 } else { 7 }) {
            return Err(());
        }
        pending.tile_cursor += 1;
        pending.target_cursor = 0;
    }
    Ok(())
}
