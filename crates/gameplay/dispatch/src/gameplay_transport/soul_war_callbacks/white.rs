//! Source white-floor damage, retained per-target progress and conditional restoration.
use super::super::soul_war_scene as scene;
use super::*;
use oteryn_gameplay_entities::carrier::soul_war::SoulWarSpawnSource;

#[derive(Debug, Default)]
pub(in crate::gameplay_transport) struct WhiteTileState {
    pub(in crate::gameplay_transport) restores: Vec<scene::SourceGround>,
    top_orders: std::collections::BTreeMap<u16, u8>,
    damage_targets: Option<Vec<(ExactActorRef, crate::foundation::GameSessionId, bool)>>,
}
impl WhiteTileState {
    pub(in crate::gameplay_transport) fn for_room(
        room: &QualifiedNativeEntryRoom,
    ) -> Result<Self, ()> {
        bounds(room)?;
        Ok(Self {
            top_orders: scene::source_top_orders(room)?,
            ..Default::default()
        })
    }
}
fn bounds(
    room: &QualifiedNativeEntryRoom,
) -> Result<(std::ops::RangeInclusive<i32>, std::ops::RangeInclusive<i32>), ()> {
    use crate::content::bosses::activity_template::PinnedActivitySourceRoom as Role;
    match room.pinned_activity_source_room() {
        Some(Role::GoshnarsMalice) => Ok((33699..=33718, 31590..=31607)),
        Some(Role::GoshnarsMegalomania) => Ok((33701..=33719, 31626..=31642)),
        _ => Err(()),
    }
}

pub(in crate::gameplay_transport) fn apply(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    source: &SoulWarSpawnSource,
    now: u64,
) -> Result<(), ()> {
    if !qualifies_source(runtime, native)
        || !runtime.soul_war_white_tile_source_current(source)
        || runtime.content_pin().frame_binding_digest() != room.frame_binding().digest()
        || runtime.map_revision_digest() != room.map_revision_digest()
        || !scene::source_map(room)
            .is_some_and(|map| map.overlay.runtime_scope() == runtime.binding().runtime_scope())
    {
        return Err(());
    }
    let (xs, ys) = bounds(room)?;
    if states.soul_war_callbacks.white.damage_targets.is_none() {
        let census = runtime.positioned_actor_census().map_err(|_| ())?;
        let mut targets = Vec::new();
        // Exact source zones, soul_war.lua:345-348/466-469; captures also include their lobbies.
        for x in xs.clone() {
            for y in ys.clone() {
                let at = MovementLocalPosition { x, y, floor: 14 };
                let Some(ground) = scene::current_ground(room, at) else {
                    continue;
                };
                if ground.appearance_id == 409 {
                    continue;
                }
                if let Some(top) = runtime.current_top_actor_on_cell(at).map_err(|_| ())?
                    && let Some(session) = census
                        .iter()
                        .find(|(actor, _, _)| *actor == top)
                        .and_then(|(_, _, session)| *session)
                {
                    targets.push((top, session, false));
                }
            }
        }
        states.soul_war_callbacks.white.damage_targets = Some(targets);
    }
    let count = states
        .soul_war_callbacks
        .white
        .damage_targets
        .as_ref()
        .ok_or(())?
        .len();
    for index in 0..count {
        let (actor, session, done) = states
            .soul_war_callbacks
            .white
            .damage_targets
            .as_ref()
            .ok_or(())?[index];
        if done {
            continue;
        }
        if runtime.player_control_facts(actor, session).is_ok()
            && !states.is_dead(actor)
            && states
                .apply_soul_war_white_tile_damage(
                    runtime,
                    native,
                    source,
                    actor,
                    session,
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
                )
                .is_none()
        {
            return Err(());
        }
        states
            .soul_war_callbacks
            .white
            .damage_targets
            .as_mut()
            .ok_or(())?[index]
            .2 = true;
    }
    // Source smoke is a tile cue, not a fabricated Item or a gameplay aura.
    // Crystal utils/utils_definitions.hpp:135 fixes CONST_ME_REDSMOKE=167.
    let owner = &states.soul_war_callbacks.white;
    for x in xs.clone() {
        for y in ys.clone() {
            let at = MovementLocalPosition { x, y, floor: 14 };
            if let Some(tile) = scene::current_tile(room, at)
                && tile
                    .entries
                    .first()
                    .is_some_and(|entry| entry.facts.appearance_id == 410)
                && tile.entries.iter().all(|entry| {
                    ![32906, 33066, 33067, 33068, 33069, 33070].contains(&entry.facts.appearance_id)
                        && owner
                            .top_orders
                            .get(&entry.facts.appearance_id)
                            .is_some_and(|order| !matches!(order, 1 | 3))
                })
            {
                let _ = scene::emit_effect(room, at, 167);
            }
        }
    }
    let map = scene::source_map(room).ok_or(())?;
    let owner = &mut states.soul_war_callbacks.white;
    let mut index = 0;
    while index < owner.restores.len() {
        let original = owner.restores[index];
        let at = MovementLocalPosition {
            x: i32::from(original.position.x),
            y: i32::from(original.position.y),
            floor: -i16::from(original.position.floor),
        };
        if scene::current_ground(room, at).is_some_and(|ground| ground.appearance_id == 409) {
            if !map
                .facts
                .transform_movement_item(original.key, original.appearance_id)
            {
                return Err(());
            }
            owner.restores.remove(index);
        } else {
            index += 1;
        }
    }
    owner.damage_targets = None;
    Ok(())
}

/// Shared executable Lua helper; callers retain their separate source cadence.
pub(in crate::gameplay_transport) fn create(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    boss: ExactActorRef,
    sequence: u64,
) -> Result<(), ()> {
    use crate::content::bosses::activity_template::PinnedActivitySourceRoom as Role;
    use oteryn_gameplay_encounters::bosses::goshnars_malice::select_white_cells;
    let (center, domain, draw_domain, keys): (RoomPosition, &[u8], &str, &[&str]) =
        match room.pinned_activity_source_room() {
            Some(Role::GoshnarsMalice) => (
                RoomPosition {
                    x: 33709,
                    y: 31599,
                    floor: 14,
                },
                b"srx3:malice:white-tiles:v1",
                "soul_war_malice_white_tiles",
                &["oteryn:creature.goshnar_s_malice"],
            ),
            Some(Role::GoshnarsMegalomania) => (
                RoomPosition {
                    x: 33710,
                    y: 31634,
                    floor: 14,
                },
                b"srx3:megalomania:white-tiles:v1",
                "soul_war_megalomania_white_tiles",
                &[
                    "oteryn:creature.goshnar_s_megalomania_purple",
                    "oteryn:creature.goshnar_s_megalomania_green",
                    "oteryn:creature.goshnar_s_megalomania_blue",
                ],
            ),
            _ => return Err(()),
        };
    let actor_key = source_actor(runtime, native, boss).ok_or(())?;
    if !keys.contains(&actor_key.as_str())
        || !runtime
            .companion_snapshot(boss)
            .is_ok_and(|boss| boss.health > 0)
        || !qualifies_source(runtime, native)
        || runtime.owner_fence().is_err()
        || !matches!(
            runtime.binding().runtime_scope(),
            crate::foundation::RuntimeScopeRefV1::Instance { .. }
        )
        || runtime.content_pin().frame_binding_digest() != room.frame_binding().digest()
        || runtime.map_revision_digest() != room.map_revision_digest()
        || !scene::source_map(room)
            .is_some_and(|map| map.overlay.runtime_scope() == runtime.binding().runtime_scope())
    {
        return Err(());
    }
    let root = GameplayDecisionRoot::from_bytes(
        Sha256::new()
            .chain_update(domain)
            .chain_update(native.source_digest())
            .chain_update(boss.placement_identity())
            .finalize()
            .into(),
    );
    let mut bytes = [0; 16];
    bytes[..8].copy_from_slice(&sequence.to_be_bytes());
    let occurrence = DecisionOccurrenceId::from_bytes(bytes);
    let mut ordinal = 0;
    let cells = select_white_cells(center, |bound| {
        let bound = u64::try_from(bound).ok()?;
        let limit = u64::MAX - u64::MAX % bound;
        for _ in 0..64 {
            let draw = deterministic_decision_u64(&root, occurrence, draw_domain, ordinal).ok()?;
            ordinal = ordinal.checked_add(1)?;
            if draw < limit {
                return usize::try_from(draw % bound).ok();
            }
        }
        None
    })
    .ok_or(())?;
    let map = scene::source_map(room).ok_or(())?;
    for at in cells {
        let at = MovementLocalPosition {
            x: at.x,
            y: at.y,
            floor: at.floor,
        };
        let Some(ground) = scene::current_ground(room, at) else {
            continue;
        };
        let owner = &mut states.soul_war_callbacks.white;
        if let Some(previous) = owner
            .restores
            .iter_mut()
            .find(|entry| entry.key == ground.key)
        {
            *previous = ground;
        } else {
            owner.restores.push(ground);
        }
        if !map.facts.transform_movement_item(ground.key, 409) {
            return Err(());
        }
    }
    Ok(())
}
