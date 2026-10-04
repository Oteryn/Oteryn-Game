//! World Project to server World Bundle compilation (ADR-0021 §4.2-§4.6).
//!
//! The placements are checked against the Transition.Teleport and House families
//! ([`crate::project`]); [`parity`] reports every teleport and house disagreement at once, and
//! [`equivalence`] proves a compiled bundle tile by tile against its source.

use std::collections::{BTreeMap, BTreeSet};

use crate::Error;
use crate::b3::{self, Region};
use crate::bundle::{
    self, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector, Terrain,
};
use crate::project::{Families, LegacyPosition};
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

    /// The terrain semantics of a resolved key (format v2): `Some` for a route to a Terrain
    /// record, `None` for a WorldObject route or a plain Item. Fails closed when a Terrain
    /// record is not classified well enough to write.
    fn terrain(&self, key: &str) -> Result<Option<Terrain>, Error>;
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
    pub families: &'a Families,
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
    /// Zero-destination teleport attributes dropped (ADR-0021 §4.5), at native positions.
    pub dropped_teleports: Vec<(u16, u16, i8)>,
}

/// How a map `teleport` attribute compares with the Transition.Teleport family (ADR-0021 §4.5,
/// ruling #162 5910173902 Q3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Teleport {
    /// A record from this tile to the same destination.
    Matched,
    /// Destination (0,0,0): not a teleport; the attribute is dropped with a diagnostic.
    ZeroDestination,
    /// A real destination and no record from this tile: compilation fails.
    NoRecord,
    /// A record from this tile to another destination: compilation fails.
    Mismatch,
}

fn teleport(families: &Families, from: LegacyPosition, to: LegacyPosition) -> Teleport {
    // A record from this tile decides first: a (0,0,0) attribute on a Transition tile is a
    // mismatch, never a silent drop.
    match families.teleports.get(&from) {
        Some(record) if *record == to => Teleport::Matched,
        Some(_) => Teleport::Mismatch,
        None if to == (0, 0, 0) => Teleport::ZeroDestination,
        None => Teleport::NoRecord,
    }
}

/// The parity report of a World Project against its families (ADR-0021 §4.5, §4.6): every
/// disagreement at once, where [`compile`] stops at the first failing one. Positions are in
/// the project frame.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Parity {
    pub tiles: usize,
    pub entries: usize,
    pub teleport_attributes: usize,
    pub teleports_matched: usize,
    /// Dropped with a diagnostic; not a release blocker.
    pub zero_destination: Vec<LegacyPosition>,
    /// `(from, to)` with no record from `from`; each fails compilation.
    pub no_record: Vec<(LegacyPosition, LegacyPosition)>,
    /// `(from, to)` whose record goes elsewhere; each fails compilation.
    pub mismatched: Vec<(LegacyPosition, LegacyPosition)>,
    /// Records whose `from` tile carries no teleport attribute; each fails compilation.
    pub records_without_attribute: Vec<LegacyPosition>,
    /// Tiles per house id that the House family does not have; each fails compilation.
    pub unknown_houses: BTreeMap<u32, usize>,
    pub max_tile_top_level_entries: usize,
    pub max_tile_entries: usize,
    /// Longest `text` or `description`, in bytes.
    pub max_text_bytes: usize,
}

/// Decodes every region and compares it with `families` without resolving keys.
pub fn parity(regions: &[Vec<u8>], families: &Families) -> Result<Parity, Error> {
    let (mut report, mut budget) = (Parity::default(), bundle::BUNDLE_BUDGET);
    let mut seen = BTreeSet::new();
    for data in regions {
        let region = b3::decode_region(data, bundle::TILE_LIMITS, &mut budget)?;
        for tile in region.sectors.iter().flat_map(|(_, _, tiles)| tiles) {
            report.tiles += 1;
            report.entries += tile.items.len();
            let top = tile.items.iter().filter(|item| item.depth == 0).count();
            report.max_tile_top_level_entries = report.max_tile_top_level_entries.max(top);
            report.max_tile_entries = report.max_tile_entries.max(tile.items.len());
            for item in &tile.items {
                for text in [&item.attrs.text, &item.attrs.description]
                    .into_iter()
                    .flatten()
                {
                    report.max_text_bytes = report.max_text_bytes.max(text.len());
                }
            }
            if tile.house != 0 && !families.houses.contains(&tile.house) {
                *report.unknown_houses.entry(tile.house).or_default() += 1;
            }
            let from = (tile.x, tile.y, region.z);
            for to in tile.items.iter().filter_map(|item| item.attrs.teleport) {
                report.teleport_attributes += 1;
                seen.insert(from);
                match teleport(families, from, to) {
                    Teleport::Matched => report.teleports_matched += 1,
                    Teleport::ZeroDestination => report.zero_destination.push(from),
                    Teleport::NoRecord => report.no_record.push((from, to)),
                    Teleport::Mismatch => report.mismatched.push((from, to)),
                }
            }
        }
    }
    report.records_without_attribute = families
        .teleports
        .keys()
        .filter(|from| !seen.contains(*from))
        .copied()
        .collect();
    report.zero_destination.sort_unstable();
    report.no_record.sort_unstable();
    report.mismatched.sort_unstable();
    Ok(report)
}

/// The palette indices that the regions place, contents of containers included.
pub fn placed_palette(regions: &[Vec<u8>]) -> Result<BTreeSet<u32>, Error> {
    let mut budget = bundle::BUNDLE_BUDGET;
    let mut placed = BTreeSet::new();
    for data in regions {
        let region = b3::decode_region(data, bundle::TILE_LIMITS, &mut budget)?;
        for tile in region.sectors.iter().flat_map(|(_, _, tiles)| tiles) {
            placed.extend(tile.items.iter().map(|item| item.palette));
        }
    }
    Ok(placed)
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
    dropped: Vec<(u16, u16, i8)>,
    dropped_keys: BTreeSet<u64>,
    teleports_seen: BTreeSet<LegacyPosition>,
}

impl State<'_> {
    /// Resolves every entry of a tile, including the contents of a skipped provisional entry,
    /// so an unknown key or a bad teleport never hides inside a skipped container.
    fn items(&mut self, tile: &Tile, z: u8, floor: i8) -> Result<Vec<Item>, Error> {
        if tile.house != 0 && !self.input.families.houses.contains(&tile.house) {
            let (x, y, house) = (tile.x, tile.y, tile.house);
            return Err(Error::Family(format!(
                "house id {house} at ({x}, {y}, {z}) is not in the House family"
            )));
        }
        let mut kept = Vec::with_capacity(tile.items.len());
        let mut skip_below: Option<u8> = None;
        // Top-level entries kept so far; the last one owns the contents that follow it.
        let mut top_level = 0u8;
        for item in &tile.items {
            if skip_below.is_some_and(|depth| item.depth <= depth) {
                skip_below = None;
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
            let mut item = item.clone();
            if let Some(to) = item.attrs.teleport {
                let from = (tile.x, tile.y, z);
                self.teleports_seen.insert(from);
                match teleport(self.input.families, from, to) {
                    Teleport::Matched => {
                        let (x, y, to_z) = to;
                        item.attrs.teleport = Some((x, y, native_floor(to_z)? as u8));
                    }
                    Teleport::ZeroDestination => {
                        item.attrs.teleport = None;
                        self.dropped.push((tile.x, tile.y, floor));
                        // The whole top-level entry leaves map-item materialization (§4.4);
                        // an entry skipped as provisional has no key.
                        let kept = match item.depth {
                            0 => matches!(self.resolver.resolve(key), Resolution::Resolved(..))
                                .then_some(top_level),
                            _ => top_level.checked_sub(1),
                        };
                        if let Some(ordinal) = kept.filter(|_| skip_below.is_none()) {
                            let placement = bundle::placement_key(floor, tile.x, tile.y, ordinal)
                                .ok_or_else(|| {
                                Error::Limit("tile exceeds 64 top-level entries".into())
                            })?;
                            self.dropped_keys.insert(placement);
                        }
                    }
                    Teleport::NoRecord | Teleport::Mismatch => {
                        return Err(Error::Family(format!(
                            "teleport at {from:?} to {to:?} has no matching Transition record"
                        )));
                    }
                }
            }
            match self.resolver.resolve(key) {
                Resolution::Resolved(family, id) if skip_below.is_none() => {
                    if item.depth == 0 {
                        top_level = top_level.saturating_add(1);
                    }
                    if !self.used.contains_key(&item.palette) {
                        let terrain = self.resolver.terrain(key)?;
                        let entry = PaletteEntry {
                            key: key.clone(),
                            family,
                            id,
                            terrain,
                        };
                        self.used.insert(item.palette, entry);
                    }
                    kept.push(item);
                }
                Resolution::Resolved(..) => {}
                Resolution::Provisional if self.input.build_class != BuildClass::Production => {
                    let (x, y, key) = (tile.x, tile.y, key.clone());
                    self.diagnostics.push(Diagnostic { x, y, floor, key });
                    skip_below = skip_below.or(Some(item.depth));
                }
                Resolution::Provisional => {
                    return Err(Error::Key(format!(
                        "provisional key {key} in a production build"
                    )));
                }
                Resolution::Unknown => return Err(Error::Key(format!("unknown key {key}"))),
            }
        }
        Ok(kept)
    }

    fn region(
        &mut self,
        region: Region,
        out: &mut BTreeMap<(i8, u16, u16), Vec<Tile>>,
    ) -> Result<(), Error> {
        let (z, floor) = (region.z, native_floor(region.z)?);
        for (sx, sy, tiles) in region.sectors {
            let mut mapped = Vec::with_capacity(tiles.len());
            for tile in tiles {
                if !self.input.world.contains(tile.x, tile.y, floor) {
                    let (x, y) = (tile.x, tile.y);
                    return Err(Error::Bounds(format!(
                        "({x}, {y}, {floor}) is outside the World"
                    )));
                }
                let items = self.items(&tile, z, floor)?;
                mapped.push(Tile { items, ..tile });
            }
            if mapped.is_empty() {
                continue;
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
        dropped: Vec::new(),
        dropped_keys: BTreeSet::new(),
        teleports_seen: BTreeSet::new(),
    };
    let (mut sectors, mut budget) = (BTreeMap::new(), bundle::BUNDLE_BUDGET);
    for data in input.regions {
        let region = b3::decode_region(data, bundle::TILE_LIMITS, &mut budget)?;
        state.region(region, &mut sectors)?;
    }
    if let Some(from) = input
        .families
        .teleports
        .keys()
        .find(|from| !state.teleports_seen.contains(*from))
    {
        return Err(Error::Family(format!(
            "Transition record from {from:?} has no teleport attribute on the map"
        )));
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
        compiler_version: bundle::compiler_version(),
        build_class: input.build_class,
        identity: input.identity.clone(),
        world: input.world.clone(),
        palette: state.used.into_values().collect(),
        draft_areas: BTreeSet::from_iter(input.draft_areas.iter().cloned())
            .into_iter()
            .collect(),
        skipped_provisional_keys: skipped.into_iter().collect(),
        dropped_teleports: state.dropped_keys.into_iter().collect(),
    };
    let bytes = bundle::write(&manifest, &sectors)?;
    let mut digest = [0; 32];
    digest.copy_from_slice(&bytes[bytes.len() - 32..]);
    Ok(Compiled {
        bytes,
        digest,
        diagnostics: state.diagnostics,
        dropped_teleports: state.dropped,
    })
}

/// What [`equivalence`] compared.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Equivalence {
    pub tiles: usize,
    pub entries: usize,
    /// Source entries left out as provisional, container contents included.
    pub skipped_entries: usize,
    pub dropped_teleports: usize,
}

/// Proves `bundle` equal to the compiler `input` it claims to come from (ADR-0021 §4.6, §4.8),
/// from the source authorities rather than the compiler state or the bundle's own claims. Every
/// source tile is at its native position with the same flags, house and zones, and every entry
/// keeps its depth, attributes and palette key, except the rules of the format document: a
/// subtree under a key `resolver` calls provisional is left out, a (0,0,0) teleport is dropped,
/// and a teleport floor is native. Every key, a skipped one included, must resolve; every house
/// id must be in the House family; every teleport is checked again against the families (§4.5),
/// and every Transition record must meet its attribute. The bundle holds no other tile, and its
/// whole manifest must be the one derived here: the input identity, World, build class and draft
/// areas; the palette of exactly the kept source indices in ascending order, each with the
/// `(family, id)` `resolver` gives it; the provisional keys met; and the placement keys of the
/// kept top-level entries that lost a (0,0,0) teleport.
pub fn equivalence(
    input: &Input,
    resolver: &dyn KeyResolver,
    bundle: &[u8],
) -> Result<Equivalence, Error> {
    let read = bundle::read(bundle)?;
    let differs = |what: String| Err(Error::Format(format!("equivalence: {what}")));
    let (palette, families) = (input.palette, input.families);
    let mut used = BTreeSet::new();
    let resolved: Vec<Resolution> = palette.iter().map(|key| resolver.resolve(key)).collect();
    let mut provisional = BTreeSet::new();
    let (mut dropped, mut records_met) = (BTreeSet::new(), BTreeSet::new());
    let (mut report, mut budget) = (Equivalence::default(), bundle::BUNDLE_BUDGET);
    let mut seen = BTreeSet::new();
    for data in input.regions {
        let region = b3::decode_region(data, bundle::TILE_LIMITS, &mut budget)?;
        let (z, floor) = (region.z, native_floor(region.z)?);
        for (sx, sy, tiles) in region.sectors {
            let at = (floor, sy, sx);
            if !seen.insert(at) {
                return differs(format!("sector {at:?} given twice"));
            }
            let found = read
                .sectors
                .binary_search_by_key(&at, |s| (s.floor, s.sy, s.sx));
            let compiled = found.map_or(&[][..], |i| &read.sectors[i].tiles[..]);
            if compiled.len() != tiles.len() {
                return differs(format!("sector {at:?} has another tile count"));
            }
            for (source, tile) in tiles.iter().zip(compiled) {
                let mut context = Context {
                    palette,
                    resolved: &resolved,
                    families,
                    z,
                    floor,
                    dropped: &mut dropped,
                    records_met: &mut records_met,
                    used: &mut used,
                    report: &mut report,
                };
                let expected = context.entries(source)?;
                provisional.extend(source.items.iter().filter_map(|item| {
                    let at = item.palette as usize;
                    (resolved.get(at) == Some(&Resolution::Provisional)).then(|| &palette[at])
                }));
                let got: Vec<_> = tile
                    .items
                    .iter()
                    .map(|item| {
                        let key = read.manifest.palette.get(item.palette as usize);
                        (
                            key.map(|entry| entry.key.as_str()),
                            item.depth,
                            item.attrs.clone(),
                        )
                    })
                    .collect();
                let place = |t: &Tile| (t.x, t.y, t.flags, t.house, t.zones.clone());
                if place(source) != place(tile) || got != expected {
                    let (x, y) = (source.x, source.y);
                    return differs(format!("tile ({x}, {y}, {floor}) differs from its source"));
                }
                report.tiles += 1;
                report.entries += got.len();
            }
        }
    }
    let total: usize = read.sectors.iter().map(|s| s.tiles.len()).sum();
    if total != report.tiles {
        return differs(format!(
            "{} bundle tiles have no source",
            total - report.tiles
        ));
    }
    if records_met.len() != families.teleports.len() {
        return differs("a Transition record meets no teleport attribute".into());
    }
    let palette = used
        .into_iter()
        .map(|at| match resolver.resolve(&palette[at as usize]) {
            Resolution::Resolved(family, id) => Ok(PaletteEntry {
                key: palette[at as usize].clone(),
                family,
                id,
                terrain: resolver.terrain(&palette[at as usize])?,
            }),
            _ => Err(Error::Format(
                "equivalence: a kept key does not resolve".into(),
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let expected = Manifest {
        format: bundle::FORMAT.into(),
        min_reader_version: bundle::VERSION,
        projection_class: "server".into(),
        compiler_version: bundle::compiler_version(),
        build_class: input.build_class,
        identity: input.identity.clone(),
        world: input.world.clone(),
        palette,
        draft_areas: BTreeSet::from_iter(input.draft_areas.iter().cloned())
            .into_iter()
            .collect(),
        skipped_provisional_keys: provisional.into_iter().cloned().collect(),
        dropped_teleports: dropped.into_iter().collect(),
    };
    let got = &read.manifest;
    for (field, same) in [
        ("format", expected.format == got.format),
        (
            "min_reader_version",
            expected.min_reader_version == got.min_reader_version,
        ),
        (
            "projection_class",
            expected.projection_class == got.projection_class,
        ),
        (
            "compiler_version",
            expected.compiler_version == got.compiler_version,
        ),
        ("build_class", expected.build_class == got.build_class),
        ("identity", expected.identity == got.identity),
        ("world", expected.world == got.world),
        ("palette", expected.palette == got.palette),
        ("draft_areas", expected.draft_areas == got.draft_areas),
        (
            "skipped_provisional_keys",
            expected.skipped_provisional_keys == got.skipped_provisional_keys,
        ),
        (
            "dropped_teleports",
            expected.dropped_teleports == got.dropped_teleports,
        ),
    ] {
        if !same {
            return differs(format!(
                "manifest {field} is not the one derived from the input"
            ));
        }
    }
    // A field the list above does not name yet is still compared.
    if expected != *got {
        return differs("manifest is not the one derived from the input".into());
    }
    Ok(report)
}

type Entry<'a> = (Option<&'a str>, u8, crate::sector::Attrs);

/// What [`equivalence`] derives one source tile from.
struct Context<'a, 'b> {
    palette: &'a [String],
    resolved: &'b [Resolution],
    families: &'b Families,
    z: u8,
    floor: i8,
    dropped: &'b mut BTreeSet<u64>,
    records_met: &'b mut BTreeSet<LegacyPosition>,
    /// Kept palette indices.
    used: &'b mut BTreeSet<u32>,
    report: &'b mut Equivalence,
}

impl<'a> Context<'a, '_> {
    /// The entries a source tile compiles to under the format rules [`equivalence`] names.
    fn entries(&mut self, tile: &Tile) -> Result<Vec<Entry<'a>>, Error> {
        let differs = |what: String| Error::Format(format!("equivalence: {what}"));
        let from = (tile.x, tile.y, self.z);
        if tile.house != 0 && !self.families.houses.contains(&tile.house) {
            return Err(differs(format!(
                "house {} at {from:?} is not in the House family",
                tile.house
            )));
        }
        let mut items = Vec::with_capacity(tile.items.len());
        let mut skip_below: Option<u8> = None;
        // Kept top-level entries so far; the last one owns the contents that follow it.
        let mut top_level = 0u8;
        for item in &tile.items {
            if skip_below.is_some_and(|depth| item.depth <= depth) {
                skip_below = None;
            }
            // The Transition rule holds for every attribute, a skipped one included (§4.5).
            let record = self.families.teleports.get(&from);
            let teleport = match (item.attrs.teleport, record) {
                (None, _) => None,
                (Some(to), Some(record)) if to == *record => {
                    self.records_met.insert(from);
                    let (x, y, to_z) = to;
                    Some(Some((x, y, native_floor(to_z)? as u8)))
                }
                (Some((0, 0, 0)), None) => Some(None),
                (Some(to), _) => {
                    return Err(differs(format!("teleport at {from:?} to {to:?} disagrees")));
                }
            };
            let key = self.palette.get(item.palette as usize).map(String::as_str);
            let own = self.resolved.get(item.palette as usize);
            // Every entry resolves, the contents of a skipped provisional entry included.
            if matches!(own, None | Some(Resolution::Unknown)) {
                return Err(differs(format!("unresolved palette entry at {from:?}")));
            }
            if skip_below.is_some() || own == Some(&Resolution::Provisional) {
                skip_below = skip_below.or(Some(item.depth));
                self.report.skipped_entries += 1;
                continue;
            }
            if item.depth == 0 {
                top_level = top_level.saturating_add(1);
            }
            let mut attrs = item.attrs.clone();
            if let Some(teleport) = teleport {
                attrs.teleport = teleport;
                if teleport.is_none() {
                    self.report.dropped_teleports += 1;
                    let ordinal = top_level
                        .checked_sub(1)
                        .ok_or_else(|| differs(format!("content without an entry at {from:?}")))?;
                    let placement = bundle::placement_key(self.floor, tile.x, tile.y, ordinal)
                        .ok_or_else(|| differs(format!("no placement key at {from:?}")))?;
                    self.dropped.insert(placement);
                }
            }
            self.used.insert(item.palette);
            items.push((key, item.depth, attrs));
        }
        Ok(items)
    }
}
