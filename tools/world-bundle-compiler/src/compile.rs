//! World Project to server World Bundle compilation (ADR-0021 §4.2-§4.6), skeleton scope.
//!
//! Real-map key resolution against the Item registry and Terrain family, the World,
//! FloorChange, Transition and House families and draft-area marking are MAP-BUNDLE-1b.

use std::collections::{BTreeMap, BTreeSet};

use crate::Error;
use crate::b3::{self, Region};
use crate::bundle::{self, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector};
use crate::sector::{Item, Tile};

/// What a palette key of the World Project resolves to in the pinned content revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution {
    Resolved(Family, u32),
    /// One of the provisional donor keys (ADR-0021 §4.5).
    Provisional,
    Unknown,
}

pub trait KeyResolver {
    fn resolve(&self, key: &str) -> Resolution;
}

pub struct Input<'a> {
    /// B3 region files, in any order.
    pub regions: &'a [Vec<u8>],
    /// The `palette` list of the placements `index.json`.
    pub palette: &'a [String],
    pub identity: Identity,
    pub world: Extent,
    pub build_class: BuildClass,
    pub draft_areas: Vec<String>,
}

/// A provisional entry skipped in a non-production build, at its native position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub x: u16,
    pub y: u16,
    pub floor: i8,
    pub key: String,
}

pub struct Compiled {
    pub bytes: Vec<u8>,
    pub digest: [u8; 32],
    pub diagnostics: Vec<Diagnostic>,
}

/// The CrystalServer import profile: `native.floor = -legacy.z`, checked.
fn native_floor(z: u8) -> Result<i8, Error> {
    i8::try_from(z)
        .ok()
        .filter(|z| *z <= 15)
        .map(|z| -z)
        .ok_or(Error::Bounds(format!("z {z}")))
}

struct State<'a> {
    input: &'a Input<'a>,
    resolver: &'a dyn KeyResolver,
    used: BTreeMap<u32, PaletteEntry>,
    diagnostics: Vec<Diagnostic>,
}

impl State<'_> {
    fn items(&mut self, tile: &Tile, floor: i8) -> Result<Vec<Item>, Error> {
        let mut kept = Vec::with_capacity(tile.items.len());
        let mut skip_below: Option<u8> = None;
        for item in &tile.items {
            match skip_below {
                Some(depth) if item.depth > depth => continue,
                _ => skip_below = None,
            }
            let key = self
                .input
                .palette
                .get(item.palette as usize)
                .ok_or_else(|| {
                    Error::Key(format!(
                        "palette index {} is not in the index",
                        item.palette
                    ))
                })?;
            match self.resolver.resolve(key) {
                Resolution::Resolved(family, id) => {
                    let entry = PaletteEntry {
                        key: key.clone(),
                        family,
                        id,
                    };
                    self.used.entry(item.palette).or_insert(entry);
                }
                Resolution::Provisional if self.input.build_class != BuildClass::Production => {
                    let (x, y, key) = (tile.x, tile.y, key.clone());
                    self.diagnostics.push(Diagnostic { x, y, floor, key });
                    skip_below = Some(item.depth);
                    continue;
                }
                Resolution::Provisional => {
                    return Err(Error::Key(format!(
                        "provisional key {key} in a production build"
                    )));
                }
                Resolution::Unknown => return Err(Error::Key(format!("unknown key {key}"))),
            }
            let mut item = item.clone();
            if let Some((x, y, z)) = item.attrs.teleport {
                item.attrs.teleport = Some((x, y, native_floor(z)? as u8));
            }
            kept.push(item);
        }
        Ok(kept)
    }

    fn region(
        &mut self,
        region: Region,
        out: &mut BTreeMap<(i8, u16, u16), Vec<Tile>>,
    ) -> Result<(), Error> {
        let floor = native_floor(region.z)?;
        for (sx, sy, tiles) in region.sectors {
            let mut mapped = Vec::with_capacity(tiles.len());
            for tile in tiles {
                if !self.input.world.contains(tile.x, tile.y, floor) {
                    let (x, y) = (tile.x, tile.y);
                    return Err(Error::Bounds(format!(
                        "({x}, {y}, {floor}) is outside the World"
                    )));
                }
                let items = self.items(&tile, floor)?;
                mapped.push(Tile { items, ..tile });
            }
            if out.insert((floor, sy, sx), mapped).is_some() {
                return Err(Error::Format(format!(
                    "sector ({sx}, {sy}, {floor}) given twice"
                )));
            }
        }
        Ok(())
    }
}

/// Compiles B3 regions into one server World Bundle, deterministically and failing closed.
pub fn compile(input: &Input<'_>, resolver: &dyn KeyResolver) -> Result<Compiled, Error> {
    let mut state = State {
        input,
        resolver,
        used: BTreeMap::new(),
        diagnostics: Vec::new(),
    };
    let mut sectors = BTreeMap::new();
    for data in input.regions {
        state.region(b3::decode_region(data, bundle::TILE_LIMITS)?, &mut sectors)?;
    }
    let remap: BTreeMap<u32, u32> = state
        .used
        .keys()
        .zip(0u32..)
        .map(|(k, i)| (*k, i))
        .collect();
    let sectors: Vec<Sector> = sectors
        .into_iter()
        .map(|((floor, sy, sx), mut tiles)| {
            for item in tiles.iter_mut().flat_map(|tile| tile.items.iter_mut()) {
                item.palette = remap.get(&item.palette).copied().unwrap_or(u32::MAX);
            }
            Sector {
                floor,
                sx,
                sy,
                tiles,
            }
        })
        .collect();
    let skipped: BTreeSet<String> = state.diagnostics.iter().map(|d| d.key.clone()).collect();
    let manifest = Manifest {
        format: bundle::FORMAT.into(),
        min_reader_version: bundle::VERSION,
        projection_class: "server".into(),
        build_class: input.build_class,
        identity: input.identity.clone(),
        world: input.world.clone(),
        palette: state.used.into_values().collect(),
        draft_areas: input.draft_areas.clone(),
        skipped_provisional_keys: skipped.into_iter().collect(),
    };
    let bytes = bundle::write(&manifest, &sectors)?;
    let mut digest = [0; 32];
    digest.copy_from_slice(&bytes[bytes.len() - 32..]);
    Ok(Compiled {
        bytes,
        digest,
        diagnostics: state.diagnostics,
    })
}
