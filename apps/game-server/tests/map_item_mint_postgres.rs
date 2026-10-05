#![allow(clippy::expect_used)]
// Dedicated PostgreSQL 17 qualification for MAP-OVERLAY-1b (DUR-03 map-item
// MINT of a map-authored entry into Ground at its tile, then the B3-1
// TRANSFER), plus the overlay side of the pickup against a fixture bundle.
// Ordinary workspace runs report PRE-ROUTING/NONCANONICAL when the routed
// database is absent.
extern crate oteryn_game_server as production_server;
extern crate self as oteryn_game_server;

pub use production_server::admission_evidence;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_recovery_fence.rs"]
pub mod character_recovery_fence;
pub use production_server::domain;
#[allow(dead_code, unused_imports)]
#[path = "../src/durability/mod.rs"]
mod durability;
#[allow(dead_code, unused_imports)]
#[path = "../src/foundation/mod.rs"]
pub mod foundation;
#[allow(dead_code, unused_imports)]
#[path = "../src/native_admission_source/mod.rs"]
pub mod native_admission_source;

// Standalone target for local focused runs; the protected PostgreSQL lane also
// includes these exact cases through character_authority_postgres.
#[path = "support/map_item_mint_postgres_cases.rs"]
mod map_item_mint_postgres_cases;

/// ADR-0021 §4.4 overlay side of a map-item pickup, against a fixture bundle
/// assembled as in `map_overlay_channel`.
mod map_overlay_pickup {
    use std::error::Error as StdError;
    use std::sync::Arc;

    use oteryn_world_bundle::bundle::placement_key;
    use oteryn_world_bundle_compiler::Error;
    use oteryn_world_bundle_compiler::bundle::{
        self, BuildClass, Extent, Family, Identity, Manifest, Terrain, TerrainKind,
    };
    use oteryn_world_bundle_compiler::compile::{Input, KeyResolver, Resolution, compile};
    use oteryn_world_bundle_compiler::project::Families;
    use oteryn_world_bundle_compiler::sector::{self, Attrs, Item, Tile};
    use oteryn_world_bundle_compiler::spawn;
    use production_server::durability::map_item_mint::{CommittedMapItemMint, MapItemPlacement};
    use production_server::foundation::{ChannelId, WorldId};
    use production_server::map::overlay::pickup::{
        MintResolution, RehideError, hide_origin_at_freeze, rehide_taken_origins, settle_origin,
    };
    use production_server::map::overlay::{ChannelOverlay, OverlayError, TilePos};
    use production_server::map::{self, BundlePins, LoadError, WorldBase};
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
        let frame =
            zstd::bulk::compress(&sector::encode(&[tile(1, 1, vec![item(0), item(1)])])?, 3)?;
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

    fn channel_overlay(
        base: &Arc<WorldBase>,
        budget: Option<usize>,
    ) -> Result<ChannelOverlay, Box<dyn StdError>> {
        let world = WorldId::decode(&uuid(1)).map_err(|e| format!("{e:?}"))?;
        let channel = ChannelId::decode(&uuid(2)).map_err(|e| format!("{e:?}"))?;
        Ok(match budget {
            Some(budget) => ChannelOverlay::with_budget(Arc::clone(base), world, channel, budget),
            None => ChannelOverlay::new(Arc::clone(base), world, channel),
        })
    }

    /// The (2, 1) coin: top-level entry 1 of its tile on floor -7.
    fn coin_key() -> Result<u64, Box<dyn StdError>> {
        Ok(placement_key(-7, 2, 1, 1).ok_or("key")?)
    }

    const COIN_TILE: TilePos = TilePos {
        x: 2,
        y: 1,
        floor: -7,
    };

    fn committed(key: u64) -> Result<CommittedMapItemMint, Box<dyn StdError>> {
        Ok(CommittedMapItemMint {
            transaction_id: [1; 16],
            event_id: [2; 16],
            item_instance_id: [3; 16],
            occurred_at_unix_ms: 1,
            envelope_sha256: [4; 32],
            quantity: 1,
            placement: MapItemPlacement::of_key(key).ok_or("placement")?,
        })
    }

    #[test]
    fn map_overlay_pickup_hides_the_origin_at_freeze_and_refuses_a_second_freeze() -> TestResult {
        let base = base()?;
        let mut overlay = channel_overlay(&base, None)?;
        let key = coin_key()?;
        hide_origin_at_freeze(&mut overlay, key)?;
        assert!(overlay.tile(COIN_TILE).ok_or("tile")?.is_hidden(1));
        // A pickup in flight, or a taken entry, refuses another freeze.
        assert_eq!(
            hide_origin_at_freeze(&mut overlay, key),
            Err(OverlayError::AlreadyHidden)
        );
        // A key naming no top-level entry of the base is refused.
        let past = placement_key(-7, 2, 1, 2).ok_or("key")?;
        assert_eq!(
            hide_origin_at_freeze(&mut overlay, past),
            Err(OverlayError::Ordinal)
        );
        assert_eq!(
            hide_origin_at_freeze(&mut overlay, u64::MAX),
            Err(OverlayError::Ordinal)
        );
        // Over the budget the freeze-time hide is refused atomically.
        let tight = channel_overlay(&base, None)?.used_bytes();
        let mut full = channel_overlay(&base, Some(tight))?;
        assert!(matches!(
            hide_origin_at_freeze(&mut full, key),
            Err(OverlayError::OverBudget { .. })
        ));
        assert!(full.tile(COIN_TILE).is_none());
        assert_eq!(full.alarm_count(), 0);
        Ok(())
    }

    #[test]
    fn map_overlay_pickup_unhides_the_origin_only_on_a_proven_non_commit() -> TestResult {
        let base = base()?;
        let mut overlay = channel_overlay(&base, None)?;
        let key = coin_key()?;
        hide_origin_at_freeze(&mut overlay, key)?;
        let minted = committed(key)?;
        assert!(settle_origin(&mut overlay, key, MintResolution::Unknown)?);
        assert!(settle_origin(
            &mut overlay,
            key,
            MintResolution::Committed(&minted)
        )?);
        assert!(overlay.tile(COIN_TILE).ok_or("tile")?.is_hidden(1));
        assert!(!settle_origin(
            &mut overlay,
            key,
            MintResolution::ProvenNotCommitted
        )?);
        assert!(
            overlay
                .tile(COIN_TILE)
                .is_none_or(|tile| !tile.is_hidden(1))
        );
        // The entry can be frozen again once shown.
        hide_origin_at_freeze(&mut overlay, key)?;
        Ok(())
    }

    #[test]
    fn map_overlay_pickup_rebuild_rehides_every_taken_origin_over_the_budget() -> TestResult {
        let base = base()?;
        let key = coin_key()?;
        let crowded: Vec<u64> = (0..3)
            .map(|ordinal| placement_key(-7, 1, 1, ordinal).ok_or("key"))
            .collect::<Result<_, _>>()?;
        let tight = channel_overlay(&base, None)?.used_bytes();
        let mut overlay = channel_overlay(&base, Some(tight))?;
        let mut taken = vec![key];
        taken.extend(&crowded);
        rehide_taken_origins(&mut overlay, &taken).map_err(|e| format!("{e:?}"))?;
        assert!(overlay.tile(COIN_TILE).ok_or("tile")?.is_hidden(1));
        let tile = overlay
            .tile(TilePos {
                x: 1,
                y: 1,
                floor: -7,
            })
            .ok_or("tile")?;
        assert!((0..3).all(|ordinal| tile.is_hidden(ordinal)));
        assert!(overlay.over_budget());
        assert_eq!(overlay.alarm_count(), 4);
        // Re-running the rebuild leaves every origin hidden.
        rehide_taken_origins(&mut overlay, &taken).map_err(|e| format!("{e:?}"))?;
        assert!(overlay.tile(COIN_TILE).ok_or("tile")?.is_hidden(1));
        // A receipt naming no top-level base entry fails closed.
        let past = placement_key(-7, 2, 1, 2).ok_or("key")?;
        assert_eq!(
            rehide_taken_origins(&mut overlay, &[past]),
            Err(RehideError {
                placement_key: past,
                reason: OverlayError::Ordinal,
            })
        );
        Ok(())
    }
}
