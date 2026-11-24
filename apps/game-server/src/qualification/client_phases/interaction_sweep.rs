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
    Claim,
    Shrine(DailyRewardContent),
}

impl Entry {
    fn steps(&self) -> &'static [&'static str] {
        match self {
            Self::Document(_) => &["bind", "walk", "open", "text"],
            Self::Claim | Self::Shrine(_) => &["bind", "walk", "use", "effect", "reset"],
        }
    }
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

fn readings(bytes: &[u8]) -> TestResult<BTreeMap<String, Option<Placement>>> {
    let mut readings = BTreeMap::new();
    oteryn_world_bundle::bundle::visit(bytes, |_, sector| {
        for tile in sector.tiles {
            let at = TilePos {
                x: tile.x,
                y: tile.y,
                floor: sector.floor,
            };
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                if let Some(text) = item.attrs.text.as_ref().filter(|text| !text.is_empty())
                    && let Ok(ordinal) = u8::try_from(ordinal)
                    && let Some(key) = crate::map::view::placement_key(at, ordinal)
                {
                    readings
                        .entry(text.clone())
                        .and_modify(|previous| *previous = None)
                        .or_insert(Some(Placement { at, key }));
                }
            }
        }
        Ok(())
    })?;
    Ok(readings)
}

fn adjacent(a: TilePos, b: TilePos) -> bool {
    a.floor == b.floor && a.x.abs_diff(b.x).max(a.y.abs_diff(b.y)) == 1
}

async fn approach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    target: TilePos,
) -> TestResult {
    let start = tile(client.world_spatial().actor_position)?;
    let path = find_route(
        start,
        |at| adjacent(at, target),
        |at, direction| destination(world, at, direction),
    )
    .map_err(|visited| format!("no-route: {start:?} to {target:?}, visited {visited} cells"))?;
    walk(client, world, &path).await?;
    if !adjacent(tile(client.world_spatial().actor_position)?, target) {
        return Err("wrong-position: client did not arrive adjacent".into());
    }
    Ok(())
}

fn reading_matches(served: &str, authored: &str) -> bool {
    served
        .split_once("\nYou read: ")
        .is_some_and(|(_, text)| text == authored)
}

async fn read_document(
    client: &mut DevClientSession,
    rows: &mut Rows<'_>,
    world: &BundleWorld,
    placement: Placement,
    expected: &str,
) -> TestResult {
    approach(client, world, placement.at).await?;
    rows.pass("adjacent-to-authored-inscription");
    let selection = look::ItemLookSelection { extended: true };
    let payload = look::encode_item_look_intent_for(
        look::ItemLookIntent {
            target: look::ItemLookTarget::WorldObject(placement.target(world)),
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
    world: &BundleWorld,
    items: &crate::content::item_ref::ItemDefinitionIndex,
    placement: &BoundPlacement,
) -> TestResult {
    let at = TilePos {
        x: placement.cell.x,
        y: placement.cell.y,
        floor: placement.cell.floor,
    };
    approach(client, world, at).await?;
    rows.pass("adjacent-to-claim-placement");
    let target = Placement {
        at,
        key: placement.bundle_placement_key,
    }
    .target(world);
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
    world: &BundleWorld,
    source_ids: &[NonZeroU32],
    start: TilePos,
) -> TestResult<TilePos> {
    let facts = world.facts().pinned();
    world.base().tiles().filter_map(|(floor, x, y, tile)| {
        let at = TilePos { x, y, floor };
        if start.x.abs_diff(x) > 160 || start.y.abs_diff(y) > 160 {
            return None;
        }
        let authored = tile.ids().iter().zip(tile.depths()).filter(|(_, depth)| **depth == 0)
            .enumerate().any(|(ordinal, (id, _))| {
                u8::try_from(ordinal).ok().and_then(|index| facts.base_entry(at, index, *id))
                    .is_some_and(|entry| matches!(entry.definition, MapDefinition::Item(id) if source_ids.contains(&id)))
            });
        (authored && route(world, start, |candidate| adjacent(candidate, at)).is_some()).then_some(at)
    }).next().ok_or_else(|| "shrine-placement-missing: no reachable authored shrine in bundle".into())
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

async fn shrine(
    client: &mut DevClientSession,
    rows: &mut Rows<'_>,
    world: &BundleWorld,
    at: TilePos,
    content: &DailyRewardContent,
) -> TestResult {
    approach(client, world, at).await?;
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
    if slot.is_none() {
        *slot = Some(connect_world(clients, 0x92).await?);
    }
    slot.as_mut().ok_or_else(|| "missing sweep session".into())
}

fn bound_claim<'a>(bindings: &'a BindingReport, key: &str) -> TestResult<&'a BoundPlacement> {
    if let Some(claim) = bindings
        .served
        .iter()
        .find(|claim| claim.key.as_str() == key)
    {
        if let [placement] = claim.placements.as_slice() {
            return Ok(placement);
        }
        return Err(
            "claim-placement-oracle-missing: multiple placements need isolated once-claim fixtures"
                .into(),
        );
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
    let (start, len) = batch(setting.as_deref())?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    verdict::Harness::global().progress("interaction-sweep-census");
    let entries = census(&root)?;
    let documents = entries
        .values()
        .filter(|entry| matches!(entry, Entry::Document(_)))
        .count();
    let claims = entries
        .values()
        .filter(|entry| matches!(entry, Entry::Claim))
        .count();
    let total = entries.len();
    evidence(&format!(
        "interaction_sweep census total={total} documents={documents} claims={claims} shrines=1"
    ));
    if start >= total {
        return Err(format!("{BATCH_VARIABLE} start {start} is beyond the {total} entries").into());
    }
    let selected = len.min(total - start);
    evidence(&format!(
        "interaction_sweep batch start={start} len={len} selected={selected} mode=seeded"
    ));
    let entries = entries.iter().skip(start).take(selected);
    let items = entries
        .clone()
        .any(|(_, entry)| !matches!(entry, Entry::Document(_)))
        .then(provisioning::item_index);
    let bindings = entries
        .clone()
        .any(|(_, entry)| matches!(entry, Entry::Claim))
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
    let readings = entries
        .clone()
        .any(|(_, entry)| matches!(entry, Entry::Document(_)))
        .then(|| readings(bytes));
    let mut client = None;
    let mut tally = Tally::default();
    let began = Instant::now();
    for (key, entry) in entries {
        verdict::Harness::global().progress("interaction-sweep-entry");
        let mut rows = Rows {
            key: row_token(key),
            steps: entry.steps(),
            next: 0,
            lap: Instant::now(),
            tally: &mut tally,
        };
        let result = tokio::time::timeout(Duration::from_secs(90), async {
            match entry {
                Entry::Document(document) => {
                    let expected = document.content.join("\n");
                    // Reports have no item/placement reference. Require a unique exact
                    // authored inscription before making a reading claim.
                    let readings = readings
                        .as_ref()
                        .ok_or("missing document map index")?
                        .as_ref()
                        .map_err(|error| format!("document-map-index: {error}"))?;
                    let placement = match readings.get(&expected) {
                        Some(Some(placement)) => *placement,
                        Some(None) => {
                            return Err(
                                "document-binding-ambiguous: multiple map inscriptions match"
                                    .into(),
                            );
                        }
                        None => return Err(
                            "document-binding-missing: no item link or matching map inscription"
                                .into(),
                        ),
                    };
                    rows.pass("unique-authored-map-inscription");
                    read_document(
                        session(&mut client, clients).await?,
                        &mut rows,
                        world,
                        placement,
                        &expected,
                    )
                    .await
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
                        world,
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
                        world,
                        &candidate_ids,
                        tile(connected.world_spatial().actor_position)?,
                    )?;
                    rows.pass("authored-shrine-source-item");
                    shrine(connected, &mut rows, world, at, content).await?;
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
                rows.fail("deadline: entry exceeded 90 seconds");
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
    evidence(&format!(
        "interaction_sweep summary total={total} start={start} selected={selected} rows_pass={} rows_fail={} seconds={:.3} mode=seeded",
        tally.pass,
        tally.fail,
        began.elapsed().as_secs_f64()
    ));
    release?;
    if tally.fail != 0 {
        return Err(format!("interaction_sweep has {} FAIL rows", tally.fail).into());
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
