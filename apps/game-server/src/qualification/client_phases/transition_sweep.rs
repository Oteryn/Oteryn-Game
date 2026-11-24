//! Full-World transition census and native client-wire checks, selected only through
//! NODE_BOOT_E2E_ONLY=transition_sweep. TS.<key>.<step> rows use disposable Character
//! prerequisites (mode=seeded) and fail the journey if any step fails.
//!
//! Oracles: authored Transition endpoints in content/world/transitions/index.json:1;
//! placed floor masks and chained landing in crates/gameplay/world/src/map/floor.rs:91,328;
//! tool landings in crates/gameplay/world/src/map/floor/rope.rs:17,209;
//! door bindings in crates/gameplay/world/src/map/facts.rs:532 and door poses and predicates
//! in crates/gameplay/world/src/map/door.rs:419,476. Door USE and key USE-WITH are owned by
//! crates/gameplay/dispatch/src/gameplay_transport/map_door.rs:179,233.

use super::*;
use crate::map::door::{AuthoredDoorFamily, authored_door};
use crate::map::floor::{
    FloorTable, Ground, is_ladder_appearance, is_rope_spot_item, palette_mask,
};
use crate::movement::CardinalStep;
use oteryn_dev_client::DevClientSession;
use oteryn_gameplay_world::terrain_tools::{TerrainTool, crystal_core_bindings};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent,
};
use oteryn_world_bundle::bundle::{Family, TerrainKind};
use serde::Deserialize;

const GROUP: &str = "transition_sweep";
const BATCH_VARIABLE: &str = "OTERYN_E2E_TRANSITION_SWEEP_BATCH";
const DEFAULT_BATCH: (usize, usize) = (0, 3);
const DIRECTIONS: [(StepDirection, CardinalStep); 8] = [
    (StepDirection::North, CardinalStep::North),
    (StepDirection::East, CardinalStep::East),
    (StepDirection::South, CardinalStep::South),
    (StepDirection::West, CardinalStep::West),
    (StepDirection::NorthEast, CardinalStep::NorthEast),
    (StepDirection::SouthEast, CardinalStep::SouthEast),
    (StepDirection::SouthWest, CardinalStep::SouthWest),
    (StepDirection::NorthWest, CardinalStep::NorthWest),
];

fn transition_batch(value: Option<&str>) -> TestResult<(usize, usize)> {
    let Some(text) = value else {
        return Ok(DEFAULT_BATCH);
    };
    let number = |part: &str| {
        (!part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| part.parse::<usize>().ok())
            .flatten()
    };
    match text
        .split_once(':')
        .and_then(|(start, len)| Some((number(start)?, number(len)?)))
    {
        Some((start, len)) if len > 0 => Ok((start, len)),
        _ => Err(
            format!("invalid {BATCH_VARIABLE}={text:?}: expected start:len with len > 0").into(),
        ),
    }
}

fn batch_from_env() -> TestResult<(usize, usize)> {
    match std::env::var(BATCH_VARIABLE) {
        Ok(text) => transition_batch(Some(&text)),
        Err(std::env::VarError::NotPresent) => transition_batch(None),
        Err(error) => Err(error.into()),
    }
}

#[derive(Deserialize)]
struct ShardIndex {
    record_count: usize,
    shards: Vec<String>,
}

#[derive(Deserialize)]
struct Shard {
    records: Vec<TransitionRecord>,
}

#[derive(Deserialize)]
struct TransitionRecord {
    declaration: Declaration,
}

#[derive(Deserialize)]
struct Declaration {
    identity: Identity,
    from: Position,
    to: Position,
    transition_kind: String,
}

#[derive(Deserialize)]
struct Identity {
    key: String,
}

#[derive(Deserialize)]
struct Position {
    x: u16,
    y: u16,
    floor: u8,
}

impl Position {
    fn tile(self) -> TestResult<TilePos> {
        if self.floor > 15 {
            return Err("authored floor outside the legacy World".into());
        }
        Ok(TilePos {
            x: self.x,
            y: self.y,
            floor: -(self.floor as i8),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gate {
    Ordinary,
    Key(Option<u16>),
    Level(Option<u32>),
    Quest(Option<u16>),
}

fn door_gate(family: AuthoredDoorFamily, action: Option<u16>) -> Gate {
    let action = action.filter(|action| *action != 0);
    match family {
        AuthoredDoorFamily::Level => {
            Gate::Level(action.map(|action| u32::from(action).saturating_sub(1000)))
        }
        AuthoredDoorFamily::Quest => Gate::Quest(action),
        AuthoredDoorFamily::Key { locked } if locked || matches!(action, Some(101 | 1001)) => {
            Gate::Key(action)
        }
        _ => Gate::Ordinary,
    }
}

enum Action {
    StepTo(TilePos),
    Floor,
    Ladder(usize),
    Rope(usize),
    Shovel {
        ordinal: usize,
        mask: u8,
    },
    Door {
        ordinal: usize,
        family: AuthoredDoorFamily,
        action: Option<u16>,
        open: bool,
    },
    Unsupported(String),
}

struct Entry {
    key: String,
    at: TilePos,
    action: Action,
}

fn authored_transitions(root: &Path) -> TestResult<Vec<Entry>> {
    let index: ShardIndex = serde_json::from_slice(&std::fs::read(
        root.join("content/world/transitions/index.json"),
    )?)?;
    let mut entries = Vec::with_capacity(index.record_count);
    for file in index.shards {
        let shard: Shard = serde_json::from_slice(&std::fs::read(root.join(file))?)?;
        for record in shard.records {
            let declaration = record.declaration;
            let target = declaration.to.tile()?;
            let action = match declaration.transition_kind.as_str() {
                "teleport" | "stairs" | "hole" => Action::StepTo(target),
                kind => Action::Unsupported(kind.to_owned()),
            };
            entries.push(Entry {
                key: declaration.identity.key,
                at: declaration.from.tile()?,
                action,
            });
        }
    }
    if entries.len() != index.record_count {
        return Err("transition shard count disagrees with its index".into());
    }
    Ok(entries)
}

fn placement_key(at: TilePos, ordinal: usize) -> Option<u64> {
    crate::map::view::placement_key(at, u8::try_from(ordinal).ok()?)
}

fn census(root: &Path, bytes: &[u8]) -> TestResult<Vec<Entry>> {
    let mut entries = authored_transitions(root)?;
    let shovels: Vec<_> = crystal_core_bindings()
        .into_iter()
        .filter(|binding| binding.tool == TerrainTool::Shovel)
        .collect();
    let mut palette = Vec::new();
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        if palette.is_empty() {
            palette = manifest
                .palette
                .iter()
                .map(|entry| {
                    let id = crate::map::palette_appearance(&entry.key).unwrap_or(0);
                    let ground = entry
                        .terrain
                        .is_some_and(|terrain| terrain.kind == TerrainKind::Ground);
                    (
                        id,
                        palette_mask(entry.family, &entry.key),
                        is_ladder_appearance(id),
                        is_rope_spot_item(u32::from(id), ground),
                        authored_door(u32::from(id)),
                    )
                })
                .collect();
        }
        for tile in sector.tiles {
            let at = TilePos {
                x: tile.x,
                y: tile.y,
                floor: sector.floor,
            };
            let mut add = |kind, ordinal, action| {
                entries.push(Entry {
                    key: format!(
                        "oteryn:transition.world.{kind}.x{}_y{}_z{}.o{ordinal}",
                        at.x, at.y, -at.floor
                    ),
                    at,
                    action,
                });
            };
            let mut floor_mask = 0;
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                let (id, mask, ladder, rope, door) = palette[item.palette as usize];
                floor_mask |= mask;
                if ladder {
                    add("ladder", ordinal, Action::Ladder(ordinal));
                }
                if rope {
                    add("rope", ordinal, Action::Rope(ordinal));
                }
                if let Some((family, open)) = door
                    && (tile.house == 0 || family != AuthoredDoorFamily::Plain)
                {
                    add(
                        "door",
                        ordinal,
                        Action::Door {
                            ordinal,
                            family,
                            action: item.attrs.action,
                            open,
                        },
                    );
                }
                if let Some(binding) = shovels.iter().find(|binding| {
                    binding.target == u32::from(id) && binding.required_action == item.attrs.action
                }) {
                    let mask = palette_mask(
                        Family::Item,
                        &format!("oteryn:item.tibia.i{}", binding.transformed),
                    );
                    add("shovel", ordinal, Action::Shovel { ordinal, mask });
                }
            }
            if floor_mask != 0 {
                add("floor", 0, Action::Floor);
            }
        }
        Ok(())
    })?;
    entries.sort_unstable_by(|left, right| left.key.cmp(&right.key));
    if entries.windows(2).any(|pair| pair[0].key == pair[1].key) {
        return Err("duplicate content transition key".into());
    }
    Ok(entries)
}

fn offset(at: TilePos, dx: i32, dy: i32) -> Option<TilePos> {
    Some(TilePos {
        x: (i32::from(at.x) + dx).try_into().ok()?,
        y: (i32::from(at.y) + dy).try_into().ok()?,
        ..at
    })
}

fn walkable(world: &BundleWorld, at: TilePos) -> bool {
    world.enterable(at)
        && world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.walkable())
}

fn approaches(
    world: &BundleWorld,
    at: TilePos,
) -> impl Iterator<Item = (TilePos, StepDirection, CardinalStep)> + '_ {
    DIRECTIONS.into_iter().filter_map(move |(wire, step)| {
        let (dx, dy) = step.delta();
        let from = offset(at, -dx, -dy)?;
        (walkable(world, from)
            && world.facts().house_id(from).is_none()
            && !world.floors().contains(from)
            && world.floors().teleport(from).is_none())
        .then_some((from, wire, step))
    })
}

fn floor_landing(
    world: &BundleWorld,
    table: &FloorTable,
    from: TilePos,
    direction: CardinalStep,
) -> Option<TilePos> {
    let position = |(x, y, z): (i32, i32, i32)| -> Option<TilePos> {
        Some(TilePos {
            x: x.try_into().ok()?,
            y: y.try_into().ok()?,
            floor: -i8::try_from(z).ok().filter(|z| (0..=15).contains(z))?,
        })
    };
    let ground = |cell| match position(cell) {
        Some(at) if walkable(world, at) => Ground::Walkable,
        Some(at) if world.base().tile(at.x, at.y, at.floor).is_some() => Ground::Blocked,
        _ => Ground::Absent,
    };
    position(
        crate::map::floor::resolve_landing(
            table,
            ground,
            |_| true,
            (i32::from(from.x), i32::from(from.y), -i32::from(from.floor)),
            direction,
        )
        .ok()?,
    )
}

// Same ordered static candidates as the rope/ladder movement owner. Live occupied
// candidates are excluded using the client entity view immediately before USE.
fn upstairs(at: TilePos) -> impl Iterator<Item = TilePos> {
    [
        (0, 1),
        (0, -1),
        (1, 0),
        (-1, 0),
        (-1, 1),
        (1, 1),
        (-1, -1),
        (1, -1),
    ]
    .into_iter()
    .filter_map(move |(dx, dy)| {
        Some(TilePos {
            floor: at.floor.checked_add(1)?.min(0),
            ..offset(at, dx, dy)?
        })
        .filter(|candidate| candidate.floor > at.floor)
    })
}

#[derive(Default)]
struct Tally {
    pass: usize,
    fail: usize,
}

struct Rows<'a> {
    key: &'a str,
    tally: &'a mut Tally,
    blocked: Option<&'static str>,
}

impl Rows<'_> {
    async fn check<T>(
        &mut self,
        step: &'static str,
        check: impl Future<Output = TestResult<T>>,
    ) -> Option<T> {
        let started = std::time::Instant::now();
        let harness = verdict::Harness::global();
        harness.progress(step);
        if let Some(root) = self.blocked {
            self.emit(
                step,
                "FAIL",
                started,
                &format!("blocked-by-{root}"),
                &format!("blocked-by root=TS.{0}.{root}", self.key),
            );
            return None;
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(40);
        harness.bounded_wait(deadline, step);
        match tokio::time::timeout_at(deadline, check).await {
            Ok(Ok(value)) => {
                self.emit(step, "PASS", started, "checked", "none");
                Some(value)
            }
            Ok(Err(error)) => {
                self.blocked = Some(step);
                self.emit(
                    step,
                    "FAIL",
                    started,
                    &error.to_string(),
                    &format!("assertion root=TS.{}.{step}", self.key),
                );
                None
            }
            Err(_) => {
                self.blocked = Some(step);
                self.emit(
                    step,
                    "FAIL",
                    started,
                    "step-deadline",
                    &format!("deadline root=TS.{}.{step}", self.key),
                );
                None
            }
        }
    }

    fn emit(
        &mut self,
        step: &str,
        status: &str,
        started: std::time::Instant,
        detail: &str,
        cause: &str,
    ) {
        if status == "PASS" {
            self.tally.pass += 1;
        } else {
            self.tally.fail += 1;
        }
        // Database errors carry fixture SQL identifiers, never a connection string.
        let detail: String = detail
            .chars()
            .take(350)
            .map(|c| {
                if c.is_ascii_alphanumeric() || "._:,-".contains(c) {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        evidence(&format!(
            "e2e_all scenario=TS.{}.{step} status={status} seconds={:.3} mode=seeded evidence={detail} cause={cause}",
            self.key,
            started.elapsed().as_secs_f64(),
        ));
    }
}

async fn release(client: &mut DevClientSession) -> TestResult {
    let (result, _, attempts, _) = logout_retrying_busy(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("logout-result={result:?} attempts={attempts}").into());
    }
    Ok(())
}

async fn fixture_connection(
    clients: &SeamClients<'_>,
    index: usize,
) -> TestResult<sqlx::PgConnection> {
    wait_character_released(clients, index).await?;
    Ok(sqlx::PgConnection::connect(clients.url)
        .await
        .map_err(|_| "fixture database connection failed")?)
}

// The initial ordinary logout supplies the real frame, map and artifact digests.
// Only the disposable released Character's last-position projection is seeded.
async fn seed_position(
    db: &mut sqlx::PgConnection,
    character: &[u8; 16],
    from: TilePos,
) -> TestResult {
    let result = sqlx::query(
        "UPDATE game_character_last_positions SET tile_x=$2,tile_y=$3,tile_floor=$4,
         health=NULL,mana=NULL,rest=NULL,write_sequence=write_sequence+1
         WHERE character_id=encode($1,'hex')::uuid
         AND NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions
             WHERE character_id=encode($1,'hex')::uuid AND session_state<>3)",
    )
    .bind(character.as_slice())
    .bind(i32::from(from.x))
    .bind(i32::from(from.y))
    .bind(-i16::from(from.floor))
    .execute(db)
    .await?;
    if result.rows_affected() != 1 {
        return Err("released last-position fixture row missing".into());
    }
    Ok(())
}

async fn seed_tool(
    tx: &mut sqlx::PgConnection,
    character: &[u8; 16],
    item: u32,
    number: Option<u16>,
) -> TestResult {
    let key = format!("oteryn:item.tibia.i{item}");
    let instance: Option<String> = sqlx::query_scalar(
        "SELECT i.item_instance_id::text FROM game_item_container_entries e
         JOIN game_item_instances i USING(item_instance_id)
         WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key=$2
         ORDER BY e.placement_ordinal DESC LIMIT 1",
    )
    .bind(character.as_slice())
    .bind(&key)
    .fetch_optional(&mut *tx)
    .await?;
    let instance = match instance {
        Some(instance) => instance,
        None => sqlx::query_scalar(
            "WITH root AS (
               SELECT character_id,world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid
             ), new_pack AS (
               INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,
                 definition_revision_ref,quantity,lifecycle,minted_transaction_id)
               SELECT game_character_uuid_v7(),world_id,'Item','oteryn:item.tibia.i2854','definition-r1',1,1,game_character_uuid_v7()
                 FROM root WHERE NOT EXISTS (SELECT 1 FROM game_item_container_slots s WHERE s.character_id=root.character_id)
               RETURNING item_instance_id,world_id
             ), put_pack AS (
               INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id)
               SELECT root.character_id,p.item_instance_id,p.world_id,game_character_uuid_v7() FROM root CROSS JOIN new_pack p
               RETURNING character_id,item_instance_id,world_id
             ), pack AS (
               SELECT character_id,item_instance_id,world_id FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid
               UNION ALL SELECT character_id,item_instance_id,world_id FROM put_pack
             ), tool AS (
               INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,
                 definition_revision_ref,quantity,lifecycle,minted_transaction_id)
               SELECT game_character_uuid_v7(),world_id,'Item',$2,'definition-r1',1,1,game_character_uuid_v7() FROM pack
               RETURNING item_instance_id,world_id
             )
             INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,item_instance_id,
               placement_ordinal,placed_transaction_id)
             SELECT pack.character_id,pack.world_id,pack.item_instance_id,tool.item_instance_id,
               1+coalesce((SELECT max(e.placement_ordinal) FROM game_item_container_entries e
                           WHERE e.parent_item_instance_id=pack.item_instance_id),0),game_character_uuid_v7()
               FROM pack CROSS JOIN tool RETURNING item_instance_id::text",
        ).bind(character.as_slice()).bind(&key).fetch_one(&mut *tx).await?,
    };
    if let Some(number) = number {
        sqlx::query(
            "INSERT INTO game_item_keys(item_instance_id,key_number,minted_transaction_id)
             VALUES($1::uuid,$2,game_character_uuid_v7())
             ON CONFLICT(item_instance_id) DO UPDATE SET key_number=EXCLUDED.key_number",
        )
        .bind(instance)
        .bind(i32::from(number))
        .execute(tx)
        .await?;
    }
    Ok(())
}

async fn seed_requirements(
    tx: &mut sqlx::PgConnection,
    character: &[u8; 16],
    gate: Gate,
    eligible: bool,
    tool: Option<u32>,
) -> TestResult {
    // Same disposable fixture route as the seeded M06 custody in
    // crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/support/m06_keys_fixtures.rs:239.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    match gate {
        Gate::Level(Some(required)) => {
            let level = if eligible {
                required.max(1)
            } else {
                required
                    .checked_sub(1)
                    .filter(|level| *level > 0)
                    .ok_or("no playable level below door threshold")?
            };
            let experience = crate::qualification_spell_character::source_experience(level)?;
            let result = sqlx::query(
                "UPDATE game_character_progression_state SET level=$2,total_experience=$3
                 WHERE character_id=encode($1,'hex')::uuid",
            )
            .bind(character.as_slice())
            .bind(i64::from(level))
            .bind(experience)
            .execute(&mut *tx)
            .await?;
            if result.rows_affected() != 1 {
                return Err("fixture progression row missing".into());
            }
        }
        Gate::Quest(Some(action)) => {
            let track = storage_track(action);
            sqlx::query(
                "INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)
                 SELECT character_id,$2,'oteryn:quest.transition-sweep',$3,greatest(character_revision,2)
                 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid
                 ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value",
            ).bind(character.as_slice()).bind(track).bind(if eligible { 1_i64 } else { -1_i64 })
                .execute(&mut *tx).await?;
        }
        Gate::Key(Some(number)) if eligible => {
            seed_tool(tx, character, key_appearance(number), Some(number)).await?
        }
        Gate::Key(_) if !eligible => {}
        Gate::Ordinary => {}
        _ => return Err("authored door predicate missing".into()),
    }
    if let Some(tool) = tool {
        seed_tool(tx, character, tool, None).await?;
    }
    Ok(())
}

fn storage_track(action: u16) -> String {
    use oteryn_gameplay_quests::quest::rotten_blood::{ACCESS_DOOR_STORAGE, ACCESS_DOOR_TRACK};
    // quest_state.rs:395 accepts the numeric storage fallback for every other action.
    if action == ACCESS_DOOR_STORAGE {
        ACCESS_DOOR_TRACK.to_owned()
    } else {
        format!("oteryn:quest-progress/storage/{action}")
    }
}

fn key_appearance(number: u16) -> u32 {
    match number {
        3940 => 2968,
        3980 => 2972,
        _ => 2967,
    }
}

struct Fixture<'a> {
    clients: &'a SeamClients<'a>,
    initialized: [bool; 2],
}

impl Fixture<'_> {
    async fn admit(
        &mut self,
        index: usize,
        from: TilePos,
        gate: Gate,
        tool: Option<u32>,
    ) -> TestResult<DevClientSession> {
        if !self.initialized[index] {
            let mut client = connect_character(self.clients, 0xd8, index).await?;
            release(&mut client).await?;
            self.initialized[index] = true;
        }
        let mut db = fixture_connection(self.clients, index).await?;
        let character = &self.clients.characters[index];
        let mut tx = db.begin().await?;
        seed_position(&mut tx, character, from).await?;
        if gate != Gate::Ordinary || tool.is_some() {
            seed_requirements(&mut tx, character, gate, index == 0, tool).await?;
        }
        tx.commit().await?;
        db.close().await?;
        let client = connect_character(self.clients, 0xd9, index).await?;
        let observed = tile(client.world_spatial().actor_position)?;
        if observed != from {
            let mut client = client;
            release(&mut client).await?;
            return Err(format!("seeded-origin={from:?} observed={observed:?}").into());
        }
        Ok(client)
    }
}

fn object(
    world: &BundleWorld,
    at: TilePos,
    ordinal: usize,
    expected_revision: u64,
) -> TestResult<WorldObjectTarget> {
    let key = placement_key(at, ordinal).ok_or("placement key outside wire bounds")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision,
    })
}

async fn use_target(
    client: &mut DevClientSession,
    target: WorldObjectTarget,
    tool: Option<u32>,
) -> TestResult<UseDisposition> {
    let Some(tool) = tool else {
        return Ok(client
            .use_object(&target.placement, target.expected_revision)
            .await?
            .disposition);
    };
    let items = provisioning::item_index()?;
    let reference = provisioning::item_reference(&items, tool)?;
    let handle = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == reference)
        })
        .ok_or_else(|| format!("seeded item {tool} missing from wire inventory"))?
        .handle;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(handle),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target)),
        use_with_position: None,
    })
    .map_err(|error| format!("USE-WITH encode {error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition =
        decode_use_result(&reply.payload).map_err(|error| format!("USE-WITH decode {error:?}"))?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "USE-WITH status={:?} disposition={disposition:?}",
            reply.status
        )
        .into());
    }
    Ok(disposition)
}

fn check_landing(
    expected: Option<TilePos>,
    actual: TilePos,
    disposition: StepDisposition,
) -> TestResult {
    if disposition != StepDisposition::Moved || Some(actual) != expected {
        return Err(format!(
            "disposition={disposition:?} expected={expected:?} observed={actual:?}"
        )
        .into());
    }
    Ok(())
}

async fn step(
    client: &mut DevClientSession,
    direction: StepDirection,
    expected: Option<TilePos>,
) -> TestResult {
    let reply = client.step_retrying(direction).await?;
    if reply.disposition == StepDisposition::Moved
        && (reply.status != CommandStatus::Accepted || reply.world_spatial_delta.is_none())
    {
        return Err("moved STEP lacks accepted status and spatial delta".into());
    }
    check_landing(
        expected,
        tile(client.world_spatial().actor_position)?,
        reply.disposition,
    )
}

async fn observe_use_landing(
    client: &mut DevClientSession,
    from: TilePos,
    expected: Option<TilePos>,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let observed = tile(client.world_spatial().actor_position)?;
        if observed != from || tokio::time::Instant::now() >= deadline {
            return check_landing(
                expected,
                observed,
                if observed != from {
                    StepDisposition::Moved
                } else {
                    StepDisposition::Blocked
                },
            );
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

async fn refusal(
    client: &mut DevClientSession,
    target: WorldObjectTarget,
    direction: StepDirection,
) -> TestResult {
    let before = client.world_spatial().actor_position;
    let use_result = use_target(client, target, None).await?;
    let movement = client.step_retrying(direction).await?;
    if use_result != UseDisposition::Locked
        || movement.disposition != StepDisposition::Blocked
        || client.world_spatial().actor_position != before
        || movement.world_spatial_delta.is_some()
    {
        return Err(format!(
            "expected locked USE and blocked STEP; observed {use_result:?}/{:?}",
            movement.disposition
        )
        .into());
    }
    Ok(())
}

async fn committed(
    client: &mut DevClientSession,
    target: WorldObjectTarget,
    tool: Option<u32>,
) -> TestResult {
    let result = use_target(client, target, tool).await?;
    if result != UseDisposition::Committed {
        return Err(format!("expected committed USE; observed {result:?}").into());
    }
    Ok(())
}

async fn sweep_entry(
    fixture: &mut Fixture<'_>,
    world: &BundleWorld,
    entry: &Entry,
    tally: &mut Tally,
) -> TestResult {
    let key = entry
        .key
        .strip_prefix("oteryn:transition.")
        .unwrap_or(&entry.key);
    let mut rows = Rows {
        key,
        tally,
        blocked: None,
    };
    let at = entry.at;
    let approach = approaches(world, at).next();
    let (gate, tool) = match entry.action {
        Action::Door {
            ordinal,
            family,
            action,
            ..
        } => {
            let binding = placement_key(at, ordinal)
                .and_then(|key| world.facts().action_at(key))
                .or(action);
            let gate = door_gate(family, binding);
            let tool = match gate {
                Gate::Key(Some(number)) => Some(key_appearance(number)),
                _ => None,
            };
            (gate, tool)
        }
        Action::Rope(_) => (Gate::Ordinary, Some(3003)),
        Action::Shovel { .. } => (Gate::Ordinary, Some(3457)),
        _ => (Gate::Ordinary, None),
    };
    if gate != Gate::Ordinary {
        let mut negative = rows
            .check("prepare-ineligible", async {
                let (from, _, _) = approach.ok_or("no enterable adjacent fixture cell")?;
                fixture.admit(1, from, gate, None).await
            })
            .await;
        rows.check("refuse", async {
            let client = negative.as_mut().ok_or("missing ineligible client")?;
            let (_, direction, _) = approach.ok_or("missing approach")?;
            let Action::Door { ordinal, .. } = entry.action else {
                return Err("missing door".into());
            };
            refusal(client, object(world, at, ordinal, 0)?, direction).await
        })
        .await;
        cleanup(&mut rows, "release-ineligible", negative).await;
        // The eligible fixture has its own admission and can still run after a failed refusal.
        rows.blocked = None;
    }
    let mut client = rows
        .check("prepare", async {
            let (from, _, _) = approach.ok_or("no enterable adjacent fixture cell")?;
            fixture
                .admit(
                    0,
                    from,
                    gate,
                    if matches!(gate, Gate::Key(_)) {
                        None
                    } else {
                        tool
                    },
                )
                .await
        })
        .await;
    let ordinal = match entry.action {
        Action::Door { ordinal, .. }
        | Action::Shovel { ordinal, .. }
        | Action::Ladder(ordinal)
        | Action::Rope(ordinal) => Some(ordinal),
        _ => None,
    };
    let use_landing = client.as_ref().and_then(|client| {
        upstairs(at).find(|candidate| {
            walkable(world, *candidate)
                && !client.world_entities().is_some_and(|entities| {
                    entities.iter().any(|entity| {
                        tile(entity.position).is_ok_and(|position| position == *candidate)
                    })
                })
        })
    });
    if let Some(ordinal) = ordinal {
        rows.check("use", async {
            let client = client.as_mut().ok_or("missing eligible client")?;
            let mut revision = 0;
            if matches!(entry.action, Action::Door { open: true, .. }) {
                committed(client, object(world, at, ordinal, 0)?, None).await?;
                // The world-action owner has a 200 ms per-actor interval (map_door.rs:27).
                tokio::time::sleep(Duration::from_millis(200)).await;
                revision = 1;
            }
            committed(client, object(world, at, ordinal, revision)?, tool).await?;
            // A keyed authored-unlocked pose with action 101/1001 refuses plain USE.
            // Its first matching key use locks it; the second unlocks and opens it.
            if matches!(
                entry.action,
                Action::Door {
                    family: AuthoredDoorFamily::Key { locked: false },
                    ..
                }
            ) && matches!(gate, Gate::Key(Some(101 | 1001)))
            {
                tokio::time::sleep(Duration::from_millis(200)).await;
                committed(client, object(world, at, ordinal, revision + 1)?, tool).await?;
            }
            Ok(())
        })
        .await;
    }
    rows.check("land", async {
        let client = client.as_mut().ok_or("missing eligible client")?;
        let (from, direction, cardinal) = approach.ok_or("missing approach")?;
        let expected = match &entry.action {
            Action::StepTo(to) => Some(*to),
            Action::Floor => floor_landing(world, world.floors(), from, cardinal)
                .filter(|to| to.floor != at.floor),
            Action::Door { .. } => Some(at),
            Action::Shovel { mask, .. } => {
                let mut floors = world.floors().clone();
                floors.add(at, *mask);
                floor_landing(world, &floors, from, cardinal).filter(|to| to.floor < at.floor)
            }
            Action::Ladder(_) | Action::Rope(_) => {
                return observe_use_landing(client, from, use_landing).await;
            }
            Action::Unsupported(kind) => {
                return Err(format!("unknown authored transition kind {kind}").into());
            }
        };
        step(client, direction, expected).await
    })
    .await;
    cleanup(&mut rows, "release", client).await;
    Ok(())
}

async fn cleanup(rows: &mut Rows<'_>, step: &'static str, client: Option<DevClientSession>) {
    if let Some(mut client) = client {
        // Cleanup still runs on an oracle failure so the next entry can admit.
        let released = tokio::time::timeout(Duration::from_secs(15), release(&mut client)).await;
        rows.check(step, async { released.map_err(|_| "logout deadline")? })
            .await;
    } else {
        rows.check::<()>(step, async { Err("missing client".into()) })
            .await;
    }
}

pub(super) async fn run(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    bytes: &[u8],
) -> TestResult {
    if !std::env::var("NODE_BOOT_E2E_ONLY")
        .is_ok_and(|list| list.split(',').any(|name| name.trim() == GROUP))
    {
        return Ok(());
    }
    let (start, len) = batch_from_env()?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let entries = census(&root, bytes)?;
    let total = entries.len();
    let end = start.saturating_add(len).min(total);
    let batch = entries
        .get(start..end)
        .filter(|batch| !batch.is_empty())
        .ok_or_else(|| format!("{BATCH_VARIABLE} start {start} is beyond {total} entries"))?;
    evidence(&format!(
        "{GROUP} census total={total} start={start} len={len} selected={} mode=seeded",
        batch.len()
    ));
    let mut fixture = Fixture {
        clients,
        initialized: [false; 2],
    };
    let mut tally = Tally::default();
    let began = std::time::Instant::now();
    for entry in batch {
        let key = entry
            .key
            .strip_prefix("oteryn:transition.")
            .unwrap_or(&entry.key);
        let scenario = Box::leak(format!("TS.{key}.prepare").into_boxed_str());
        verdict::indexed(
            scenario,
            Box::pin(sweep_entry(&mut fixture, world, entry, &mut tally)),
        )
        .await;
    }
    evidence(&format!(
        "{GROUP} summary total={total} start={start} entries={} rows_pass={} rows_fail={} seconds={:.3} mode=seeded",
        batch.len(),
        tally.pass,
        tally.fail,
        began.elapsed().as_secs_f64(),
    ));
    if tally.fail > 0 {
        return Err(format!("{GROUP}: {} FAIL rows (see TS.*)", tally.fail).into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_requires_plain_bounded_decimal_and_positive_length() -> TestResult {
        assert_eq!(transition_batch(None)?, (0, 3));
        assert_eq!(transition_batch(Some("123:20"))?, (123, 20));
        for bad in [
            "",
            "1",
            "1:0",
            "-1:1",
            "1:+1",
            " 1:2",
            "1:2:3",
            "99999999999999999999999999:1",
        ] {
            assert!(transition_batch(Some(bad)).is_err(), "{bad:?}");
        }
        Ok(())
    }

    #[test]
    fn reads_authored_teleport_endpoint_and_legacy_floor() -> TestResult {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let entries = authored_transitions(&root)?;
        let first = entries
            .iter()
            .find(|entry| entry.key == "oteryn:transition.teleport.x18999_y19000_z7")
            .ok_or("missing authored teleport")?;
        assert_eq!(
            first.at,
            TilePos {
                x: 18999,
                y: 19000,
                floor: -7
            }
        );
        assert!(matches!(
            first.action,
            Action::StepTo(TilePos {
                x: 32369,
                y: 32241,
                floor: -7
            })
        ));
        assert!(check_landing(Some(first.at), first.at, StepDisposition::Blocked).is_err());
        assert!(check_landing(None, first.at, StepDisposition::Moved).is_err());
        Ok(())
    }

    #[test]
    fn door_predicates_use_family_and_native_action() {
        assert_eq!(
            door_gate(AuthoredDoorFamily::Level, Some(1050)),
            Gate::Level(Some(50))
        );
        assert_eq!(
            door_gate(AuthoredDoorFamily::Quest, Some(1050)),
            Gate::Quest(Some(1050))
        );
        assert_eq!(
            door_gate(AuthoredDoorFamily::Key { locked: true }, Some(3940)),
            Gate::Key(Some(3940))
        );
        assert_eq!(
            door_gate(AuthoredDoorFamily::Key { locked: false }, Some(101)),
            Gate::Key(Some(101))
        );
        assert_eq!(
            door_gate(AuthoredDoorFamily::Quest, None),
            Gate::Quest(None)
        );
        assert_eq!(
            door_gate(AuthoredDoorFamily::Plain, Some(1050)),
            Gate::Ordinary
        );
    }

    #[test]
    fn tool_landing_prefers_south_and_never_wraps() {
        let at = TilePos {
            x: 0,
            y: 0,
            floor: -2,
        };
        assert_eq!(
            upstairs(at).next(),
            Some(TilePos {
                y: 1,
                floor: -1,
                ..at
            })
        );
        assert_eq!(upstairs(TilePos { floor: 0, ..at }).count(), 0);
    }
}
