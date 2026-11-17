//! Hunt through network attack, visibility, corpse USE and item-move replies.
use super::*;
use oteryn_dev_client::{
    DevClientSession, EntityDetail, EntityKind, EntityRef, SessionEvent, WorldSpatialEntity,
};
use oteryn_protocol_oteryn::{
    item_view::ItemHandle,
    world_map::{MapDefinition, MapOrigin, MapTile},
    world_spatial::ActorPosition,
};
use std::collections::BTreeSet;
pub(super) fn matches_species(
    kind: oteryn_dev_client::EntityKind,
    shown: Option<&str>,
    expected: &str,
) -> bool {
    kind == oteryn_dev_client::EntityKind::Creature
        && shown.is_some_and(|name| name.eq_ignore_ascii_case(expected))
}

#[test]
fn live_hunt_rejects_an_adjacent_creature_of_another_species() {
    use oteryn_dev_client::EntityKind;
    assert!(!matches_species(
        EntityKind::Creature,
        Some("Cave Rat"),
        "Rat"
    ));
    assert!(!matches_species(EntityKind::Creature, None, "Rat"));
    assert!(matches_species(EntityKind::Creature, Some("Rat"), "Rat"));
}

pub(super) async fn hunt(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1)
    {
        evidence("mechanic=hunt skipped=entry_room_has_no_native_combat_and_loot_content");
        return Ok(());
    }
    hunt_target(client, None, &required("NODE_BOOT_DATABASE_URL")?).await
}

pub(super) async fn hunt_target(
    client: &mut oteryn_dev_client::DevClientSession,
    requested: Option<EntityRef>,
    url: &str,
) -> TestResult {
    let position = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == EntityKind::Creature
                    && requested.is_none_or(|identity| identity == entry.entity)
                    && entry.position.floor == position.floor
                    && entry
                        .position
                        .x
                        .abs_diff(position.x)
                        .max(entry.position.y.abs_diff(position.y))
                        <= 1
            })
        })
        .copied()
        .ok_or("hunt route has no visible monster")?;
    let handle = kill(client, target, url, Duration::from_secs(120)).await?;
    let used = client.use_item(handle).await?;
    if used.status != CommandStatus::Accepted
        || used.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err("hunt corpse USE refused".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while client
        .open_container()
        .is_none_or(|view| view.container_handle != Some(handle))
    {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
        if tokio::time::Instant::now() >= deadline {
            return Err("hunt corpse container delta missing".into());
        }
    }
    let entries = client
        .open_container()
        .ok_or("corpse view")?
        .entries
        .clone();
    let mut picked = 0;
    if let Some(item) = entries.first() {
        let before = client
            .inventory()
            .ok_or("hunt backpack")?
            .entries
            .iter()
            .filter(|entry| entry.item_definition_ref == item.item_definition_ref)
            .map(|entry| u64::from(entry.count.get()))
            .sum::<u64>();
        let corpse_before = entries
            .iter()
            .filter(|entry| entry.item_definition_ref == item.item_definition_ref)
            .map(|entry| u64::from(entry.count.get()))
            .sum::<u64>();
        let moved = client
            .move_item(&oteryn_dev_client::ItemMoveIntent {
                source: item.handle,
                destination: oteryn_dev_client::ItemMoveDestination::MainBackpack,
            })
            .await?;
        if moved.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved {
            return Err(format!("hunt loot move: {moved:?}").into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let after = client
                .inventory()
                .ok_or("hunt inventory delta")?
                .entries
                .iter()
                .filter(|entry| entry.item_definition_ref == item.item_definition_ref)
                .map(|entry| u64::from(entry.count.get()))
                .sum::<u64>();
            let corpse_after = client
                .open_container()
                .ok_or("loot corpse delta")?
                .entries
                .iter()
                .filter(|entry| entry.item_definition_ref == item.item_definition_ref)
                .map(|entry| u64::from(entry.count.get()))
                .sum::<u64>();
            if after == before + u64::from(item.count.get())
                && corpse_after + u64::from(item.count.get()) == corpse_before
            {
                picked = item.count.get();
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("hunt loot inventory conservation failed".into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
        }
    }
    if !loot_move_proven(picked) {
        return Err(EmptyLoot.into());
    }
    evidence(&format!(
        "mechanic=hunt state=killed target={:?} corpse={:?} loot_entries={} picked={picked}",
        target.entity,
        handle,
        entries.len()
    ));
    Ok(())
}

fn observed_kill(damaged: bool, removed: bool, corpse: bool) -> bool {
    damaged && removed && corpse
}
#[test]
fn disappearance_requires_damage_removal_and_a_correlated_corpse() {
    assert!(!observed_kill(false, true, true));
    assert!(!observed_kill(true, false, true));
    assert!(!observed_kill(true, true, false));
    assert!(observed_kill(true, true, true));
}

fn corpse_candidate(matches_target: bool, present_before: bool, otherwise_ambiguous: bool) -> bool {
    matches_target && !present_before && !otherwise_ambiguous
}
#[test]
fn unrelated_or_old_corpses_are_not_accepted_as_the_attack_targets() {
    assert!(!corpse_candidate(false, false, false));
    assert!(!corpse_candidate(true, true, false));
    assert!(!corpse_candidate(true, false, true));
    assert!(corpse_candidate(true, false, false));
}

fn loot_move_proven(moved: u32) -> bool {
    moved > 0
}
#[test]
fn an_empty_roll_does_not_qualify_item_movement() {
    assert!(!loot_move_proven(0));
    assert!(loot_move_proven(1));
}

#[derive(Debug)]
pub(super) struct EmptyLoot;

impl std::fmt::Display for EmptyLoot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("the correlated corpse rolled empty before any loot moved")
    }
}

impl std::error::Error for EmptyLoot {}

pub(super) fn health(entry: &WorldSpatialEntity) -> Option<u8> {
    match entry.detail {
        EntityDetail::Actor { health_percent, .. }
        | EntityDetail::ActorWithIcons { health_percent, .. } => Some(health_percent),
        _ => None,
    }
}

fn map_handles(tiles: &[MapTile], position: ActorPosition) -> Option<Vec<(u32, ItemHandle)>> {
    tiles
        .iter()
        .find(|tile| tile.position == position)
        .map(|tile| {
            tile.items
                .iter()
                .filter_map(|item| match (item.definition, item.origin) {
                    (MapDefinition::Item(definition), MapOrigin::Handle(handle)) => {
                        Some((definition.get(), handle))
                    }
                    _ => None,
                })
                .collect()
        })
}

fn native_position(position: ActorPosition) -> ActorPosition {
    ActorPosition {
        floor: position.floor.min(position.floor.saturating_neg()),
        ..position
    }
}

fn corpse_item(entry: &WorldSpatialEntity) -> Option<(u32, ItemHandle)> {
    match entry.detail {
        EntityDetail::Object {
            item_definition_ref,
            item_handle: Some(handle),
            ..
        }
        | EntityDetail::ObjectWithAppearance {
            item_definition_ref,
            item_handle: Some(handle),
            ..
        } => Some((item_definition_ref, handle)),
        _ => None,
    }
}

struct CorpseProof {
    definition: u32,
    identity: [u8; 16],
    top_damage: bool,
    earlier_same_definition: usize,
}

async fn exact_corpse(
    url: &str,
    session: &[u8; 16],
    target: EntityRef,
    position: ActorPosition,
    started_at: i64,
    deadline: tokio::time::Instant,
) -> TestResult<Option<CorpseProof>> {
    use crate::foundation::{
        ActorLocalGeneration, ActorLocalId, ActorRef, ExactActorRef, RuntimeScopeRefV1,
        ScopeOwnershipGeneration,
    };
    let mut packed = position.x.to_be_bytes().to_vec();
    packed.extend_from_slice(&position.y.to_be_bytes());
    packed.extend_from_slice(&position.floor.to_be_bytes());
    let mut native = packed.clone();
    native[8..].copy_from_slice(&native_position(position).floor.to_be_bytes());
    type CorpseRow = (i64, String, String, String, bool, i64);
    type ScopedCorpses = (Vec<u8>, Vec<u8>, String, Vec<CorpseRow>);
    let (world, channel, generation, rows): ScopedCorpses = within_database_deadline(deadline, async {
        let mut db = sqlx::PgConnection::connect(url).await?;
        let (world, channel, generation, character): (Vec<u8>, Vec<u8>, String, Vec<u8>) = sqlx::query_as(
            "SELECT uuid_send(runtime_scope_world_id), uuid_send(runtime_scope_channel_id), scope_ownership_generation::text, uuid_send(character_id)
             FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid AND runtime_scope_kind=1",
        ).bind(session.as_slice()).fetch_one(&mut db).await?;
        let rows = sqlx::query_as(
            "SELECT m.death_actor_local_id::bigint, m.death_actor_local_generation::text, i.definition_production_key, i.definition_revision_ref, \
             COALESCE(m.corpse_top_damage_character_id=encode($6,'hex')::uuid,false), m.materialized_at \
             FROM game_item_mint_receipts m JOIN game_item_ground_locations g USING (item_instance_id) \
             JOIN game_item_instances i USING (item_instance_id) \
             WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid \
             AND g.runtime_scope_ownership_generation=$3::text::numeric AND i.lifecycle=1 \
             AND (g.spatial_position=$4 OR g.spatial_position=$5) \
             AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0 AND m.materialized_at IS NOT NULL",
        ).bind(&world).bind(&channel)
          .bind(&generation).bind(packed).bind(native).bind(character).fetch_all(&mut db).await?;
        db.close().await?;
        Ok((world, channel, generation, rows))
    }).await?;
    let scope = RuntimeScopeRefV1::channel(
        crate::foundation::WorldId::decode(&world)?,
        crate::foundation::ChannelId::decode(&channel)?,
    );
    let generation = ScopeOwnershipGeneration::new(generation.parse()?)?;
    let mut matching = None;
    for (slot, actor_generation, key, revision, top_damage, materialized) in &rows {
        if *materialized < started_at || actor_generation.parse::<u64>()? != target.generation {
            continue;
        }
        let actor = ExactActorRef(ActorRef::from_scoped_owner_parts(
            scope,
            generation,
            ActorLocalId(u32::try_from(*slot)?),
            ActorLocalGeneration(target.generation),
        ));
        if actor.placement_identity() != target.identity {
            continue;
        }
        let same_definition = |row: &&CorpseRow| row.2 == *key && row.3 == *revision;
        let earlier_same_definition = rows
            .iter()
            .filter(same_definition)
            .filter(|row| row.5 < started_at)
            .count();
        if rows
            .iter()
            .filter(same_definition)
            .filter(|row| row.5 >= started_at)
            .count()
            != 1
        {
            return Err(
                "multiple new corpses of the target's definition share its death tile".into(),
            );
        }
        let items = client_world::provisioning::item_index()?;
        let reference = items
            .definition_ref("Item", key, revision)
            .ok_or("durable corpse definition is absent from the pin")?;
        matching = Some(CorpseProof {
            definition: reference.get(),
            identity: actor.corpse_identity(),
            top_damage: *top_damage,
            earlier_same_definition,
        });
    }
    Ok(matching)
}

async fn database_clock(url: &str, deadline: tokio::time::Instant) -> TestResult<i64> {
    within_database_deadline(deadline, async {
        let mut db = sqlx::PgConnection::connect(url).await?;
        let millis =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&mut db)
                .await?;
        db.close().await?;
        Ok(millis)
    })
    .await
}

/// Observe damage and the explicit leave event before using any corpse handle.
pub(super) async fn kill(
    client: &mut DevClientSession,
    target: WorldSpatialEntity,
    url: &str,
    budget: Duration,
) -> TestResult<ItemHandle> {
    let session = *client.join_snapshot().game_session_id.as_bytes();
    let health_before = health(&target).ok_or("target has no wire health")?;
    let mut position = target.position;
    let mut before: BTreeSet<_> = client
        .world_entities()
        .into_iter()
        .flat_map(|entries| entries.iter())
        .filter(|entry| entry.kind == EntityKind::Corpse)
        .filter_map(corpse_item)
        .map(|(_, handle)| handle)
        .collect();
    let mut remember = |tiles: &[MapTile]| {
        before.extend(
            tiles
                .iter()
                .flat_map(|tile| &tile.items)
                .filter_map(|item| {
                    if let MapOrigin::Handle(handle) = item.origin {
                        Some(handle)
                    } else {
                        None
                    }
                }),
        );
    };
    if let Some(snapshot) = client.session_mut().world_map_snapshot() {
        remember(&snapshot.tiles);
    }
    for event in client.take_events() {
        match event {
            SessionEvent::WorldMap(delta) => remember(&delta.value.tiles),
            SessionEvent::WorldMapSnapshot(snapshot) => remember(&snapshot.value.tiles),
            _ => {}
        }
    }
    let after = client.last_server_sequence();
    let deadline = tokio::time::Instant::now() + budget;
    let started_at = database_clock(url, deadline).await?;
    let observed: TestResult<ItemHandle> = async {
        let attacked = client.session_mut().attack_target(Some(&target.entity)).await?;
        if attacked.status != CommandStatus::Accepted || attacked.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
            return Err(format!("hunt attack refused: {attacked:?}").into());
        }
        let mut damaged = false;
        let mut left = false;
        let mut tile = None;
        let mut exact: Option<CorpseProof> = None;
        loop {
            for event in client.take_events() {
                match event {
                    SessionEvent::WorldSpatialEntities(delta) if delta.server_sequence > after => {
                        for entry in delta.value.update.iter().filter(|entry| entry.entity == target.entity) {
                            if entry.position != position {
                                tile = None;
                            }
                            position = entry.position;
                            damaged |= health(entry).is_some_and(|current| current < health_before);
                        }
                        left |= delta.value.leave.contains(&target.entity);
                    }
                    SessionEvent::WorldMap(delta) if delta.server_sequence > after => {
                        let native = native_position(position);
                        if let Some(handles) = map_handles(&delta.value.tiles, native) {
                            tile = Some(handles);
                        } else if delta.value.cleared.contains(&native) {
                            tile = None;
                        }
                    }
                    _ => {}
                }
            }
            let entities = client.world_entities().ok_or("hunt has no entity store")?;
            let here = client.world_spatial().actor_position;
            let nearby = here.floor == position.floor && here.x.abs_diff(position.x).max(here.y.abs_diff(position.y)) <= 2;
            if left && nearby && entities.get(&target.entity).is_none() {
                if exact.is_none() {
                    exact = exact_corpse(url, &session, target.entity, position, started_at, deadline).await?;
                }
                let candidates: Vec<_> = if let Some(tile) = &tile {
                    exact.as_ref().map_or_else(Vec::new, |proof| {
                        let present_before = tile.iter().filter(|(definition, handle)| *definition == proof.definition && before.contains(handle)).count();
                        tile.iter().copied().filter(|(definition, handle)| corpse_candidate(
                            proof.definition == *definition, before.contains(handle),
                            present_before != proof.earlier_same_definition,
                        )).map(|(_, handle)| handle).collect()
                    })
                } else {
                    entities.iter().filter(|entry| entry.kind == EntityKind::Corpse && entry.position == position)
                        .filter_map(|entry| corpse_item(entry).map(|(_, handle)| (entry.entity.identity, handle)))
                        .filter(|(identity, handle)| corpse_candidate(
                            exact.as_ref().is_some_and(|proof| proof.identity == *identity),
                            before.contains(handle), false))
                        .map(|(_, handle)| handle).collect()
                };
                if observed_kill(damaged || exact.as_ref().is_some_and(|proof| proof.top_damage), left, candidates.len() == 1) {
                    return Ok(candidates[0]);
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("hunt could not correlate damage={damaged}, leave={left} and one corpse of {:?}", target.entity).into());
            }
            client.service_liveness(Duration::from_millis(100)).await?;
        }
    }.await;
    let stopped = client.session_mut().attack_target(None).await;
    match (observed, stopped) {
        (Err(error), _) => Err(error),
        (Ok(handle), Ok(reply))
            if reply.status == CommandStatus::Accepted
                && reply.disposition
                    == oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok =>
        {
            Ok(handle)
        }
        (Ok(_), Ok(reply)) => Err(format!("hunt attack cancellation refused: {reply:?}").into()),
        (Ok(_), Err(error)) => Err(error.into()),
    }
}
