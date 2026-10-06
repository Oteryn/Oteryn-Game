#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use super::*;
use crate::durability::item_mint::{GroundItemInstance, GroundPlacement, TypedDefinitionRef};
use crate::foundation::{ChannelId, WorldId};
use crate::gameplay_transport::item_view::ItemViewContinuity;
use crate::map::WorldBase;
use crate::map::overlay::{AddedItem, Admission, VolatileItem, map_revision};
use crate::map::view::{ComposedEntry, ComposedTile, EntryFacts};
use oteryn_protocol_oteryn::world_map::{
    MapDefinition, decode_world_map_delta, decode_world_map_snapshot,
};
use oteryn_world_bundle::bundle::TerrainKind;
use oteryn_world_bundle_compiler::bundle::{
    self, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector, Terrain,
};
use oteryn_world_bundle_compiler::sector::{Attrs, Item, Tile};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use std::sync::Arc;

// Palette compact ids.
const GRASS: u32 = 1;
const WALL: u32 = 2;
const ROOF: u32 = 3;
const COIN: u32 = 4;
const DOOR: u32 = 5;
const CHEST: u32 = 6;
const TABLE: u32 = 7;
const BAG: u32 = 8;
const PILLAR: u32 = 9;

const ACTOR: TilePos = TilePos {
    x: 20,
    y: 20,
    floor: -7,
};

const fn tp(x: u16, y: u16, floor: i8) -> TilePos {
    TilePos { x, y, floor }
}

const fn at(pos: TilePos) -> ActorPosition {
    ActorPosition {
        x: pos.x as i32,
        y: pos.y as i32,
        floor: pos.floor as i16,
    }
}

fn nz(id: u32) -> NonZeroU32 {
    NonZeroU32::new(id).expect("fixture")
}

/// The facts of palette entry `id`.
fn facts_of(id: u32) -> EntryFacts {
    let terrain = |kind| EntryFacts {
        definition: MapDefinition::Terrain(nz(id)),
        terrain_kind: Some(kind),
        appearance_id: 100 + id as u16,
        blocks_projectile: false,
        pickupable: false,
        bound: false,
        count: 1,
        sub_type: 0,
    };
    let item = |pickupable, bound, blocks_projectile| EntryFacts {
        definition: MapDefinition::Item(nz(id)),
        terrain_kind: None,
        appearance_id: 100 + id as u16,
        blocks_projectile,
        pickupable,
        bound,
        count: 1,
        sub_type: 0,
    };
    match id {
        GRASS => terrain(TerrainKind::Ground),
        WALL => terrain(TerrainKind::Wall),
        ROOF => terrain(TerrainKind::Roof),
        COIN | BAG => item(true, false, false),
        DOOR | CHEST => item(false, true, false),
        PILLAR => item(false, false, true),
        _ => item(false, false, false),
    }
}

#[derive(Default)]
struct Facts {
    houses: BTreeSet<TilePos>,
    revisions: BTreeMap<u64, u64>,
}

impl MapFacts for Facts {
    fn base_entry(&self, _: TilePos, _: u8, id: u32) -> Option<EntryFacts> {
        Some(facts_of(id))
    }

    fn added_entry(&self, item: &AddedItem) -> Option<EntryFacts> {
        match item {
            AddedItem::Volatile { item, .. } => Some(facts_of(item.id)),
            AddedItem::Ground(ground) => {
                (ground.definition.production_key == "item:coin").then(|| facts_of(COIN))
            }
        }
    }

    fn house_tile(&self, pos: TilePos) -> bool {
        self.houses.contains(&pos)
    }

    fn object_revision(&self, placement_key: u64) -> u64 {
        self.revisions.get(&placement_key).copied().unwrap_or(0)
    }
}

/// The entry of palette compact id `id` (palette index `id - 1`).
fn item(id: u32) -> Item {
    Item {
        palette: id - 1,
        depth: 0,
        attrs: Attrs::default(),
    }
}

fn content(id: u32) -> Item {
    Item {
        palette: id - 1,
        depth: 1,
        attrs: Attrs::default(),
    }
}

/// A server bundle over `0..=127` holding `tiles`, loaded as a base.
fn base(tiles: Vec<(TilePos, Vec<Item>)>) -> Arc<WorldBase> {
    let (bytes, pins) = bundle_bytes(tiles);
    Arc::new(crate::map::load(&bytes, &pins).expect("loaded base"))
}

/// The bytes and pins of a server bundle over `0..=127` holding `tiles`.
fn bundle_bytes(tiles: Vec<(TilePos, Vec<Item>)>) -> (Vec<u8>, crate::map::BundlePins) {
    let terrain = |key: &str, id, kind| PaletteEntry {
        key: key.into(),
        family: Family::Terrain,
        id,
        terrain: Some(match kind {
            TerrainKind::Ground => Terrain {
                kind,
                walkable: Some(true),
                ground_speed: Some(150),
            },
            _ => Terrain {
                kind,
                walkable: None,
                ground_speed: None,
            },
        }),
    };
    let plain = |key: &str, id| PaletteEntry {
        key: key.into(),
        family: Family::Item,
        id,
        terrain: None,
    };
    // Sectors ascend by (floor, sy, sx).
    let mut sectors: BTreeMap<(i8, u16, u16), Vec<Tile>> = BTreeMap::new();
    for (pos, items) in tiles {
        sectors
            .entry((pos.floor, pos.y / 32, pos.x / 32))
            .or_default()
            .push(Tile {
                x: pos.x,
                y: pos.y,
                flags: 0,
                house: 0,
                zones: Vec::new(),
                items,
            });
    }
    let mut floors: Vec<i8> = sectors.keys().map(|(floor, _, _)| *floor).collect();
    floors.sort_unstable();
    floors.dedup();
    let sectors: Vec<Sector> = sectors
        .into_iter()
        .map(|((floor, sy, sx), mut tiles)| {
            tiles.sort_unstable_by_key(|tile| (tile.y, tile.x));
            Sector {
                floor,
                sx,
                sy,
                tiles,
            }
        })
        .collect();
    let manifest = Manifest {
        format: bundle::FORMAT.into(),
        min_reader_version: bundle::VERSION,
        projection_class: "server".into(),
        compiler_version: "test".into(),
        build_class: BuildClass::Production,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            ..Identity::default()
        },
        world: Extent {
            min_x: 0,
            min_y: 0,
            max_x: 127,
            max_y: 127,
            floors,
        },
        palette: vec![
            terrain("terrain:grass", GRASS, TerrainKind::Ground),
            terrain("terrain:wall", WALL, TerrainKind::Wall),
            terrain("terrain:roof", ROOF, TerrainKind::Roof),
            plain("item:coin", COIN),
            plain("item:door", DOOR),
            plain("item:chest", CHEST),
            plain("item:table", TABLE),
            plain("item:bag", BAG),
            plain("item:pillar", PILLAR),
        ],
        draft_areas: Vec::new(),
        skipped_provisional_keys: Vec::new(),
        dropped_teleports: Vec::new(),
        spawns: Default::default(),
    };
    let bytes = bundle::write(&manifest, &sectors, &Default::default()).expect("written bundle");
    let digest = bundle::read(&bytes).expect("read bundle").digest;
    let pins = crate::map::BundlePins {
        digest,
        project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
        world_schema_version: "world-schema-1".into(),
        content_revision: "rev-1".into(),
        production: true,
    };
    (bytes, pins)
}

fn uuid(seed: u8) -> [u8; 16] {
    let mut bytes = [seed; 16];
    bytes[6] = 0x70 | (seed & 0x0f);
    bytes[8] = 0x80 | (seed & 0x3f);
    bytes
}

fn world() -> WorldId {
    WorldId::decode(&uuid(1)).expect("fixture")
}

fn channel() -> ChannelId {
    ChannelId::decode(&uuid(2)).expect("fixture")
}

struct Fixture {
    overlay: ChannelOverlay,
    facts: Facts,
    reset_epoch: u64,
    items: SessionItemView,
    view: SessionMapView,
    client: Option<Client>,
}

impl Fixture {
    fn new(tiles: Vec<(TilePos, Vec<Item>)>) -> Self {
        Self {
            overlay: ChannelOverlay::new(base(tiles), world(), channel()),
            facts: Facts::default(),
            reset_epoch: 1,
            items: SessionItemView::resume(ItemViewContinuity::default()).with_map_view(),
            view: SessionMapView::default(),
            client: None,
        }
    }

    fn source(&self) -> MapViewSource<'_, Facts> {
        MapViewSource {
            overlay: &self.overlay,
            facts: &self.facts,
            content_generation: [7; 32],
            reset_epoch: self.reset_epoch,
        }
    }

    fn digest(&self) -> [u8; 32] {
        self.overlay.base().digest()
    }

    /// The join snapshot at `actor`, applied by the client.
    fn join(&mut self, actor: TilePos) -> ItemViewSnapshotDomain {
        let source = MapViewSource {
            overlay: &self.overlay,
            facts: &self.facts,
            content_generation: [7; 32],
            reset_epoch: self.reset_epoch,
        };
        let snapshot = self
            .view
            .snapshot(&mut self.items, &source, at(actor))
            .expect("snapshot");
        self.client = Some(Client::from_snapshot(&snapshot));
        snapshot
    }

    /// The update at `actor`, applied by the client.
    fn step(&mut self, actor: TilePos) -> Option<MapUpdate> {
        let source = MapViewSource {
            overlay: &self.overlay,
            facts: &self.facts,
            content_generation: [7; 32],
            reset_epoch: self.reset_epoch,
        };
        let update = self
            .view
            .update(&mut self.items, &source, at(actor))
            .expect("update");
        match &update {
            Some(MapUpdate::Snapshot(snapshot)) => {
                self.client = Some(Client::from_snapshot(snapshot));
            }
            Some(MapUpdate::Delta(delta)) => {
                let client = self.client.as_mut().expect("joined");
                assert_eq!(delta.from, client.revision);
                client.apply(delta);
            }
            None => {}
        }
        update
    }

    /// The client's window equals a fresh snapshot at its origin.
    fn assert_client_matches_a_fresh_snapshot(&mut self) {
        let client = self.client.clone().expect("joined");
        let origin = client.header.origin;
        let source = MapViewSource {
            overlay: &self.overlay,
            facts: &self.facts,
            content_generation: [7; 32],
            reset_epoch: self.reset_epoch,
        };
        let mut view = SessionMapView::default();
        let fresh = view
            .snapshot(&mut self.items, &source, origin)
            .expect("snapshot");
        let fresh = Client::from_snapshot(&fresh);
        assert_eq!(client.header, fresh.header);
        assert_eq!(client.tiles, fresh.tiles);
    }

    fn client_tile(&self, pos: TilePos) -> Option<&MapTile> {
        self.client
            .as_ref()
            .expect("joined")
            .tiles
            .get(&tile_order(&at(pos)))
    }

    fn hide(&mut self, pos: TilePos, ordinal: u8) {
        self.overlay
            .hide(pos, ordinal, Admission::Refusable)
            .expect("hidden");
    }

    fn add(&mut self, pos: TilePos, id: u32) -> u64 {
        self.overlay
            .add_volatile(
                pos,
                VolatileItem {
                    id,
                    count: 1,
                    attributes: Vec::new(),
                },
                None,
            )
            .expect("added")
            .get()
    }
}

/// What a client holds of domain 17.
#[derive(Debug, Clone)]
struct Client {
    revision: u64,
    header: MapViewHeader,
    tiles: BTreeMap<(i16, i32, i32), MapTile>,
}

impl Client {
    fn from_snapshot(snapshot: &ItemViewSnapshotDomain) -> Self {
        assert_eq!(snapshot.domain_id, STATE_DOMAIN_WORLD_MAP_VIEW);
        assert_eq!(
            snapshot.snapshot_type,
            SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1
        );
        let decoded = decode_world_map_snapshot(&snapshot.payload).expect("snapshot decodes");
        Self {
            revision: snapshot.revision,
            header: decoded.header,
            tiles: decoded
                .tiles
                .into_iter()
                .map(|tile| (tile_order(&tile.position), tile))
                .collect(),
        }
    }

    fn apply(&mut self, delta: &ItemViewDelta) {
        assert_eq!(delta.domain_id, STATE_DOMAIN_WORLD_MAP_VIEW);
        assert_eq!(delta.delta_type, DELTA_TYPE_WORLD_MAP_VIEW_DELTA_V1);
        let decoded = decode_world_map_delta(&self.header, &delta.payload).expect("delta decodes");
        let origin = decoded.header.origin;
        self.tiles
            .retain(|_, tile| in_window(origin, tile.position));
        for tile in decoded.tiles {
            self.tiles.insert(tile_order(&tile.position), tile);
        }
        for cleared in decoded.cleared {
            self.tiles.remove(&tile_order(&cleared));
        }
        self.header = decoded.header;
        self.revision = delta.to;
    }
}

fn decoded_delta(
    fixture: &Fixture,
    update: &Option<MapUpdate>,
    view: &MapViewHeader,
) -> WorldMapViewDelta {
    let Some(MapUpdate::Delta(delta)) = update else {
        panic!("expected a delta, got {update:?}");
    };
    let _ = fixture;
    decode_world_map_delta(view, &delta.payload).expect("delta decodes")
}

fn definitions(tile: &MapTile) -> Vec<MapDefinition> {
    tile.items.iter().map(|item| item.definition).collect()
}

fn item_def(id: u32) -> MapDefinition {
    MapDefinition::Item(nz(id))
}

fn target(digest: [u8; 32], key: u64, expected_revision: u64) -> WorldObjectTarget {
    let mut placement = digest.to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    WorldObjectTarget {
        placement,
        expected_revision,
    }
}

fn key(pos: TilePos, ordinal: u8) -> u64 {
    view::placement_key(pos, ordinal).expect("fixture")
}

// --- Composition -------------------------------------------------------------------------------

#[test]
fn a_hidden_entry_is_absent_an_added_item_has_a_handle_and_an_eleven_entry_stack_has_more() {
    let stack = |n| {
        let mut items = vec![item(GRASS)];
        items.extend((0..n).map(|_| item(TABLE)));
        items
    };
    let hidden_at = tp(21, 20, -7);
    let long = tp(22, 20, -7);
    let mut fixture = Fixture::new(vec![
        (ACTOR, vec![item(GRASS)]),
        (hidden_at, vec![item(GRASS), item(DOOR), item(TABLE)]),
        (long, stack(10)),
    ]);
    fixture.hide(hidden_at, 1);
    let added = fixture.add(ACTOR, COIN);
    fixture.join(ACTOR);

    let hidden = fixture.client_tile(hidden_at).expect("tile");
    assert_eq!(
        definitions(hidden),
        vec![MapDefinition::Terrain(nz(GRASS)), item_def(TABLE)]
    );
    // The table keeps its base ordinal 2.
    assert!(matches!(
        hidden.items[1].origin,
        MapOrigin::BaseOrdinal { ordinal: 2, .. }
    ));

    let actor = fixture.client_tile(ACTOR).expect("tile");
    let handle = fixture
        .items
        .table()
        .handle(&ItemKey::MapAdded(added))
        .expect("added item has a handle");
    assert_eq!(actor.items[1].origin, MapOrigin::Handle(handle));
    assert_eq!(actor.ground_speed, 150);

    let long = fixture.client_tile(long).expect("tile");
    assert_eq!(long.items.len(), 10);
    assert!(long.more);
}

#[test]
fn the_cut_of_a_twelve_entry_stack_sends_entry_0_and_entries_3_to_11() {
    let twelve: Vec<u8> = (0..12).collect();
    let (kept, more) = view::cut(&twelve);
    assert_eq!(
        kept,
        [0, 3, 4, 5, 6, 7, 8, 9, 10, 11].iter().collect::<Vec<_>>()
    );
    assert!(more);
    let ten: Vec<u8> = (0..10).collect();
    let (kept, more) = view::cut(&ten);
    assert_eq!(kept.len(), 10);
    assert!(!more);

    // On the wire: ground, two tables, then nine coins; the tables are cut.
    let pos = tp(21, 20, -7);
    let mut items = vec![item(GRASS), item(TABLE), item(TABLE)];
    items.extend((0..9).map(|_| item(DOOR)));
    let mut fixture = Fixture::new(vec![(pos, items)]);
    fixture.join(ACTOR);
    let tile = fixture.client_tile(pos).expect("tile");
    assert!(tile.more);
    assert_eq!(tile.items[0].definition, MapDefinition::Terrain(nz(GRASS)));
    let ordinals: Vec<u8> = tile
        .items
        .iter()
        .map(|item| match item.origin {
            MapOrigin::BaseOrdinal { ordinal, .. } => ordinal,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(ordinals, [0, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
}

#[test]
fn a_ground_item_is_in_its_tile_and_not_in_domain_1() {
    let pos = tp(21, 21, -7);
    let mut fixture = Fixture::new(vec![(pos, vec![item(GRASS)])]);
    let mut spatial = Vec::new();
    spatial.extend_from_slice(&i32::from(pos.x).to_be_bytes());
    spatial.extend_from_slice(&i32::from(pos.y).to_be_bytes());
    spatial.extend_from_slice(&i16::from(pos.floor).to_be_bytes());
    let id = uuid(9);
    fixture
        .overlay
        .add_ground(GroundItemInstance {
            item_instance_id: id,
            world_id: world(),
            channel_id: channel(),
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: "item:coin".into(),
                revision_ref: "rev-1".into(),
            },
            quantity: 1,
            runtime_scope_ownership_generation: 1,
            ground: GroundPlacement {
                spatial_position: spatial,
                corpse_ref: Vec::new(),
                map_revision: map_revision(fixture.overlay.base()),
                content_revision: "rev-1".into(),
                native_room_placement_context: Vec::new(),
            },
            minted_transaction_id: uuid(10),
        })
        .expect("ground item");
    fixture.join(ACTOR);
    let tile = fixture.client_tile(pos).expect("tile");
    let handle = fixture
        .items
        .table()
        .handle(&ItemKey::Instance(id))
        .expect("ground item has a handle");
    assert_eq!(tile.items[1].origin, MapOrigin::Handle(handle));
    assert_eq!(
        resolve_map_handle(&fixture.source(), ItemKey::Instance(id)),
        Ok(MapHandleTarget::Ground {
            pos,
            item_instance_id: id
        })
    );

    assert!(!carried_in_domain_1(EntityKind::GroundItem, true));
    assert!(!carried_in_domain_1(EntityKind::Corpse, true));
    for kind in [EntityKind::Player, EntityKind::Creature, EntityKind::Npc] {
        assert!(carried_in_domain_1(kind, true));
    }
    for kind in [EntityKind::GroundItem, EntityKind::Corpse] {
        assert!(carried_in_domain_1(kind, false));
    }
}

#[test]
fn appearance_ids_come_from_the_definition_key() {
    assert_eq!(view::appearance_id("oteryn:item.tibia.i2854", None), 2854);
    assert_eq!(
        view::appearance_id("oteryn:terrain.tibia.i4526", None),
        4526
    );
    assert_eq!(
        view::appearance_id("oteryn:item.donor.chest", Some(28827)),
        28827
    );
    let overflow_key = format!("oteryn:item.tibia.i{}", u32::from(u16::MAX) + 1);
    for key in [
        "oteryn:item.tibia.i",
        "oteryn:item.tibia.i012",
        "oteryn:item.tibia.i12a",
        overflow_key.as_str(),
        "item:coin",
    ] {
        assert_eq!(view::appearance_id(key, None), 0, "{key}");
    }
    assert_eq!(
        view::appearance_id("oteryn:item.donor.big", Some(65_536)),
        0
    );
}

// --- Move source and object revision -----------------------------------------------------------

#[test]
fn only_a_pickupable_unbound_base_entry_carries_a_handle() {
    let open = tp(21, 20, -7);
    let house = tp(22, 20, -7);
    let mut fixture = Fixture::new(vec![
        (
            open,
            vec![
                item(GRASS),
                item(COIN),
                item(DOOR),
                item(CHEST),
                item(TABLE),
                item(BAG),
                content(COIN),
            ],
        ),
        (house, vec![item(GRASS), item(COIN)]),
    ]);
    fixture.facts.houses.insert(house);
    fixture.join(ACTOR);
    let tile = fixture.client_tile(open).expect("tile").clone();
    let coin = ItemKey::MapBase {
        bundle_digest: fixture.digest(),
        placement_key: key(open, 1),
        reset_epoch: 1,
    };
    let handle = fixture.items.table().handle(&coin).expect("coin handle");
    assert_eq!(tile.items[1].origin, MapOrigin::Handle(handle));
    // The door, the bound chest, the furniture and the bag holding contents keep their ordinals.
    for (index, ordinal) in [(2, 2), (3, 3), (4, 4), (5, 5)] {
        assert_eq!(
            tile.items[index].origin,
            MapOrigin::BaseOrdinal {
                ordinal,
                object_revision: 0
            }
        );
    }
    // Contents are never on the map.
    assert_eq!(tile.items.len(), 6);
    // A house tile's coin is not movable.
    assert!(matches!(
        fixture.client_tile(house).expect("tile").items[1].origin,
        MapOrigin::BaseOrdinal { ordinal: 1, .. }
    ));

    // Command 9 from it is NOT_SUPPORTED, and nothing is resent.
    assert_eq!(
        move_source_outcome(&coin),
        Some(ItemMoveOutcome::NotSupported)
    );
    assert_eq!(move_source_outcome(&ItemKey::Instance(uuid(3))), None);
    assert_eq!(fixture.step(ACTOR), None);

    // The handle resolves until the overlay hides the entry, then it is STALE.
    assert!(matches!(
        resolve_map_handle(&fixture.source(), coin),
        Ok(MapHandleTarget::Base { ordinal: 1, .. })
    ));
    fixture.hide(open, 1);
    assert_eq!(
        resolve_map_handle(&fixture.source(), coin),
        Err(UseDisposition::StaleState)
    );
    // Another epoch or digest is STALE too.
    let ItemKey::MapBase { placement_key, .. } = coin else {
        unreachable!()
    };
    for stale in [
        ItemKey::MapBase {
            bundle_digest: fixture.digest(),
            placement_key: key(open, 2),
            reset_epoch: 2,
        },
        ItemKey::MapBase {
            bundle_digest: [0; 32],
            placement_key,
            reset_epoch: 1,
        },
        ItemKey::MapAdded(77),
    ] {
        assert_eq!(
            resolve_map_handle(&fixture.source(), stale),
            Err(UseDisposition::StaleState)
        );
    }
}

#[test]
fn a_door_revision_is_sent_checked_and_resent_on_each_transition() {
    let door = tp(21, 20, -7);
    let mut fixture = Fixture::new(vec![(door, vec![item(GRASS), item(DOOR)])]);
    let door_key = key(door, 1);
    fixture.join(ACTOR);
    assert_eq!(
        fixture.client_tile(door).expect("tile").items[1].origin,
        MapOrigin::BaseOrdinal {
            ordinal: 1,
            object_revision: 0
        }
    );
    let digest = fixture.digest();
    let resolved = resolve_map_target(
        &fixture.source(),
        &target(digest, door_key, 0),
        |_| None::<()>,
    )
    .expect("USE with 0 resolves");
    assert_eq!((resolved.pos, resolved.ordinal), (door, 1));

    // The open: revision 1, and the tile is resent.
    fixture.facts.revisions.insert(door_key, 1);
    let view = fixture.client.as_ref().expect("joined").header;
    let update = fixture.step(ACTOR);
    let delta = decoded_delta(&fixture, &update, &view);
    assert_eq!(delta.tiles.len(), 1);
    assert_eq!(
        delta.tiles[0].items[1].origin,
        MapOrigin::BaseOrdinal {
            ordinal: 1,
            object_revision: 1
        }
    );
    assert!(
        resolve_map_target(
            &fixture.source(),
            &target(digest, door_key, 1),
            |_| None::<()>
        )
        .is_ok()
    );
    assert_eq!(
        resolve_map_target(
            &fixture.source(),
            &target(digest, door_key, 0),
            |_| None::<()>
        ),
        Err(MapTargetRefusal::Use(UseDisposition::StaleState))
    );
}

#[test]
fn a_forty_byte_target_resolves_only_on_the_active_digest_and_an_unhidden_entry() {
    let pos = tp(21, 20, -7);
    let mut fixture = Fixture::new(vec![(pos, vec![item(GRASS), item(DOOR)])]);
    let digest = fixture.digest();
    let door = key(pos, 1);
    let resolve = |fixture: &Fixture, target: &WorldObjectTarget| {
        resolve_map_target(&fixture.source(), target, |_| None::<()>).map(|target| target.pos)
    };
    assert_eq!(resolve(&fixture, &target(digest, door, 0)), Ok(pos));
    let mut stale = digest;
    stale[0] ^= 1;
    assert_eq!(
        resolve(&fixture, &target(stale, door, 0)),
        Err(MapTargetRefusal::StateRevisionMismatch)
    );
    let nothing = Err(MapTargetRefusal::Use(UseDisposition::NothingToUse));
    // Not 40 bytes, no such entry, an impossible key.
    let mut short = target(digest, door, 0);
    short.placement.pop();
    assert_eq!(resolve(&fixture, &short), nothing);
    assert_eq!(resolve(&fixture, &target(digest, key(pos, 2), 0)), nothing);
    assert_eq!(resolve(&fixture, &target(digest, u64::MAX, 0)), nothing);
    // A hidden entry.
    fixture.hide(pos, 1);
    assert_eq!(resolve(&fixture, &target(digest, door, 0)), nothing);
}

#[test]
fn a_bound_chest_resolves_to_its_canonical_placement_key() {
    use crate::gameplay_transport::ComposedFreshAdmission;
    use crate::interaction_chest_use::{entry_chest, with_entry_chest};

    let world_id = world();
    let room = crate::content::qualify_native_entry_room(world_id).expect("entry room");
    let content = with_entry_chest(room.door()).expect("entry chest");
    let canonical =
        ComposedFreshAdmission::chest_target(&content, entry_chest::PLACEMENT.as_bytes())
            .expect("chest placement");

    let pos = tp(21, 20, -7);
    let fixture = Fixture::new(vec![(pos, vec![item(GRASS), item(CHEST)])]);
    let chest_key = key(pos, 1);
    // The in-memory binding maps the bundle key to the canonical placement; it writes nothing.
    let binding = |placement_key| {
        (placement_key == chest_key)
            .then(|| {
                ComposedFreshAdmission::chest_target(&content, entry_chest::PLACEMENT.as_bytes())
            })
            .flatten()
            .map(|placement| placement.key.clone())
    };
    let resolved = resolve_map_target(
        &fixture.source(),
        &target(fixture.digest(), chest_key, 0),
        binding,
    )
    .expect("resolves");
    assert_eq!(resolved.canonical.as_ref(), Some(&canonical.key));
    assert_eq!(resolved.placement_key, chest_key);
}

// --- Handle budget -----------------------------------------------------------------------------

#[test]
fn the_1024_nearest_entries_carry_handles_and_a_step_resends_the_tile_that_gains_one() {
    // 4 coins on each of the 252 tiles of the actor's floor in view: 1,008; then 10 and 7 coins
    // on two tiles of floor -6: 1,025 in all.
    let mut tiles = Vec::new();
    for pos in view::window(ACTOR)
        .into_iter()
        .filter(|pos| pos.floor == -7)
    {
        tiles.push((pos, vec![item(COIN); 4]));
    }
    assert_eq!(tiles.len(), 252);
    let near = tp(21, 21, -6);
    let far = tp(25, 21, -6);
    tiles.push((near, vec![item(COIN); 10]));
    tiles.push((far, vec![item(COIN); 7]));
    let mut fixture = Fixture::new(tiles);
    fixture.join(ACTOR);

    let client = fixture.client.as_ref().expect("joined");
    let origins: Vec<MapOrigin> = client
        .tiles
        .values()
        .flat_map(|tile| tile.items.iter().map(|item| item.origin))
        .collect();
    assert_eq!(origins.len(), 1_025);
    let handles = origins
        .iter()
        .filter(|origin| matches!(origin, MapOrigin::Handle(_)))
        .count();
    assert_eq!(handles, MAX_MAP_VIEW_HANDLES);
    let far_tile = fixture.client_tile(far).expect("tile");
    assert_eq!(far_tile.items[6].origin, MapOrigin::DisplayOnly);
    assert!(matches!(far_tile.items[5].origin, MapOrigin::Handle(_)));
    assert_eq!(fixture.items.table().live(), MAX_MAP_VIEW_HANDLES);

    // A step east drops the column x 12 (56 coins): the farthest coin gains a handle and its tile
    // is resent.
    let view = fixture.client.as_ref().expect("joined").header;
    let update = fixture.step(tp(21, 20, -7));
    let delta = decoded_delta(&fixture, &update, &view);
    let resent = delta
        .tiles
        .iter()
        .find(|tile| tile.position == at(far))
        .expect("the far tile is resent");
    assert!(
        resent
            .items
            .iter()
            .all(|item| matches!(item.origin, MapOrigin::Handle(_)))
    );
    fixture.assert_client_matches_a_fresh_snapshot();

    // Stepping back takes it away again, and the tile is resent display_only.
    let view = fixture.client.as_ref().expect("joined").header;
    let update = fixture.step(ACTOR);
    let delta = decoded_delta(&fixture, &update, &view);
    let resent = delta
        .tiles
        .iter()
        .find(|tile| tile.position == at(far))
        .expect("the far tile is resent");
    assert_eq!(resent.items[6].origin, MapOrigin::DisplayOnly);
    fixture.assert_client_matches_a_fresh_snapshot();
}

#[test]
fn the_budget_ranks_the_actor_floor_then_distance_then_stack_order() {
    let a = ACTOR;
    let order = |pos, index| view::budget_order(a, pos, index);
    // The actor's floor before any other, whatever the distance.
    assert!(order(tp(28, 26, -7), 9) < order(tp(21, 21, -6), 0));
    // Floor distance, then the perspective position: (21, 21) is distance 0 on -6.
    assert!(order(tp(29, 29, -6), 0) < order(tp(22, 22, -5), 0));
    assert!(order(tp(21, 21, -6), 5) < order(tp(20, 20, -6), 0));
    // Chebyshev distance, then (y, x), then stack order.
    assert!(order(tp(21, 21, -7), 9) < order(tp(22, 20, -7), 0));
    assert!(order(tp(21, 19, -7), 0) < order(tp(19, 20, -7), 0));
    assert!(order(tp(20, 20, -7), 1) < order(tp(20, 20, -7), 2));
}

// --- Snapshots and deltas ----------------------------------------------------------------------

fn plane(floors: &[i8], extent: std::ops::Range<u16>) -> Vec<(TilePos, Vec<Item>)> {
    let mut tiles = Vec::new();
    for floor in floors {
        for y in extent.clone() {
            for x in extent.clone() {
                tiles.push((tp(x, y, *floor), vec![item(GRASS)]));
            }
        }
    }
    tiles
}

#[test]
fn a_diagonal_step_sends_at_most_31_tiles_per_floor_and_matches_a_fresh_snapshot() {
    let mut fixture = Fixture::new(plane(&[-7, -6], 0..64));
    let start = tp(30, 30, -7);
    let first = fixture.join(start);
    let view = fixture.client.as_ref().expect("joined").header;
    let update = fixture.step(tp(31, 31, -7));
    let delta = decoded_delta(&fixture, &update, &view);
    for floor in [-7, -6] {
        let count = delta
            .tiles
            .iter()
            .filter(|tile| tile.position.floor == floor)
            .count();
        assert_eq!(count, 31, "floor {floor}");
    }
    assert!(delta.cleared.is_empty());
    let Some(MapUpdate::Delta(raw)) = update else {
        unreachable!()
    };
    assert_eq!(raw.from, first.revision);
    assert_eq!(raw.to, first.revision + 1);
    fixture.assert_client_matches_a_fresh_snapshot();

    // Nothing changed: nothing is sent.
    assert_eq!(fixture.step(tp(31, 31, -7)), None);
}

#[test]
fn a_step_onto_empty_tiles_sends_the_origin_alone() {
    let mut fixture = Fixture::new(vec![(ACTOR, vec![item(GRASS)])]);
    fixture.join(ACTOR);
    let view = fixture.client.as_ref().expect("joined").header;
    let next = tp(21, 20, -7);
    let update = fixture.step(next);
    let delta = decoded_delta(&fixture, &update, &view);
    assert!(delta.tiles.is_empty() && delta.cleared.is_empty());
    assert_eq!(delta.header.origin, at(next));
    fixture.assert_client_matches_a_fresh_snapshot();
}

#[test]
fn an_emptied_tile_in_view_is_cleared() {
    let pos = tp(21, 20, -7);
    let mut fixture = Fixture::new(vec![(ACTOR, vec![item(GRASS)]), (pos, vec![item(TABLE)])]);
    fixture.join(ACTOR);
    fixture.hide(pos, 0);
    let view = fixture.client.as_ref().expect("joined").header;
    let update = fixture.step(ACTOR);
    let delta = decoded_delta(&fixture, &update, &view);
    assert!(delta.tiles.is_empty());
    assert_eq!(delta.cleared, vec![at(pos)]);
    assert!(fixture.client_tile(pos).is_none());
    fixture.assert_client_matches_a_fresh_snapshot();
}

#[test]
fn a_teleport_a_floor_change_an_epoch_change_and_an_oversized_change_send_a_snapshot() {
    let mut fixture = Fixture::new(plane(&[-7, -6], 0..64));
    fixture.join(ACTOR);
    let snapshot = |update: Option<MapUpdate>| matches!(update, Some(MapUpdate::Snapshot(_)));
    assert!(snapshot(fixture.step(tp(22, 20, -7))), "teleport");
    assert!(snapshot(fixture.step(tp(22, 20, -6))), "floor change");
    fixture.reset_epoch = 2;
    assert!(snapshot(fixture.step(tp(22, 20, -6))), "reset epoch");
    fixture.assert_client_matches_a_fresh_snapshot();

    // 249 changed tiles exceed MAPW-RL-03.
    let floor = tp(22, 20, -6);
    let positions: Vec<TilePos> = view::window(floor)
        .into_iter()
        .filter(|pos| pos.floor == -6)
        .take(MAX_DELTA_ENTRIES + 1)
        .collect();
    for pos in &positions {
        fixture.add(*pos, TABLE);
    }
    assert!(snapshot(fixture.step(floor)), "over MAPW-RL-03");
    // 248 changed tiles fit in one delta.
    for pos in &positions[..MAX_DELTA_ENTRIES] {
        fixture.add(*pos, TABLE);
    }
    let view = fixture.client.as_ref().expect("joined").header;
    let update = fixture.step(floor);
    assert_eq!(
        decoded_delta(&fixture, &update, &view).tiles.len(),
        MAX_DELTA_ENTRIES
    );
    fixture.assert_client_matches_a_fresh_snapshot();
}

#[test]
fn an_update_that_does_not_encode_sends_nothing_and_keeps_the_view() {
    let mut fixture = Fixture::new(vec![(ACTOR, vec![item(GRASS)])]);
    fixture.join(ACTOR);
    let revision = fixture.view.revision();
    let live = fixture.items.table().live();
    let mut view = std::mem::take(&mut fixture.view);
    let source = fixture.source();
    let mut items = SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
    assert_eq!(
        view.update(
            &mut items,
            &source,
            ActorPosition {
                x: -1,
                y: 0,
                floor: -7
            }
        ),
        Err(MapViewError::Position)
    );
    assert_eq!(view.revision(), revision);
    assert_eq!(fixture.items.table().live(), live);
}

// --- Visible floors ----------------------------------------------------------------------------

fn entry(id: u32) -> ComposedEntry {
    ComposedEntry {
        source: EntrySource::Base {
            ordinal: 0,
            placement_key: 0,
            movable: false,
        },
        facts: facts_of(id),
    }
}

/// The resolver over hand-built stacks.
struct Stacks(BTreeMap<TilePos, ComposedTile>);

impl Stacks {
    fn new(stacks: &[(TilePos, &[u32])]) -> Self {
        Self(
            stacks
                .iter()
                .map(|(pos, ids)| {
                    (
                        *pos,
                        ComposedTile {
                            pos: *pos,
                            entries: ids.iter().map(|id| entry(*id)).collect(),
                            ground_speed: 0,
                        },
                    )
                })
                .collect(),
        )
    }

    /// The same stacks with every non-bottom part reversed.
    fn reversed(&self) -> Self {
        let mut stacks = self.0.clone();
        for tile in stacks.values_mut() {
            tile.entries[1..].reverse();
        }
        Self(stacks)
    }

    fn first_visible_floor(&self, actor: TilePos) -> i8 {
        let first = view::first_visible_floor(actor, |pos| RoofFacts::of(self.0.get(&pos)));
        let reversed = self.reversed();
        assert_eq!(
            view::first_visible_floor(actor, |pos| RoofFacts::of(reversed.0.get(&pos))),
            first,
            "the stack order of the non-bottom entries does not matter"
        );
        first
    }
}

#[test]
fn visible_floors_follow_the_roofs_above_and_around_the_actor() {
    let a = ACTOR;
    let fvf = |stacks: &[(TilePos, &[u32])]| Stacks::new(stacks).first_visible_floor(a);
    // In the open on the surface.
    assert_eq!(fvf(&[(a, &[GRASS])]), 0);
    // Indoors under a roof or a ground directly above.
    assert_eq!(fvf(&[(tp(20, 20, -6), &[ROOF])]), -7);
    assert_eq!(fvf(&[(tp(20, 20, -6), &[GRASS, TABLE])]), -7);
    // A roof two floors up with nothing on -6.
    assert_eq!(fvf(&[(tp(20, 20, -5), &[ROOF])]), -6);
    // A roof that only the east neighbour sees, in perspective, limits it through that open
    // neighbour ...
    let east_roof = (tp(22, 21, -6), &[ROOF][..]);
    assert_eq!(fvf(&[(tp(21, 20, -7), &[GRASS]), east_roof]), -7);
    // ... but not behind a wall or a blocks_projectile item.
    assert_eq!(fvf(&[(tp(21, 20, -7), &[GRASS, WALL]), east_roof]), 0);
    assert_eq!(fvf(&[(tp(21, 20, -7), &[GRASS, PILLAR]), east_roof]), 0);
    // A roof at the diagonal neighbour does not limit it.
    assert_eq!(fvf(&[(tp(21, 19, -6), &[ROOF])]), 0);
    // A doorway: the open side limits it, the walled side does not.
    assert_eq!(
        fvf(&[
            (a, &[GRASS, DOOR]),
            (tp(20, 19, -7), &[WALL]),
            (tp(20, 19, -6), &[ROOF]),
        ]),
        0
    );
    assert_eq!(
        fvf(&[
            (a, &[GRASS, DOOR]),
            (tp(20, 21, -7), &[GRASS]),
            (tp(20, 21, -6), &[ROOF]),
        ]),
        -7
    );
    // Perspective: (x + k, y + k) on -7 + k covers the actor's tile; (x - k, y - k) does not.
    assert_eq!(fvf(&[(tp(21, 21, -6), &[ROOF])]), -7);
    assert_eq!(fvf(&[(tp(22, 22, -5), &[ROOF])]), -6);
    assert_eq!(fvf(&[(tp(19, 19, -6), &[ROOF])]), 0);
    // A 12-entry stack whose limiting ground is entry 0 limits it.
    let mut long = vec![GRASS];
    long.extend([TABLE; 11]);
    assert_eq!(fvf(&[(tp(20, 20, -6), &long)]), -7);
    // A blocks_projectile item the cut drops still stops the look-through.
    let mut blocked = vec![GRASS, PILLAR];
    blocked.extend([TABLE; 10]);
    assert!(!view::cut(&blocked).0.contains(&&PILLAR));
    assert_eq!(fvf(&[(tp(21, 20, -7), &blocked), east_roof]), 0);
}

#[test]
fn visible_floors_underground_stop_at_the_ceiling() {
    let a = tp(20, 20, -10);
    let fvf = |stacks: &[(TilePos, &[u32])]| Stacks::new(stacks).first_visible_floor(a);
    assert_eq!(fvf(&[]), -8);
    assert_eq!(fvf(&[(tp(20, 20, -8), &[GRASS])]), -9);
    assert_eq!(fvf(&[(tp(20, 20, -9), &[GRASS])]), -10);
}

#[test]
fn walking_out_of_a_house_changes_only_the_header() {
    // A grass row on -7 and a roof above (20, 20) only. From (21, 20) the open west neighbour's
    // roof still limits the view; at (22, 20) nothing does.
    let mut tiles: Vec<(TilePos, Vec<Item>)> = (19..=23)
        .map(|x| (tp(x, 20, -7), vec![item(GRASS)]))
        .collect();
    tiles.push((tp(20, 20, -6), vec![item(ROOF)]));
    let mut fixture = Fixture::new(tiles);
    fixture.join(ACTOR);
    assert_eq!(
        fixture
            .client
            .as_ref()
            .expect("joined")
            .header
            .first_visible_floor,
        -7
    );
    assert!(matches!(
        fixture.step(tp(21, 20, -7)),
        Some(MapUpdate::Delta(_))
    ));
    assert_eq!(
        fixture
            .client
            .as_ref()
            .expect("joined")
            .header
            .first_visible_floor,
        -7
    );

    let view = fixture.client.as_ref().expect("joined").header;
    let update = fixture.step(tp(22, 20, -7));
    let delta = decoded_delta(&fixture, &update, &view);
    assert_eq!(delta.header.first_visible_floor, 0);
    assert!(delta.tiles.is_empty() && delta.cleared.is_empty());

    let view = fixture.client.as_ref().expect("joined").header;
    let update = fixture.step(tp(21, 20, -7));
    let delta = decoded_delta(&fixture, &update, &view);
    assert_eq!(delta.header.first_visible_floor, -7);
    fixture.assert_client_matches_a_fresh_snapshot();

    // A stair up sends a snapshot with the new floor's value.
    let Some(MapUpdate::Snapshot(snapshot)) = fixture.step(tp(21, 20, -6)) else {
        panic!("a floor change sends a snapshot");
    };
    let decoded = decode_world_map_snapshot(&snapshot.payload).expect("decodes");
    assert_eq!(decoded.header.first_visible_floor, 0);
}

#[test]
fn an_overlay_change_that_removes_the_roof_sends_the_new_value() {
    let roof = tp(20, 20, -6);
    let mut fixture = Fixture::new(vec![(ACTOR, vec![item(GRASS)]), (roof, vec![item(ROOF)])]);
    fixture.join(ACTOR);
    assert_eq!(
        fixture
            .client
            .as_ref()
            .expect("joined")
            .header
            .first_visible_floor,
        -7
    );
    fixture.hide(roof, 0);
    let view = fixture.client.as_ref().expect("joined").header;
    let update = fixture.step(ACTOR);
    let delta = decoded_delta(&fixture, &update, &view);
    assert_eq!(delta.header.first_visible_floor, 0);
    assert_eq!(delta.cleared, vec![at(roof)]);
    fixture.assert_client_matches_a_fresh_snapshot();
}

// --- Golden ------------------------------------------------------------------------------------

/// A seeded xorshift64* stream.
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

/// A seeded world over `floors` and `extent`: grass, walls and roofs, stacks of up to 14 entries
/// (past the 10-entry cut) with contents, and holes.
fn seeded_world(
    seed: u64,
    floors: &[i8],
    extent: std::ops::Range<u16>,
) -> Vec<(TilePos, Vec<Item>)> {
    let mut rng = Seeded(seed);
    let mut tiles = Vec::new();
    for floor in floors {
        for y in extent.clone() {
            for x in extent.clone() {
                let kind = rng.below(100);
                if kind < 4 {
                    continue;
                }
                let mut items = vec![item(match kind {
                    4..=11 => WALL,
                    12..=19 => ROOF,
                    _ => GRASS,
                })];
                let extra = match rng.below(10) {
                    0..=4 => 0,
                    5..=7 => rng.below(4),
                    8 => rng.below(8),
                    _ => 6 + rng.below(9),
                };
                for _ in 0..extra {
                    let id = [TABLE, COIN, DOOR, CHEST, BAG, PILLAR, COIN][rng.below(7) as usize];
                    items.push(item(id));
                    if id == BAG && rng.below(2) == 0 {
                        items.push(content(COIN));
                    }
                }
                tiles.push((tp(x, y, *floor), items));
            }
        }
    }
    tiles
}

/// A running digest of every update a fixture sends.
#[derive(Default)]
struct Golden {
    digest: sha2::Sha256,
    updates: usize,
    bytes: usize,
}

impl Golden {
    fn record(&mut self, update: &Option<MapUpdate>) {
        use sha2::Digest;
        let (tag, revision, payload) = match update {
            None => (0u8, 0u64, &[][..]),
            Some(MapUpdate::Snapshot(snapshot)) => {
                (1, snapshot.revision, snapshot.payload.as_slice())
            }
            Some(MapUpdate::Delta(delta)) => (2, delta.to, delta.payload.as_slice()),
        };
        self.digest.update([tag]);
        self.digest.update(revision.to_be_bytes());
        self.digest.update((payload.len() as u64).to_be_bytes());
        self.digest.update(payload);
        self.updates += 1;
        self.bytes += payload.len();
    }
}

/// A walk of `steps` steps from `start` over a seeded world, with seeded overlay changes in
/// view: steps, floor changes and teleports. Returns the digest of every payload.
fn golden_walk(seed: u64, floors: &[i8], start: TilePos, steps: usize) -> Golden {
    let mut fixture = Fixture::new(seeded_world(seed, floors, 24..72));
    let mut rng = Seeded(seed ^ 0x9e37_79b9_7f4a_7c15);
    let mut golden = Golden::default();
    let mut actor = start;
    let join = fixture.join(actor);
    golden.record(&Some(MapUpdate::Snapshot(join)));
    for _ in 0..steps {
        let (dx, dy, dfloor) = match rng.below(20) {
            0..=13 => (rng.below(3) as i32 - 1, rng.below(3) as i32 - 1, 0),
            14 => (0, 0, 0),
            15 | 16 => (0, 0, if rng.below(2) == 0 { -1 } else { 1 }),
            17 => (rng.below(9) as i32 - 4, rng.below(9) as i32 - 4, 0),
            _ => (rng.below(40) as i32 - 20, rng.below(40) as i32 - 20, 0),
        };
        let next = tp(
            (i32::from(actor.x) + dx).clamp(32, 63) as u16,
            (i32::from(actor.y) + dy).clamp(32, 63) as u16,
            (i32::from(actor.floor) + dfloor)
                .clamp(i32::from(floors[0]), i32::from(floors[floors.len() - 1])) as i8,
        );
        actor = next;
        for _ in 0..rng.below(3) {
            let near = tp(
                (i32::from(actor.x) + rng.below(17) as i32 - 8).clamp(24, 71) as u16,
                (i32::from(actor.y) + rng.below(13) as i32 - 6).clamp(24, 71) as u16,
                floors[rng.below(floors.len() as u64) as usize],
            );
            match rng.below(3) {
                0 => {
                    let id = [COIN, TABLE, BAG][rng.below(3) as usize];
                    let _ = fixture.overlay.add_volatile(
                        near,
                        VolatileItem {
                            id,
                            count: 1,
                            attributes: Vec::new(),
                        },
                        None,
                    );
                }
                1 => {
                    let _ = fixture
                        .overlay
                        .hide(near, rng.below(3) as u8, Admission::Refusable);
                }
                _ => {
                    fixture.facts.houses.insert(near);
                }
            }
        }
        let update = fixture.step(actor);
        golden.record(&update);
        fixture.assert_client_matches_a_fresh_snapshot();
    }
    golden
}

/// Every snapshot and delta byte of seeded walks, on the surface and underground, is the digest
/// recorded before MAP-VIEWPORT-PERF-1: the optimisation changes no wire byte.
#[test]
fn seeded_walks_send_the_bytes_recorded_before_the_viewport_optimisation() {
    use sha2::Digest;
    let surface: Vec<i8> = (-7..=0).collect();
    let walks = [
        golden_walk(11, &surface, tp(48, 48, -7), 80),
        golden_walk(12, &surface, tp(40, 50, -4), 80),
        golden_walk(13, &[-11, -10, -9, -8, -7], tp(48, 48, -9), 80),
        golden_walk(14, &[-9, -8, -7], tp(48, 48, -8), 80),
    ];
    let mut all = sha2::Sha256::new();
    let mut updates = 0;
    let mut bytes = 0;
    for walk in walks {
        all.update(walk.digest.finalize());
        updates += walk.updates;
        bytes += walk.bytes;
    }
    let digest: String = all
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    println!("GOLDEN updates={updates} bytes={bytes} digest={digest}");
    assert_eq!((updates, bytes, digest.as_str()), GOLDEN);
}

const GOLDEN: (usize, usize, &str) = (
    324,
    5_431_963,
    "d9f1866f0a425c14443e5b16339cc4856be3fb4dc29679ae4da1f2efc80b5a01",
);

// --- MAP01-VIEWPORT-US -------------------------------------------------------------------------

/// One 18x14 domain 17 update over all floors in view, against the `MAP01-VIEWPORT-US` p99 of
/// 100 us: 20,000 seeded viewports (floors -7..=0), each a snapshot, and a one-tile step from it
/// as a delta, with the plan / handles / encode split. The server's work only; the test client's
/// decode is not timed. Run in release:
/// `cargo test --release -p oteryn-game-server map_viewport_measure -- --ignored --nocapture`.
#[test]
#[ignore = "measurement; run in release"]
fn map_viewport_measure() {
    let floors: Vec<i8> = (-7..=0).collect();
    let fixture = Fixture::new(seeded_world(7, &floors, 24..72));
    measure_viewports("", &fixture.overlay, &fixture.facts, &floors);
}

/// [`map_viewport_measure`] over the production [`BundleFacts`](crate::map::facts::BundleFacts)
/// of the same seeded world booted as a bundle World (MAP-CUTOVER-1b).
#[test]
#[ignore = "measurement; run in release"]
fn map_viewport_measure_production_facts() {
    let floors: Vec<i8> = (-7..=0).collect();
    let world = booted(seeded_world(7, &floors, 24..72));
    measure_viewports(" production-facts", world.overlay(), world.facts(), &floors);
}

/// The served definition of a test palette Item key: its §1.6 reference is 1 + its palette `id`.
fn production_item(key: &str) -> Option<crate::map::facts::ItemDefinition> {
    let id = match key {
        "item:coin" => COIN,
        "item:door" => DOOR,
        "item:chest" => CHEST,
        "item:table" => TABLE,
        "item:bag" => BAG,
        "item:pillar" => PILLAR,
        _ => return None,
    };
    let solid = matches!(id, DOOR | CHEST | PILLAR);
    Some(crate::map::facts::ItemDefinition {
        reference: nz(id + 1),
        solid: Some(solid),
        blocks_projectile: id == PILLAR,
        pickupable: matches!(id, COIN | BAG),
    })
}

/// `tiles` booted as a bundle World with [`production_item`], starting at the first tile that
/// holds grass alone.
fn booted(tiles: Vec<(TilePos, Vec<Item>)>) -> crate::map::boot::BundleWorld {
    let start = tiles
        .iter()
        .find(|(_, items)| items.len() == 1 && items[0].palette == GRASS - 1)
        .map(|(pos, _)| *pos)
        .expect("a grass tile");
    let (bytes, bundle) = bundle_bytes(tiles);
    let map_revision = map_revision(&crate::map::load(&bytes, &bundle).expect("loaded base"));
    let pins = crate::map::boot::BootPins {
        bundle,
        map_revision,
        start,
    };
    crate::map::boot::boot(&bytes, &pins, world(), channel(), production_item).expect("booted")
}

fn measure_viewports<F: MapFacts>(label: &str, overlay: &ChannelOverlay, facts: &F, floors: &[i8]) {
    use std::time::{Duration, Instant};
    const SAMPLES: usize = 20_000;
    let mut items = SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
    let mut view = SessionMapView::default();
    let mut rng = Seeded(0x5eed_0007);
    let (mut snapshots, mut deltas) = (Vec::with_capacity(SAMPLES), Vec::with_capacity(SAMPLES));
    let mut plans = Vec::with_capacity(SAMPLES);
    let source = MapViewSource {
        overlay,
        facts,
        content_generation: [7; 32],
        reset_epoch: 1,
    };
    for _ in 0..SAMPLES {
        let actor = tp(
            34 + rng.below(28) as u16,
            34 + rng.below(28) as u16,
            floors[rng.below(floors.len() as u64) as usize],
        );
        let started = Instant::now();
        let planned = plan(&source, at(actor)).expect("plan");
        plans.push(started.elapsed());
        std::hint::black_box(planned);
        let started = Instant::now();
        let snapshot = view
            .snapshot(&mut items, &source, at(actor))
            .expect("snapshot");
        snapshots.push(started.elapsed());
        std::hint::black_box(snapshot);
        let stepped = tp(actor.x + 1, actor.y, actor.floor);
        let started = Instant::now();
        let delta = view
            .update(&mut items, &source, at(stepped))
            .expect("delta");
        deltas.push(started.elapsed());
        std::hint::black_box(delta);
    }
    let report = |name: &str, samples: &mut Vec<Duration>| {
        samples.sort_unstable();
        println!(
            "MAP01-VIEWPORT-US{label} {name} p50={:?} p99={:?} max={:?}",
            samples[samples.len() / 2],
            samples[samples.len() * 99 / 100],
            samples[samples.len() - 1]
        );
    };
    report("snapshot", &mut snapshots);
    report("delta", &mut deltas);
    report("stage plan (compose + rank)", &mut plans);
}
