//! CHEST-PLACE-BIND-1 (ARCH-WORLD-CONTENT-SERVE-1 §1.4, §1.5, §2.3): the `unique` table of the
//! base map and the RewardClaim binder, against a small fixture bundle that each test compiles
//! with the compiler.

use std::collections::BTreeSet;
use std::error::Error as StdError;

use oteryn_game_server::content::world_reward_claims::{
    BindingReport, LeftOut, UnboundReason, bind_reward_claims,
};
use oteryn_game_server::content::{PlacementKey, TypedDefinitionRef};
use oteryn_game_server::map::{self, BundlePins, UniqueEntry, WorldBase};
use oteryn_world_bundle_compiler::Error;
use oteryn_world_bundle_compiler::bundle::{
    self, BuildClass, Extent, Family, Identity, Terrain, TerrainKind,
};
use oteryn_world_bundle_compiler::compile::{Input, KeyResolver, Resolution, compile};
use oteryn_world_bundle_compiler::project::Families;
use oteryn_world_bundle_compiler::sector::{self, Attrs, Item, Tile};

type TestResult = Result<(), Box<dyn StdError>>;

/// Palette keys of the fixture, by index.
const KEYS: [&str; 7] = [
    "oteryn:terrain.tibia.i4526",         // 0: ground
    "oteryn:item.tibia.i1740",            // 1: a chest Item
    "oteryn:terrain.tibia.i103",          // 2: a chest that is a Terrain record, not an Item
    "oteryn:item.tibia.i2000",            // 3: another Item
    "donor:crystalserver@abc:item/28827", // 4: a provisional donor key
    "oteryn:item.custom.chest",           // 5: an Item with no appearance id in its key
    "oteryn:terrain.tibia.i70000",        // 6: an id that is no appearance id
];

fn resolved(key: &str) -> Option<(Family, u32, Option<Terrain>)> {
    let terrain = |kind: TerrainKind, ground: bool| {
        Some(Terrain {
            kind,
            walkable: ground.then_some(true),
            ground_speed: ground.then_some(150),
        })
    };
    Some(match key {
        "oteryn:terrain.tibia.i4526" => (Family::Terrain, 10, terrain(TerrainKind::Ground, true)),
        "oteryn:item.tibia.i1740" => (Family::Item, 11, None),
        "oteryn:terrain.tibia.i103" => (Family::Terrain, 12, terrain(TerrainKind::Common, false)),
        "oteryn:item.tibia.i2000" => (Family::Item, 13, None),
        "oteryn:item.custom.chest" => (Family::Item, 15, None),
        "oteryn:terrain.tibia.i70000" => (Family::Terrain, 16, terrain(TerrainKind::Common, false)),
        _ => return None,
    })
}

struct Resolver;

impl KeyResolver for Resolver {
    fn resolve(&self, key: &str) -> Resolution {
        if key.starts_with("donor:") {
            return Resolution::Provisional;
        }
        resolved(key).map_or(Resolution::Unknown, |(family, id, _)| {
            Resolution::Resolved(family, id)
        })
    }

    fn terrain(&self, key: &str) -> Result<Option<Terrain>, Error> {
        Ok(resolved(key).and_then(|(_, _, terrain)| terrain))
    }

    fn floor_change(&self, _: &str) -> bool {
        false
    }
}

fn item(palette: u32, unique: Option<u16>) -> Item {
    Item {
        palette,
        depth: 0,
        attrs: Attrs {
            unique,
            ..Attrs::default()
        },
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

/// A B3 region file with one sector.
fn region(z: u8, tiles: &[Tile]) -> Result<Vec<u8>, Box<dyn StdError>> {
    let frame = zstd::bulk::compress(&sector::encode(tiles)?, 3)?;
    let mut out = b"OTRB".to_vec();
    out.extend_from_slice(&[1, z]);
    for value in [0u16, 0, 1] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.push(0);
    out.extend_from_slice(&21u32.to_le_bytes());
    out.extend_from_slice(&(frame.len() as u32).to_le_bytes());
    out.extend_from_slice(&frame);
    Ok(out)
}

/// The fixture map at native floor -7. `moved` adds a ground before the chest on (2, 1) and an
/// unrelated tile, which moves entries and tiles but not the chests' cells.
fn tiles(moved: bool) -> Vec<Tile> {
    let mut chest_tile = vec![item(1, Some(101))];
    if moved {
        chest_tile.insert(0, item(0, None));
    }
    let mut tiles = vec![tile(1, 1, vec![item(0, None), item(1, Some(100))])];
    tiles.push(tile(2, 1, chest_tile));
    tiles.extend([
        tile(3, 1, vec![item(2, Some(102))]),
        tile(4, 1, vec![item(3, Some(103))]),
        tile(6, 1, vec![item(1, Some(104)), item(1, Some(104))]),
        tile(7, 1, vec![item(5, Some(105))]),
        tile(8, 1, vec![item(0, None), item(4, Some(106))]),
        tile(10, 1, vec![item(6, Some(107))]),
        tile(11, 1, vec![item(1, None)]),
    ]);
    if moved {
        tiles.push(tile(0, 0, vec![item(0, None)]));
        tiles.sort_by_key(|tile| (tile.y, tile.x));
    }
    tiles
}

fn build(moved: bool) -> Result<(WorldBase, Vec<u8>), Box<dyn StdError>> {
    let regions = vec![region(7, &tiles(moved))?];
    let palette = KEYS.map(String::from).to_vec();
    let families = Families::default();
    let compiled = compile(
        &Input {
            regions: &regions,
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
                max_x: 64,
                max_y: 64,
                floors: vec![-7],
            },
            build_class: BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &families,
        },
        &Resolver,
    )?;
    let pins = BundlePins {
        digest: compiled.digest,
        project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
        world_schema_version: "world-schema-1".into(),
        content_revision: "rev-1".into(),
        production: false,
    };
    let base = map::load(&compiled.bytes, &pins)?;
    Ok((base, compiled.bytes))
}

/// `(appearance, crystal unique id, (x, y, z))` of one placement.
type PlacementSpec = (u32, Option<u32>, (i64, i64, i64));

/// A plain claim record as a shard holds it. Placements are `(appearance, crystal unique id,
/// (x, y, z))`; each rewards the one Item `reward`.
fn claim(key: &str, readiness: &str, placements: &[PlacementSpec], reward: &str) -> String {
    let placements: Vec<String> = placements
        .iter()
        .map(|(appearance, unique, (x, y, z))| {
            let mut ids = r#"{"server":"canary","unique_id":1}"#.to_owned();
            if let Some(unique) = unique {
                ids.push_str(&format!(r#",{{"server":"crystalserver","unique_id":{unique}}}"#));
            }
            format!(
                r#"{{"appearance_tibia_id":{appearance},"reward":{{"items":[{{"count":1,"item":{{"family":"Item","key":"{reward}","revision":"definition-r1"}}}}]}},"source_binding":{{"legacy_unique_ids":[{ids}],"project_position":{{"x":{x},"y":{y},"z":{z}}}}}}}"#
            )
        })
        .collect();
    format!(
        r#"{{"definition":{{"claim":{{"per":"character","repeat":{{"kind":"once"}}}},"identity":{{"key":"{key}","revision":"reward-claim-r1"}},"placements":[{}],"provenance":{{"pilot_key":"p","pilot_revision":"r"}},"quest":{{"family":"Quest","key":"canary:quest/q","revision":"r"}},"readiness":"{readiness}"}}}}"#,
        placements.join(",")
    )
}

fn shard(records: &[String]) -> String {
    format!(
        r#"{{"family":"RewardClaim","records":[{}],"schema":"OTERYN_REWARD_CLAIM_SHARD/v1","shard":{{}}}}"#,
        records.join(",")
    )
}

const REWARD: &str = "oteryn:item.tibia.i3031";

fn bind(base: &WorldBase, records: &[String]) -> Result<BindingReport, Box<dyn StdError>> {
    let admitted = |item: &TypedDefinitionRef| item.key().as_str() == REWARD;
    Ok(bind_reward_claims(base, &[&shard(records)], &admitted)?)
}

fn unbound(report: &BindingReport) -> Vec<(&str, usize, UnboundReason)> {
    report
        .unbound
        .iter()
        .map(|entry| (entry.claim.as_str(), entry.index, entry.reason))
        .collect()
}

#[test]
fn world_reward_claims_the_base_returns_each_unique_as_the_compiler_wrote_it() -> TestResult {
    let (base, bytes) = build(false)?;
    let expected = |unique, appearance| Some(UniqueEntry { unique, appearance });
    let unique = |x, ordinal| base.tile(x, 1, -7).and_then(|tile| tile.unique(ordinal));
    assert_eq!(unique(1, 0), None);
    assert_eq!(unique(1, 1), expected(100, Some(1740)));
    assert_eq!(unique(2, 0), expected(101, Some(1740)));
    assert_eq!(unique(3, 0), expected(102, Some(103)));
    assert_eq!(unique(6, 1), expected(104, Some(1740)));
    assert_eq!(unique(7, 0), expected(105, None));
    assert_eq!(unique(10, 0), expected(107, None));
    // An entry without a `unique` and an ordinal past the top-level entries.
    assert_eq!(unique(11, 0), None);
    assert_eq!(unique(2, 1), None);
    // The table agrees with the compiler's own read of the bundle.
    let read = bundle::read(&bytes)?;
    let mut written = 0;
    for sector in &read.sectors {
        for tile in &sector.tiles {
            for (ordinal, entry) in tile.items.iter().filter(|e| e.depth == 0).enumerate() {
                let view = base.tile(tile.x, tile.y, sector.floor).ok_or("tile")?;
                assert_eq!(
                    view.unique(u8::try_from(ordinal)?).map(|e| e.unique),
                    entry.attrs.unique
                );
                written += usize::from(entry.attrs.unique.is_some());
            }
        }
    }
    assert!(written >= 8);
    Ok(())
}

#[test]
fn world_reward_claims_the_palette_appearance_id_follows_the_key_form() {
    use oteryn_game_server::map::palette_appearance as appearance;
    assert_eq!(appearance("oteryn:item.tibia.i1740"), Some(1740));
    assert_eq!(appearance("oteryn:terrain.tibia.i103"), Some(103));
    assert_eq!(
        appearance("donor:crystalserver@9f5a72c6:item/28827"),
        Some(28827)
    );
    assert_eq!(appearance("oteryn:item.custom.chest"), None);
    assert_eq!(appearance("oteryn:item.tibia.i"), None);
    assert_eq!(appearance("oteryn:item.tibia.i17x"), None);
    assert_eq!(appearance("oteryn:item.tibia.i70000"), None);
    assert_eq!(appearance("oteryn:creature.tibia.i5"), None);
}

#[test]
fn world_reward_claims_each_unbound_reason_has_one_vector() -> TestResult {
    let (base, _) = build(false)?;
    let k = |name: &str| format!("oteryn:reward-claim.{name}");
    let records = [
        claim(
            &k("bound"),
            "ready",
            &[(1740, Some(100), (1, 1, 7))],
            REWARD,
        ),
        claim(
            &k("no_crystal"),
            "ready",
            &[(1740, None, (1, 1, 7))],
            REWARD,
        ),
        claim(
            &k("out_of_bounds"),
            "ready",
            &[(1740, Some(100), (1, 1, 16))],
            REWARD,
        ),
        claim(
            &k("wide_x"),
            "ready",
            &[(1740, Some(100), (70000, 1, 7))],
            REWARD,
        ),
        claim(
            &k("no_entry"),
            "ready",
            &[(1740, Some(999), (1, 1, 7))],
            REWARD,
        ),
        claim(
            &k("no_tile"),
            "ready",
            &[(1740, Some(100), (20, 20, 7))],
            REWARD,
        ),
        claim(
            &k("ambiguous"),
            "ready",
            &[(1740, Some(104), (6, 1, 7))],
            REWARD,
        ),
        claim(
            &k("mismatch"),
            "ready",
            &[(1740, Some(103), (4, 1, 7))],
            REWARD,
        ),
        claim(
            &k("no_appearance"),
            "ready",
            &[(1740, Some(105), (7, 1, 7))],
            REWARD,
        ),
    ];
    let report = bind(&base, &records)?;
    let served: Vec<_> = report.served.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(served, [k("bound")]);
    assert_eq!(
        unbound(&report),
        [
            (
                k("no_crystal").as_str(),
                0,
                UnboundReason::NoCrystalUniqueId
            ),
            (
                k("out_of_bounds").as_str(),
                0,
                UnboundReason::CellOutOfBounds
            ),
            (k("wide_x").as_str(), 0, UnboundReason::CellOutOfBounds),
            (k("no_entry").as_str(), 0, UnboundReason::NoEntry),
            (k("no_tile").as_str(), 0, UnboundReason::NoEntry),
            (k("ambiguous").as_str(), 0, UnboundReason::AmbiguousEntry),
            (k("mismatch").as_str(), 0, UnboundReason::AppearanceMismatch),
            (
                k("no_appearance").as_str(),
                0,
                UnboundReason::AppearanceMismatch
            ),
        ]
    );
    assert!(report.left_out.is_empty());
    Ok(())
}

#[test]
fn world_reward_claims_a_bound_placement_carries_its_cell_and_placement_key() -> TestResult {
    let (base, _) = build(false)?;
    let key = "oteryn:reward-claim.kv.chest";
    let records = [claim(
        key,
        "ready",
        &[(1740, Some(100), (1, 1, 7)), (1740, Some(101), (2, 1, 7))],
        REWARD,
    )];
    let report = bind(&base, &records)?;
    let [served] = report.served.as_slice() else {
        return Err("one served claim".into());
    };
    assert_eq!(served.revision.as_str(), "reward-claim-r1");
    let [first, second] = served.placements.as_slice() else {
        return Err("two placements".into());
    };
    assert_eq!(
        first.placement.placement,
        PlacementKey::new("oteryn:placement/reward-claim.kv.chest/0")?
    );
    assert_eq!(
        second.placement.placement.as_str(),
        "oteryn:placement/reward-claim.kv.chest/1"
    );
    assert_eq!((first.cell.x, first.cell.y, first.cell.floor), (1, 1, -7));
    // Entry ordinal 1 on (1, 1), ordinal 0 on (2, 1), by the compiler's own formula.
    assert_eq!(
        first.bundle_placement_key,
        bundle::placement_key(-7, 1, 1, 1).ok_or("key")?
    );
    assert_eq!(
        second.bundle_placement_key,
        bundle::placement_key(-7, 2, 1, 0).ok_or("key")?
    );
    assert_eq!(first.placement.items.len(), 1);
    assert_eq!(first.placement.items[0].count, 1);
    assert_eq!(first.placement.achievement, None);
    Ok(())
}

#[test]
fn world_reward_claims_the_canonical_key_is_stable_when_entries_move() -> TestResult {
    let records = [claim(
        "oteryn:reward-claim.kv.chest",
        "ready",
        &[(1740, Some(101), (2, 1, 7))],
        REWARD,
    )];
    let (before, _) = build(false)?;
    let (after, _) = build(true)?;
    assert_ne!(before.digest(), after.digest());
    let (a, b) = (bind(&before, &records)?, bind(&after, &records)?);
    let (a, b) = (&a.served[0].placements[0], &b.served[0].placements[0]);
    assert_eq!(a.placement.placement, b.placement.placement);
    assert_eq!(a.cell, b.cell);
    // The entry moved to ordinal 1 behind the new ground, so only the bundle key changed.
    assert_ne!(a.bundle_placement_key, b.bundle_placement_key);
    Ok(())
}

#[test]
fn world_reward_claims_a_claim_with_one_unbound_placement_is_not_served() -> TestResult {
    let (base, _) = build(false)?;
    let records = [claim(
        "oteryn:reward-claim.kv.half",
        "ready",
        &[(1740, Some(100), (1, 1, 7)), (1740, Some(999), (2, 1, 7))],
        REWARD,
    )];
    let report = bind(&base, &records)?;
    assert!(report.served.is_empty());
    assert_eq!(
        unbound(&report),
        [("oteryn:reward-claim.kv.half", 1, UnboundReason::NoEntry)]
    );
    assert_eq!(
        report
            .unbound_claims_by_reason()
            .get(&UnboundReason::NoEntry),
        Some(&1)
    );
    Ok(())
}

#[test]
fn world_reward_claims_variants_and_other_records_are_filtered_out() -> TestResult {
    let (base, _) = build(false)?;
    let ok = &[(1740, Some(100), (1, 1, 7))];
    let plain = claim("oteryn:reward-claim.kv.a", "ready", ok, REWARD);
    let variant = plain
        .replace("oteryn:reward-claim.kv.a", "oteryn:reward-claim.kv.variant")
        .replace(
            r#""claim":"#,
            r#""definition_profile":"authored_variant_v1","claim":"#,
        );
    let waiting = claim("oteryn:reward-claim.kv.waiting", "waiting_data", ok, REWARD);
    let hours = plain
        .replace("oteryn:reward-claim.kv.a", "oteryn:reward-claim.kv.hours")
        .replace(r#"{"kind":"once"}"#, r#"{"kind":"cooldown","hours":24}"#);
    let container = plain
        .replace(
            "oteryn:reward-claim.kv.a",
            "oteryn:reward-claim.kv.container",
        )
        .replace(
            r#"{"items":"#,
            r#"{"container":{"family":"Item","key":"x:y","revision":"r"},"items":"#,
        );
    let two = plain
        .replace("oteryn:reward-claim.kv.a", "oteryn:reward-claim.kv.two")
        .replacen(r#"{"count":1,"item":"#, r#"{"count":1,"item":{"family":"Item","key":"oteryn:item.tibia.i3031","revision":"definition-r1"}},{"count":1,"item":"#, 1);
    let mut grant = plain.replace("oteryn:reward-claim.kv.a", "oteryn:reward-claim.kv.grant");
    grant = grant.replacen(
        r#""appearance_tibia_id""#,
        r#""written_text":"x","appearance_tibia_id""#,
        1,
    );
    let records = [plain, variant, waiting, hours, container, two, grant];
    let report = bind(&base, &records)?;
    assert_eq!(report.served.len(), 1);
    let left: Vec<_> = report
        .left_out
        .iter()
        .map(|(k, why)| (k.as_str(), *why))
        .collect();
    assert_eq!(
        left,
        [
            ("oteryn:reward-claim.kv.variant", LeftOut::Variant),
            ("oteryn:reward-claim.kv.waiting", LeftOut::NotReady),
            ("oteryn:reward-claim.kv.hours", LeftOut::UnencodedField),
            ("oteryn:reward-claim.kv.container", LeftOut::UnencodedField),
            ("oteryn:reward-claim.kv.two", LeftOut::UnencodedField),
            ("oteryn:reward-claim.kv.grant", LeftOut::UnencodedField),
        ]
    );
    Ok(())
}

#[test]
fn world_reward_claims_a_reward_item_without_a_definition_is_not_served() -> TestResult {
    let (base, _) = build(false)?;
    let records = [claim(
        "oteryn:reward-claim.kv.unadmitted",
        "ready",
        &[(1740, Some(100), (1, 1, 7))],
        "oteryn:item.tibia.i9999",
    )];
    let report = bind(&base, &records)?;
    assert!(report.served.is_empty() && report.unbound.is_empty());
    assert_eq!(
        report.left_out,
        [(
            "oteryn:reward-claim.kv.unadmitted".to_owned(),
            LeftOut::ItemNotAdmitted
        )]
    );
    Ok(())
}

#[test]
fn world_reward_claims_a_chest_that_is_not_an_item_binds() -> TestResult {
    let (base, _) = build(false)?;
    let records = [claim(
        "oteryn:reward-claim.kv.terrain_chest",
        "ready",
        &[(103, Some(102), (3, 1, 7))],
        REWARD,
    )];
    assert_eq!(bind(&base, &records)?.served.len(), 1);
    Ok(())
}

#[test]
fn world_reward_claims_a_provisional_chest_has_no_entry_in_a_non_production_bundle() -> TestResult {
    let (base, _) = build(false)?;
    // The compiler left the provisional donor entry out, so (8, 1) holds its ground only.
    let tile = base.tile(8, 1, -7).ok_or("tile")?;
    assert_eq!(tile.ids().len(), 1);
    let records = [claim(
        "oteryn:reward-claim.kv.provisional",
        "ready",
        &[(28827, Some(106), (8, 1, 7))],
        REWARD,
    )];
    let report = bind(&base, &records)?;
    assert!(report.served.is_empty());
    assert_eq!(
        unbound(&report),
        [(
            "oteryn:reward-claim.kv.provisional",
            0,
            UnboundReason::NoEntry
        )]
    );
    let unique: BTreeSet<_> = KEYS
        .iter()
        .filter(|key| key.starts_with("donor:"))
        .collect();
    assert_eq!(unique.len(), 1);
    Ok(())
}

#[test]
fn world_reward_claims_the_embedded_shards_decode() -> TestResult {
    let (base, _) = build(false)?;
    let admitted = |_: &TypedDefinitionRef| true;
    let report = oteryn_game_server::content::world_reward_claims::bind_embedded_reward_claims(
        &base, &admitted,
    )?;
    // The fixture map holds none of the real chests: every ready claim is a candidate that
    // does not bind, and the 12 waiting plain records are left out.
    let candidates = report.unbound_claims_by_reason().values().sum::<usize>();
    assert_eq!(candidates, 219);
    assert!(report.served.is_empty());
    assert_eq!(
        report
            .left_out
            .iter()
            .filter(|(_, why)| *why == LeftOut::NotReady)
            .count(),
        12
    );
    Ok(())
}
