//! Opt-in node-boot sweep over the authored Interaction and Document catalogues.
//! Every selected entry emits its steps, including FAIL rows for blocked steps.
//! Chest placement oracle: the served binding owner in
//! `crates/gameplay/content/src/content/world_reward_claims.rs:207`. Reward quantities
//! come from authored claim content. USE and once-per-character custody are owned by
//! `crates/gameplay/dispatch/src/gameplay_transport/bundle_chest.rs:101`.
//! Shrine placement and opening are owned by
//! `crates/gameplay/dispatch/src/gameplay_transport/daily_reward.rs:184` and :223.
//! Document oracle: exact authored paragraphs in order. Map LOOK's reading path is
//! `crates/gameplay/dispatch/src/gameplay_transport/item_look.rs:610`.

use super::*;
use crate::content::daily_reward::DailyRewardContent;
use crate::content::world_reward_claims::{
    BindingReport, BoundPlacement, bind_embedded_reward_claims,
};
use crate::map::view::MapFacts;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::daily_reward::{
    DailyRewardCommand, DailyRewardReply, decode_reply,
};
use oteryn_protocol_oteryn::daily_reward::{self as daily, DailyRewardWall};
use oteryn_protocol_oteryn::item_look as look;
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use serde::Deserialize;
use serde_json::Value;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::time::Instant;

const BATCH_VARIABLE: &str = "OTERYN_E2E_INTERACTION_SWEEP_BATCH";
const DEFAULT_BATCH: (usize, usize) = (0, 3);

fn batch(value: Option<&str>) -> TestResult<(usize, usize)> {
    let Some(value) = value else {
        return Ok(DEFAULT_BATCH);
    };
    let decimal = |part: &str| {
        if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        part.parse::<usize>().ok()
    };
    match value
        .split_once(':')
        .and_then(|(start, len)| Some((decimal(start)?, decimal(len)?)))
    {
        Some((start, len)) if len != 0 => Ok((start, len)),
        _ => Err(
            format!("invalid {BATCH_VARIABLE}={value:?}, expected start:len with len > 0").into(),
        ),
    }
}

#[derive(Deserialize)]
struct Index {
    record_count: usize,
    shards: Vec<PathBuf>,
}

#[derive(Deserialize)]
struct Shard {
    records: Vec<Value>,
}

fn records(root: &Path, index: &str) -> TestResult<Vec<Value>> {
    let index: Index = serde_json::from_reader(std::fs::File::open(root.join(index))?)?;
    let mut records = Vec::new();
    for path in index.shards {
        let shard: Shard = serde_json::from_reader(std::fs::File::open(root.join(path))?)?;
        records.extend(shard.records);
    }
    if records.len() != index.record_count {
        return Err(format!(
            "catalogue count mismatch: expected {}, read {}",
            index.record_count,
            records.len()
        )
        .into());
    }
    Ok(records)
}

#[derive(Deserialize)]
struct Document {
    content: Vec<String>,
}

enum Entry {
    Document(Document),
    MapDocument(Placement, String),
    NestedMapDocument,
    Interaction(Placement, MapAction),
    Claim,
    Shrine(DailyRewardContent),
}

impl Entry {
    fn steps(&self) -> &'static [&'static str] {
        match self {
            Self::Document(_) => &["bind", "query", "text"],
            Self::MapDocument(..) | Self::NestedMapDocument => &["bind", "walk", "open", "text"],
            Self::Interaction(..) | Self::Claim => &["bind", "walk", "use", "effect", "reset"],
            Self::Shrine(_) => &[
                "bind",
                "walk",
                "use",
                "effect",
                "reset",
                "claim",
                "replay",
                "durability",
            ],
        }
    }
}

impl Entry {
    fn family(&self) -> &'static str {
        match self {
            Self::Document(_) => "documents",
            Self::MapDocument(..) | Self::NestedMapDocument => "map_documents",
            Self::Interaction(..) => "interactions",
            Self::Claim => "claims",
            Self::Shrine(_) => "shrines",
        }
    }
}

const FAMILIES: &[&str] = &[
    "documents",
    "map_documents",
    "interactions",
    "claims",
    "shrines",
];

fn slices(value: Option<&str>) -> TestResult<Vec<(&str, usize, usize)>> {
    let Some(value) = value else {
        return Ok(vec![("all", DEFAULT_BATCH.0, DEFAULT_BATCH.1)]);
    };
    if !value.contains('=') {
        let (start, len) = batch(Some(value))?;
        return Ok(vec![("all", start, len)]);
    }
    let mut selected = Vec::new();
    for slice in value.split(',') {
        let (family, range) = slice.split_once('=').ok_or("expected family=start:len")?;
        if !FAMILIES.contains(&family) || selected.iter().any(|&(name, _, _)| name == family) {
            return Err(format!("unknown or repeated sweep family {family:?}").into());
        }
        let (start, len) = batch(Some(range))?;
        selected.push((family, start, len));
    }
    Ok(selected)
}

fn census(root: &Path) -> TestResult<BTreeMap<String, Entry>> {
    let mut entries = BTreeMap::new();
    for (index, field) in [
        ("content/documents/index.json", "declaration"),
        (
            "content/interactions/reward_claims/index.json",
            "definition",
        ),
    ] {
        for mut record in records(root, index)? {
            let key = record[field]["identity"]["key"]
                .as_str()
                .ok_or_else(|| format!("missing entry key in {index}"))?
                .to_owned();
            let entry = match field {
                "declaration" => Entry::Document(serde_json::from_value(record[field].take())?),
                _ => Entry::Claim,
            };
            if entries.insert(key.clone(), entry).is_some() {
                return Err(format!("duplicate catalogue key {key}").into());
            }
        }
    }
    let shrine: DailyRewardContent = serde_json::from_reader(std::fs::File::open(
        root.join("content/interactions/daily-rewards.json"),
    )?)?;
    if entries
        .insert(shrine.entry_shrine.placement.clone(), Entry::Shrine(shrine))
        .is_some()
    {
        return Err("duplicate daily shrine key".into());
    }
    Ok(entries)
}

fn row_token(value: &str) -> String {
    // Keep punctuation distinguishable, including slashes in document keys.
    use std::fmt::Write as _;
    let mut token = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b".-".contains(&byte) {
            token.push(char::from(byte));
        } else {
            let _ = write!(token, "_{byte:02x}_");
        }
    }
    token
}

#[derive(Default)]
struct Tally {
    pass: usize,
    fail: usize,
}

struct Rows<'a> {
    key: String,
    steps: &'static [&'static str],
    next: usize,
    lap: Instant,
    tally: &'a mut Tally,
}

impl Rows<'_> {
    fn emit(&mut self, status: &str, cause: &str, detail: &str) {
        let seconds = self.lap.elapsed().as_secs_f64();
        self.lap = Instant::now();
        let step = self.steps[self.next];
        self.next += 1;
        let detail = detail.split_whitespace().collect::<Vec<_>>().join("-");
        let detail: String = detail.chars().take(320).collect();
        evidence(&format!(
            "e2e_all scenario=IS.{}.{step}.full status={status} seconds={seconds:.3} mode=seeded evidence={detail} cause={cause}",
            self.key,
        ));
    }

    fn pass(&mut self, detail: &str) {
        self.tally.pass += 1;
        self.emit("PASS", "observed", detail);
    }

    fn fail(&mut self, detail: &str) {
        let root = self.steps[self.next];
        self.tally.fail += 1;
        self.emit(
            "FAIL",
            detail
                .split_once(':')
                .map(|(cause, _)| cause)
                .filter(|cause| cause.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'))
                .unwrap_or(root),
            detail,
        );
        while self.next < self.steps.len() {
            self.tally.fail += 1;
            self.emit("FAIL", &format!("blocked-by-{root}"), "earlier-step-failed");
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Placement {
    at: TilePos,
    key: u64,
}

impl Placement {
    fn target(self, world: &BundleWorld) -> WorldObjectTarget {
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&self.key.to_be_bytes());
        WorldObjectTarget {
            placement,
            expected_revision: 0,
        }
    }
}

#[derive(Clone, Copy)]
enum MapAction {
    Door,
    Ladder,
    Rope,
    Sewer,
    Teleport(TilePos),
    CallbackTeleport(TilePos),
    Terrain {
        tool: u32,
        after: u16,
        decay: Duration,
        final_state: Option<u16>,
    },
    Lever {
        poses: [u16; 2],
        reversible: bool,
        has_secondary_effects: bool,
    },
    Callback,
}

// quest_callbacks/execute.rs:1682 commits a local callback teleport through
// the source-step owner. Keep callbacks with other authored effects on
// their own path until those effects can also be checked.
fn has_additional_callback_effects(
    callback: &oteryn_gameplay_quests::quest::world_callbacks::Callback,
) -> bool {
    matches!(
        callback.key.as_str(),
        "oteryn:quest-callback/dryad-garden-enter" | "oteryn:quest-callback/dryad-garden-leave"
    ) || callback.facing.is_some()
        || callback.disguise.is_some()
        || callback.native_rule.is_some()
        || !callback.passages.is_empty()
        || !callback.effects.is_empty()
        || callback.remote_teleport.is_some()
        || callback.has_state_writes()
        || callback.world_change.is_some()
        || callback.experience.is_some()
        || callback.completes
        || callback.monk_reward.is_some()
        || !callback.offerings.is_empty()
        || callback.item_on_target.is_some()
        || callback.success_message.is_some()
        || callback.item_reward.is_some()
        || !callback.achievements.is_empty()
}

fn callback_teleport(
    callback: &oteryn_gameplay_quests::quest::world_callbacks::Callback,
) -> Option<TilePos> {
    let destination = callback.teleport?;
    if callback.lever.is_some() || has_additional_callback_effects(callback) {
        return None;
    }
    Some(TilePos {
        x: destination.x,
        y: destination.y,
        floor: destination.floor,
    })
}

fn map_entries(
    bytes: &[u8],
    world: &BundleWorld,
    entries: &mut BTreeMap<String, Entry>,
) -> TestResult {
    use crate::map::floor::{is_ladder_appearance, is_rope_spot_item};
    use oteryn_gameplay_quests::quest::world_callbacks::{self as callbacks, Trigger};
    use oteryn_gameplay_world::terrain_tools::{TerrainTool, crystal_core_bindings};
    let terrain = crystal_core_bindings();
    let mut authored = BTreeMap::<TilePos, Vec<callbacks::Callback>>::new();
    let mut by_appearance = BTreeMap::<u16, Vec<callbacks::Callback>>::new();
    for callback in callbacks::load()? {
        if callback.trigger != Trigger::Use {
            continue;
        }
        if let Some(position) = callback
            .selector
            .native
            .as_ref()
            .map(|native| native.position)
            .or(callback.selector.position)
        {
            let at = TilePos {
                x: position.x,
                y: position.y,
                floor: position.floor,
            };
            authored.entry(at).or_default().push(callback);
        } else if let Some(appearance) = callback.selector.appearance {
            by_appearance.entry(appearance).or_default().push(callback);
        }
    }
    let facts = world.facts().pinned();
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        for tile in sector.tiles {
            let at = TilePos {
                x: tile.x,
                y: tile.y,
                floor: sector.floor,
            };
            for (index, item) in tile.items.iter().enumerate() {
                if item.depth != 0
                    && item
                        .attrs
                        .text
                        .as_ref()
                        .is_some_and(|text| !text.is_empty())
                {
                    let key = format!(
                        "map-document.{:02}.{:05}.{:05}.nested-{index:03}",
                        -i16::from(at.floor),
                        at.x,
                        at.y
                    );
                    entries.insert(key, Entry::NestedMapDocument);
                }
            }
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                let Some(ordinal) = u8::try_from(ordinal).ok() else {
                    continue;
                };
                let Some(key) = crate::map::view::placement_key(at, ordinal) else {
                    continue;
                };
                let placement = Placement { at, key };
                let suffix = format!(
                    "{:02}.{:05}.{:05}.{:03}",
                    -i16::from(at.floor),
                    at.x,
                    at.y,
                    ordinal
                );
                if let Some(text) = item.attrs.text.as_ref().filter(|text| !text.is_empty()) {
                    entries.insert(
                        format!("map-document.{suffix}"),
                        Entry::MapDocument(placement, text.clone()),
                    );
                }
                let Some(palette) = manifest.palette.get(item.palette as usize) else {
                    continue;
                };
                let appearance = facts
                    .base_entry(at, ordinal, palette.id)
                    .map_or(0, |entry| entry.appearance_id);
                let source = palette
                    .key
                    .rsplit_once(".i")
                    .and_then(|(_, id)| id.parse::<u32>().ok())
                    .unwrap_or(u32::from(appearance));
                let callback = authored
                    .get(&at)
                    .into_iter()
                    .flatten()
                    .chain(by_appearance.get(&appearance).into_iter().flatten())
                    .find(|callback| {
                        let selector = &callback.selector;
                        if let Some(native) = &selector.native
                            && native.position
                                == (callbacks::Position {
                                    x: at.x,
                                    y: at.y,
                                    floor: at.floor,
                                })
                            && native.ordinal == ordinal
                            && world.facts().quest_callback_native_object(
                                key,
                                palette.id,
                                native.appearance,
                            )
                        {
                            return true;
                        }
                        (selector.position.is_some()
                            || selector.action.is_some()
                            || selector.unique.is_some()
                            || selector.appearance.is_some())
                            && selector.matches_position(callbacks::Position {
                                x: at.x,
                                y: at.y,
                                floor: at.floor,
                            })
                            && selector
                                .ordinal
                                .is_none_or(|wanted| wanted == u16::from(ordinal))
                            && world.facts().quest_callback_matches(
                                key,
                                selector.unique,
                                selector.action,
                                selector.appearance,
                            )
                    });
                let action = if let Some(callback) = callback {
                    Some(match callback.lever {
                        Some(poses) => MapAction::Lever {
                            poses,
                            reversible: callback.reversible,
                            has_secondary_effects: callback.teleport.is_some()
                                || has_additional_callback_effects(callback),
                        },
                        None => callback_teleport(callback)
                            .map_or(MapAction::Callback, MapAction::CallbackTeleport),
                    })
                } else if world.facts().doors().contains(key) || item.attrs.door.is_some() {
                    Some(MapAction::Door)
                } else if is_ladder_appearance(appearance) {
                    Some(MapAction::Ladder)
                } else if appearance == 435 {
                    Some(MapAction::Sewer)
                } else if is_rope_spot_item(source, ordinal == 0) {
                    Some(MapAction::Rope)
                } else if let Some((x, y, floor)) = item.attrs.teleport {
                    Some(MapAction::Teleport(TilePos {
                        x,
                        y,
                        floor: floor as i8,
                    }))
                } else if let Some(binding) = terrain.iter().find(|binding| {
                    binding.target == source && binding.required_action == item.attrs.action
                }) {
                    let tool = match binding.tool {
                        TerrainTool::Shovel => 3457,
                        TerrainTool::Pick => 3456,
                        TerrainTool::Machete => 3308,
                        TerrainTool::Scythe => 3453,
                        TerrainTool::Sickle => 3293,
                        TerrainTool::Crowbar => 3304,
                        TerrainTool::SlimeGobbler => 12077,
                        TerrainTool::ElementalSoil(tool) => u32::from(tool),
                    };
                    u16::try_from(binding.transformed)
                        .ok()
                        .map(|after| MapAction::Terrain {
                            tool,
                            after,
                            decay: binding.decay.iter().map(|step| step.after).sum(),
                            final_state: binding
                                .decay
                                .last()
                                .and_then(|step| u16::try_from(step.state).ok()),
                        })
                } else if world.facts().quest_callback_source_bound(key) {
                    Some(MapAction::Callback)
                } else {
                    None
                };
                if let Some(action) = action {
                    entries.insert(
                        format!("map-action.{suffix}"),
                        Entry::Interaction(placement, action),
                    );
                }
            }
        }
        Ok(())
    })?;
    Ok(())
}

// The reference profile binds each report to a creature; canonical Bestiary order
// is the wire ordinal in charm_progression.rs:91. Some authoring creatures have
// no Bestiary race. The detail owner (catalogue.rs:204) currently projects stats
// and loot but omits the encyclopedia paragraphs.
fn report_races(root: &Path) -> TestResult<BTreeMap<String, Option<u16>>> {
    use crate::content::{ProjectEvidenceLimits, canonical_bestiary_rows};
    let rows = canonical_bestiary_rows(ProjectEvidenceLimits {
        max_documents: 5,
        max_document_bytes: 2_500_000,
        max_total_bytes: 8_000_000,
        max_json_depth: 10,
        max_decoded_fields: 65_000,
        max_string_bytes: 1_000_000,
        max_locator_bytes: 128,
        max_locator_segments: 5,
        max_reference_records: 4096,
        max_import_records: 1,
        max_reimport_states: 1,
    })
    .map_err(|error| format!("bestiary-catalogue: {error:?}"))?;
    let ordinals = rows
        .iter()
        .enumerate()
        .map(|(index, (race, _))| (race.key(), u16::try_from(index + 1).ok()))
        .collect::<BTreeMap<_, _>>();
    let mut reports = BTreeMap::new();
    for file in std::fs::read_dir(root.join("content/creatures/definitions"))? {
        let file = file?;
        if !file.file_name().to_string_lossy().starts_with("creatures-") {
            continue;
        }
        let shard: Shard = serde_json::from_reader(std::fs::File::open(file.path())?)?;
        for record in shard.records {
            let Some(report) =
                record["authoring"]["profile"]["details"]["encyclopedia_document"]["key"].as_str()
            else {
                continue;
            };
            let creature = record["definition"]["identity"]["key"]
                .as_str()
                .ok_or("encyclopedia profile has no creature key")?;
            let race = ordinals.get(creature).copied().flatten();
            // Several profiles may cite the same report. Never select a race by
            // matching its name to the report title.
            reports
                .entry(report.to_owned())
                .and_modify(|current: &mut Option<u16>| {
                    if let Some(race) = race {
                        *current = Some(current.map_or(race, |old| old.min(race)));
                    }
                })
                .or_insert(race);
        }
    }
    Ok(reports)
}

async fn read_report(
    client: &mut DevClientSession,
    rows: &mut Rows<'_>,
    race: u16,
    expected: &str,
) -> TestResult {
    use oteryn_protocol_oteryn::charm_progression as charm;
    let request = charm::catalogue::encode_request(charm::catalogue::Request {
        after: 0,
        race,
        category: 0,
    })
    .map_err(|error| format!("bestiary-request: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            charm::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1,
            charm::COMMAND_TYPE_CHARM_PROGRESSION,
            &request,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("bestiary-query-refused: {:?}", reply.status).into());
    }
    let page = charm::catalogue::decode_page(&reply.payload)
        .map_err(|error| format!("bestiary-decode: {error:?}"))?;
    let [card] = page.cards.as_slice() else {
        return Err("bestiary-card-missing: expected one race".into());
    };
    if card.race != race {
        return Err("bestiary-race-mismatch: wrong race in detail reply".into());
    }
    rows.pass("canonical-bestiary-detail-reply");
    if card.detail != expected {
        return Err(
            "bestiary-report-text-missing: detail did not serve the exact authored report".into(),
        );
    }
    rows.pass("byte-identical-authored-report");
    Ok(())
}

fn map_face(client: &mut DevClientSession, placement: Placement) -> Option<(u16, u64)> {
    use oteryn_protocol_oteryn::world_map::MapOrigin;
    let (_, expected_ordinal) = crate::map::view::placement_of(placement.key)?;
    let snapshot = client.session_mut().world_map_snapshot()?;
    let tile = snapshot.tiles.iter().find(|tile| {
        tile.position.x == i32::from(placement.at.x)
            && tile.position.y == i32::from(placement.at.y)
            && tile.position.floor == -i16::from(placement.at.floor)
    })?;
    tile.items.iter().find_map(|item| match item.origin {
        MapOrigin::BaseOrdinal {
            ordinal,
            object_revision,
        } if ordinal == expected_ordinal => Some((item.appearance_id, object_revision)),
        _ => None,
    })
}

async fn wait_face(
    client: &mut DevClientSession,
    placement: Placement,
    matches: impl Fn((u16, u64)) -> bool,
) -> TestResult<(u16, u64)> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(face) = map_face(client, placement).filter(|face| matches(*face)) {
            return Ok(face);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("map-appearance-mismatch: expected placement state absent on wire".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

async fn tool_use(
    client: &mut DevClientSession,
    tool: oteryn_protocol_oteryn::item_view::ItemHandle,
    target: WorldObjectTarget,
) -> TestResult<UseDisposition> {
    use oteryn_protocol_oteryn::world_object::{
        MapUseTarget, UseIntent, UseTarget, decode_use_result, encode_item_use_intent,
    };
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(tool),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target)),
        use_with_position: None,
    })
    .map_err(|error| format!("tool-intent: {error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("tool-command-refused: {:?}", reply.status).into());
    }
    decode_use_result(&reply.payload).map_err(|error| format!("tool-result: {error:?}").into())
}

async fn use_map_action(
    client: &mut DevClientSession,
    world: &BundleWorld,
    placement: Placement,
    action: MapAction,
    tool: Option<oteryn_protocol_oteryn::item_view::ItemHandle>,
    revision: u64,
) -> TestResult {
    if let MapAction::Teleport(_) = action {
        let from = tile(client.world_spatial().actor_position)?;
        let direction = [
            (StepDirection::North, 0, -1),
            (StepDirection::East, 1, 0),
            (StepDirection::South, 0, 1),
            (StepDirection::West, -1, 0),
        ]
        .into_iter()
        .find_map(|(direction, dx, dy)| {
            (from.x.checked_add_signed(dx) == Some(placement.at.x)
                && from.y.checked_add_signed(dy) == Some(placement.at.y))
            .then_some(direction)
        })
        .ok_or("teleport-approach: no cardinal activation step")?;
        let outcome = client.step_retrying(direction).await?;
        if outcome.disposition != StepDisposition::Moved {
            return Err(format!("map-step-refused: {:?}", outcome.disposition).into());
        }
        return Ok(());
    }
    let mut target = placement.target(world);
    target.expected_revision = revision;
    let disposition = if let Some(tool) = tool {
        tool_use(client, tool, target).await?
    } else {
        use_target(client, &target).await?
    };
    if disposition != UseDisposition::Committed {
        return Err(format!("map-use-refused: {disposition:?}").into());
    }
    Ok(())
}

fn movement_landing(world: &BundleWorld, placement: TilePos, action: MapAction) -> Option<TilePos> {
    if let MapAction::Teleport(destination) | MapAction::CallbackTeleport(destination) = action {
        return Some(destination);
    }
    if matches!(action, MapAction::Sewer) {
        return Some(TilePos {
            floor: placement.floor.checked_sub(1)?,
            ..placement
        });
    }
    if !matches!(action, MapAction::Ladder | MapAction::Rope) {
        return None;
    }
    // Floor ascent selects the first walkable adjacent upstairs tile in this order
    // (map/floor/rope.rs:18, :207).
    let floor = placement.floor.checked_add(1)?;
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
    .filter_map(|(dx, dy)| {
        Some(TilePos {
            x: placement.x.checked_add_signed(dx)?,
            y: placement.y.checked_add_signed(dy)?,
            floor,
        })
    })
    .find(|at| {
        world.enterable(*at)
            && world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| tile.walkable())
    })
}

// USE returns its disposition before the client necessarily receives the
// spatial projection (session_exchange.rs:131). Wait for the wire observation.
async fn movement_observation(
    client: &mut DevClientSession,
    before: TilePos,
) -> TestResult<TilePos> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let actual = tile(client.world_spatial().actor_position)?;
        if actual != before || tokio::time::Instant::now() >= deadline {
            return Ok(actual);
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

#[derive(Clone, Copy)]
enum RouteStep {
    Walk(StepDirection),
    Use {
        placement: Placement,
        action: MapAction,
        landing: TilePos,
    },
}

/// Cache one exhausted reachability search over the immutable bundle. If a later
/// origin was reached by that search, everything it can reach is in the same
/// set, even when floor changes and teleports make step edges one-way.
struct SweepRoutes<'a> {
    world: &'a BundleWorld,
    exhausted: std::cell::RefCell<std::collections::HashSet<TilePos>>,
    connections: BTreeMap<TilePos, Vec<RouteStep>>,
}

impl SweepRoutes<'_> {
    fn find(
        &self,
        from: TilePos,
        neighbors: impl Iterator<Item = TilePos>,
    ) -> TestResult<(TilePos, Vec<RouteStep>)> {
        use std::collections::{HashMap, hash_map::Entry};
        let goals: std::collections::HashSet<_> = neighbors.collect();
        let exhausted = self.exhausted.borrow();
        if exhausted.contains(&from) && goals.is_disjoint(&exhausted) {
            return Err(format!(
                "no-route: no approach in the {} reachable cells",
                exhausted.len()
            )
            .into());
        }
        drop(exhausted);

        let mut queue = VecDeque::from([from]);
        let mut parents = HashMap::from([(from, None)]);
        while let Some(at) = queue.pop_front() {
            if goals.contains(&at) {
                let mut cursor = at;
                let mut path = Vec::new();
                while let Some(Some((previous, direction))) = parents.get(&cursor) {
                    path.push(*direction);
                    cursor = *previous;
                }
                path.reverse();
                return Ok((at, path));
            }
            for direction in [
                StepDirection::North,
                StepDirection::East,
                StepDirection::South,
                StepDirection::West,
                StepDirection::NorthEast,
                StepDirection::SouthEast,
                StepDirection::SouthWest,
                StepDirection::NorthWest,
            ] {
                if let Some(next) = destination(self.world, at, direction)
                    && let Entry::Vacant(entry) = parents.entry(next)
                {
                    entry.insert(Some((at, RouteStep::Walk(direction))));
                    queue.push_back(next);
                }
            }
            for &step in self.connections.get(&at).into_iter().flatten() {
                if let RouteStep::Use { landing, .. } = step
                    && let Entry::Vacant(entry) = parents.entry(landing)
                {
                    entry.insert(Some((at, step)));
                    queue.push_back(landing);
                }
            }
        }
        let count = parents.len();
        *self.exhausted.borrow_mut() = parents.into_keys().collect();
        Err(format!("no-route: exhausted {count} reachable cells").into())
    }
    async fn follow(&self, client: &mut DevClientSession, path: &[RouteStep]) -> TestResult<bool> {
        let mut walking = Vec::new();
        for &step in path {
            match step {
                RouteStep::Walk(direction) => walking.push(direction),
                RouteStep::Use {
                    placement,
                    action,
                    landing,
                } => {
                    if !self.walk_segment(client, &walking).await? {
                        return Ok(false);
                    }
                    walking.clear();
                    let revision = map_face(client, placement).map_or(0, |(_, revision)| revision);
                    let before = tile(client.world_spatial().actor_position)?;
                    use_map_action(client, self.world, placement, action, None, revision)
                        .await
                        .map_err(|error| format!("route-transit-refused: {error}"))?;
                    let actual = movement_observation(client, before).await?;
                    if actual != landing {
                        evidence(&format!(
                            "interaction_sweep transit=redirected expected={landing:?} observed={actual:?}"
                        ));
                        return Ok(false);
                    }
                }
            }
        }
        self.walk_segment(client, &walking).await
    }

    async fn walk_segment(
        &self,
        client: &mut DevClientSession,
        walking: &[StepDirection],
    ) -> TestResult<bool> {
        match walk(client, self.world, walking).await {
            Ok(()) => Ok(true),
            Err(error) if error.is::<WalkLandingRedirect>() => {
                evidence(&format!("interaction_sweep step=redirected detail={error}"));
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

    async fn reach(
        &self,
        client: &mut DevClientSession,
        neighbors: impl Iterator<Item = TilePos> + Clone,
    ) -> TestResult<TilePos> {
        let mut visited = std::collections::HashSet::new();
        // Live STEP may take a different floor transition, and a ladder
        // can choose a different unoccupied candidate (map/floor/rope.rs:229).
        // Replan from the wire position. The action under test still checks
        // its own landing.
        for _ in 0..4 {
            let from = tile(client.world_spatial().actor_position)?;
            if !visited.insert(from) {
                break;
            }
            let (goal, path) = self.find(from, neighbors.clone())?;
            if self.follow(client, &path).await? {
                if tile(client.world_spatial().actor_position)? != goal {
                    return Err(
                        "wrong-position: client did not arrive at the selected approach".into(),
                    );
                }
                return Ok(goal);
            }
        }
        Err("route-transit-repeated-redirect: failed to reach an authored approach".into())
    }
}

fn approaches(target: TilePos, cardinal: bool) -> impl Iterator<Item = TilePos> + Clone {
    (-1_i16..=1).flat_map(move |dy| {
        (-1_i16..=1).filter_map(move |dx| {
            let distance = dx.abs() + dy.abs();
            if distance == 0 || (cardinal && distance != 1) {
                return None;
            }
            Some(TilePos {
                x: target.x.checked_add_signed(dx)?,
                y: target.y.checked_add_signed(dy)?,
                ..target
            })
        })
    })
}

async fn approach_action(
    client: &mut DevClientSession,
    routes: &SweepRoutes<'_>,
    target: TilePos,
    action: MapAction,
) -> TestResult {
    let cardinal = matches!(action, MapAction::Teleport(_));
    routes.reach(client, approaches(target, cardinal)).await?;
    Ok(())
}

async fn map_action(
    slot: &mut Option<DevClientSession>,
    clients: &SeamClients<'_>,
    rows: &mut Rows<'_>,
    routes: &SweepRoutes<'_>,
    placement: Placement,
    action: MapAction,
) -> TestResult {
    let world = routes.world;
    let tool_id = match action {
        MapAction::Rope => Some(3003),
        MapAction::Terrain { tool, .. } => Some(tool),
        _ => None,
    };
    let reference = tool_id
        .map(|id| provisioning::item_reference(&provisioning::item_index()?, id))
        .transpose()?;
    let has_tool = |client: &DevClientSession| {
        let reference = reference?;
        client
            .inventory()?
            .entries
            .iter()
            .find(|entry| entry.item_definition_ref == reference)
            .map(|entry| entry.handle)
    };
    session(slot, clients).await?;
    // The existing fixture owner seeds the canonical rope and shovel. Preserve
    // the session slot if the command refuses before it can be re-established.
    if matches!(tool_id, Some(3003 | 3457)) && slot.as_ref().and_then(has_tool).is_none() {
        let client = slot.take().ok_or("missing sweep session")?;
        *slot = Some(
            provisioning::provision(client, clients, 1)
                .await
                .map_err(|error| format!("fixture-seed: {error}"))?,
        );
    }
    let client = session(slot, clients).await?;
    approach_action(client, routes, placement.at, action).await?;
    rows.pass("adjacent-to-authored-action");
    let tool = has_tool(client);
    if reference.is_some() && tool.is_none() {
        return Err(format!("fixture-tool-unavailable: {tool_id:?}").into());
    }
    let before = map_face(client, placement);
    let prior_position = tile(client.world_spatial().actor_position)?;
    let revision = before.map_or(0, |(_, revision)| revision);
    if matches!(
        action,
        MapAction::Door | MapAction::Lever { .. } | MapAction::Terrain { .. }
    ) && before.is_none()
    {
        return Err("map-target-not-visible: missing placement and revision".into());
    }
    use_map_action(client, world, placement, action, tool, revision).await?;
    rows.pass("committed-map-action");
    match action {
        MapAction::Door => {
            let (appearance, revision) = before.ok_or("missing prior door pose")?;
            let (_, next) = wait_face(client, placement, |(face, next)| {
                face != appearance && next > revision
            })
            .await?;
            rows.pass("same-door-revised-with-changed-appearance");
            // Doors can step the user through; reacquire the reachable pose before toggling.
            approach_action(client, routes, placement.at, action).await?;
            use_map_action(client, world, placement, action, None, next).await?;
            wait_face(client, placement, |(face, latest)| {
                face == appearance && latest > next
            })
            .await?;
        }
        MapAction::Terrain {
            after,
            decay,
            final_state,
            ..
        } => {
            wait_face(client, placement, |(face, next)| {
                face == after && next > revision
            })
            .await?;
            rows.pass("exact-authored-terrain-transform");
            let Some(final_state) = final_state else {
                rows.pass("content-has-no-decay-reset");
                return Ok(());
            };
            if std::env::var_os("OTERYN_QUALIFICATION_CLOCK_FILE").is_none() {
                return Err(
                    "terrain-reset-harness-unavailable: requires authored decay clock control"
                        .into(),
                );
            }
            full_e2ef7::advance_qualification_time(decay + Duration::from_secs(1))?;
            wait_face(client, placement, |(face, next)| {
                face == final_state && next > revision + 1
            })
            .await?;
        }
        MapAction::Lever {
            poses,
            reversible,
            has_secondary_effects,
        } => {
            let (appearance, revision) = before.ok_or("missing prior lever pose")?;
            let expected = if appearance == poses[0] {
                poses[1]
            } else if appearance == poses[1] {
                poses[0]
            } else {
                return Err("lever-pose-mismatch: original face is outside authored pair".into());
            };
            let (_, next) = wait_face(client, placement, |(face, next)| {
                face == expected && next > revision
            })
            .await?;
            if has_secondary_effects {
                if reversible {
                    approach(client, routes, placement.at).await?;
                    use_map_action(client, world, placement, action, None, next).await?;
                    wait_face(client, placement, |(face, latest)| {
                        face == appearance && latest > next
                    })
                    .await?;
                }
                return Err("callback-effect-harness-unavailable: lever changed but authored secondary effects were not proved".into());
            }
            rows.pass("exact-authored-lever-pose");
            if !reversible {
                return Err(
                    "callback-reset-harness-unavailable: one-way lever requires its authored reset"
                        .into(),
                );
            }
            approach(client, routes, placement.at).await?;
            use_map_action(client, world, placement, action, None, next).await?;
            wait_face(client, placement, |(face, latest)| {
                face == appearance && latest > next
            })
            .await?;
        }
        MapAction::Callback => {
            return Err(
                "callback-effect-harness-unavailable: no placement-specific effect oracle".into(),
            );
        }
        _ => {
            let expected = movement_landing(world, placement.at, action)
                .ok_or("movement-landing-unavailable: owner landing absent from served map")?;
            let actual = movement_observation(client, prior_position).await?;
            if actual != expected {
                return Err(format!(
                    "movement-effect-mismatch: expected {expected:?}, observed {actual:?}"
                )
                .into());
            }
            rows.pass("exact-authored-movement-landing");
            approach_action(client, routes, placement.at, action).await?;
            let prior_position = tile(client.world_spatial().actor_position)?;
            use_map_action(client, world, placement, action, tool, revision).await?;
            if movement_observation(client, prior_position).await? != expected {
                return Err("movement-reset-mismatch: second activation changed landing".into());
            }
        }
    }
    rows.pass("authored-reset-observed");
    Ok(())
}

fn adjacent(a: TilePos, b: TilePos) -> bool {
    a.floor == b.floor && a.x.abs_diff(b.x).max(a.y.abs_diff(b.y)) == 1
}

async fn approach(
    client: &mut DevClientSession,
    routes: &SweepRoutes<'_>,
    target: TilePos,
) -> TestResult {
    approach_action(client, routes, target, MapAction::Callback).await
}

fn reading_matches(served: &str, authored: &str) -> bool {
    served
        .split_once("\nYou read: ")
        .is_some_and(|(_, text)| text == authored)
}

async fn read_document(
    client: &mut DevClientSession,
    rows: &mut Rows<'_>,
    routes: &SweepRoutes<'_>,
    placement: Placement,
    expected: &str,
) -> TestResult {
    approach(client, routes, placement.at).await?;
    rows.pass("adjacent-to-authored-inscription");
    let selection = look::ItemLookSelection { extended: true };
    let payload = look::encode_item_look_intent_for(
        look::ItemLookIntent {
            target: look::ItemLookTarget::WorldObject(placement.target(routes.world)),
        },
        selection,
    )
    .map_err(|error| format!("look-encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            look::ITEM_LOOK_CAPABILITY_ID.ok_or("LOOK capability unallocated")?,
            look::ITEM_LOOK_COMMAND_ID.ok_or("LOOK command unallocated")?,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("look-refused: {:?}", reply.status).into());
    }
    let result = look::decode_item_look_result_for(&reply.payload, selection)
        .map_err(|error| format!("look-decode: {error:?}"))?;
    let look::ItemLookResult::Text(text) = result else {
        return Err(format!("look-refused: {result:?}").into());
    };
    rows.pass("accepted-item-look");
    if !reading_matches(&text, expected) {
        return Err("document-text-mismatch: complete authored text required".into());
    }
    rows.pass("byte-identical-authored-text");
    Ok(())
}

type Quantities = BTreeMap<NonZeroU32, u64>;

fn inventory(client: &DevClientSession) -> TestResult<Quantities> {
    let inventory = client.inventory().ok_or("inventory snapshot unavailable")?;
    let mut quantities = BTreeMap::new();
    for item in inventory
        .entries
        .iter()
        .chain(&inventory.main_backpack)
        .chain(inventory.equipment.iter().map(|entry| &entry.item))
    {
        *quantities.entry(item.item_definition_ref).or_default() += u64::from(item.count.get());
    }
    Ok(quantities)
}

fn exact_reward(before: &Quantities, after: &Quantities, reward: &Quantities) -> bool {
    let mut expected = before.clone();
    for (&item, &count) in reward {
        *expected.entry(item).or_default() += count;
    }
    !reward.is_empty() && *after == expected
}

async fn use_target(
    client: &mut DevClientSession,
    target: &WorldObjectTarget,
) -> TestResult<UseDisposition> {
    Ok(client
        .session_mut()
        .use_object(&target.placement, target.expected_revision)
        .await?
        .disposition)
}

async fn chest(
    client: &mut DevClientSession,
    rows: &mut Rows<'_>,
    routes: &SweepRoutes<'_>,
    items: &crate::content::item_ref::ItemDefinitionIndex,
    placements: &[BoundPlacement],
) -> TestResult {
    let positions = placements.iter().map(|placement| TilePos {
        x: placement.cell.x,
        y: placement.cell.y,
        floor: placement.cell.floor,
    });
    let goal = routes
        .reach(
            client,
            positions.clone().flat_map(|at| approaches(at, false)),
        )
        .await?;
    let (placement, at) = placements
        .iter()
        .zip(positions)
        .find(|(_, at)| adjacent(goal, *at))
        .ok_or("claim route has no authored placement at its approach")?;
    rows.pass("adjacent-to-claim-placement");
    let target = Placement {
        at,
        key: placement.bundle_placement_key,
    }
    .target(routes.world);
    let before = inventory(client)?;
    let before_handles: std::collections::BTreeSet<_> = client
        .inventory()
        .ok_or("inventory snapshot unavailable")?
        .entries
        .iter()
        .map(|item| item.handle)
        .collect();
    let result = use_target(client, &target).await?;
    if result != UseDisposition::Committed {
        return Err(format!("use-refused: {result:?}").into());
    }
    rows.pass("committed-use");
    if placement.reward_callback.is_some() {
        return Err("reward-callback-oracle-missing: claim defines a custom reward owner".into());
    }
    let mut reward = Quantities::new();
    for entry in &placement.placement.items {
        let reference = items
            .definition_ref(
                "Item",
                entry.item.key().as_str(),
                entry.item.revision().as_str(),
            )
            .ok_or("authored reward item not indexed")?;
        *reward.entry(reference).or_default() += u64::from(entry.count);
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let awarded = loop {
        let observed = inventory(client)?;
        if exact_reward(&before, &observed, &reward) {
            break observed;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                "reward-delta-mismatch: exact authored quantities absent from wire inventory"
                    .into(),
            );
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    };
    if !placement.contents.is_empty() {
        check_reward_container(client, items, placement, &before_handles).await?;
    }
    rows.pass("exact-authored-inventory-delta");
    let result = use_target(client, &target).await?;
    if result != UseDisposition::NothingToUse || inventory(client)? != awarded {
        return Err(
            format!("once-claim-mismatch: repeat USE={result:?} or inventory changed").into(),
        );
    }
    rows.pass("once-claim-refused-without-reward");
    Ok(())
}

async fn check_reward_container(
    client: &mut DevClientSession,
    items: &crate::content::item_ref::ItemDefinitionIndex,
    placement: &BoundPlacement,
    before: &std::collections::BTreeSet<oteryn_protocol_oteryn::item_view::ItemHandle>,
) -> TestResult {
    use oteryn_protocol_oteryn::container_tree::{ContainerViewIntent, ContainerViewOutcome};

    let root = &placement
        .placement
        .items
        .first()
        .ok_or("reward container root missing")?
        .item;
    let reference = items
        .definition_ref("Item", root.key().as_str(), root.revision().as_str())
        .ok_or("reward container root is not indexed")?;
    let handle = client
        .inventory()
        .ok_or("inventory snapshot unavailable")?
        .entries
        .iter()
        .find(|item| item.item_definition_ref == reference && !before.contains(&item.handle))
        .ok_or("reward-container-missing: no new authored container handle")?
        .handle;

    let opened = client
        .session_mut()
        .container_view(ContainerViewIntent::Open {
            handle,
            replace_view: None,
        })
        .await?;
    if opened != Some(ContainerViewOutcome::Opened) {
        return Err(format!("reward-container-refused: {opened:?}").into());
    }

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let (view_id, contents) = loop {
        if let Some(view) = client.session_mut().container_views().and_then(|views| {
            views
                .views
                .iter()
                .find(|view| view.container_handle == handle)
        }) {
            let mut counts = Quantities::new();
            for item in &view.entries {
                *counts.entry(item.item_definition_ref).or_default() += u64::from(item.count.get());
            }
            break (view.view_id, counts);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("reward-container-missing: opened container view not observed".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    };
    let result = client
        .session_mut()
        .container_view(ContainerViewIntent::Close { view_id })
        .await?;
    if result != Some(ContainerViewOutcome::Closed) {
        return Err(format!("reward-container-close-refused: {result:?}").into());
    }

    let mut expected = Quantities::new();
    for item in &placement.contents {
        let reference = items
            .definition_ref(
                "Item",
                item.item.key().as_str(),
                item.item.revision().as_str(),
            )
            .ok_or("authored container reward is not indexed")?;
        *expected.entry(reference).or_default() += u64::from(item.count);
    }
    if expected != contents {
        return Err("reward-container-mismatch: exact authored child quantities required".into());
    }
    Ok(())
}

fn shrine_position(
    routes: &SweepRoutes<'_>,
    source_ids: &[NonZeroU32],
    start: TilePos,
) -> TestResult<TilePos> {
    let world = routes.world;
    let facts = world.facts().pinned();
    let placements: Vec<_> = world.base().tiles().filter_map(|(floor, x, y, tile)| {
        let at = TilePos { x, y, floor };
        let authored = tile.ids().iter().zip(tile.depths()).filter(|(_, depth)| **depth == 0)
            .enumerate().any(|(ordinal, (id, _))| {
                u8::try_from(ordinal).ok().and_then(|index| facts.base_entry(at, index, *id))
                    .is_some_and(|entry| matches!(entry.definition, MapDefinition::Item(id) if source_ids.contains(&id)))
            });
        authored.then_some(at)
    }).collect();
    if placements.is_empty() {
        return Err("shrine-placement-missing: no authored shrine in bundle".into());
    }
    let (goal, _) = routes.find(
        start,
        placements.iter().flat_map(|at| approaches(*at, false)),
    )?;
    placements
        .into_iter()
        .find(|at| adjacent(goal, *at))
        .ok_or_else(|| "shrine route has no authored placement at its approach".into())
}

async fn wall(client: &mut DevClientSession) -> TestResult<DailyRewardWall> {
    // The panel's send(Query) opens the shrine first. Query the raw wall so
    // the opening and reset assertions cannot be satisfied by the query.
    let reply = client
        .session_mut()
        .gated_command(
            daily::CAPABILITY_DAILY_REWARD_V2,
            daily::COMMAND_TYPE_DAILY_REWARD_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("shrine-wall-refused: {:?}", reply.status).into());
    }
    match decode_reply(DailyRewardCommand::Query, &reply.payload)? {
        DailyRewardReply::Wall(wall) => Ok(wall),
        result => Err(format!("shrine-wall-refused: {result:?}").into()),
    }
}

async fn claim_daily(client: &mut DevClientSession, disposition: u8) -> TestResult {
    let reply = client
        .session_mut()
        .gated_command(
            daily::CAPABILITY_DAILY_REWARD_V2,
            daily::COMMAND_TYPE_DAILY_REWARD_CLAIM,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || !matches!(
            decode_reply(DailyRewardCommand::Claim, &reply.payload)?,
            DailyRewardReply::Result(result)
                if result.disposition == disposition
                    && result.transaction_id.is_some() == (disposition == 1)
        )
    {
        return Err(format!(
            "shrine-claim-refused: expected disposition {disposition}, got {reply:?}"
        )
        .into());
    }
    Ok(())
}

async fn shrine(
    client: &mut DevClientSession,
    rows: &mut Rows<'_>,
    routes: &SweepRoutes<'_>,
    at: TilePos,
    content: &DailyRewardContent,
) -> TestResult {
    approach(client, routes, at).await?;
    rows.pass("adjacent-to-authored-shrine");
    let before = wall(client).await?;
    if before.claim_enabled || before.claimed {
        return Err("shrine-precondition: expected unopened-unclaimed wall".into());
    }
    let opened = client
        .session_mut()
        .use_object(content.entry_shrine.placement.as_bytes(), 0)
        .await?;
    if opened.disposition != UseDisposition::Committed {
        return Err(format!("use-refused: {:?}", opened.disposition).into());
    }
    rows.pass("committed-shrine-use");
    let after = wall(client).await?;
    if after
        != (DailyRewardWall {
            claim_enabled: true,
            ..before
        })
    {
        return Err(format!("shrine-wall-mismatch: {after:?}").into());
    }
    rows.pass("claim-enabled-with-unchanged-entitlement");
    Ok(())
}

async fn release(client: &mut DevClientSession) -> TestResult {
    let (result, detail, attempts, busy) = logout_retrying_busy(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("logout-refused: {detail}, attempts={attempts}, busy={busy}").into());
    }
    Ok(())
}

async fn session<'a>(
    slot: &'a mut Option<DevClientSession>,
    clients: &SeamClients<'_>,
) -> TestResult<&'a mut DevClientSession> {
    if let Some(client) = slot.as_mut()
        && let Err(error) = client
            .session_mut()
            .service_liveness_discarding_events(Duration::from_millis(25))
            .await
    {
        evidence(&format!(
            "interaction_sweep session=lost before=entry error={error}"
        ));
        *slot = None;
    }
    if slot.is_none() {
        *slot = Some(connect_world(clients, 0x92).await?);
    }
    slot.as_mut().ok_or_else(|| "missing sweep session".into())
}

fn bound_claim<'a>(bindings: &'a BindingReport, key: &str) -> TestResult<&'a [BoundPlacement]> {
    if let Some(claim) = bindings
        .served
        .iter()
        .find(|claim| claim.key.as_str() == key)
    {
        if claim.placements.is_empty() {
            return Err("claim-binding-missing: served claim has no placements".into());
        }
        return Ok(&claim.placements);
    }
    let reason = bindings
        .unbound
        .iter()
        .find(|entry| entry.claim == key)
        .map(|entry| format!("{:?}", entry.reason))
        .or_else(|| {
            bindings
                .left_out
                .iter()
                .find(|(claim, _)| claim == key)
                .map(|(_, reason)| format!("{reason:?}"))
        })
        .unwrap_or_else(|| "absent from server binding catalogue".to_owned());
    Err(format!("claim-binding-missing: {reason}").into())
}

pub(super) async fn run(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    bytes: &[u8],
) -> TestResult {
    let setting = match std::env::var(BATCH_VARIABLE) {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => return Err(format!("invalid {BATCH_VARIABLE}: {error}").into()),
    };
    let slices = slices(setting.as_deref())?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    verdict::Harness::global().progress("interaction-sweep-census");
    let mut entries = census(&root)?;
    map_entries(bytes, world, &mut entries)?;
    let total = entries.len();
    for family in FAMILIES {
        let count = entries
            .values()
            .filter(|entry| entry.family() == *family)
            .count();
        evidence(&format!(
            "interaction_sweep census family={family} entries={count} total={total}"
        ));
    }
    let reports = slices
        .iter()
        .any(|&(family, _, _)| matches!(family, "all" | "documents"))
        .then(|| report_races(&root))
        .transpose()?
        .unwrap_or_default();
    let mut selected_entries = Vec::new();
    for &(family, start, len) in &slices {
        let matching = entries
            .iter()
            .filter(|(_, entry)| family == "all" || entry.family() == family);
        let count = matching.clone().count();
        if start >= count {
            return Err(format!(
                "{BATCH_VARIABLE} start {start} is beyond {family} census of {count}"
            )
            .into());
        }
        let selected = len.min(count - start);
        evidence(&format!(
            "interaction_sweep batch family={family} start={start} len={len} selected={selected} mode=seeded"
        ));
        selected_entries.extend(
            matching
                .skip(start)
                .take(selected)
                .map(|(key, entry)| (family, key, entry)),
        );
    }
    let mut connections = BTreeMap::<TilePos, Vec<RouteStep>>::new();
    for entry in entries.values() {
        if let Entry::Interaction(placement, action @ (MapAction::Ladder | MapAction::Sewer)) =
            entry
            && let Some(landing) = movement_landing(world, placement.at, *action)
            && world.enterable(landing)
            && world
                .base()
                .tile(landing.x, landing.y, landing.floor)
                .is_some_and(|tile| tile.walkable())
        {
            for at in approaches(placement.at, false) {
                connections.entry(at).or_default().push(RouteStep::Use {
                    placement: *placement,
                    action: *action,
                    landing,
                });
            }
        }
    }
    let selected = selected_entries.len();
    let entries = selected_entries.into_iter();
    let items = entries
        .clone()
        .any(|(_, _, entry)| matches!(entry, Entry::Claim | Entry::Shrine(_)))
        .then(provisioning::item_index);
    let bindings = entries
        .clone()
        .any(|(_, _, entry)| matches!(entry, Entry::Claim))
        .then(|| {
            let items = items
                .as_ref()
                .ok_or("missing claim item index")?
                .as_ref()
                .map_err(|error| format!("item-index: {error}"))?;
            Ok::<_, Box<dyn std::error::Error>>(bind_embedded_reward_claims(
                world.base().as_ref(),
                &|item| {
                    items
                        .definition_ref("Item", item.key().as_str(), item.revision().as_str())
                        .is_some()
                },
            )?)
        });
    let routes = SweepRoutes {
        world,
        exhausted: Default::default(),
        connections,
    };
    let mut client = None;
    let mut tallies = BTreeMap::<&str, Tally>::new();
    let began = Instant::now();
    for (family, key, entry) in entries {
        verdict::Harness::global().progress("interaction-sweep-entry");
        let mut rows = Rows {
            key: row_token(key),
            steps: entry.steps(),
            next: 0,
            lap: Instant::now(),
            tally: tallies.entry(family).or_default(),
        };
        // A live map approach can require hundreds of paced STEP exchanges.
        // The protocol still bounds each exchange and the entry has a finite
        // wall deadline, while reports have no walking phase.
        let deadline_seconds = match entry {
            Entry::Document(_) | Entry::NestedMapDocument => 90,
            _ => 600,
        };
        let result = tokio::time::timeout(Duration::from_secs(deadline_seconds), async {
            match entry {
                Entry::Document(document) => {
                    let race = reports.get(key).copied().flatten().ok_or(
                        "bestiary-race-unavailable: authored report has no canonical wire race",
                    )?;
                    rows.pass("authored-creature-profile-to-bestiary-race");
                    read_report(
                        session(&mut client, clients).await?,
                        &mut rows,
                        race,
                        &document.content.join("\n"),
                    )
                    .await
                }
                Entry::MapDocument(placement, text) => {
                    rows.pass("authored-bundle-placement-text");
                    read_document(
                        session(&mut client, clients).await?,
                        &mut rows,
                        &routes,
                        *placement,
                        text,
                    )
                    .await
                }
                Entry::NestedMapDocument => Err("nested-document-harness-unavailable: requires acquiring authored container custody".into()),
                Entry::Interaction(placement, action) => {
                    rows.pass("authored-action-placement");
                    map_action(&mut client, clients, &mut rows, &routes, *placement, *action).await
                }
                Entry::Claim => {
                    let items = items
                        .as_ref()
                        .ok_or("missing claim item index")?
                        .as_ref()
                        .map_err(|error| format!("item-index: {error}"))?;
                    let bindings = bindings
                        .as_ref()
                        .ok_or("missing claim bindings")?
                        .as_ref()
                        .map_err(|error| format!("claim-index: {error}"))?;
                    let placement = bound_claim(bindings, key)?;
                    rows.pass("server-bound-authored-chest");
                    chest(
                        session(&mut client, clients).await?,
                        &mut rows,
                        &routes,
                        items,
                        placement,
                    )
                    .await
                }
                Entry::Shrine(content) => {
                    let items = items
                        .as_ref()
                        .ok_or("missing shrine item index")?
                        .as_ref()
                        .map_err(|error| format!("item-index: {error}"))?;
                    let candidate_ids = content
                        .shrine_source_ids
                        .iter()
                        .map(|&id| provisioning::item_reference(items, id))
                        .collect::<TestResult<Vec<_>>>()?;
                    let connected = session(&mut client, clients).await?;
                    let at = shrine_position(
                        &routes,
                        &candidate_ids,
                        tile(connected.world_spatial().actor_position)?,
                    )?;
                    rows.pass("authored-shrine-source-item");
                    shrine(connected, &mut rows, &routes, at, content).await?;
                    release(connected).await?;
                    client = None;
                    if wall(session(&mut client, clients).await?)
                        .await?
                        .claim_enabled
                    {
                        return Err(
                            "shrine-reset-mismatch: claim remains enabled after fresh admission"
                                .into(),
                        );
                    }
                    rows.pass("shrine-open-state-reset-on-admission");
                    let connected = session(&mut client, clients).await?;
                    approach(connected, &routes, at).await?;
                    let before = wall(connected).await?;
                    let opened = connected.session_mut().use_object(content.entry_shrine.placement.as_bytes(), 0).await?;
                    if opened.disposition != UseDisposition::Committed {
                        return Err(format!("shrine-reopen-refused: {:?}", opened.disposition).into());
                    }
                    claim_daily(connected, 1).await?;
                    let claimed = wall(connected).await?;
                    if !claimed.claimed || claimed.claim_enabled
                        || claimed.streak != before.streak.saturating_add(1).min(100_000)
                    {
                        return Err(format!("shrine-claim-effect-mismatch: {before:?} -> {claimed:?}").into());
                    }
                    let custody = inventory(connected)?;
                    rows.pass("daily-claim-committed-once");
                    claim_daily(connected, 2).await?;
                    if wall(connected).await? != claimed || inventory(connected)? != custody {
                        return Err("shrine-replay-mismatch: duplicate changed the wall or inventory".into());
                    }
                    rows.pass("duplicate-daily-claim-did-not-mutate-state");
                    release(connected).await?;
                    client = None;
                    let connected = session(&mut client, clients).await?;
                    if wall(connected).await? != claimed || inventory(connected)? != custody {
                        return Err("shrine-durability-mismatch: daily claim lost after fresh admission".into());
                    }
                    rows.pass("daily-claim-persisted-across-admissions");
                    Ok(())
                }
            }
        })
        .await;
        match result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => rows.fail(&error.to_string()),
            Err(_) => {
                client = None;
                rows.fail(&format!(
                    "deadline: entry exceeded {deadline_seconds} seconds"
                ));
            }
        }
    }
    let release = match client.as_mut() {
        Some(client) => {
            match tokio::time::timeout(Duration::from_secs(30), release(client)).await {
                Ok(result) => result,
                Err(_) => Err("interaction sweep final logout deadline".into()),
            }
        }
        None => Ok(()),
    };
    for (family, tally) in &tallies {
        evidence(&format!(
            "interaction_sweep slice family={family} rows_pass={} rows_fail={}",
            tally.pass, tally.fail
        ));
    }
    let pass: usize = tallies.values().map(|tally| tally.pass).sum();
    let fail: usize = tallies.values().map(|tally| tally.fail).sum();
    evidence(&format!(
        "interaction_sweep summary total={total} selected={selected} rows_pass={} rows_fail={} seconds={:.3} mode=seeded",
        pass,
        fail,
        began.elapsed().as_secs_f64()
    ));
    release?;
    if fail != 0 {
        return Err(format!("interaction_sweep has {fail} FAIL rows").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualification_interaction_census_reads_authored_shards() -> TestResult {
        let entries = census(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))?;
        assert_eq!(entries.len(), 1956);
        assert!(matches!(
            entries.first_key_value(),
            Some((_, Entry::Document(_)))
        ));
        assert!(matches!(
            entries.get("oteryn:placement/daily-reward-shrine"),
            Some(Entry::Shrine(_))
        ));
        assert!(matches!(
            entries.get("oteryn:reward-claim.halls_of_hope.reward1"),
            Some(Entry::Claim)
        ));
        Ok(())
    }

    #[test]
    fn qualification_interaction_report_races_follow_profile_references() -> TestResult {
        let reports = report_races(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))?;
        assert_eq!(reports.len(), 1609);
        let prefix = "oteryn:document/monster-encyclopedia";
        assert_eq!(reports.get(&format!("{prefix}/a_greedy_eye")), Some(&None));
        assert!(
            reports
                .get(&format!("{prefix}/acid_blob"))
                .copied()
                .flatten()
                .is_some()
        );
        Ok(())
    }

    #[test]
    fn qualification_interaction_family_batches_are_independent() -> TestResult {
        assert_eq!(
            slices(Some(
                "documents=0:40,map_documents=0:40,interactions=0:60,claims=0:40"
            ))?
            .len(),
            4
        );
        assert_eq!(slices(Some("1:2"))?, vec![("all", 1, 2)]);
        assert!(slices(Some("documents=0:40,documents=5:1")).is_err());
        assert!(slices(Some("bogus=0:1")).is_err());
        Ok(())
    }

    #[test]
    fn qualification_interaction_batch_requires_plain_nonzero_length() -> TestResult {
        assert_eq!(batch(None)?, (0, 3));
        assert_eq!(batch(Some("0:20"))?, (0, 20));
        assert_eq!(batch(Some("1609:3"))?, (1609, 3));
        for invalid in [
            "",
            "1",
            "1:0",
            "-1:1",
            "+1:1",
            " 0:1",
            "1:1:1",
            "1:",
            "184467440737095516160:1",
        ] {
            assert!(batch(Some(invalid)).is_err(), "{invalid}");
        }
        Ok(())
    }

    #[test]
    fn qualification_interaction_document_oracle_requires_complete_exact_text() {
        let authored = "Uma inscrição.\nA segunda linha.";
        assert!(reading_matches(
            &format!("You see a sign.\nYou read: {authored}"),
            authored
        ));
        assert!(!reading_matches(
            "You see a sign.\nYou read: Uma inscrição.",
            authored
        ));
        assert!(!reading_matches(&format!("You read: {authored}"), authored));
        assert!(!reading_matches(
            &format!("You see a sign.\nYou read: {authored}\n"),
            authored
        ));
    }

    #[test]
    fn qualification_interaction_callback_teleport_requires_a_complete_oracle() -> TestResult {
        use oteryn_gameplay_quests::quest::world_callbacks as callbacks;

        let mut callback = callbacks::load()?
            .into_iter()
            .find(|callback| callback.key == "oteryn:quest-callback/deeper-fibula-well")
            .ok_or("missing fixture callback")?;
        assert_eq!(
            callback_teleport(&callback),
            Some(TilePos {
                x: 32172,
                y: 32439,
                floor: -8,
            })
        );
        callback.success_message = Some("A second authored effect".into());
        assert_eq!(callback_teleport(&callback), None);
        Ok(())
    }

    #[test]
    fn qualification_interaction_reward_oracle_rejects_missing_extra_and_unrelated_changes()
    -> TestResult {
        let item = NonZeroU32::new(11).ok_or("item")?;
        let other = NonZeroU32::new(22).ok_or("other item")?;
        let before = Quantities::from([(item, 1), (other, 1)]);
        let reward = Quantities::from([(item, 2)]);
        assert!(exact_reward(
            &before,
            &Quantities::from([(item, 3), (other, 1)]),
            &reward
        ));
        assert!(!exact_reward(&before, &before, &reward));
        assert!(!exact_reward(
            &before,
            &Quantities::from([(item, 4), (other, 1)]),
            &reward
        ));
        assert!(!exact_reward(
            &before,
            &Quantities::from([(item, 3)]),
            &reward
        ));
        Ok(())
    }
}
