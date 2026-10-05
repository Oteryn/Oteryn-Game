//! `OTERYN_WORLD_BUNDLE/v3`: the writer. The byte layout, its limits and the fail-closed reader
//! live in `oteryn-world-bundle` and are re-exported here.

use sha2::{Digest, Sha256};

use crate::Error;
use crate::sector;
use crate::spawn;
pub use oteryn_world_bundle::bundle::*;

const ZSTD_LEVEL: i32 = 3;

/// The `compiler_version` this build writes: the crate version and the zstd library version,
/// because both decide the output bytes (format document §6).
pub fn compiler_version() -> String {
    let zstd = zstd::zstd_safe::version_number();
    format!(
        "oteryn-world-bundle-compiler/{} zstd/{}.{}.{}",
        env!("CARGO_PKG_VERSION"),
        zstd / 10000,
        zstd / 100 % 100,
        zstd % 100
    )
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

/// Writes a bundle. `sectors` must be strictly ascending by `(floor, sy, sx)`.
pub fn write(
    manifest: &Manifest,
    sectors: &[Sector],
    spawns: &spawn::Table,
) -> Result<Vec<u8>, Error> {
    validate_manifest(manifest)?;
    validate_spawns(manifest, spawns)?;
    let json = serde_json::to_vec(manifest).map_err(|e| Error::Format(e.to_string()))?;
    limit(json.len() <= MAX_MANIFEST_BYTES, "manifest too large")?;
    limit(sectors.len() <= MAX_SECTORS, "too many sectors")?;
    check(
        sectors.windows(2).all(|p| order(&p[0]) < order(&p[1])),
        "sectors unsorted",
    )?;
    let mut compressor = zstd::bulk::Compressor::new(ZSTD_LEVEL).map_err(zstd_error)?;
    compressor.include_checksum(true).map_err(zstd_error)?;
    compressor.include_contentsize(true).map_err(zstd_error)?;
    let (mut frames, mut total_raw) = (Vec::with_capacity(sectors.len()), 0usize);
    let mut budget = BUNDLE_BUDGET;
    for sector in sectors {
        validate_sector(manifest, sector)?;
        let raw = sector::encode(&sector.tiles)?;
        // Write only what the reader reads back identically, within the same limits.
        let decoded = sector::decode(&raw, (sector.sx, sector.sy), TILE_LIMITS, &mut budget)?;
        check(decoded == sector.tiles, "sector does not round-trip")?;
        limit(
            raw.len() <= MAX_SECTOR_RAW_BYTES,
            "sector payload too large",
        )?;
        let frame = compressor.compress(&raw).map_err(zstd_error)?;
        limit(
            raw.len() <= frame.len().saturating_mul(MAX_SECTOR_RATIO),
            "sector ratio too high",
        )?;
        total_raw += raw.len();
        frames.push((raw.len(), frame));
    }
    // The spawn frame is written like a sector frame and read back before it is accepted.
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
    total_raw += spawn_raw.len();
    limit(total_raw <= MAX_TOTAL_RAW_BYTES, "bundle payload too large")?;
    for key in &manifest.dropped_teleports {
        check(
            names_entry(sectors, *key),
            "dropped teleport key names no top-level entry",
        )?;
    }
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(&u32_of(json.len()));
    out.extend_from_slice(&u32_of(sectors.len()));
    out.extend_from_slice(&json);
    let mut offset = out.len() + ENTRY * sectors.len() + SPAWN_ROW;
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
    frames
        .iter()
        .for_each(|(_, frame)| out.extend_from_slice(frame));
    out.extend_from_slice(&spawn_frame);
    let digest = digest_of(&out);
    out.extend_from_slice(&digest);
    limit(out.len() <= MAX_FILE_BYTES, "bundle file too large")?;
    Ok(out)
}
