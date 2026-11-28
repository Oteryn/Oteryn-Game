//! I06/I07 use the served spawn table for travel and the live client for all assertions.
use super::full_e2ef3_e2ei_d::WOLF_BLOCKED_LANDING;
use super::*;
use oteryn_dev_client::{DevClientSession, EntityDetail, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::{
    attack::{AttackIntentDisposition, ChaseMode, FightModes},
    item_view::{ItemEntry, ItemHandle, ItemMoveOutcome},
    logout::LogoutResult,
    quick_loot::{self as loot, QuickLootAction as Action, QuickLootFilterMode as Filter},
    world_object::UseDisposition,
    world_spatial_entities::WorldSpatialEntity,
};
use std::collections::BTreeSet;
use tokio::time::Instant;

const I06: &str = "I06";
const I07: &str = "I07";
pub(super) const COVERED_ROWS: &[&str] = &[I06, I07];

const WOLF_TILE: TilePos = TilePos {
    x: 32427,
    y: 32158,
    floor: -7,
};
// A walkable road west of the wolf spawn. The existing 120-second hunt
// budget cannot cover the 223 cardinal steps from Thais to a Rat approach.
const RAT_HUNT_START: TilePos = TilePos {
    x: 32414,
    y: 32158,
    floor: -7,
};
const RAT: &str = "oteryn:creature.rat";

pub(super) struct Hunt<'a> {
    pub(super) world: &'a BundleWorld,
    pub(super) spawns: &'a oteryn_world_bundle::spawn::Table,
}

pub(super) struct Corpse {
    entity: WorldSpatialEntity,
    handle: ItemHandle,
    pub(super) entries: Vec<ItemEntry>,
}

fn nearby(a: TilePos, b: TilePos) -> bool {
    a.floor == b.floor && a.x.abs_diff(b.x).max(a.y.abs_diff(b.y)) <= 1
}

fn hunt_landing_available(at: TilePos, occupied: &BTreeSet<TilePos>) -> bool {
    route_available(at) && at != WOLF_TILE && at != WOLF_BLOCKED_LANDING && !occupied.contains(&at)
}

fn spawn_route(
    client: &DevClientSession,
    world: &BundleWorld,
    from: TilePos,
    approaches: &BTreeSet<TilePos>,
) -> Result<Vec<StepDirection>, usize> {
    let occupied = occupied_tiles(client);
    find_route(
        from,
        |at| approaches.contains(&at) && hunt_landing_available(at, &occupied),
        |at, step| {
            destination(world, at, step).filter(|next| hunt_landing_available(*next, &occupied))
        },
    )
}

async fn travel(
    client: &mut DevClientSession,
    world: &BundleWorld,
    goal: impl Fn(TilePos) -> bool,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let occupied = occupied_tiles(client);
    let path = route_avoiding(world, from, goal, |at| {
        hunt_landing_available(at, &occupied)
    })
    .ok_or_else(|| {
        format!("no served route from {from:?} avoiding occupied cells and {WOLF_TILE:?}")
    })?;
    walk(client, world, &path).await
}

pub(super) fn near_vendor_placement(actor: TilePos, placement: TilePos) -> bool {
    actor.floor == placement.floor
        && actor
            .x
            .abs_diff(placement.x)
            .max(actor.y.abs_diff(placement.y))
            <= 2
}

pub(super) async fn approach_vendor(
    client: &mut DevClientSession,
    world: &BundleWorld,
    name: &str,
    near_placement: impl Fn(TilePos) -> bool,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let here = tile(client.world_spatial().actor_position)?;
        let actor = client
            .world_entities()
            .into_iter()
            .flat_map(|entities| entities.others())
            .filter(|entry| entry.kind == EntityKind::Npc)
            .filter(|entry| {
                entry.display_name.is_some_and(|seen| {
                    seen.as_str()
                        .is_ok_and(|seen| seen.eq_ignore_ascii_case(name))
                })
            })
            .filter_map(|entry| {
                let at = tile(entry.position).ok()?;
                (at.floor == here.floor && near_placement(at)).then_some((at, entry.entity))
            })
            .min_by_key(|(at, _)| at.x.abs_diff(here.x).max(at.y.abs_diff(here.y)));
        if let Some((at, entity)) = actor {
            if here.x.abs_diff(at.x).max(here.y.abs_diff(at.y)) <= 3 {
                return Ok(entity);
            }
            travel(client, world, |cell| {
                cell.floor == at.floor
                    && cell != at
                    && cell.x.abs_diff(at.x).max(cell.y.abs_diff(at.y)) <= 2
            })
            .await?;
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "vendor {name} not talkable within two tiles of compiled placement from {here:?}"
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

async fn settle(
    client: &mut DevClientSession,
    mut observe: impl FnMut(&mut DevClientSession) -> TestResult<bool>,
) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !observe(client)? {
        if Instant::now() >= deadline {
            return Err(format!(
                "delta deadline;inventory={:?};open-corpse={:?}",
                client.inventory(),
                client.open_container()
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    Ok(())
}

impl Hunt<'_> {
    async fn kill(
        &self,
        client: &mut DevClientSession,
        deadline: Instant,
        sources: &BTreeSet<&str>,
    ) -> TestResult<(WorldSpatialEntity, ItemHandle)> {
        let names: Vec<_> = sources
            .iter()
            .filter_map(|key| key.strip_prefix("oteryn:creature."))
            .map(|name| name.replace('_', " "))
            .collect();
        if Instant::now() >= deadline {
            return Err(format!("hunt budget expired before routing to {sources:?}").into());
        }
        let outside_pz = |at: TilePos| {
            self.world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|ground| !ground.protection_zone())
        };
        let mut approaches = BTreeSet::new();
        for (point, _) in ordinary_spawn_points(self.spawns)
            .filter(|(at, key)| sources.contains(key) && outside_pz(*at))
        {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if let (Some(x), Some(y)) = (
                        point.x.checked_add_signed(dx),
                        point.y.checked_add_signed(dy),
                    ) {
                        let at = TilePos { x, y, ..point };
                        if at != point && outside_pz(at) {
                            approaches.insert(at);
                        }
                    }
                }
            }
        }
        if approaches.is_empty() {
            return Err(format!("no compiled non-protection approach to {sources:?}").into());
        }
        let from = tile(client.world_spatial().actor_position)?;
        let path = spawn_route(client, self.world, from, &approaches).map_err(|visited| {
            format!("no served route to {sources:?} from {from:?} after {visited} cells")
        })?;
        for step in path {
            if Instant::now() >= deadline {
                return Err(
                    format!("hunt budget expired en route to {sources:?} from {from:?}").into(),
                );
            }
            walk(client, self.world, &[step]).await?;
        }
        // A previous row may have depleted this spawn. Give its live
        // projection time to appear, then try a distinct approach.
        let mut search_again = Instant::now() + Duration::from_secs(5);
        let target = loop {
            let here = tile(client.world_spatial().actor_position)?;
            let candidate = client
                .world_entities()
                .into_iter()
                .flat_map(|entities| entities.others())
                .filter(|entry| entry.kind == EntityKind::Creature)
                .filter(|entry| {
                    entry.display_name.is_some_and(|name| {
                        name.as_str().is_ok_and(|name| {
                            names.iter().any(|wanted| name.eq_ignore_ascii_case(wanted))
                        })
                    })
                })
                .filter_map(|entry| {
                    let at = tile(entry.position).ok()?;
                    (here.floor == at.floor && outside_pz(at)).then_some((
                        *entry,
                        at,
                        here.x.abs_diff(at.x).max(here.y.abs_diff(at.y)),
                    ))
                })
                .min_by_key(|(_, _, distance)| *distance);
            if let Some((creature, at, _)) = candidate {
                if nearby(here, at) {
                    break creature;
                }
                travel(client, self.world, |cell| {
                    outside_pz(cell) && nearby(cell, at) && cell != at
                })
                .await?;
            } else if Instant::now() >= search_again {
                approaches.retain(|cell| {
                    cell.floor != here.floor
                        || cell.x.abs_diff(here.x).max(cell.y.abs_diff(here.y)) > 8
                });
                if !approaches.is_empty()
                    && let Ok(path) = spawn_route(client, self.world, here, &approaches)
                {
                    for step in path {
                        if Instant::now() >= deadline {
                            return Err(format!(
                                "hunt budget expired routing to another {sources:?} spawn"
                            )
                            .into());
                        }
                        walk(client, self.world, &[step]).await?;
                    }
                }
                search_again = Instant::now() + Duration::from_secs(5);
            }
            if Instant::now() >= deadline {
                return Err(format!("spawn depleted or no attackable {sources:?} reached at {here:?} by hunt deadline").into());
            }
            client.service_liveness(Duration::from_millis(100)).await?;
            client.take_events();
        };
        let prior_modes = client
            .session_mut()
            .combat_state()
            .map_or(FightModes::DEFAULT, |state| state.modes);
        let modes = FightModes {
            chase: ChaseMode::Chase,
            ..prior_modes
        };
        let reply = client.session_mut().set_fight_modes(&modes).await?;
        if reply.status != CommandStatus::Accepted
            || reply.disposition != AttackIntentDisposition::Ok
        {
            return Err(format!("hunt chase mode refused: {reply:?}").into());
        }
        let mut position = target.position;
        let mut removed = false;
        let mut corpses = Vec::new();
        client.take_events();
        let attack = client
            .session_mut()
            .attack_target(Some(&target.entity))
            .await?;
        if attack.status != CommandStatus::Accepted
            || attack.disposition != AttackIntentDisposition::Ok
        {
            let restored = client.session_mut().set_fight_modes(&prior_modes).await?;
            return Err(format!(
                "ATTACK target={:?} reply={attack:?}; restore={restored:?}",
                target.entity
            )
            .into());
        }
        verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
        loop {
            for event in client.take_events() {
                if let SessionEvent::WorldSpatialEntities(delta) = event {
                    if let Some(updated) = delta
                        .value
                        .update
                        .iter()
                        .find(|entity| entity.entity == target.entity)
                    {
                        position = updated.position;
                    }
                    removed |= delta.value.leave.contains(&target.entity);
                    corpses.extend(
                        delta
                            .value
                            .enter
                            .into_iter()
                            .filter(|entity| entity.kind == EntityKind::Corpse),
                    );
                }
            }
            if removed
                && let Some((corpse, handle)) = corpses
                    .iter()
                    .filter(|corpse| corpse.position == position)
                    .find_map(|entered| {
                        let corpse = *client.world_entities()?.get(&entered.entity)?;
                        match corpse.detail {
                            EntityDetail::Object { item_handle, .. }
                            | EntityDetail::ObjectWithAppearance { item_handle, .. } => {
                                item_handle.map(|handle| (corpse, handle))
                            }
                            _ => None,
                        }
                    })
            {
                let restored = client.session_mut().set_fight_modes(&prior_modes).await?;
                if restored.status != CommandStatus::Accepted
                    || restored.disposition != AttackIntentDisposition::Ok
                {
                    return Err(format!("restore fight mode after kill: {restored:?}").into());
                }
                verdict::Harness::global().progress("kill");
                return Ok((corpse, handle));
            }
            if Instant::now() >= deadline {
                let stop = client.session_mut().attack_target(None).await?;
                let restore = client.session_mut().set_fight_modes(&prior_modes).await?;
                let missing = if removed {
                    "bound-corpse-item-handle"
                } else {
                    "target-removal"
                };
                return Err(format!("attack deadline;missing={missing};sources={sources:?};target={:?};position={position:?};corpses={corpses:?};stop={stop:?};restore={restore:?}", target.entity).into());
            }
            client.service_liveness(Duration::from_millis(100)).await?;
        }
    }

    async fn lootable(
        &self,
        client: &mut DevClientSession,
        step: &mut &'static str,
        kill_step: &'static str,
        open_step: &'static str,
        deadline: Instant,
    ) -> TestResult<Corpse> {
        // Rat drops are random. Another real kill is the ordinary route after an empty roll.
        for _ in 0..5 {
            *step = kill_step;
            let killed = self.kill(client, deadline, &BTreeSet::from([RAT])).await?;
            *step = open_step;
            let corpse = Self::open(client, killed).await?;
            if !corpse.entries.is_empty() {
                return Ok(corpse);
            }
        }
        Err("five rat corpse USE replies opened empty loot views".into())
    }
    pub(super) async fn hunt_sources(
        &self,
        client: &mut DevClientSession,
        deadline: Instant,
        sources: &BTreeSet<&str>,
    ) -> TestResult<Corpse> {
        let killed = self.kill(client, deadline, sources).await?;
        Self::open(client, killed).await
    }

    async fn open(
        client: &mut DevClientSession,
        (entity, handle): (WorldSpatialEntity, ItemHandle),
    ) -> TestResult<Corpse> {
        let used = client.use_item(handle).await?;
        if used.status != CommandStatus::Accepted || used.disposition != UseDisposition::Committed {
            return Err(format!("corpse USE reply={used:?};corpse={entity:?}").into());
        }
        settle(client, |client| {
            Ok(client
                .open_container()
                .is_some_and(|view| view.container_handle == Some(handle)))
        })
        .await?;
        let entries = client
            .open_container()
            .ok_or("corpse view absent")?
            .entries
            .clone();
        Ok(Corpse {
            entity,
            handle,
            entries,
        })
    }
}

async fn relog(
    client: &mut Option<DevClientSession>,
    clients: &SeamClients<'_>,
    tag: u8,
    fixture_position: Option<TilePos>,
) -> TestResult {
    let live = client.as_mut().ok_or("connection closed")?;
    let old = live.join_snapshot().game_session_id;
    let deadline = Instant::now() + Duration::from_secs(65);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    loop {
        let (result, reply, ..) = logout_retrying_busy(live).await?;
        match result {
            Some(LogoutResult::Accepted) => break,
            Some(LogoutResult::InFight { retry_after_ms })
                if Instant::now() + Duration::from_millis(u64::from(retry_after_ms)) < deadline =>
            {
                live.service_liveness(Duration::from_millis(u64::from(retry_after_ms)))
                    .await?;
                live.take_events();
            }
            _ => return Err(format!("logout result={reply}").into()),
        }
    }
    drop(client.take());
    if let Some(at) = fixture_position {
        super::reset_core_fixture_position(
            clients,
            ActorPosition {
                x: i32::from(at.x),
                y: i32::from(at.y),
                floor: -i16::from(at.floor),
            },
        )
        .await?;
    }
    *client = Some(connect_world(clients, tag).await?);
    let fresh = client.as_ref().ok_or("fresh inventory admission")?;
    if fresh.join_snapshot().game_session_id == old {
        return Err(format!("fresh admission reused session {old:?}").into());
    }
    if let Some(expected) = fixture_position {
        let actual = tile(fresh.world_spatial().actor_position)?;
        if actual != expected {
            return Err(
                format!("rat hunt fixture expected {expected:?}, arrived at {actual:?}").into(),
            );
        }
    }
    Ok(())
}

fn loot_deadline() -> Instant {
    let deadline = Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(deadline, "rat-hunt-budget");
    deadline
}

/// Bind the durable MINT to the exact corpse seen by the client before its Ground row is removed.
async fn corpse_materialization(
    clients: &SeamClients<'_>,
    corpse: &Corpse,
) -> TestResult<Option<(String, i64)>> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let scope = [
        clients.world.as_slice(),
        clients.channel.as_slice(),
        &clients.scope_generation.to_be_bytes(),
    ]
    .concat();
    Ok(sqlx::query_as(
        "SELECT m.item_instance_id::text, m.materialized_at
         FROM game_item_mint_receipts m JOIN game_item_ground_locations g USING (item_instance_id)
         WHERE m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0
         AND m.loot_table_production_key='oteryn:item.tibia.i5964'
         AND octet_length(g.corpse_ref)=52 AND substring(g.corpse_ref FROM 1 FOR 40)=$1
         AND substring(sha256(convert_to('oteryn:runtime-corpse-placement:v1','UTF8')
             || g.corpse_ref) FROM 1 FOR 16)=$2",
    )
    .bind(scope)
    .bind(corpse.entity.entity.identity.as_slice())
    .fetch_optional(&mut database)
    .await?)
}

async fn corpse_retirement(
    clients: &SeamClients<'_>,
    materialization: Option<(String, i64)>,
) -> TestResult<Option<(i64, i64, i64)>> {
    let Some((item, materialized_at)) = materialization else {
        return Ok(None);
    };
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let retirement: Option<(i64, i64)> = sqlx::query_as(
        "SELECT deadline, committed_at FROM game_item_decay_retire_receipts
         WHERE item_instance_id=$1::text::uuid AND corpse_item_instance_id=item_instance_id
         AND cause_kind='CORPSE_DECAY'",
    )
    .bind(item)
    .fetch_optional(&mut database)
    .await?;
    Ok(retirement.map(|(deadline, retired_at)| (materialized_at, deadline, retired_at)))
}

fn verify_corpse_retirement_timing(
    step: &mut &'static str,
    leave_sequence: u64,
    stale_use: UseDisposition,
    timing: Option<(i64, i64, i64)>,
) -> TestResult {
    *step = "verify-server-corpse-retirement-timing";
    let Some((materialized_at, deadline, retired_at)) = timing else {
        return Err(format!("SERVER-GAP missing server timestamps for rat corpse materialization and retirement; leave-sequence={leave_sequence}; stale-use={stale_use:?}; 60s fallback at 1357_instance_corpse_mint.sql:164; enter/leave projection at world_spatial_entities.rs:188").into());
    };
    if materialized_at.checked_add(60_000) != Some(deadline) || retired_at < deadline {
        return Err(format!("rat corpse 60s fallback timing: materialized-at={materialized_at} deadline={deadline} retired-at={retired_at} leave-sequence={leave_sequence} stale-use={stale_use:?}").into());
    }
    Ok(())
}

async fn decay(
    client: &mut Option<DevClientSession>,
    clients: &SeamClients<'_>,
    hunt: &Hunt<'_>,
    step: &mut &'static str,
    town: TilePos,
) -> TestResult {
    *step = "stage-rat-hunt";
    let live = client.as_mut().ok_or("connection closed")?;
    travel(live, hunt.world, |at| at == town).await?;
    relog(client, clients, 0xc5, Some(RAT_HUNT_START)).await?;
    let live = client.as_mut().ok_or("connection closed")?;
    let corpse = hunt
        .lootable(live, step, "kill", "leave-loot", loot_deadline())
        .await?;
    let materialization = corpse_materialization(clients, &corpse).await?;
    let before = quantities(live)?;
    let at = tile(corpse.entity.position)?;
    let stationary = live.world_spatial().actor_position;
    *step = "observe-decay";
    let deadline = Instant::now() + Duration::from_secs(75);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let leave_sequence = loop {
        let position = live.world_spatial().actor_position;
        if position != stationary {
            return Err(format!("actor left the decay observation cell;expected={stationary:?};observed={position:?}").into());
        }
        if let Some(sequence) = live
            .take_events()
            .into_iter()
            .find_map(|event| match event {
                SessionEvent::WorldSpatialEntities(delta)
                    if delta.value.leave.contains(&corpse.entity.entity) =>
                {
                    Some(delta.server_sequence)
                }
                _ => None,
            })
        {
            break sequence;
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "no leave delta at {at:?};corpse={:?};loot={:?}",
                corpse.entity.entity, corpse.entries
            )
            .into());
        }
        live.service_liveness(Duration::from_millis(100)).await?;
    };
    // A receipt first read after fresh admission could belong to a retirement
    // that happened after the observed leave. Sample it at the leave boundary.
    let timing = corpse_retirement(clients, materialization).await;
    *step = "retired-loot-unavailable";
    let stale_use = live.use_item(corpse.handle).await?;
    if stale_use.status != CommandStatus::Accepted
        || stale_use.disposition != UseDisposition::StaleState
    {
        return Err(format!("retired corpse USE reply={stale_use:?}").into());
    }
    for item in &corpse.entries {
        let reply = live
            .move_item(&oteryn_dev_client::ItemMoveIntent {
                source: item.handle,
                destination: oteryn_dev_client::ItemMoveDestination::MainBackpack,
            })
            .await?;
        if reply.outcome != ItemMoveOutcome::Stale {
            return Err(format!("retired loot MOVE handle={} reply={reply:?}", item.handle).into());
        }
    }
    if quantities(live)? != before {
        return Err(format!(
            "retired loot changed inventory;before={before:?};after={:?}",
            quantities(live)?
        )
        .into());
    }
    *step = "relogin-retirement";
    travel(live, hunt.world, |cell| cell == town).await?;
    relog(client, clients, 0xc6, None).await?;
    let fresh = client.as_mut().ok_or("fresh connection absent")?;
    travel(fresh, hunt.world, |cell| nearby(cell, at)).await?;
    if fresh
        .world_entities()
        .ok_or("fresh spatial projection absent")?
        .iter()
        .any(|entity| {
            entity.kind == EntityKind::Corpse
                && entity.entity.identity == corpse.entity.entity.identity
        })
        || quantities(fresh)? != before
    {
        return Err(format!(
            "fresh admission restored corpse or loot;cell={at:?};inventory={:?}",
            quantities(fresh)?
        )
        .into());
    }
    *step = "verify-server-corpse-retirement-timing";
    let timing = timing?;
    verify_corpse_retirement_timing(step, leave_sequence, stale_use.disposition, timing)?;
    if let Some((materialized_at, deadline, retired_at)) = timing {
        evidence(&format!(
            "I06 corpse={:?} leave_sequence={leave_sequence} materialized_at={materialized_at} deadline={deadline} retired_at={retired_at}",
            corpse.entity.entity.identity
        ));
    }
    Ok(())
}

fn request(action: Action) -> loot::QuickLootRequest {
    loot::QuickLootRequest {
        action,
        corpse: None,
        category: None,
        recipient: None,
        recipient_id: None,
        fallback_to_main: false,
        filter_mode: None,
        filter_definition_ref: None,
    }
}

async fn quick(
    client: &mut DevClientSession,
    request: loot::QuickLootRequest,
) -> TestResult<(CommandStatus, loot::QuickLootResult)> {
    let payload = loot::encode_quick_loot_request(&request)
        .map_err(|error| format!("quick loot encode: {error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            loot::CAPABILITY_QUICK_LOOT_V1,
            loot::COMMAND_TYPE_QUICK_LOOT,
            &payload,
        )
        .await?;
    let result = loot::decode_quick_loot_result(&response.payload)
        .map_err(|error| format!("quick loot decode: {error:?}"))?;
    Ok((response.status, result))
}

async fn filter(
    client: &mut DevClientSession,
    action: Action,
    mode: Filter,
    reference: Option<u32>,
) -> TestResult {
    let (status, result) = quick(
        client,
        loot::QuickLootRequest {
            filter_mode: Some(mode),
            filter_definition_ref: reference,
            ..request(action)
        },
    )
    .await?;
    if status != CommandStatus::Accepted || !result.items.is_empty() {
        return Err(
            format!("filter {action:?}/{mode:?}/{reference:?}: {status:?} {result:?}").into(),
        );
    }
    Ok(())
}

async fn loot_entries(
    client: &mut DevClientSession,
    corpse: &mut Corpse,
    action: Action,
    admits: impl Fn(&ItemEntry) -> bool,
) -> TestResult {
    let mut expected = backpack(client).await?;
    let taking = corpse.entries.iter().filter(|entry| admits(entry)).count();
    for entry in corpse.entries.iter().filter(|entry| admits(entry)) {
        *expected.entry(entry.item_definition_ref.get()).or_default() +=
            u64::from(entry.count.get());
    }
    let (status, result) = quick(
        client,
        loot::QuickLootRequest {
            corpse: (action == Action::Click).then_some(corpse.entity.entity.identity),
            ..request(action)
        },
    )
    .await?;
    let unique = result
        .items
        .iter()
        .map(|entry| entry.item)
        .collect::<std::collections::BTreeSet<_>>();
    if status != CommandStatus::Accepted
        || result.items.len() != corpse.entries.len()
        || unique.len() != result.items.len()
        || result.items.iter().filter(|entry| entry.accepted).count() != taking
    {
        return Err(format!(
            "{action:?} {status:?} {result:?};expected picked={taking};corpse={:?}",
            corpse.entries
        )
        .into());
    }
    corpse.entries.retain(|entry| !admits(entry));
    // Explicitly reopen to refresh the corpse even when a command's delta only carries inventory.
    let used = client.use_item(corpse.handle).await?;
    if used.status != CommandStatus::Accepted || used.disposition != UseDisposition::Committed {
        return Err(format!("{action:?} corpse refresh USE reply={used:?}").into());
    }
    settle(client, |client| {
        Ok(client.open_container().is_some_and(|open| {
            open.container_handle == Some(corpse.handle) && open.entries == corpse.entries
        }))
    })
    .await?;
    let observed = backpack(client).await?;
    if observed != expected {
        return Err(
            format!("{action:?} backpack expected={expected:?};observed={observed:?}").into(),
        );
    }
    Ok(())
}

async fn view_command(
    client: &mut DevClientSession,
    intent: tree::ContainerViewIntent,
    expected: tree::ContainerViewOutcome,
) -> TestResult {
    let payload = tree::encode_container_view_intent(&intent)
        .map_err(|error| format!("container intent: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            tree::CAPABILITY_CONTAINER_TREE_V1,
            tree::COMMAND_TYPE_CONTAINER_VIEW_INTENT,
            &payload,
        )
        .await?;
    let outcome = tree::decode_container_view_result(&reply.payload)
        .map_err(|error| format!("container reply: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || outcome != expected {
        return Err(format!(
            "container intent={intent:?} status={:?} outcome={outcome:?};expected={expected:?}",
            reply.status
        )
        .into());
    }
    Ok(())
}

fn visible_container(
    client: &mut DevClientSession,
    handle: ItemHandle,
) -> Option<&tree::ContainerView> {
    client
        .session_mut()
        .container_views()?
        .views
        .iter()
        .find(|view| view.container_handle == handle)
}

async fn container(
    client: &mut DevClientSession,
    handle: ItemHandle,
) -> TestResult<Vec<ItemEntry>> {
    if let Some(view_id) = visible_container(client, handle).map(|view| view.view_id) {
        view_command(
            client,
            tree::ContainerViewIntent::Close { view_id },
            tree::ContainerViewOutcome::Closed,
        )
        .await?;
        settle(client, |client| {
            Ok(visible_container(client, handle).is_none())
        })
        .await?;
    }
    view_command(
        client,
        tree::ContainerViewIntent::Open {
            handle,
            replace_view: None,
        },
        tree::ContainerViewOutcome::Opened,
    )
    .await?;
    settle(client, |client| {
        Ok(visible_container(client, handle).is_some())
    })
    .await?;
    Ok(visible_container(client, handle)
        .ok_or("opened container view absent")?
        .entries
        .clone())
}

fn totals(entries: &[ItemEntry]) -> BTreeMap<u32, u64> {
    let mut totals = BTreeMap::new();
    for entry in entries {
        *totals.entry(entry.item_definition_ref.get()).or_default() += u64::from(entry.count.get());
    }
    totals
}

async fn backpack(client: &mut DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let handle = client
        .inventory()
        .and_then(|inventory| inventory.main_backpack)
        .ok_or("backpack absent")?
        .handle;
    Ok(totals(&container(client, handle).await?))
}

async fn gold_destination(client: &mut DevClientSession) -> TestResult {
    let assign_main = loot::QuickLootRequest {
        category: Some(loot::QuickLootCategory::Gold),
        recipient: Some(loot::QuickLootRecipient::MainContainer),
        ..request(Action::Assign)
    };
    let (status, reply) = quick(client, assign_main).await?;
    if status != CommandStatus::Accepted || !reply.items.is_empty() {
        return Err(format!("Gold/MainContainer assignment refused: {status:?} {reply:?}").into());
    }
    let present = provisioning::item_reference(&provisioning::item_index()?, 2855)?;
    let handle = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == present)
        })
        .ok_or("category routing needs a carried present from ordinary trade")?
        .handle;
    let assign = loot::QuickLootRequest {
        action: Action::AssignLoot,
        recipient: Some(loot::QuickLootRecipient::Container),
        ..assign_main
    };
    let payload = loot::encode_quick_loot_view_request(&assign, handle)
        .map_err(|error| format!("Gold/container handle command: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            loot::CAPABILITY_QUICK_LOOT_DESTINATIONS_V1,
            assign.action.command_type(),
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted || !reply.payload.is_empty() {
        return Err(format!("Gold/container assignment refused: {:?}", reply.status).into());
    }
    let reviewed = async {
        let action = Action::ReviewDestinations;
        let payload = loot::encode_quick_loot_request(&request(action))
            .map_err(|error| format!("quick-loot review command: {error:?}"))?;
        let reply = client
            .session_mut()
            .gated_command(
                loot::CAPABILITY_QUICK_LOOT_DESTINATIONS_V1,
                action.command_type(),
                &payload,
            )
            .await?;
        if reply.status != CommandStatus::Accepted {
            return Err(
                format!("quick-loot destination review refused: {:?}", reply.status).into(),
            );
        }
        let destinations = loot::decode_quick_loot_destinations(&reply.payload)
            .map_err(|error| format!("quick-loot destination review: {error:?}"))?;
        let gold = destinations
            .loot
            .iter()
            .find(|row| row.category == assign.category);
        if !gold.is_some_and(|row| {
            row.recipient == loot::QuickLootRecipient::Container && row.recipient_id.is_some()
        }) {
            return Err(
                format!("Gold/container destination missing from live review: {gold:?}").into(),
            );
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    }
    .await;
    // Later loot probes expect Gold to land in the main backpack.
    let (status, reply) = quick(client, assign_main).await?;
    if status != CommandStatus::Accepted || !reply.items.is_empty() {
        return Err(format!("failed to restore Gold/MainContainer: {status:?} {reply:?}").into());
    }
    reviewed?;
    evidence("I07 Gold/container assignment and review verified through a carried item handle");
    Ok(())
}

async fn loot_policy(
    client: &mut Option<DevClientSession>,
    clients: &SeamClients<'_>,
    hunt: &Hunt<'_>,
    step: &mut &'static str,
    town: TilePos,
) -> TestResult {
    let live = client.as_mut().ok_or("connection closed")?;
    let deadline = loot_deadline();
    let mut corpse = hunt
        .lootable(live, step, "loot-all", "loot-all", deadline)
        .await?;
    *step = "manual-click";
    let before = quantities(live)?;
    let non_corpse = *live
        .world_entities()
        .ok_or("Click needs the served own-actor identity")?
        .own_identity();
    let (status, reply) = quick(
        live,
        loot::QuickLootRequest {
            corpse: Some(non_corpse),
            ..request(Action::Click)
        },
    )
    .await?;
    let refreshed = live.use_item(corpse.handle).await?;
    if refreshed.status != CommandStatus::Accepted
        || refreshed.disposition != UseDisposition::Committed
    {
        return Err(format!("Click corpse readback refused: {refreshed:?}").into());
    }
    settle(live, |live| {
        Ok(live.open_container().is_some_and(|view| {
            view.container_handle == Some(corpse.handle) && view.entries == corpse.entries
        }))
    })
    .await?;
    if status != CommandStatus::Accepted || !reply.items.is_empty() || quantities(live)? != before {
        return Err(format!(
            "negative Click changed custody or did not return an empty result: {status:?}/{reply:?}"
        )
        .into());
    }
    evidence(&format!(
        "I07 negative Click spatial={:?} status={status:?} reply={reply:?}",
        non_corpse
    ));
    loot_entries(live, &mut corpse, Action::Click, |_| true).await?;
    corpse = hunt
        .lootable(live, step, "loot-all", "loot-all", deadline)
        .await?;
    *step = "accept-skip-filters";
    filter(live, Action::FilterClear, Filter::Skip, None).await?;
    let refs: std::collections::BTreeSet<_> = corpse
        .entries
        .iter()
        .map(|item| item.item_definition_ref.get())
        .collect();
    for reference in refs {
        filter(live, Action::FilterAdd, Filter::Skip, Some(reference)).await?;
    }
    loot_entries(live, &mut corpse, Action::LootAll, |_| false).await?;
    filter(live, Action::FilterClear, Filter::Accept, None).await?;
    loot_entries(live, &mut corpse, Action::LootAll, |_| false).await?;
    let reference = corpse
        .entries
        .first()
        .ok_or("corpse lost its filtered loot")?
        .item_definition_ref
        .get();
    filter(live, Action::FilterAdd, Filter::Accept, Some(reference)).await?;
    *step = "loot-all";
    loot_entries(live, &mut corpse, Action::LootAll, |entry| {
        entry.item_definition_ref.get() == reference
    })
    .await?;
    // Leave an empty active accept list. Prove the mode by attempting real loot
    // after admission without first sending any policy writes on that session.
    *step = "relogin-policy-custody";
    filter(live, Action::FilterClear, Filter::Accept, None).await?;
    filter(live, Action::FilterClear, Filter::Skip, None).await?;
    if !corpse.entries.is_empty() {
        loot_entries(live, &mut corpse, Action::LootAll, |_| true).await?;
    }
    filter(live, Action::FilterSelect, Filter::Accept, None).await?;
    let saved = backpack(live).await?;
    travel(live, hunt.world, |at| at == town).await?;
    relog(client, clients, 0xc7, None).await?;
    let fresh = client.as_mut().ok_or("fresh connection absent")?;
    if quantities(fresh)? != saved {
        return Err(format!(
            "fresh inventory differs;expected={saved:?};observed={:?}",
            quantities(fresh)?
        )
        .into());
    }
    let mut corpse = hunt
        .lootable(
            fresh,
            step,
            "relogin-policy-custody",
            "relogin-policy-custody",
            loot_deadline(),
        )
        .await?;
    loot_entries(fresh, &mut corpse, Action::LootAll, |_| false).await?;
    filter(fresh, Action::FilterSelect, Filter::Skip, None).await?;
    loot_entries(fresh, &mut corpse, Action::LootAll, |_| true).await?;
    Ok(())
}

async fn shop_quiver(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    refs: [std::num::NonZeroU32; 2],
) -> TestResult {
    use oteryn_dev_client::client_runtime::npc_shop as shop;
    use oteryn_protocol_oteryn::npc_service as wire;
    // Both offers occur in these Crystal merchant catalogues. Confirm the
    // selected merchant's current window and prices over the wire before buying.
    let catalogues: serde_json::Value = serde_json::from_slice(&std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/services/trade/trade-00000-00323.json"),
    )?)?;
    let sellers: std::collections::BTreeSet<_> = catalogues["records"]
        .as_array()
        .ok_or("authored trade catalogues absent")?
        .iter()
        .filter_map(|record| {
            let declaration = &record["declaration"];
            let offers = declaration["offers"].as_array()?;
            ["oteryn:item.tibia.i35562", "oteryn:item.tibia.i3447"]
                .into_iter()
                .all(|key| {
                    offers.iter().any(|offer| {
                        offer["direction"] == "SellToPlayer" && offer["item"]["key"] == key
                    })
                })
                .then(|| declaration["identity"]["key"].as_str())?
                .and_then(|key| {
                    key.strip_prefix("oteryn:service.trade.")
                        .map(|name| format!("oteryn:npc.{name}"))
                })
        })
        .collect();
    let here = tile(client.world_spatial().actor_position)?;
    let in_reach = |at: TilePos, p: &oteryn_world_bundle::npc::Placement| {
        at.floor == p.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3
    };
    let (npc, path) = npcs.npcs.iter().filter(|npc| sellers.contains(&npc.key))
        .filter_map(|npc| {
            route_avoiding(world, here,
                |at| npc.placements.iter().any(|p| in_reach(at, p)),
                |at| route_available(at) && at != WOLF_TILE && !npc.placements.iter().any(|p| p.x == at.x && p.y == at.y && p.floor == at.floor),
            ).map(|path| (npc, path))
        }).min_by_key(|(_, path)| path.len())
        .ok_or_else(|| format!("NPC trade route from {here:?} to {sellers:?} unreachable for quiver 35562 / arrow 3447"))?;
    walk(client, world, &path).await?;
    let name = npc
        .key
        .strip_prefix("oteryn:npc.")
        .ok_or("quiver NPC key prefix")?
        .replace('_', " ");
    let actor = approach_vendor(client, world, &name, |at| {
        npc.placements.iter().any(|p| {
            near_vendor_placement(
                at,
                TilePos {
                    x: p.x,
                    y: p.y,
                    floor: p.floor,
                },
            )
        })
    })
    .await?;
    for text in ["hi", "trade"] {
        let reply = shop::talk(
            client.session_mut(),
            &wire::NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await?;
        if reply != wire::NpcIntentDisposition::Ok {
            return Err(format!("seller {} {text} reply={reply:?}", npc.key).into());
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = wire::decode_npc_trade_window(&delta.payload)
        .map_err(|error| format!("trade window: {error:?}"))?;
    let (_, revision) = window.open.ok_or("live trade window closed")?;
    for reference in refs {
        let (i, offer) = window
            .offers
            .iter()
            .enumerate()
            .find(|(_, offer)| {
                offer.side == wire::NpcTradeSide::Buy
                    && offer.item_definition_ref == reference.get()
            })
            .ok_or_else(|| {
                format!(
                    "seller {} has no live buy offer definition={reference}",
                    npc.key
                )
            })?;
        let before = quantities(client)?
            .get(&reference.get())
            .copied()
            .unwrap_or(0);
        let reply = shop::buy(
            client.session_mut(),
            &wire::NpcTradeIntent {
                npc_actor: actor,
                catalogue_revision: revision,
                offer_index: u32::try_from(i)?,
                side: offer.side,
                quantity: 1,
                expected_unit_price: offer.unit_price,
            },
        )
        .await?;
        if reply != wire::NpcIntentDisposition::Ok {
            return Err(format!(
                "seller {} buy definition={reference} price={} reply={reply:?}",
                npc.key, offer.unit_price
            )
            .into());
        }
        settle(client, |client| {
            Ok(quantities(client)?.get(&reference.get()).copied()
                == Some(before + u64::from(offer.count)))
        })
        .await?;
    }
    Ok(())
}

fn equipped_quiver(
    client: &DevClientSession,
    definition: std::num::NonZeroU32,
) -> Option<ItemHandle> {
    client
        .inventory()?
        .equipment
        .iter()
        .find(|entry| entry.item.item_definition_ref == definition)
        .map(|entry| entry.item.handle)
}

async fn quiver(
    client: &mut Option<DevClientSession>,
    clients: &SeamClients<'_>,
    hunt: &Hunt<'_>,
    npcs: &oteryn_world_bundle::npc::Table,
    town: TilePos,
) -> TestResult {
    let index = provisioning::item_index()?;
    let quiver = provisioning::item_reference(&index, 35562)?;
    let arrow = provisioning::item_reference(&index, 3447)?;
    let live = client.as_mut().ok_or("connection closed")?;
    let coins = [
        (provisioning::item_reference(&index, 3031)?.get(), 1),
        (provisioning::item_reference(&index, 3035)?.get(), 100),
        (provisioning::item_reference(&index, 3043)?.get(), 10_000),
    ];
    let balance = |inventory: &BTreeMap<u32, u64>| {
        coins
            .iter()
            .map(|(reference, value)| inventory.get(reference).copied().unwrap_or(0) * value)
            .sum::<u64>()
    };
    let deadline = loot_deadline();
    loop {
        let gold = balance(&backpack(live).await?);
        if gold >= 403 {
            break;
        }
        if Instant::now() >= deadline {
            return Err(format!("rat-loot funding deadline;carried-gold={gold};budget=403 for quiver 35562 and arrow 3447").into());
        }
        let mut step = "quiver-ammunition";
        let corpse = hunt
            .lootable(
                live,
                &mut step,
                "quiver-ammunition",
                "quiver-ammunition",
                deadline,
            )
            .await?;
        for item in corpse.entries.iter().filter(|item| {
            coins
                .iter()
                .any(|(reference, _)| *reference == item.item_definition_ref.get())
        }) {
            let reply = live
                .move_item(&oteryn_dev_client::ItemMoveIntent {
                    source: item.handle,
                    destination: oteryn_dev_client::ItemMoveDestination::MainBackpack,
                })
                .await?;
            if reply.outcome != ItemMoveOutcome::Moved {
                return Err(format!("rat coin MOVE item={item:?} reply={reply:?}").into());
            }
        }
    }
    shop_quiver(live, hunt.world, npcs, [quiver, arrow]).await?;
    use oteryn_dev_client::client_runtime::hotkey_equip::equip_definition;
    let reply = equip_definition(live.session_mut(), quiver).await?;
    if reply != ItemMoveOutcome::Moved {
        return Err(format!("quiver 35562 equip reply={reply:?}").into());
    }
    settle(live, |live| Ok(equipped_quiver(live, quiver).is_some())).await?;
    let handle = equipped_quiver(live, quiver).ok_or("equipped quiver absent")?;
    let before = totals(&container(live, handle).await?);
    let mut carried = quantities(live)?;
    let moved = u64::from(
        live.inventory()
            .and_then(|inventory| {
                inventory
                    .entries
                    .iter()
                    .find(|entry| entry.item_definition_ref == arrow)
            })
            .ok_or("purchased arrows absent from backpack")?
            .count
            .get(),
    );
    let remaining = carried.get_mut(&arrow.get()).ok_or("arrow total absent")?;
    *remaining -= moved;
    if *remaining == 0 {
        carried.remove(&arrow.get());
    }
    let mut expected = before;
    *expected.entry(arrow.get()).or_default() += moved;
    let reply = equip_definition(live.session_mut(), arrow).await?;
    if reply != ItemMoveOutcome::Moved {
        return Err(format!("arrow 3447 into equipped quiver reply={reply:?}").into());
    }
    settle(live, |live| Ok(quantities(live)? == carried)).await?;
    let observed = totals(&container(live, handle).await?);
    if observed != expected {
        return Err(format!("quiver credit expected={expected:?};observed={observed:?}").into());
    }
    travel(live, hunt.world, |at| at == town).await?;
    relog(client, clients, 0xc8, None).await?;
    let fresh = client.as_mut().ok_or("fresh connection absent")?;
    let handle = equipped_quiver(fresh, quiver).ok_or("quiver absent after admission")?;
    let actual = totals(&container(fresh, handle).await?);
    if actual != expected || quantities(fresh)? != carried {
        return Err(format!(
            "fresh quiver credit expected={expected:?};observed={actual:?};backpack={:?}",
            quantities(fresh)?
        )
        .into());
    }
    Ok(())
}

fn result(row: &str, started: Instant, pass: &str, gap: Option<(&str, &str, &verdict::Cause)>) {
    let elapsed = started.elapsed().as_secs_f64();
    if let Some((step, observed, cause)) = gap {
        let suffix = cause.suffix(&format!("{row}/{step}"));
        evidence(&format!(
            "e2e_all scenario={row}.full status=FAIL seconds={elapsed:.3}{suffix}"
        ));
        evidence(&format!(
            "E2EF3_GAP row={row} unexecuted={step} observed={}",
            observed.replace(['\r', '\n'], " "),
        ));
    } else {
        evidence(&format!(
            "e2e_all scenario={row}.full status=PASS seconds={elapsed:.3} evidence={pass}"
        ));
    }
}

fn setup_failure(started: Instant, error: &(dyn std::error::Error + 'static)) {
    let cause = verdict::classify(error);
    let observed = error.to_string();
    for (row, step) in [(I06, "kill"), (I07, "manual-click")] {
        result(row, started, "", Some((step, &observed, &cause)));
    }
}

pub(super) async fn run(
    starter: TestResult<DevClientSession>,
    clients: &SeamClients<'_>,
) -> TestResult<DevClientSession> {
    let started = Instant::now();
    let live = match starter {
        Ok(client) => client,
        // The parent observation owns the session and drops it on error.
        Err(_) => match connect_world(clients, 0xc5).await {
            Ok(client) => client,
            Err(error) => {
                setup_failure(started, error.as_ref());
                return Err(error);
            }
        },
    };
    let loaded = (|| {
        let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
        let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
        let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
        let world = load_world(
            bytes,
            &pin,
            WorldId::decode(&clients.world)?,
            ChannelId::decode(&clients.channel)?,
        )?;
        Ok::<_, Box<dyn std::error::Error>>((metadata, world))
    })();
    let (metadata, world) = match loaded {
        Ok(value) => value,
        Err(error) => {
            setup_failure(started, error.as_ref());
            return Ok(live);
        }
    };
    let town = TOWN;
    let hunt = Hunt {
        world,
        spawns: &metadata.spawns,
    };
    let mut client = Some(live);
    let mut step = "kill";
    match decay(&mut client, clients, &hunt, &mut step, town).await {
        Ok(()) => result(
            I06,
            started,
            "rat-kill-untouched-loot-decay-stale-use-move-fresh-absence",
            None,
        ),
        Err(error) => {
            result(
                I06,
                started,
                "",
                Some((step, &error.to_string(), &verdict::classify(error.as_ref()))),
            );
        }
    }

    let started = Instant::now();
    let mut step = "loot-all";
    let policy = loot_policy(&mut client, clients, &hunt, &mut step, town).await;
    let policy_failure = policy
        .as_ref()
        .err()
        .map(|error| (step, verdict::classify(error.as_ref())));
    let policy = policy.map_or_else(
        |error| format!("{step}={error}"),
        |()| "manual-click,loot-all,accept-skip-filters,relogin-policy-custody=verified".into(),
    );

    let category = match client.as_mut() {
        Some(live) => gold_destination(live).await,
        None => Err("connection closed".into()),
    };
    let category_failure = category
        .as_ref()
        .err()
        .map(|error| ("category-routing", verdict::classify(error.as_ref())));
    let category = category.map_or_else(
        |error| error.to_string(),
        |()| "Gold/MainContainer and carried category destination accepted and reviewed".into(),
    );
    let ammunition = quiver(&mut client, clients, &hunt, &metadata.npcs, town).await;
    let ammunition_failure = ammunition
        .as_ref()
        .err()
        .map(|error| ("quiver-ammunition", verdict::classify(error.as_ref())));
    let ammunition = ammunition.map_or_else(
        |error| error.to_string(),
        |()| "NPC buy, ammunition credit, and fresh custody verified".into(),
    );

    let observed = format!(
        "{policy}; category-routing={category} (item_quick_loot.rs:170); \
        gold-pouch=23721 requires Inbox custody and durable recipient UUID (item_quick_loot.rs:88); \
        no served NPC buy offer or starter Inbox acquisition; quiver-ammunition={ammunition}"
    );
    let (failed_step, cause) = policy_failure
        .or(category_failure)
        .or(ammunition_failure)
        .unwrap_or_else(|| {
            (
                "gold-pouch",
                verdict::Cause::Precondition {
                    name: "quick-loot-gold-pouch-acquisition".into(),
                },
            )
        });
    result(I07, started, "", Some((failed_step, &observed, &cause)));

    let mut client = client.ok_or("E2EF3 connection closed during fresh admission")?;
    travel(&mut client, world, |at| at == TOWN)
        .await
        .map_err(|error| format!("I07 return to town: {error}"))?;
    if tile(client.world_spatial().actor_position)? != TOWN {
        return Err("I07 return did not reach town".into());
    }
    Ok(client)
}

#[test]
fn rat_retirement_requires_server_timestamps() {
    let mut step = "relogin-retirement";
    let result = verify_corpse_retirement_timing(&mut step, 17, UseDisposition::StaleState, None);
    assert_eq!(step, "verify-server-corpse-retirement-timing");
    assert!(result.is_err_and(|error| {
        let error = error.to_string();
        error.contains("missing server timestamps")
            && error.contains("leave-sequence=17")
            && error.contains("stale-use=StaleState")
            && error.contains("60s fallback")
    }));
}

#[test]
fn rat_retirement_requires_the_exact_server_deadline() -> TestResult {
    let mut step = "relogin-retirement";
    let verify = |step: &mut &'static str, materialized_at, deadline, retired_at| {
        verify_corpse_retirement_timing(
            step,
            17,
            UseDisposition::StaleState,
            Some((materialized_at, deadline, retired_at)),
        )
    };
    assert!(verify(&mut step, 1000, 61_000, 60_999).is_err());
    assert!(verify(&mut step, 1000, 61_001, 61_001).is_err());
    verify(&mut step, 1000, 61_000, 61_037)
}

#[test]
fn vendor_placement_allows_two_tiles_of_wander_on_same_floor() {
    let at = TOWN;
    assert!(near_vendor_placement(
        TilePos {
            x: at.x + 2,
            y: at.y - 2,
            ..at
        },
        at
    ));
    assert!(!near_vendor_placement(TilePos { x: at.x + 3, ..at }, at));
    assert!(!near_vendor_placement(
        TilePos {
            floor: at.floor - 1,
            ..at
        },
        at
    ));
}

#[test]
fn hunt_landing_rejects_a_visible_occupant() {
    let adjacent = TilePos {
        x: TOWN.x + 1,
        ..TOWN
    };
    let occupied = BTreeSet::from([adjacent]);
    assert!(!hunt_landing_available(adjacent, &occupied));
    assert!(hunt_landing_available(TOWN, &occupied));
    assert!(!hunt_landing_available(WOLF_TILE, &BTreeSet::new()));
}
