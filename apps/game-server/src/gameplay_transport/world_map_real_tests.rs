//! MAP-VIEWPORT-MEASURE-1 (`ARCH-MAP-VIEWPORT-BUDGET-V1` §2, amended by CP D823): a release,
//! `#[ignore]` measurement of the domain-17 snapshot and delta over the real map, with realistic
//! positions, walks and concurrency. A measurement, not a gate: nothing fails on the numbers.
//!
//! `cargo test --release --locked -p oteryn-game-server --lib map_viewport_real -- --ignored --nocapture`
//!
//! The real map is compiled from `content/world` like `map_load_budget_compile_the_real_map`
//! (see `tests/support/real_map_bundle.rs`). The measured path is the server's own
//! `SessionMapView` over a `ChannelOverlay` of the loaded base; the entry facts come from the
//! palette (Terrain kind, appearance id) and the Item definitions (`pickupable`).

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![allow(clippy::print_stdout, clippy::print_stderr)]

use super::*;
use crate::foundation::{ChannelId, WorldId};
use crate::gameplay_transport::item_view::ItemViewContinuity;
use crate::map::overlay::AddedItem;
use crate::map::view::EntryFacts;
use crate::map::{BundlePins, WorldBase, load, palette_appearance};
use oteryn_protocol_oteryn::world_map::MapDefinition;
use std::collections::HashSet;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[path = "../../tests/support/real_map_bundle.rs"]
#[allow(dead_code, reason = "shared with the loader measurement")]
mod real_map_bundle;

use real_map_bundle::{Fallible, RealMap, repo_root, shards};

const SEEDS: [u64; 3] = [0x5eed_0b01, 0x5eed_0b02, 0x5eed_0b03];
/// Steps of one walk (§2: at least 200).
const WALK_STEPS: usize = 200;
/// Views in total at every concurrency level (§2, D128: at least 500).
const VIEWS: usize = 504;
const THREADS: [usize; 3] = [1, 4, 8];
/// A step every 200 ms: the upper bound of §2.
const STEPS_PER_SECOND: f64 = 5.0;
/// Positions drawn from towns and spawn areas, per 100 (§2: at least half).
const ANCHORED_PER_100: u64 = 60;

struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }
}

/// A native position: `(x, y, floor)`. The project's legacy `z` maps to floor `-z`.
type Pos = (u16, u16, i8);

fn actor(pos: Pos) -> ActorPosition {
    ActorPosition {
        x: i32::from(pos.0),
        y: i32::from(pos.1),
        floor: i16::from(pos.2),
    }
}

fn uuid(seed: u8) -> [u8; 16] {
    let mut bytes = [seed; 16];
    bytes[6] = 0x70 | (seed & 0x0f);
    bytes[8] = 0x80 | (seed & 0x3f);
    bytes
}

fn nz(id: u32) -> NonZeroU32 {
    NonZeroU32::new(id).expect("compact id")
}

/// The entry facts of the real palette, by compact id (palette index + 1).
struct RealFacts {
    by_id: Vec<Option<EntryFacts>>,
}

impl RealFacts {
    fn new(map: &RealMap) -> Fallible<Self> {
        let pickupable = real_map_bundle::pickupable_keys()?;
        let mut by_id = vec![None; map.keys.len() + 1];
        for (index, key) in map.keys.iter().enumerate() {
            let id = index as u32 + 1;
            let appearance_id = palette_appearance(key).unwrap_or(0);
            by_id[id as usize] = Some(match map.terrain.get(key) {
                Some(terrain) => EntryFacts {
                    definition: MapDefinition::Terrain(nz(id)),
                    terrain_kind: Some(terrain.kind),
                    appearance_id,
                    blocks_projectile: false,
                    pickupable: false,
                    bound: false,
                    count: 1,
                    sub_type: 0,
                },
                None => EntryFacts {
                    definition: MapDefinition::Item(nz(id)),
                    terrain_kind: None,
                    appearance_id,
                    blocks_projectile: false,
                    pickupable: pickupable.contains(key),
                    bound: false,
                    count: 1,
                    sub_type: 0,
                },
            });
        }
        Ok(Self { by_id })
    }
}

impl MapFacts for RealFacts {
    fn base_entry(&self, _: TilePos, _: u8, id: u32) -> Option<EntryFacts> {
        self.by_id.get(id as usize).copied().flatten()
    }

    fn added_entry(&self, _: &AddedItem) -> Option<EntryFacts> {
        None
    }

    fn house_tile(&self, _: TilePos) -> bool {
        false
    }

    fn object_revision(&self, _: u64) -> u64 {
        0
    }
}

/// The positions a measurement draws from, and the teleports of the World Project.
struct Sampler {
    /// Every enterable tile.
    enterable: Vec<Pos>,
    /// Enterable tiles of towns (house entrances) and spawn areas (spawn centres and cells).
    anchored: Vec<Pos>,
    /// Teleport tiles `(from, to)` whose both ends are enterable tiles of the base.
    teleports: Vec<(Pos, Pos)>,
    teleport_from: HashSet<Pos>,
}

fn native(x: u64, y: u64, z: i64) -> Option<Pos> {
    Some((
        u16::try_from(x).ok()?,
        u16::try_from(y).ok()?,
        i8::try_from(-z).ok()?,
    ))
}

fn pos_of(value: &serde_json::Value, floor: &str) -> Option<Pos> {
    native(
        value["x"].as_u64()?,
        value["y"].as_u64()?,
        value[floor].as_i64()?,
    )
}

fn enterable(base: &WorldBase, pos: Pos) -> bool {
    base.tile(pos.0, pos.1, pos.2)
        .is_some_and(|tile| tile.walkable())
}

impl Sampler {
    fn new(base: &WorldBase) -> Fallible<Self> {
        let root = repo_root();
        let enterable_tiles: Vec<Pos> = base
            .tiles()
            .filter(|(.., tile)| tile.walkable())
            .map(|(floor, x, y, _)| (x, y, floor))
            .collect();
        let mut anchored = Vec::new();
        for shard in shards(&root, "content/world/spawns", "spawns-")? {
            let shard: serde_json::Value = serde_json::from_slice(&shard)?;
            for record in shard["records"].as_array().ok_or("spawn records")? {
                let declaration = &record["declaration"];
                anchored.extend(pos_of(&declaration["centre"], "floor"));
                for point in declaration["points"].as_array().into_iter().flatten() {
                    anchored.extend(pos_of(&point["cell"], "floor"));
                }
            }
        }
        for shard in shards(&root, "content/houses", "houses-")? {
            let shard: serde_json::Value = serde_json::from_slice(&shard)?;
            for house in shard["houses"].as_array().ok_or("houses")? {
                anchored.extend(pos_of(&house["entrance"], "z"));
            }
        }
        let anchors_total = anchored.len();
        anchored.retain(|pos| enterable(base, *pos));
        println!(
            "anchors (spawn centres and cells, house entrances): {anchors_total}, on an enterable tile: {}",
            anchored.len()
        );
        let mut teleports = Vec::new();
        for shard in shards(&root, "content/world/transitions", "teleports-")? {
            let shard: serde_json::Value = serde_json::from_slice(&shard)?;
            for record in shard["records"].as_array().ok_or("teleport records")? {
                let declaration = &record["declaration"];
                if let (Some(from), Some(to)) = (
                    pos_of(&declaration["from"], "floor"),
                    pos_of(&declaration["to"], "floor"),
                ) && enterable(base, from)
                    && enterable(base, to)
                {
                    teleports.push((from, to));
                }
            }
        }
        println!("teleports with both ends enterable: {}", teleports.len());
        let teleport_from = teleports.iter().map(|(from, _)| *from).collect();
        Ok(Self {
            enterable: enterable_tiles,
            anchored,
            teleports,
            teleport_from,
        })
    }

    /// A start: 60 per 100 from towns and spawn areas, the rest uniform over enterable tiles.
    fn start(&self, rng: &mut Seeded) -> (Pos, bool) {
        if !self.anchored.is_empty() && rng.below(100) < ANCHORED_PER_100 {
            (
                self.anchored[rng.below(self.anchored.len() as u64) as usize],
                true,
            )
        } else {
            (
                self.enterable[rng.below(self.enterable.len() as u64) as usize],
                false,
            )
        }
    }
}

const DIRECTIONS: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

fn step(pos: Pos, direction: (i32, i32)) -> Option<Pos> {
    Some((
        u16::try_from(i32::from(pos.0) + direction.0).ok()?,
        u16::try_from(i32::from(pos.1) + direction.1).ok()?,
        pos.2,
    ))
}

/// A floor-change destination: an enterable tile one floor up or down within three tiles.
fn floor_change(base: &WorldBase, pos: Pos, rng: &mut Seeded) -> Option<Pos> {
    let mut found = Vec::new();
    for floor in [pos.2.checked_sub(1)?, pos.2.checked_add(1)?] {
        for dy in -3i32..=3 {
            for dx in -3i32..=3 {
                if let Some(candidate) = step((pos.0, pos.1, floor), (dx, dy))
                    && enterable(base, candidate)
                {
                    found.push(candidate);
                }
            }
        }
    }
    (!found.is_empty()).then(|| found[rng.below(found.len() as u64) as usize])
}

// ---------------------------------------------------------------------------------------------
// The staged update: `SessionMapView::update` with a clock between its stages. The body is the
// production body; `staged_equals_update` checks that it sends the same bytes.

#[derive(Default, Clone, Copy)]
struct Stages {
    plan: u64,
    handles: u64,
    tiles: u64,
    encode: u64,
    total: u64,
}

fn staged_update(
    view: &mut SessionMapView,
    items: &mut SessionItemView,
    source: &MapViewSource<'_, RealFacts>,
    actor: ActorPosition,
    force_snapshot: bool,
) -> Result<(Option<MapUpdate>, Stages), MapViewError> {
    let started = Instant::now();
    if force_snapshot {
        view.sent = None;
    }
    let plan = plan_with(source, actor, std::mem::take(&mut view.buffers))?;
    let planned_at = started.elapsed();
    let sent = view.sent.as_ref();
    let (mut tiles_ns, mut encode_ns) = (Duration::ZERO, Duration::ZERO);
    let map_started = Instant::now();
    let (planned, tiles) = items.map_view(plan.keys(), |table| {
        let at = Instant::now();
        let tiles = plan
            .tiles(table, source)
            .map_err(|_| ItemViewError::Encode)?;
        tiles_ns = at.elapsed();
        let at = Instant::now();
        let encoded = encode(sent, &plan.header, tiles).map_err(|_| ItemViewError::Encode);
        encode_ns = at.elapsed();
        encoded
    })?;
    let map_ns = map_started.elapsed();
    let update = match planned {
        Planned::Nothing => None,
        Planned::Snapshot(payload) => Some(MapUpdate::Snapshot(ItemViewSnapshotDomain {
            domain_id: STATE_DOMAIN_WORLD_MAP_VIEW,
            revision: advance(&mut view.revision)?,
            snapshot_type: SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1,
            payload,
        })),
        Planned::Delta(payload) => {
            let from = view.revision;
            Some(MapUpdate::Delta(ItemViewDelta {
                domain_id: STATE_DOMAIN_WORLD_MAP_VIEW,
                from,
                to: advance(&mut view.revision)?,
                delta_type: DELTA_TYPE_WORLD_MAP_VIEW_DELTA_V1,
                payload,
            }))
        }
    };
    view.sent = Some(SentView {
        header: plan.header,
        tiles,
    });
    view.buffers = plan.into_buffers();
    let ns = |d: Duration| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX);
    Ok((
        update,
        Stages {
            plan: ns(planned_at),
            handles: ns(map_ns.saturating_sub(tiles_ns).saturating_sub(encode_ns)),
            tiles: ns(tiles_ns),
            encode: ns(encode_ns),
            total: ns(started.elapsed()),
        },
    ))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Admission,
    FloorChange,
    Teleport,
    Resync,
    Delta,
    /// A walk step that sent nothing (the origin alone is a delta, so this is rare).
    Nothing,
    /// A walk step that sent a snapshot (a step past `MAPW-RL-03`'s changed-tile bound).
    WalkSnapshot,
}

const KINDS: [Kind; 7] = [
    Kind::Admission,
    Kind::FloorChange,
    Kind::Teleport,
    Kind::Resync,
    Kind::Delta,
    Kind::Nothing,
    Kind::WalkSnapshot,
];

fn index(kind: Kind) -> usize {
    KINDS.iter().position(|k| *k == kind).expect("kind")
}

#[derive(Default)]
struct Samples {
    totals: Vec<u64>,
    stages: Vec<Stages>,
    bytes: u64,
}

struct Results {
    by_kind: Vec<Samples>,
    walk_updates: u64,
    walk_snapshots: u64,
    walks: u64,
    stuck: u64,
    anchored_starts: u64,
    starts: u64,
    floor_change_missing: u64,
    cpu_ns: u64,
    wall: Duration,
    floors_seen: [u64; 256],
}

impl Results {
    fn new() -> Self {
        Self {
            by_kind: KINDS.iter().map(|_| Samples::default()).collect(),
            walk_updates: 0,
            walk_snapshots: 0,
            walks: 0,
            stuck: 0,
            anchored_starts: 0,
            starts: 0,
            floor_change_missing: 0,
            cpu_ns: 0,
            wall: Duration::ZERO,
            floors_seen: [0; 256],
        }
    }

    fn merge(&mut self, other: Self) {
        for (mine, theirs) in self.by_kind.iter_mut().zip(other.by_kind) {
            mine.totals.extend(theirs.totals);
            mine.stages.extend(theirs.stages);
            mine.bytes += theirs.bytes;
        }
        self.walk_updates += other.walk_updates;
        self.walk_snapshots += other.walk_snapshots;
        self.walks += other.walks;
        self.stuck += other.stuck;
        self.anchored_starts += other.anchored_starts;
        self.starts += other.starts;
        self.floor_change_missing += other.floor_change_missing;
        self.cpu_ns += other.cpu_ns;
        for (mine, theirs) in self.floors_seen.iter_mut().zip(other.floors_seen) {
            *mine += theirs;
        }
    }
}

/// The CPU time of this thread in ns (`/proc/thread-self/schedstat`, first field).
fn thread_cpu_ns() -> u64 {
    std::fs::read_to_string("/proc/thread-self/schedstat")
        .ok()
        .and_then(|text| text.split_whitespace().next()?.parse().ok())
        .unwrap_or(0)
}

fn payload_len(update: &Option<MapUpdate>) -> u64 {
    match update {
        None => 0,
        Some(MapUpdate::Snapshot(snapshot)) => snapshot.payload.len() as u64,
        Some(MapUpdate::Delta(delta)) => delta.payload.len() as u64,
    }
}

struct World<'a> {
    base: &'a WorldBase,
    overlay: &'a ChannelOverlay,
    facts: &'a RealFacts,
    sampler: &'a Sampler,
    digest: [u8; 32],
}

fn source<'a>(world: &World<'a>) -> MapViewSource<'a, RealFacts> {
    MapViewSource {
        overlay: world.overlay,
        facts: world.facts,
        content_generation: [7; 32],
        reset_epoch: 1,
    }
}

fn fresh_items() -> SessionItemView {
    SessionItemView::resume(ItemViewContinuity::default()).with_map_view()
}

fn record(results: &mut Results, kind: Kind, update: &Option<MapUpdate>, stages: Stages) {
    let samples = &mut results.by_kind[index(kind)];
    samples.totals.push(stages.total);
    samples.stages.push(stages);
    samples.bytes += payload_len(update);
}

/// One session: an admission snapshot, walks of `WALK_STEPS` steps (a teleport tile ends a walk
/// with a snapshot and the next walk starts at its destination), then a floor change, a teleport
/// and a resync.
fn session(world: &World<'_>, seed: u64, results: &mut Results) {
    let source = source(world);
    let mut rng = Seeded(seed);
    let (mut pos, anchored) = world.sampler.start(&mut rng);
    results.starts += 1;
    results.anchored_starts += u64::from(anchored);
    let (mut view, mut items) = (SessionMapView::default(), fresh_items());
    let admit = |view: &mut SessionMapView, items: &mut SessionItemView, pos: Pos| {
        staged_update(view, items, &source, actor(pos), true).expect("admission")
    };
    let (update, stages) = admit(&mut view, &mut items, pos);
    record(results, Kind::Admission, &update, stages);
    let mut steps = 0usize;
    let mut direction = rng.below(8) as usize;
    results.walks += 1;
    while steps < WALK_STEPS {
        // Keep the heading most of the time, else turn; try every heading before giving up.
        let first = if rng.below(4) == 0 {
            rng.below(8) as usize
        } else {
            direction
        };
        let next = (0..8).map(|offset| (first + offset) % 8).find_map(|d| {
            step(pos, DIRECTIONS[d])
                .filter(|candidate| enterable(world.base, *candidate))
                .map(|candidate| (d, candidate))
        });
        let Some((d, candidate)) = next else {
            // Isolated tile: relocate with a snapshot, counted as a teleport.
            results.stuck += 1;
            pos = world.sampler.start(&mut rng).0;
            let (update, stages) =
                staged_update(&mut view, &mut items, &source, actor(pos), false).expect("relocate");
            record(results, Kind::Teleport, &update, stages);
            continue;
        };
        direction = d;
        pos = candidate;
        steps += 1;
        let (update, stages) =
            staged_update(&mut view, &mut items, &source, actor(pos), false).expect("step");
        let kind = match &update {
            None => Kind::Nothing,
            Some(MapUpdate::Delta(_)) => Kind::Delta,
            Some(MapUpdate::Snapshot(_)) => Kind::WalkSnapshot,
        };
        results.walk_updates += 1;
        results.walk_snapshots += u64::from(kind == Kind::WalkSnapshot);
        results.floors_seen[(i32::from(pos.2) + 128) as usize] += 1;
        record(results, kind, &update, stages);
        if world.sampler.teleport_from.contains(&pos) {
            // A teleport tile ends the walk with a snapshot at its destination.
            let to = world
                .sampler
                .teleports
                .iter()
                .find(|(from, _)| *from == pos)
                .map(|(_, to)| *to)
                .expect("teleport");
            pos = to;
            let (update, stages) =
                staged_update(&mut view, &mut items, &source, actor(pos), false).expect("teleport");
            record(results, Kind::Teleport, &update, stages);
            results.walks += 1;
        }
    }
    match floor_change(world.base, pos, &mut rng) {
        Some(to) => {
            pos = to;
            let (update, stages) = staged_update(&mut view, &mut items, &source, actor(pos), false)
                .expect("floor change");
            record(results, Kind::FloorChange, &update, stages);
        }
        None => results.floor_change_missing += 1,
    }
    if !world.sampler.teleports.is_empty() {
        let (_, to) =
            world.sampler.teleports[rng.below(world.sampler.teleports.len() as u64) as usize];
        pos = to;
        let (update, stages) =
            staged_update(&mut view, &mut items, &source, actor(pos), false).expect("teleport");
        record(results, Kind::Teleport, &update, stages);
    }
    let (update, stages) =
        staged_update(&mut view, &mut items, &source, actor(pos), true).expect("resync");
    record(results, Kind::Resync, &update, stages);
}

/// `threads` threads, each with its own sessions (views) over the one shared base and overlay.
fn run(world: &World<'_>, threads: usize, seed: u64) -> Results {
    let per_thread = VIEWS.div_ceil(threads);
    let started = Instant::now();
    let mut total = Results::new();
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                scope.spawn(move || {
                    let mut results = Results::new();
                    let cpu = thread_cpu_ns();
                    for s in 0..per_thread {
                        let id = (t * per_thread + s) as u64;
                        session(
                            world,
                            seed ^ id.wrapping_mul(0x9e37_79b9_7f4a_7c15),
                            &mut results,
                        );
                    }
                    results.cpu_ns = thread_cpu_ns().saturating_sub(cpu);
                    results
                })
            })
            .collect();
        for handle in handles {
            total.merge(handle.join().expect("thread"));
        }
    });
    total.wall = started.elapsed();
    total
}

fn percentile(sorted: &[u64], q: f64) -> u64 {
    sorted
        .get(((sorted.len() as f64 - 1.0) * q).round() as usize)
        .copied()
        .unwrap_or(0)
}

fn us(ns: u64) -> String {
    format!("{:.1}", ns as f64 / 1000.0)
}

fn line(name: &str, samples: &[u64]) {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    println!(
        "| {name} | {} | {} | {} | {} |",
        sorted.len(),
        us(percentile(&sorted, 0.5)),
        us(percentile(&sorted, 0.99)),
        us(sorted.last().copied().unwrap_or(0)),
    );
}

fn report(threads: usize, results: &Results) {
    println!("\n### {threads} thread(s), {VIEWS} views\n");
    println!("| update | n | p50 us | p99 us | max us |\n|---|---|---|---|---|");
    let snapshots: Vec<u64> = [
        Kind::Admission,
        Kind::FloorChange,
        Kind::Teleport,
        Kind::Resync,
        Kind::WalkSnapshot,
    ]
    .iter()
    .flat_map(|k| results.by_kind[index(*k)].totals.iter().copied())
    .collect();
    line("snapshot (all kinds)", &snapshots);
    for kind in [
        Kind::Admission,
        Kind::FloorChange,
        Kind::Teleport,
        Kind::Resync,
        Kind::WalkSnapshot,
    ] {
        line(
            &format!("snapshot: {kind:?}"),
            &results.by_kind[index(kind)].totals,
        );
    }
    let walk: Vec<u64> = [Kind::Delta, Kind::Nothing]
        .iter()
        .flat_map(|k| results.by_kind[index(*k)].totals.iter().copied())
        .collect();
    line("delta (walk steps, incl. empty)", &walk);
    line(
        "delta: Delta only",
        &results.by_kind[index(Kind::Delta)].totals,
    );
    let all_walk: Vec<u64> = walk
        .iter()
        .copied()
        .chain(
            results.by_kind[index(Kind::WalkSnapshot)]
                .totals
                .iter()
                .copied(),
        )
        .collect();
    line("every walk step", &all_walk);

    println!("\nStage split (p50 / p99 us), snapshots and deltas:\n");
    println!(
        "| stage | snapshot p50 | snapshot p99 | delta p50 | delta p99 |\n|---|---|---|---|---|"
    );
    let pick = |kinds: &[Kind], f: fn(&Stages) -> u64| -> Vec<u64> {
        let mut v: Vec<u64> = kinds
            .iter()
            .flat_map(|k| results.by_kind[index(*k)].stages.iter().map(f))
            .collect();
        v.sort_unstable();
        v
    };
    let snap_kinds = [
        Kind::Admission,
        Kind::FloorChange,
        Kind::Teleport,
        Kind::Resync,
        Kind::WalkSnapshot,
    ];
    let delta_kinds = [Kind::Delta, Kind::Nothing];
    for (name, f) in [
        ("plan", (|s: &Stages| s.plan) as fn(&Stages) -> u64),
        ("handles", |s| s.handles),
        ("tiles", |s| s.tiles),
        ("encode", |s| s.encode),
        ("total", |s| s.total),
    ] {
        let (s, d) = (pick(&snap_kinds, f), pick(&delta_kinds, f));
        println!(
            "| {name} | {} | {} | {} | {} |",
            us(percentile(&s, 0.5)),
            us(percentile(&s, 0.99)),
            us(percentile(&d, 0.5)),
            us(percentile(&d, 0.99)),
        );
    }

    println!("\nBytes per update (mean):\n");
    for kind in KINDS {
        let samples = &results.by_kind[index(kind)];
        if !samples.totals.is_empty() {
            println!(
                "- {kind:?}: {} B over {} updates",
                samples.bytes / samples.totals.len() as u64,
                samples.totals.len()
            );
        }
    }
    let walk_total = results.walk_updates;
    println!(
        "\nWalks: {} walks, {} walk updates, {} walk snapshots ({:.3}% of walk updates); with the scenario snapshots, snapshots are {:.2}% of all updates; stuck relocations {}; floor change unavailable {}; starts from anchors {}/{}.",
        results.walks,
        walk_total,
        results.walk_snapshots,
        100.0 * results.walk_snapshots as f64 / walk_total.max(1) as f64,
        100.0 * snapshots.len() as f64 / (snapshots.len() as f64 + walk.len() as f64),
        results.stuck,
        results.floor_change_missing,
        results.anchored_starts,
        results.starts,
    );
    let surface: u64 = (121..=128).map(|i| results.floors_seen[i]).sum();
    let all_floors: u64 = results.floors_seen.iter().sum();
    println!(
        "Walk steps on the surface (floors -7..=0): {surface} of {all_floors} ({:.1}%); underground {}.",
        100.0 * surface as f64 / all_floors.max(1) as f64,
        all_floors - surface
    );
    let updates = (all_walk.len() + snapshots.len()) as u64 - results.walk_snapshots;
    let cpu_per_update = results.cpu_ns as f64 / updates.max(1) as f64;
    println!(
        "CPU per update (thread CPU time, scheduling excluded): {:.1} us over {updates} updates ({:.2} s). \
         Per player per second at {STEPS_PER_SECOND} steps/s: {:.0} us; at 500 players: {:.3} cores (budget 0.200). \
         Wall {:.2} s.",
        cpu_per_update / 1000.0,
        results.cpu_ns as f64 / 1e9,
        cpu_per_update * STEPS_PER_SECOND / 1000.0,
        500.0 * cpu_per_update * STEPS_PER_SECOND / 1e9,
        results.wall.as_secs_f64(),
    );
}

/// The staged update sends the same bytes as `SessionMapView::update`.
fn staged_equals_update(world: &World<'_>) {
    let source = source(world);
    let mut rng = Seeded(SEEDS[0] ^ 0xabcd);
    let (mut pos, _) = world.sampler.start(&mut rng);
    let (mut a, mut a_items) = (SessionMapView::default(), fresh_items());
    let (mut b, mut b_items) = (SessionMapView::default(), fresh_items());
    for n in 0..400 {
        if n > 0 {
            let moved = (0..8).find_map(|d| {
                step(pos, DIRECTIONS[(n + d) % 8]).filter(|c| enterable(world.base, *c))
            });
            if let Some(moved) = moved {
                pos = moved;
            }
        }
        let real = a.update(&mut a_items, &source, actor(pos)).expect("update");
        let (staged, _) =
            staged_update(&mut b, &mut b_items, &source, actor(pos), false).expect("staged");
        let bytes = |u: &Option<MapUpdate>| match u {
            None => (0u8, Vec::new()),
            Some(MapUpdate::Snapshot(s)) => (1, s.payload.clone()),
            Some(MapUpdate::Delta(d)) => (2, d.payload.clone()),
        };
        assert_eq!(bytes(&real), bytes(&staged), "update {n} at {pos:?}");
    }
}

fn machine() -> String {
    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|text| {
            text.lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split_once(':').map(|(_, v)| v.trim().to_owned()))
        })
        .unwrap_or_else(|| "unknown".into());
    let cores = std::thread::available_parallelism().map_or(0, usize::from);
    format!("{cpu}, {cores} vCPU")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
#[ignore = "measurement (MAP-VIEWPORT-MEASURE-1); run in release"]
fn map_viewport_real_measure() {
    println!("machine: {}", machine());
    let started = Instant::now();
    let map = real_map_bundle::compile_real_map().expect("compile the real map");
    println!(
        "bundle: {} bytes, digest {}, palette {} keys, compiled in {:.1} s",
        map.bytes.len(),
        hex(&map.digest),
        map.keys.len(),
        started.elapsed().as_secs_f64()
    );
    let pins = BundlePins {
        digest: map.digest,
        project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
        world_schema_version: "world-schema-1".into(),
        content_revision: "rev-1".into(),
        production: true,
    };
    let loaded = Instant::now();
    let base = Arc::new(load(&map.bytes, &pins).expect("load"));
    println!(
        "base: {} tiles, {} entries, loaded in {:.2} s",
        base.tile_count(),
        base.entry_count(),
        loaded.elapsed().as_secs_f64()
    );
    let facts = RealFacts::new(&map).expect("facts");
    let sampler = Sampler::new(&base).expect("sampler");
    println!("enterable tiles: {}", sampler.enterable.len());
    let overlay = ChannelOverlay::new(
        Arc::clone(&base),
        WorldId::decode(&uuid(1)).expect("world"),
        ChannelId::decode(&uuid(2)).expect("channel"),
    );
    let world = World {
        base: &base,
        overlay: &overlay,
        facts: &facts,
        sampler: &sampler,
        digest: map.digest,
    };
    let _ = world.digest;
    staged_equals_update(&world);
    println!("staged update == SessionMapView::update on 400 steps: ok");
    println!("seeds: {SEEDS:x?}");
    for (threads, seed) in THREADS.iter().zip(SEEDS) {
        let results = run(&world, *threads, seed);
        report(*threads, &results);
    }
    println!(
        "\nChannel writer held per update: not composed (observe_world_map is the trait default on main; no authority composes a bundle World's map yet)."
    );
}
