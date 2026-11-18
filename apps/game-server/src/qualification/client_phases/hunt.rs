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
    kind == oteryn_dev_client::EntityKind::Creature && matches_target_name(Some(expected), shown)
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
    hunt_target(client, None).await
}

pub(super) async fn hunt_target(
    client: &mut oteryn_dev_client::DevClientSession,
    requested: Option<EntityRef>,
) -> TestResult {
    if requested.is_some()
        && !client
            .selected_capabilities()
            .contains(&oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1)
    {
        return Err("routed hunt requires the world-map capability".into());
    }
    let position = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| select_hunt_target(entities.iter(), position, requested))
        .copied()
        .ok_or("hunt route has no visible monster")?;
    hunt_observed_target(client, target).await
}

pub(super) async fn hunt_named_target(
    client: &mut DevClientSession,
    target: EntityRef,
    name: &str,
) -> TestResult {
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1)
    {
        return Err(format!("hunt for {name} requires the world-map capability").into());
    }
    let entry = client
        .world_entities()
        .and_then(|entities| {
            select_hunt_target(
                entities.iter(),
                client.world_spatial().actor_position,
                Some(target),
            )
        })
        .filter(|entry| client_world::has_display_name(entry, name))
        .copied()
        .ok_or_else(|| format!("hunt route has no adjacent creature matching {name:?}"))?;
    hunt_observed_target(client, entry).await
}

pub(super) async fn hunt_observed_target(
    client: &mut DevClientSession,
    target: WorldSpatialEntity,
) -> TestResult {
    let handle = kill(
        client,
        target,
        &required("NODE_BOOT_DATABASE_URL")?,
        Duration::from_secs(120),
    )
    .await?;
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

fn matches_target_name(wanted: Option<&str>, observed: Option<&str>) -> bool {
    wanted.is_none_or(|name| observed.is_some_and(|label| label.eq_ignore_ascii_case(name)))
}

#[test]
fn a_nearby_other_creature_cannot_count_as_court_warlock() {
    assert!(matches_target_name(
        Some("Court Warlock"),
        Some("Court Warlock")
    ));
    assert!(!matches_target_name(Some("Court Warlock"), Some("Rat")));
    assert!(!matches_target_name(Some("Court Warlock"), None));
    assert!(matches_target_name(None, Some("Rat")));
}

fn corpse_handle(detail: oteryn_dev_client::EntityDetail) -> Option<oteryn_dev_client::ItemHandle> {
    match detail {
        oteryn_dev_client::EntityDetail::Object { item_handle, .. }
        | oteryn_dev_client::EntityDetail::ObjectWithAppearance { item_handle, .. } => item_handle,
        _ => None,
    }
}

#[test]
fn round2_appearance_corpse_retains_its_item_handle() {
    use oteryn_dev_client::{EntityDetail, ItemHandle};
    assert_eq!(
        corpse_handle(EntityDetail::ObjectWithAppearance {
            item_definition_ref: 1,
            quantity: 1,
            item_handle: Some(ItemHandle::MIN),
            appearance_id: std::num::NonZeroU16::MIN,
        }),
        Some(ItemHandle::MIN)
    );
}

struct HuntEvidence {
    damaged: bool,
    last_position: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    removed: bool,
    prior_corpses: BTreeSet<oteryn_dev_client::EntityRef>,
    corpses_during_absence: BTreeSet<oteryn_dev_client::EntityRef>,
}

impl HuntEvidence {
    fn apply(
        &mut self,
        target: oteryn_dev_client::EntityRef,
        delta: &oteryn_dev_client::WorldSpatialEntitiesDelta,
    ) {
        use oteryn_dev_client::{EntityDetail, EntityKind};
        if let Some(actor) = delta
            .update
            .iter()
            .chain(&delta.enter)
            .find(|entity| entity.entity == target)
        {
            // A live return invalidates corpses that entered during the previous absence.
            self.prior_corpses.append(&mut self.corpses_during_absence);
            self.last_position = actor.position;
            self.removed = false;
            self.damaged |= matches!(actor.detail, EntityDetail::Actor { health_percent, .. }
                | EntityDetail::ActorWithIcons { health_percent, .. } if health_percent < 100);
        }
        self.removed |= delta.leave.contains(&target);
        let corpses = if self.removed {
            &mut self.corpses_during_absence
        } else {
            &mut self.prior_corpses
        };
        corpses.extend(
            delta
                .enter
                .iter()
                .filter(|entry| entry.kind == EntityKind::Corpse)
                .map(|entry| entry.entity),
        );
    }

    fn corpse<'a>(
        &self,
        entities: impl Iterator<Item = &'a oteryn_dev_client::WorldSpatialEntity>,
        target: oteryn_dev_client::EntityRef,
    ) -> Option<oteryn_dev_client::WorldSpatialEntity> {
        if !self.removed || !self.damaged {
            return None;
        }
        named_corpse(entities, target, self.last_position, &self.prior_corpses)
    }
}

fn named_corpse<'a>(
    entities: impl Iterator<Item = &'a oteryn_dev_client::WorldSpatialEntity>,
    target: oteryn_dev_client::EntityRef,
    last_position: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    prior_corpses: &BTreeSet<oteryn_dev_client::EntityRef>,
) -> Option<oteryn_dev_client::WorldSpatialEntity> {
    use oteryn_dev_client::EntityKind;
    let mut found = None;
    for entry in entities {
        if entry.entity == target {
            return None;
        }
        if entry.kind == EntityKind::Corpse
            && entry.position == last_position
            && !prior_corpses.contains(&entry.entity)
            && corpse_handle(entry.detail).is_some()
            && found.replace(*entry).is_some()
        {
            return None;
        }
    }
    found
}

#[test]
fn round5_live_warlock_reentry_excludes_corpse_from_separate_delta() {
    use oteryn_dev_client::{EntityDetail, EntityKind, EntityRef, ItemHandle, WorldSpatialEntity};
    use oteryn_protocol_oteryn::world_spatial::ActorPosition;
    let at = ActorPosition {
        x: 20,
        y: 30,
        floor: 7,
    };
    let id = EntityRef {
        identity: [1; 16],
        generation: 1,
    };
    let actor = WorldSpatialEntity {
        display_name: None,
        entity: id,
        kind: EntityKind::Creature,
        position: at,
        detail: EntityDetail::Actor {
            direction: StepDirection::North,
            appearance_ref: 1,
            health_percent: 35,
        },
    };
    let corpse = WorldSpatialEntity {
        display_name: None,
        entity: EntityRef {
            identity: [2; 16],
            generation: 1,
        },
        kind: EntityKind::Corpse,
        position: at,
        detail: EntityDetail::Object {
            item_definition_ref: 1,
            quantity: 1,
            item_handle: Some(ItemHandle::MIN),
        },
    };
    let mut observed = HuntEvidence {
        damaged: false,
        last_position: at,
        removed: false,
        prior_corpses: BTreeSet::new(),
        corpses_during_absence: BTreeSet::new(),
    };
    let mut delta = oteryn_dev_client::WorldSpatialEntitiesDelta {
        content_generation: [0; 32],
        actor_position: at,
        enter: vec![],
        update: vec![],
        leave: vec![id],
    };
    observed.apply(id, &delta);
    delta.leave.clear();
    delta.enter = vec![corpse];
    observed.apply(id, &delta);
    delta.enter = vec![actor];
    observed.apply(id, &delta);
    assert_eq!(observed.corpse([actor, corpse].iter(), id), None);
    delta.enter.clear();
    delta.leave.push(id);
    observed.apply(id, &delta);
    assert_eq!(observed.corpse([corpse].iter(), id), None);
    let final_corpse = WorldSpatialEntity {
        entity: EntityRef {
            identity: [3; 16],
            generation: 1,
        },
        ..corpse
    };
    delta.leave.clear();
    delta.enter.push(final_corpse);
    observed.apply(id, &delta);
    assert_eq!(
        observed.corpse([corpse, final_corpse].iter(), id),
        Some(final_corpse)
    );
}

fn select_hunt_target<'a>(
    mut entities: impl Iterator<Item = &'a WorldSpatialEntity>,
    position: ActorPosition,
    target: Option<EntityRef>,
) -> Option<&'a WorldSpatialEntity> {
    entities.find(|entry| {
        entry.kind == EntityKind::Creature
            && target.is_none_or(|expected| entry.entity == expected)
            && entry.position.floor == position.floor
            && entry
                .position
                .x
                .abs_diff(position.x)
                .max(entry.position.y.abs_diff(position.y))
                <= 1
    })
}

#[test]
fn qualification_hunt_keeps_routed_entity() {
    let here = ActorPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    let make = |id| WorldSpatialEntity {
        entity: EntityRef {
            identity: [id; 16],
            generation: 1,
        },
        kind: EntityKind::Creature,
        display_name: None,
        position: ActorPosition { x: 101, ..here },
        detail: EntityDetail::Actor {
            direction: oteryn_protocol_oteryn::world_spatial::StepDirection::North,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let first = make(1);
    let selected = make(2);
    assert_eq!(
        select_hunt_target([&first, &selected].into_iter(), here, Some(selected.entity)),
        Some(&selected),
    );
    assert_eq!(
        select_hunt_target([&first].into_iter(), here, Some(selected.entity)),
        None
    );
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
    instance: [u8; 16],
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
    type CorpseRow = (i64, String, String, String, bool, i64, Vec<u8>);
    type ScopedCorpses = (Vec<u8>, Vec<u8>, String, Vec<CorpseRow>);
    let (world, channel, generation, rows): ScopedCorpses = within_database_deadline(deadline, async {
        let mut db = sqlx::PgConnection::connect(url).await?;
        let (world, channel, generation, character): (Vec<u8>, Vec<u8>, String, Vec<u8>) = sqlx::query_as(
            "SELECT uuid_send(runtime_scope_world_id), uuid_send(runtime_scope_channel_id), scope_ownership_generation::text, uuid_send(character_id)
             FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid AND runtime_scope_kind=1",
        ).bind(session.as_slice()).fetch_one(&mut db).await?;
        let rows = sqlx::query_as(
            "SELECT m.death_actor_local_id::bigint, m.death_actor_local_generation::text, i.definition_production_key, i.definition_revision_ref, \
             COALESCE(m.corpse_top_damage_character_id=encode($6,'hex')::uuid,false), m.materialized_at, uuid_send(i.item_instance_id) \
             FROM game_item_mint_receipts m JOIN game_item_ground_locations g USING (item_instance_id) \
             JOIN game_item_instances i USING (item_instance_id) \
             WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid \
             AND g.runtime_scope_ownership_generation=$3::text::numeric AND i.lifecycle=1 \
             AND (g.spatial_position=$4 OR g.spatial_position=$5) \
             AND m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0 AND m.materialized_at >= $7 AND m.death_actor_local_generation=$8::text::numeric LIMIT 65",
        ).bind(&world).bind(&channel)
          .bind(&generation).bind(packed).bind(native).bind(character).bind(started_at).bind(target.generation.to_string()).fetch_all(&mut db).await?;
        db.close().await?;
        Ok((world, channel, generation, rows))
    }).await?;
    let scope = RuntimeScopeRefV1::channel(
        crate::foundation::WorldId::decode(&world)?,
        crate::foundation::ChannelId::decode(&channel)?,
    );
    let generation = ScopeOwnershipGeneration::new(generation.parse()?)?;
    if rows.len() > 64 {
        return Err("corpse proof exceeded the qualification tile budget".into());
    }
    let mut matching = None;
    for (slot, actor_generation, key, revision, top_damage, materialized, instance) in &rows {
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
        let items = client_world::provisioning::item_index()?;
        let reference = items
            .definition_ref("Item", key, revision)
            .ok_or("durable corpse definition is absent from the pin")?;
        matching = Some(CorpseProof {
            definition: reference.get(),
            identity: actor.corpse_identity(),
            top_damage: *top_damage,
            instance: instance.as_slice().try_into()?,
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
    let mut wire = HuntEvidence {
        damaged: false,
        last_position: target.position,
        removed: false,
        prior_corpses: client
            .world_entities()
            .into_iter()
            .flat_map(|entries| entries.iter())
            .filter(|entry| entry.kind == EntityKind::Corpse)
            .map(|entry| entry.entity)
            .collect(),
        corpses_during_absence: BTreeSet::new(),
    };
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
                        wire.apply(target.entity, &delta.value);
                        left = wire.removed;
                        if !left {
                            exact = None;
                        }
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
                        tile.iter().copied().filter(|(definition, handle)|
                            corpse_candidate(proof.definition == *definition, before.contains(handle), false))
                            .map(|(_, handle)| handle).collect()
                    })
                } else {
                    wire.corpse(entities.iter(), target.entity).into_iter()
                        .filter_map(|entry| corpse_item(&entry).map(|(_, handle)| (entry.entity.identity, handle)))
                        .filter(|(identity, handle)| corpse_candidate(
                            exact.as_ref().is_some_and(|proof| proof.identity == *identity),
                            before.contains(handle), false))
                        .map(|(_, handle)| handle).collect()
                };
                if observed_kill(damaged || exact.as_ref().is_some_and(|proof| proof.top_damage), left, !candidates.is_empty()) {
                    if tile.is_none() && candidates.len() == 1 {
                        return Ok(candidates[0]);
                    }
                    if let Some(proof) = &exact {
                        for handle in candidates {
                            let actual = client_readmission::within_retry_budget(deadline, client_world::observed_item_instance(session, handle)).await?;
                            if actual == Some(proof.instance) {
                                return Ok(handle);
                            }
                        }
                    }
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
