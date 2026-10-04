//! MAP-OVERLAY-1a (ADR-0021 §4.4, §4.8): the per-channel overlay over a shared base, its expiry
//! index, the `MAP01-CHANNEL-OVERLAY-BYTES` budget and the Ground rebuild, against a fixture
//! bundle each test assembles.

use std::error::Error as StdError;
use std::sync::Arc;

use oteryn_game_server::durability::item_mint::{
    GroundItemInstance, GroundPlacement, TypedDefinitionRef,
};
use oteryn_game_server::foundation::{ChannelId, WorldId};
use oteryn_game_server::map::overlay::{
    AddedItem, Admission, ChannelOverlay, EntryId, OVERLAY_BUDGET_BYTES, OverlayError, TilePos,
    VolatileItem, map_revision,
};
use oteryn_game_server::map::{self, BundlePins, LoadError, WorldBase};
use oteryn_world_bundle_compiler::Error;
use oteryn_world_bundle_compiler::bundle::{
    self, BuildClass, Extent, Family, Identity, Manifest, Terrain, TerrainKind,
};
use oteryn_world_bundle_compiler::compile::{Input, KeyResolver, Resolution, compile};
use oteryn_world_bundle_compiler::project::Families;
use oteryn_world_bundle_compiler::sector::{self, Attrs, Item, Tile};
use oteryn_world_bundle_compiler::spawn;
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn StdError>>;

const KEYS: [&str; 2] = ["terrain:grass", "item:coin"];

struct Resolver;

impl KeyResolver for Resolver {
    fn resolve(&self, key: &str) -> Resolution {
        match key {
            "terrain:grass" => Resolution::Resolved(Family::Terrain, 101),
            "item:coin" => Resolution::Resolved(Family::Item, 9),
            _ => Resolution::Unknown,
        }
    }

    fn terrain(&self, key: &str) -> Result<Option<Terrain>, Error> {
        Ok((key == "terrain:grass").then_some(Terrain {
            kind: TerrainKind::Ground,
            walkable: Some(true),
            ground_speed: Some(150),
        }))
    }

    fn floor_change(&self, _: &str) -> bool {
        false
    }
}

fn item(palette: u32) -> Item {
    Item {
        palette,
        depth: 0,
        attrs: Attrs::default(),
    }
}

fn tile(x: u16, y: u16, items: Vec<Item>) -> Tile {
    Tile {
        x,
        y,
        flags: 0,
        house: 0,
        zones: Vec::new(),
        items,
    }
}

/// The manifest of a one-tile compiled bundle; the fixtures reuse it with their own sectors.
fn manifest() -> Result<Manifest, Box<dyn StdError>> {
    let frame = zstd::bulk::compress(&sector::encode(&[tile(1, 1, vec![item(0), item(1)])])?, 3)?;
    let mut region = b"OTRB".to_vec();
    region.extend_from_slice(&[1, 7]);
    for value in [0u16, 0, 1] {
        region.extend_from_slice(&value.to_le_bytes());
    }
    region.push(0);
    region.extend_from_slice(&21u32.to_le_bytes());
    region.extend_from_slice(&(frame.len() as u32).to_le_bytes());
    region.extend_from_slice(&frame);
    let palette: Vec<String> = KEYS.map(String::from).to_vec();
    let families = Families::default();
    let input = Input {
        regions: &[region],
        palette: &palette,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            ..Identity::default()
        },
        world: Extent {
            min_x: 0,
            min_y: 0,
            max_x: 512,
            max_y: 256,
            floors: vec![-7],
        },
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &families,
    };
    Ok(bundle::read(&compile(&input, &Resolver)?.bytes)?.manifest)
}

fn compress(raw: &[u8]) -> Result<Vec<u8>, Box<dyn StdError>> {
    let mut compressor = zstd::bulk::Compressor::new(3)?;
    compressor.include_checksum(true)?;
    compressor.include_contentsize(true)?;
    Ok(compressor.compress(raw)?)
}

/// A bundle of one sector at native floor -7, assembled byte by byte (as the MAP-LOAD-1 tests
/// do) so a tile can hold more top-level entries than the compiler writes, and its digest.
fn assemble(tiles: &[Tile]) -> Result<(Vec<u8>, [u8; 32]), Box<dyn StdError>> {
    let json = serde_json::to_vec(&manifest()?)?;
    let raw = sector::encode(tiles)?;
    let frame = compress(&raw)?;
    let spawn_raw = spawn::encode(&spawn::Table::default());
    let spawn_frame = compress(&spawn_raw)?;
    let u32_of = |value: usize| (value as u32).to_le_bytes();
    let mut out = b"OTWB".to_vec();
    out.extend_from_slice(&3u16.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(&u32_of(json.len()));
    out.extend_from_slice(&u32_of(1));
    out.extend_from_slice(&json);
    let offset = out.len() + 50 + 44;
    out.extend_from_slice(&[-7i8 as u8, 0]);
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&u32_of(offset));
    out.extend_from_slice(&u32_of(frame.len()));
    out.extend_from_slice(&u32_of(raw.len()));
    out.extend_from_slice(&Sha256::digest(&frame));
    out.extend_from_slice(&u32_of(offset + frame.len()));
    out.extend_from_slice(&u32_of(spawn_frame.len()));
    out.extend_from_slice(&u32_of(spawn_raw.len()));
    out.extend_from_slice(&Sha256::digest(&spawn_frame));
    out.extend_from_slice(&frame);
    out.extend_from_slice(&spawn_frame);
    let body = out.len();
    let digest: [u8; 32] = Sha256::new()
        .chain_update(b"OTERYN_WORLD_BUNDLE/v3\0")
        .chain_update(&out)
        .finalize()
        .into();
    out.extend_from_slice(&digest);
    debug_assert_eq!(out.len(), body + 32);
    Ok((out, digest))
}

fn load(tiles: &[Tile]) -> Result<Result<WorldBase, LoadError>, Box<dyn StdError>> {
    let (bytes, digest) = assemble(tiles)?;
    Ok(map::load(
        &bytes,
        &BundlePins {
            digest,
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            production: false,
        },
    ))
}

/// The fixture base: (1, 1) holds 64 top-level entries, (2, 1) a grass ground under one coin,
/// and every other tile of the sector a grass ground.
fn base() -> Result<Arc<WorldBase>, Box<dyn StdError>> {
    let mut tiles = Vec::new();
    for y in 0..32 {
        for x in 0..32 {
            let items = match (x, y) {
                (1, 1) => vec![item(0); 64],
                (2, 1) => vec![item(0), item(1)],
                _ => vec![item(0)],
            };
            tiles.push(tile(x, y, items));
        }
    }
    Ok(Arc::new(load(&tiles)??))
}

/// A UUIDv7 of `seed`.
fn uuid(seed: u8) -> [u8; 16] {
    let mut bytes = [seed; 16];
    bytes[6] = 0x70 | (seed & 0x0f);
    bytes[8] = 0x80 | (seed & 0x3f);
    bytes
}

fn world() -> Result<WorldId, Box<dyn StdError>> {
    WorldId::decode(&uuid(1)).map_err(|e| format!("{e:?}").into())
}

fn channel(seed: u8) -> Result<ChannelId, Box<dyn StdError>> {
    ChannelId::decode(&uuid(seed)).map_err(|e| format!("{e:?}").into())
}

const POS: TilePos = TilePos {
    x: 1,
    y: 1,
    floor: -7,
};

fn coin(count: u16) -> VolatileItem {
    VolatileItem {
        id: 9,
        count,
        attributes: Vec::new(),
    }
}

fn spatial(pos: TilePos) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(10);
    bytes.extend_from_slice(&i32::from(pos.x).to_be_bytes());
    bytes.extend_from_slice(&i32::from(pos.y).to_be_bytes());
    bytes.extend_from_slice(&i16::from(pos.floor).to_be_bytes());
    bytes
}

fn ground_item(
    base: &WorldBase,
    channel_id: ChannelId,
    seed: u8,
    pos: TilePos,
) -> Result<GroundItemInstance, Box<dyn StdError>> {
    Ok(GroundItemInstance {
        item_instance_id: uuid(seed),
        world_id: world()?,
        channel_id,
        definition: TypedDefinitionRef {
            family: "Item".into(),
            production_key: "item:coin".into(),
            revision_ref: "rev-1".into(),
        },
        quantity: 1,
        runtime_scope_ownership_generation: 1,
        ground: GroundPlacement {
            spatial_position: spatial(pos),
            corpse_ref: Vec::new(),
            map_revision: map_revision(base),
            content_revision: "rev-1".into(),
            native_room_placement_context: Vec::new(),
        },
        minted_transaction_id: uuid(seed.wrapping_add(100)),
    })
}

#[test]
fn map_overlay_two_channels_share_one_base_and_keep_separate_overlays() -> TestResult {
    let base = base()?;
    let mut one = ChannelOverlay::new(Arc::clone(&base), world()?, channel(2)?);
    let mut two = ChannelOverlay::new(Arc::clone(&base), world()?, channel(3)?);
    assert!(Arc::ptr_eq(one.base(), two.base()));
    assert_eq!(Arc::strong_count(&base), 3);
    one.hide(POS, 0, Admission::Refusable)?;
    let added = two.add_volatile(POS, coin(5), None)?;
    let tile = one.tile(POS).ok_or("one")?;
    assert!(tile.is_hidden(0) && tile.added().is_empty());
    let tile = two.tile(POS).ok_or("two")?;
    assert!(!tile.is_hidden(0));
    assert_eq!(tile.added().len(), 1);
    assert_eq!(tile.added()[0].id(), added);
    // The shared base is untouched.
    assert_eq!(base.tile(1, 1, -7).ok_or("base")?.ids().len(), 64);
    Ok(())
}

#[test]
fn map_overlay_hides_and_adds_up_to_the_64_item_reach_and_load_refuses_a_65th() -> TestResult {
    let base = base()?;
    let mut overlay = ChannelOverlay::new(Arc::clone(&base), world()?, channel(2)?);
    for ordinal in 0..64 {
        overlay.hide(POS, ordinal, Admission::Refusable)?;
    }
    assert_eq!(overlay.tile(POS).ok_or("tile")?.hidden(), u64::MAX);
    assert_eq!(
        overlay.hide(POS, 64, Admission::Refusable),
        Err(OverlayError::Ordinal)
    );
    assert_eq!(
        overlay.hide(POS, 63, Admission::Refusable),
        Err(OverlayError::AlreadyHidden)
    );
    // (2, 1) has two top-level entries: ordinal 2 does not exist there.
    let small = TilePos { x: 2, ..POS };
    assert_eq!(
        overlay.hide(small, 2, Admission::Refusable),
        Err(OverlayError::Ordinal)
    );
    let missing = TilePos { x: 40, ..POS };
    assert_eq!(
        overlay.hide(missing, 0, Admission::Refusable),
        Err(OverlayError::NoBaseTile)
    );
    assert_eq!(
        overlay.add_volatile(missing, coin(1), None),
        Err(OverlayError::NoBaseTile)
    );
    for count in 1..=64 {
        overlay.add_volatile(POS, coin(count), None)?;
    }
    assert_eq!(overlay.tile(POS).ok_or("tile")?.added().len(), 64);
    // Unhiding every ordinal leaves the added items alone.
    let used = overlay.used_bytes();
    for ordinal in 0..64 {
        overlay.unhide(POS, ordinal)?;
    }
    assert_eq!(overlay.unhide(POS, 0), Err(OverlayError::NotHidden));
    assert!(overlay.used_bytes() < used);
    assert_eq!(overlay.tile(POS).ok_or("tile")?.added().len(), 64);
    // A 65th base top-level entry is refused at load, whole.
    let mut tiles = vec![tile(1, 1, vec![item(0); 65])];
    assert!(matches!(
        load(&tiles)?,
        Err(LoadError::Bundle(Error::Limit(_)))
    ));
    tiles[0].items.pop();
    assert!(load(&tiles)?.is_ok());
    Ok(())
}

#[test]
fn map_overlay_never_merges_a_base_stack() -> TestResult {
    let base = base()?;
    let mut overlay = ChannelOverlay::new(Arc::clone(&base), world()?, channel(2)?);
    // (2, 1) holds a base coin (id 9) at ordinal 1; two more coins land on it.
    let pos = TilePos { x: 2, ..POS };
    let first = overlay.add_volatile(pos, coin(3), None)?;
    let second = overlay.add_volatile(pos, coin(4), None)?;
    assert_ne!(first, second);
    let tile = overlay.tile(pos).ok_or("tile")?;
    let counts: Vec<u16> = tile
        .added()
        .iter()
        .map(|entry| match entry.item() {
            AddedItem::Volatile { item, .. } => item.count,
            AddedItem::Ground(_) => 0,
        })
        .collect();
    assert_eq!(counts, [3, 4]);
    assert_eq!(tile.hidden(), 0);
    assert_eq!(base.tile(2, 1, -7).ok_or("base")?.ids(), [101, 9]);
    Ok(())
}

#[test]
fn map_overlay_expiry_removes_a_volatile_item_within_1_s_of_its_decay() -> TestResult {
    let base = base()?;
    let mut overlay = ChannelOverlay::new(Arc::clone(&base), world()?, channel(2)?);
    let empty = overlay.used_bytes();
    let decay = 10_250;
    let decaying = overlay.add_volatile(POS, coin(1), Some(decay))?;
    let whole = overlay.add_volatile(POS, coin(2), Some(11_000))?;
    let kept = overlay.add_volatile(POS, coin(3), None)?;
    // Never before the decay.
    assert!(overlay.expire(decay - 1).is_empty());
    assert!(overlay.expire(10_999).is_empty());
    // At the next whole second, within 1 s of the decay.
    let at = 11_000;
    assert!(at - decay < 1000);
    assert_eq!(overlay.expire(at), [(POS, decaying), (POS, whole)]);
    let ids: Vec<EntryId> = overlay
        .tile(POS)
        .ok_or("tile")?
        .added()
        .iter()
        .map(|entry| entry.id())
        .collect();
    assert_eq!(ids, [kept]);
    assert!(overlay.expire(u64::MAX).is_empty());
    // Removing the last entry refunds everything, the expiry slots included.
    overlay.remove(POS, kept)?;
    assert!(overlay.tile(POS).is_none());
    assert_eq!(overlay.used_bytes(), empty);
    // A volatile item removed before its decay leaves nothing in the index.
    let early = overlay.add_volatile(POS, coin(4), Some(20_000))?;
    overlay.remove(POS, early)?;
    assert!(overlay.expire(30_000).is_empty());
    assert_eq!(overlay.used_bytes(), empty);
    Ok(())
}

#[test]
fn map_overlay_refuses_a_volatile_entry_over_the_budget_atomically() -> TestResult {
    let base = base()?;
    let probe = ChannelOverlay::new(Arc::clone(&base), world()?, channel(2)?);
    let empty = probe.used_bytes();
    let mut sizing = ChannelOverlay::new(Arc::clone(&base), world()?, channel(2)?);
    sizing.add_volatile(POS, coin(1), Some(5_000))?;
    let first = sizing.used_bytes() - empty;
    sizing.add_volatile(POS, coin(1), Some(5_000))?;
    let next = sizing.used_bytes() - empty - first;
    // Room for exactly two entries on one tile.
    let budget = empty + first + next;
    let mut overlay = ChannelOverlay::with_budget(Arc::clone(&base), world()?, channel(2)?, budget);
    overlay.add_volatile(POS, coin(1), Some(5_000))?;
    overlay.add_volatile(POS, coin(2), Some(5_000))?;
    assert_eq!(overlay.used_bytes(), budget);
    let before = (
        overlay.used_bytes(),
        overlay.tile(POS).ok_or("tile")?.added().to_vec(),
    );
    let refused = overlay.add_volatile(POS, coin(3), Some(5_000));
    assert!(matches!(
        refused,
        Err(OverlayError::OverBudget { available: 0, .. })
    ));
    // A freeze-time hide is refusable too.
    assert!(matches!(
        overlay.hide(POS, 0, Admission::Refusable),
        Err(OverlayError::OverBudget { .. })
    ));
    let after = (
        overlay.used_bytes(),
        overlay.tile(POS).ok_or("tile")?.added().to_vec(),
    );
    assert_eq!(before, after);
    assert_eq!(overlay.tile(POS).ok_or("tile")?.hidden(), 0);
    assert_eq!(overlay.alarm_count(), 0);
    // The refused entry never reached the expiry index: both remaining entries expire, nothing
    // else does.
    assert_eq!(overlay.expire(5_000).len(), 2);
    Ok(())
}

#[test]
fn map_overlay_admits_a_durable_item_over_the_budget_and_raises_the_alarm() -> TestResult {
    let base = base()?;
    let channel_id = channel(2)?;
    let tight = ChannelOverlay::new(Arc::clone(&base), world()?, channel_id).used_bytes();
    let mut overlay = ChannelOverlay::with_budget(Arc::clone(&base), world()?, channel_id, tight);
    assert!(matches!(
        overlay.add_volatile(POS, coin(1), None),
        Err(OverlayError::OverBudget { .. })
    ));
    let entry = overlay.add_ground(ground_item(&base, channel_id, 10, POS)?)?;
    assert!(overlay.over_budget());
    assert_eq!(overlay.alarm_count(), 1);
    // A rebuild re-hide is counted, never refused.
    overlay.hide(POS, 0, Admission::Durable)?;
    assert_eq!(overlay.alarm_count(), 2);
    assert_eq!(overlay.ground_entry(&uuid(10)), Some((POS, entry)));
    let tile = overlay.tile(POS).ok_or("tile")?;
    assert!(tile.is_hidden(0));
    assert!(
        matches!(tile.added()[0].item(), AddedItem::Ground(item) if item.item_instance_id == uuid(10))
    );
    assert_eq!(
        overlay.add_ground(ground_item(&base, channel_id, 10, POS)?),
        Err(OverlayError::DuplicateItem)
    );
    assert_eq!(OVERLAY_BUDGET_BYTES, 67_108_864);
    Ok(())
}

#[test]
fn map_overlay_rebuild_restores_every_durable_ground_item_and_fails_closed() -> TestResult {
    let base = base()?;
    let channel_id = channel(2)?;
    let positions = [
        POS,
        TilePos { x: 2, ..POS },
        TilePos {
            x: 31,
            y: 31,
            floor: -7,
        },
    ];
    let items = || -> Result<Vec<GroundItemInstance>, Box<dyn StdError>> {
        positions
            .iter()
            .zip(10..)
            .map(|(pos, seed)| ground_item(&base, channel_id, seed, *pos))
            .collect()
    };
    // Every item is rebuilt, even far over a tiny budget.
    let rebuilt = ChannelOverlay::rebuild(Arc::clone(&base), world()?, channel_id, 0, items()?)?;
    for (pos, seed) in positions.iter().zip(10..) {
        assert_eq!(
            rebuilt.ground_entry(&uuid(seed)).map(|(at, _)| at),
            Some(*pos)
        );
    }
    assert_eq!(rebuilt.alarm_count(), 3);
    let full = ChannelOverlay::rebuild(
        Arc::clone(&base),
        world()?,
        channel_id,
        OVERLAY_BUDGET_BYTES,
        items()?,
    )?;
    assert_eq!(full.alarm_count(), 0);
    assert_eq!(full.tiles().count(), 3);
    // One item placed on another bundle fails the whole rebuild closed.
    let mut stale = items()?;
    stale[1].ground.map_revision = format!("sha256:{}", "00".repeat(32));
    let refused = ChannelOverlay::rebuild(Arc::clone(&base), world()?, channel_id, 0, stale)
        .err()
        .ok_or("stale rebuild")?;
    assert_eq!(refused.item_instance_id, uuid(11));
    assert_eq!(refused.reason, OverlayError::MapRevision);
    // So does an item of another channel, another World, an undecodable or unmapped position, or
    // a duplicate.
    let cases: [(fn(&mut GroundItemInstance), OverlayError); 5] = [
        (
            |item| item.channel_id = channel(3).unwrap_or(item.channel_id),
            OverlayError::Channel,
        ),
        (
            |item| item.world_id = WorldId::decode(&uuid(4)).unwrap_or(item.world_id),
            OverlayError::World,
        ),
        (
            |item| item.ground.spatial_position.pop().map_or((), drop),
            OverlayError::Position,
        ),
        (
            |item| item.ground.spatial_position[8] = 0x7f,
            OverlayError::Position,
        ),
        (
            |item| item.ground.spatial_position[3] = 200,
            OverlayError::NoBaseTile,
        ),
    ];
    for (corrupt, reason) in cases {
        let mut bad = items()?;
        corrupt(&mut bad[2]);
        let refused = ChannelOverlay::rebuild(Arc::clone(&base), world()?, channel_id, 0, bad)
            .err()
            .ok_or("bad rebuild")?;
        assert_eq!(
            (refused.item_instance_id, refused.reason),
            (uuid(12), reason)
        );
    }
    let mut twice = items()?;
    twice.push(twice[0].clone());
    let refused = ChannelOverlay::rebuild(Arc::clone(&base), world()?, channel_id, 0, twice)
        .err()
        .ok_or("duplicate rebuild")?;
    assert_eq!(refused.reason, OverlayError::DuplicateItem);
    Ok(())
}

/// A `kB` field of `/proc/self/status`, in bytes.
fn status_bytes(field: &str) -> Result<usize, Box<dyn StdError>> {
    let status = std::fs::read_to_string("/proc/self/status")?;
    let line = status
        .lines()
        .find(|line| line.starts_with(field))
        .ok_or("status field")?;
    let kb: usize = line
        .trim_start_matches(field)
        .trim()
        .trim_end_matches("kB")
        .trim()
        .parse()?;
    Ok(kb * 1024)
}

/// The budget measurement (docs/agents/evidence/MAP-OVERLAY-1a-overlay-budget.md): one overlay
/// filled to the default budget with volatile entries over every tile of the fixture, then
/// hides and Ground items admitted over it; the resident growth is compared with the accounted
/// bytes. Run by hand in release mode, alone.
#[test]
#[ignore = "budget measurement (ADR-0021 §4.8), run by hand in release mode"]
fn map_overlay_budget_measure() -> TestResult {
    let base = base()?;
    let channel_id = channel(2)?;
    let before = status_bytes("VmRSS:")?;
    let mut overlay = ChannelOverlay::new(Arc::clone(&base), world()?, channel_id);
    let positions: Vec<TilePos> = (0..32u16)
        .flat_map(|y| (0..32u16).map(move |x| TilePos { x, y, floor: -7 }))
        .collect();
    let mut seed = 0u64;
    let mut decaying = 0;
    let start = std::time::Instant::now();
    'fill: loop {
        for pos in &positions {
            seed += 1;
            let item = VolatileItem {
                id: 9,
                count: 1,
                attributes: vec![0; (seed % 24) as usize],
            };
            let decay = (seed % 3 == 0).then_some(seed * 7);
            match overlay.add_volatile(*pos, item, decay) {
                Ok(_) => decaying += usize::from(decay.is_some()),
                Err(OverlayError::OverBudget { .. }) => break 'fill,
                Err(error) => return Err(error.into()),
            }
        }
    }
    let fill = start.elapsed();
    let entries = seed - 1;
    let full = overlay.used_bytes();
    let resident = status_bytes("VmRSS:")?.saturating_sub(before);
    for pos in &positions {
        overlay.hide(*pos, 0, Admission::Durable)?;
    }
    for (pos, seed) in positions.iter().take(250).zip(1u8..) {
        let mut item = ground_item(&base, channel_id, seed, *pos)?;
        item.item_instance_id[1] = (pos.y as u8) ^ 0x5a;
        overlay.add_ground(item)?;
    }
    let (over, alarms) = (overlay.over_budget(), overlay.alarm_count());
    let start = std::time::Instant::now();
    let expired = overlay.expire(u64::MAX).len();
    let expire = start.elapsed();
    println!(
        "entries {entries} ({decaying} decaying) on {} tiles; accounted {full} B of {} B; \
         resident growth {resident} B (ratio {:.3}); fill {fill:?}; over budget after 1,024 \
         durable hides and 250 Ground items: {} (alarms {}); expired {expired} in {expire:?}",
        positions.len(),
        overlay.budget(),
        resident as f64 / full as f64,
        over,
        alarms,
    );
    assert!(full <= OVERLAY_BUDGET_BYTES);
    assert!(
        resident <= full,
        "the accounting under-counts the resident heap"
    );
    assert!(over && alarms > 0);
    assert_eq!(expired, decaying);
    Ok(())
}
