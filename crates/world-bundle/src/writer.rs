//! `OTERYN_WORLD_BUNDLE/v8`: the shared byte writer used by the compiler and test fixtures.

use sha2::{Digest, Sha256};

use crate::Error;
pub use crate::bundle::*;
use crate::npc;
use crate::sector;
use crate::spawn;

const ZSTD_LEVEL: i32 = 3;

// Count before encoding: these mirror the sector/spawn varint grammars without retaining bytes.
struct Size {
    bytes: usize,
    max: usize,
    what: &'static str,
}

impl Size {
    fn new(max: usize, what: &'static str) -> Self {
        Self {
            bytes: 0,
            max,
            what,
        }
    }

    fn add(&mut self, bytes: usize) -> Result<(), Error> {
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .filter(|size| *size <= self.max)
            .ok_or_else(|| Error::Limit(self.what.into()))?;
        Ok(())
    }

    fn varint(&mut self, value: u64) -> Result<(), Error> {
        self.add(((64 - value.leading_zeros()).max(1) as usize).div_ceil(7))
    }

    fn text(&mut self, value: &str) -> Result<(), Error> {
        self.varint(value.len() as u64)?;
        self.add(value.len())
    }
}

impl std::io::Write for Size {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.add(buf.len()).map_err(std::io::Error::other)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn manifest_size(manifest: &Manifest, layers: &[crate::layers::Layer]) -> Result<usize, Error> {
    #[derive(serde::Serialize)]
    struct Envelope<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        layers: Option<&'a [crate::layers::Layer]>,
        #[serde(flatten)]
        manifest: &'a Manifest,
    }
    let mut size = Size::new(MAX_MANIFEST_BYTES, "manifest too large");
    serde_json::to_writer(
        &mut size,
        &Envelope {
            layers: (!layers.is_empty()).then_some(layers),
            manifest,
        },
    )
    .map_err(|error| {
        if error.is_io() {
            Error::Limit("manifest too large".into())
        } else {
            Error::Format(error.to_string())
        }
    })?;
    Ok(size.bytes)
}

fn sector_size(tiles: &[sector::Tile], max: usize) -> Result<usize, Error> {
    let mut size = Size::new(max, "sector payload too large");
    size.varint(tiles.len() as u64)?;
    let mut previous = -1i64;
    for tile in tiles {
        let index = i64::from(tile.y % 32) * 32 + i64::from(tile.x % 32);
        check(index > previous, "sector tiles must be strictly ascending")?;
        size.varint((index - previous - 1) as u64)?;
        previous = index;
        size.varint(
            (tile.items.len() as u64) << 3
                | u64::from(!tile.zones.is_empty()) << 2
                | u64::from(tile.house != 0) << 1
                | u64::from(tile.flags != 0),
        )?;
        for value in [tile.flags, tile.house].into_iter().filter(|v| *v != 0) {
            size.varint(value.into())?;
        }
        if !tile.zones.is_empty() {
            size.varint(tile.zones.len() as u64)?;
            for zone in &tile.zones {
                size.varint((*zone).into())?;
            }
        }
        for item in &tile.items {
            let a = &item.attrs;
            let mask = [
                item.depth != 0,
                a.count.is_some(),
                a.action.is_some(),
                a.unique.is_some(),
                a.door.is_some(),
                a.text.is_some(),
                a.charges.is_some(),
                a.description.is_some(),
                a.teleport.is_some(),
                a.depot.is_some(),
            ]
            .into_iter()
            .enumerate()
            .fold(0u64, |mask, (bit, on)| mask | (u64::from(on) << bit));
            size.varint(u64::from(item.palette) << 1 | u64::from(mask != 0))?;
            if mask != 0 {
                size.varint(mask)?;
            }
            size.add(
                usize::from(item.depth != 0)
                    + usize::from(a.count.is_some())
                    + usize::from(a.door.is_some()),
            )?;
            for value in [a.action, a.unique, a.charges, a.depot]
                .into_iter()
                .flatten()
            {
                size.varint(value.into())?;
            }
            for text in [&a.text, &a.description].into_iter().flatten() {
                size.text(text)?;
            }
            if let Some((x, y, _)) = a.teleport {
                size.varint(x.into())?;
                size.varint(y.into())?;
                size.add(1)?;
            }
        }
    }
    Ok(size.bytes)
}

fn spawn_size(table: &spawn::Table) -> Result<usize, Error> {
    let mut size = Size::new(spawn::MAX_RAW_BYTES, "spawn payload too large");
    size.varint(table.creatures.len() as u64)?;
    for creature in &table.creatures {
        size.text(&creature.key)?;
        size.add(1)?;
    }
    size.varint(table.sources.len() as u64)?;
    for source in &table.sources {
        size.text(&source.key)?;
        size.add(1)?;
        size.varint(source.x.into())?;
        size.varint(source.y.into())?;
        size.varint(source.points.len() as u64)?;
        for point in &source.points {
            size.varint(point.creature.into())?;
            for (at, centre) in [(point.x, source.x), (point.y, source.y)] {
                let delta = i32::from(at) - i32::from(centre);
                size.varint(((delta << 1) ^ (delta >> 31)) as u32 as u64)?;
            }
            size.add(1)?;
            size.varint(point.respawn_ms.into())?;
            let policy = match point.boss_policy {
                Some(spawn::BossPolicy::Respawn { min_s, max_s }) => Some((min_s, max_s)),
                Some(spawn::BossPolicy::AtReset { chance }) => {
                    Some((chance.numerator, chance.denominator))
                }
                None => None,
            };
            if let Some((a, b)) = policy {
                size.add(1)?;
                size.varint(a)?;
                size.varint(b)?;
            }
        }
    }
    Ok(size.bytes)
}

fn limit(ok: bool, what: &str) -> Result<(), Error> {
    if ok {
        Ok(())
    } else {
        Err(Error::Limit(what.into()))
    }
}

fn check(ok: bool, what: &str) -> Result<(), Error> {
    if ok {
        Ok(())
    } else {
        Err(Error::Format(what.into()))
    }
}

fn u32_of(value: usize) -> [u8; 4] {
    (value as u32).to_le_bytes()
}

/// Writes a bundle without NPCs: [`write_with_npcs`] with the empty NPC table.
pub fn write(
    manifest: &Manifest,
    sectors: &[Sector],
    spawns: &spawn::Table,
) -> Result<Vec<u8>, Error> {
    write_with_npcs(manifest, sectors, spawns, &npc::Table::default())
}

/// Writes a bundle. `sectors` must be strictly ascending by `(floor, sy, sx)`.
pub fn write_with_npcs(
    manifest: &Manifest,
    sectors: &[Sector],
    spawns: &spawn::Table,
    npcs: &npc::Table,
) -> Result<Vec<u8>, Error> {
    write_with_layers(manifest, sectors, spawns, npcs, &[])
}

/// Write the same manifest envelope with compiler-admitted optional epoch layers.
pub fn write_with_layers(
    manifest: &Manifest,
    sectors: &[Sector],
    spawns: &spawn::Table,
    npcs: &npc::Table,
    layers: &[crate::layers::Layer],
) -> Result<Vec<u8>, Error> {
    write_with_caps(manifest, sectors, spawns, npcs, layers, READ_CAPS)
}

fn write_with_caps(
    manifest: &Manifest,
    sectors: &[Sector],
    spawns: &spawn::Table,
    npcs: &npc::Table,
    layers: &[crate::layers::Layer],
    caps: ReadCaps,
) -> Result<Vec<u8>, Error> {
    validate_manifest(manifest)?;
    limit(sectors.len() <= caps.sectors, "too many sectors")?;
    let json_length = manifest_size(manifest, layers)?;
    let mut file_size = Size::new(caps.file_bytes, "bundle file too large");
    file_size.add(HEADER + json_length + ENTRY * sectors.len() + SPAWN_ROW + NPC_ROW + DIGEST)?;
    crate::layers::validate(manifest, layers)?;
    validate_spawns(manifest, spawns)?;
    validate_npcs(manifest, npcs)?;
    let json = crate::manifest::encode(manifest, layers)?;
    limit(json.len() <= MAX_MANIFEST_BYTES, "manifest too large")?;
    check(
        sectors.windows(2).all(|p| order(&p[0]) < order(&p[1])),
        "sectors unsorted",
    )?;
    let mut compressor = zstd::bulk::Compressor::new(ZSTD_LEVEL).map_err(zstd_error)?;
    compressor.include_checksum(true).map_err(zstd_error)?;
    compressor.include_contentsize(true).map_err(zstd_error)?;
    let mut total_raw = Size::new(caps.total_raw_bytes, "bundle payload too large");
    let mut frames = Vec::new();
    let mut budget = BUNDLE_BUDGET;
    for sector in sectors {
        validate_sector(manifest, sector)?;
        let raw_length = sector_size(&sector.tiles, caps.sector_raw_bytes)?;
        total_raw.add(raw_length)?;
        let raw = sector::encode(&sector.tiles)?;
        // Write only what the reader reads back identically, within the same limits.
        let decoded = sector::decode(&raw, (sector.sx, sector.sy), TILE_LIMITS, &mut budget)?;
        check(decoded == sector.tiles, "sector does not round-trip")?;
        limit(
            raw.len() <= caps.sector_raw_bytes,
            "sector payload too large",
        )?;
        let frame = compressor.compress(&raw).map_err(zstd_error)?;
        limit(
            raw.len() <= frame.len().saturating_mul(MAX_SECTOR_RATIO),
            "sector ratio too high",
        )?;
        file_size.add(frame.len())?;
        frames.push((raw.len(), frame));
    }
    // The spawn frame is written like a sector frame and read back before it is accepted.
    total_raw.add(spawn_size(spawns)?)?;
    let spawn_raw = spawn::encode(spawns);
    limit(
        spawn_raw.len() <= spawn::MAX_RAW_BYTES,
        "spawn payload too large",
    )?;
    check(
        spawn::decode(&spawn_raw, &manifest.world)? == *spawns,
        "spawn family does not round-trip",
    )?;
    let spawn_frame = compressor.compress(&spawn_raw).map_err(zstd_error)?;
    limit(
        spawn_raw.len() <= spawn_frame.len().saturating_mul(MAX_SECTOR_RATIO),
        "spawn ratio too high",
    )?;
    file_size.add(spawn_frame.len())?;
    // The NPC frame likewise (decision NPC-PLACE-1 §3), the last frame.
    // Validation bounds NPC encoding below 1 MiB: at most 2,048 keys of 128 bytes
    // and 2,048 placements of at most eight bytes, plus counts.
    let mut npc_size = Size::new(npc::MAX_RAW_BYTES, "NPC payload too large");
    npc_size.varint(npcs.npcs.len() as u64)?;
    for npc in &npcs.npcs {
        npc_size.text(&npc.key)?;
        npc_size.varint(npc.placements.len() as u64)?;
        for placement in &npc.placements {
            npc_size.add(2)?;
            npc_size.varint(placement.x.into())?;
            npc_size.varint(placement.y.into())?;
        }
    }
    total_raw.add(npc_size.bytes)?;
    let npc_raw = npc::encode(npcs);
    limit(npc_raw.len() <= npc::MAX_RAW_BYTES, "NPC payload too large")?;
    check(
        npc::decode(&npc_raw, &manifest.world)? == *npcs,
        "NPC family does not round-trip",
    )?;
    let npc_frame = compressor.compress(&npc_raw).map_err(zstd_error)?;
    limit(
        npc_raw.len() <= npc_frame.len().saturating_mul(MAX_SECTOR_RATIO),
        "NPC ratio too high",
    )?;
    file_size.add(npc_frame.len())?;
    for key in &manifest.dropped_teleports {
        check(
            names_entry(sectors, *key),
            "dropped teleport key names no top-level entry",
        )?;
    }
    let mut out = Vec::with_capacity(file_size.bytes);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(&u32_of(json.len()));
    out.extend_from_slice(&u32_of(sectors.len()));
    out.extend_from_slice(&json);
    let mut offset = out.len() + ENTRY * sectors.len() + SPAWN_ROW + NPC_ROW;
    for (sector, (raw_length, frame)) in sectors.iter().zip(&frames) {
        out.extend_from_slice(&[sector.floor as u8, 0]);
        out.extend_from_slice(&sector.sx.to_le_bytes());
        out.extend_from_slice(&sector.sy.to_le_bytes());
        out.extend_from_slice(&u32_of(offset));
        out.extend_from_slice(&u32_of(frame.len()));
        out.extend_from_slice(&u32_of(*raw_length));
        out.extend_from_slice(&Sha256::digest(frame));
        offset += frame.len();
    }
    out.extend_from_slice(&u32_of(offset));
    out.extend_from_slice(&u32_of(spawn_frame.len()));
    out.extend_from_slice(&u32_of(spawn_raw.len()));
    out.extend_from_slice(&Sha256::digest(&spawn_frame));
    out.extend_from_slice(&u32_of(offset + spawn_frame.len()));
    out.extend_from_slice(&u32_of(npc_frame.len()));
    out.extend_from_slice(&u32_of(npc_raw.len()));
    out.extend_from_slice(&Sha256::digest(&npc_frame));
    frames
        .iter()
        .for_each(|(_, frame)| out.extend_from_slice(frame));
    out.extend_from_slice(&spawn_frame);
    out.extend_from_slice(&npc_frame);
    let digest = digest_of(&out);
    out.extend_from_slice(&digest);
    limit(out.len() <= caps.file_bytes, "bundle file too large")?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        Manifest {
            subareas: Default::default(),
            format: FORMAT.into(),
            min_reader_version: VERSION,
            projection_class: "server".into(),
            compiler_version: "test".into(),
            build_class: BuildClass::Production,
            identity: Identity::default(),
            world: Extent {
                min_x: 0,
                min_y: 0,
                max_x: 64,
                max_y: 32,
                floors: vec![-7],
            },
            palette: vec![],
            draft_areas: vec![],
            skipped_provisional_keys: vec![],
            dropped_teleports: vec![],
            spawns: Default::default(),
            npcs: Default::default(),
        }
    }

    fn sector() -> Sector {
        Sector {
            floor: -7,
            sx: 0,
            sy: 0,
            tiles: vec![sector::Tile {
                x: 0,
                y: 0,
                flags: 0,
                house: 0,
                zones: vec![],
                items: vec![],
            }],
        }
    }

    #[test]
    fn writer_checks_raw_budget_before_visiting_next_sector() {
        let first = sector();
        let second = Sector {
            sx: 1,
            tiles: vec![],
            ..sector()
        };
        let caps = ReadCaps {
            total_raw_bytes: 0,
            ..READ_CAPS
        };
        assert_eq!(
            write_with_caps(
                &manifest(),
                &[first, second],
                &spawn::Table::default(),
                &npc::Table::default(),
                &[],
                caps
            ),
            Err(Error::Limit("bundle payload too large".into()))
        );
    }

    #[test]
    fn writer_checks_sector_size_before_round_trip() {
        let mut large = sector();
        large.tiles[0].zones = vec![0; MAX_SECTOR_RAW_BYTES];
        assert_eq!(
            write(&manifest(), &[large], &spawn::Table::default()),
            Err(Error::Limit("sector payload too large".into()))
        );
    }

    #[test]
    fn writer_checks_file_budget_before_visiting_sectors() {
        let caps = ReadCaps {
            file_bytes: 0,
            ..READ_CAPS
        };
        let bad = Sector {
            tiles: vec![],
            ..sector()
        };
        assert_eq!(
            write_with_caps(
                &manifest(),
                &[bad],
                &spawn::Table::default(),
                &npc::Table::default(),
                &[],
                caps
            ),
            Err(Error::Limit("bundle file too large".into()))
        );
    }
    #[test]
    fn writer_accepts_exact_raw_and_file_budgets() -> Result<(), Error> {
        let mut manifest = manifest();
        manifest.identity.min_runtime_version = "3".into();
        manifest.palette.push(PaletteEntry {
            key: "item".into(),
            family: Family::Item,
            id: 1,
            terrain: None,
        });
        let mut sector = sector();
        let tile = &mut sector.tiles[0];
        tile.flags = u32::MAX;
        tile.house = 128;
        tile.zones = vec![1, 128, u16::MAX];
        tile.items = vec![
            sector::Item {
                palette: 0,
                depth: 0,
                attrs: Default::default(),
            },
            sector::Item {
                palette: 0,
                depth: 1,
                attrs: sector::Attrs {
                    count: Some(0),
                    action: Some(128),
                    unique: Some(u16::MAX),
                    door: Some(0),
                    text: Some("\\\"\n".into()),
                    charges: Some(127),
                    description: Some("text".into()),
                    teleport: None,
                    depot: Some(128),
                },
            },
        ];
        let sectors = [sector];
        let spawns = spawn::Table {
            creatures: vec![spawn::Creature {
                key: "rat".into(),
                period: spawn::Period::Night,
            }],
            sources: vec![spawn::Source {
                key: "source".into(),
                floor: -7,
                x: u16::MAX,
                y: 0,
                points: [
                    None,
                    Some(spawn::BossPolicy::Respawn {
                        min_s: 128,
                        max_s: 16_384,
                    }),
                    Some(spawn::BossPolicy::AtReset {
                        chance: spawn::BossChance {
                            numerator: 1,
                            denominator: i64::MAX as u64,
                        },
                    }),
                ]
                .into_iter()
                .map(|boss_policy| spawn::Point {
                    creature: 0,
                    x: 0,
                    y: 31,
                    direction: spawn::Direction::West,
                    respawn_ms: if boss_policy.is_some() { 0 } else { 1_000 },
                    boss_policy,
                })
                .collect(),
            }],
        };
        manifest.spawns = SpawnCounts {
            sources: 1,
            points: 3,
        };
        let npcs = npc::Table {
            npcs: vec![npc::Npc {
                key: "oteryn:npc.test".into(),
                placements: vec![npc::Placement {
                    floor: -7,
                    x: 1,
                    y: 1,
                    direction: npc::Direction::South,
                }],
            }],
        };
        manifest.npcs.npcs = 1;
        manifest.npcs.placements = 1;
        let bytes = write_with_npcs(&manifest, &sectors, &spawns, &npcs)?;
        let raw_bytes = sector::encode(&sectors[0].tiles)?.len()
            + spawn::encode(&spawns).len()
            + npc::encode(&npcs).len();
        let caps = ReadCaps {
            total_raw_bytes: raw_bytes,
            file_bytes: bytes.len(),
            ..READ_CAPS
        };
        assert_eq!(
            write_with_caps(&manifest, &sectors, &spawns, &npcs, &[], caps)?,
            bytes
        );
        assert_eq!(read(&bytes)?.sectors, sectors);
        for caps in [
            ReadCaps {
                total_raw_bytes: raw_bytes - 1,
                ..caps
            },
            ReadCaps {
                file_bytes: bytes.len() - 1,
                ..caps
            },
        ] {
            assert!(matches!(
                write_with_caps(&manifest, &sectors, &spawns, &npcs, &[], caps),
                Err(Error::Limit(_))
            ));
        }
        Ok(())
    }

    #[test]
    fn writer_rejects_oversized_manifest_and_spawn_table() {
        let mut manifest = manifest();
        // Escaped JSON can exceed the cap even when the input string does not.
        manifest.compiler_version = "\n".repeat(MAX_MANIFEST_BYTES / 2);
        assert_eq!(
            write(&manifest, &[], &spawn::Table::default()),
            Err(Error::Limit("manifest too large".into()))
        );
        manifest.compiler_version = "test".into();
        let spawns = spawn::Table {
            creatures: (0..spawn::MAX_POINTS)
                .map(|index| spawn::Creature {
                    key: format!("{index:0128}"),
                    period: spawn::Period::All,
                })
                .collect(),
            sources: vec![],
        };
        assert_eq!(
            write(&manifest, &[], &spawns),
            Err(Error::Limit("spawn payload too large".into()))
        );
    }
}
