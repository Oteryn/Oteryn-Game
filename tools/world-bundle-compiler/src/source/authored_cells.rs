//! First-party ground additions use the normal B3 compiler and never replace captured tiles.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::b3::{self, Region};
use crate::sector::{self, Attrs, Item, Tile};
use crate::{Error, bundle, project};

const SCHEMA: &str = "OTERYN_AUTHORED_GROUND_CELLS/v1";
const MAX_CELLS: usize = 4096;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: String,
    coordinate_frame: String,
    provenance: Value,
    cells: Vec<Cell>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cell {
    x: u16,
    y: u16,
    z: u8,
    ground: String,
}

fn invalid(message: &str) -> Error {
    Error::Format(format!("authored ground: {message}"))
}

pub(super) fn apply(
    root: &Path,
    selected: Option<&[(u16, u16)]>,
    index: &Value,
    regions: &mut Vec<Vec<u8>>,
) -> Result<(), Error> {
    let path = root.join("content/npcs/authored-ground-cells.json");
    if !path.exists() {
        return Ok(());
    }
    let document: Document = serde_json::from_slice(&super::read(&path)?)
        .map_err(|error| invalid(&error.to_string()))?;
    if document.schema != SCHEMA
        || document.coordinate_frame != project::FRAME
        || !document
            .provenance
            .as_object()
            .is_some_and(|p| !p.is_empty())
    {
        return Err(invalid("schema, frame or provenance"));
    }
    // Reuse the same authoritative resolver used by the complete compilation.
    let probe = super::Project {
        index: index.clone(),
        regions: Vec::new(),
        families: project::Families::default(),
    };
    let (registry, _) = super::registry(root, &probe)?;
    for cell in &document.cells {
        if registry
            .terrain_of(&cell.ground)?
            .is_none_or(|terrain| terrain.kind != bundle::TerrainKind::Ground)
        {
            return Err(invalid(
                "ground key does not resolve to classified ground terrain",
            ));
        }
    }
    apply_cells(document.cells, selected, index, regions)
}

fn apply_cells(
    cells: Vec<Cell>,
    selected: Option<&[(u16, u16)]>,
    index: &Value,
    regions: &mut Vec<Vec<u8>>,
) -> Result<(), Error> {
    if cells.is_empty() || cells.len() > MAX_CELLS {
        return Err(invalid("cell count outside 1..=4096"));
    }
    let palette = index["palette"]
        .as_array()
        .ok_or_else(|| invalid("missing palette"))?;
    let mut grouped = BTreeMap::<(u8, u16, u16), Vec<(Cell, u32)>>::new();
    let mut seen = BTreeSet::new();
    for cell in cells {
        if cell.z > 15 || !seen.insert((cell.x, cell.y, cell.z)) {
            return Err(invalid("invalid floor or duplicate cell"));
        }
        let mut matches = palette.iter().enumerate().filter(|(_, entry)| {
            entry["key"].as_str() == Some(cell.ground.as_str())
                && entry["provisional"] == Value::Bool(false)
        });
        let (position, _) = matches
            .next()
            .ok_or_else(|| invalid("unadmitted ground key"))?;
        if matches.next().is_some() {
            return Err(invalid("ambiguous ground key"));
        }
        let position = u32::try_from(position).map_err(|_| invalid("palette overflow"))?;
        let (rx, ry) = (cell.x / 256, cell.y / 256);
        if selected.is_none_or(|selection| selection.contains(&(rx, ry))) {
            grouped
                .entry((cell.z, rx, ry))
                .or_default()
                .push((cell, position));
        }
    }
    let mut budget = bundle::BUNDLE_BUDGET;
    for ((z, rx, ry), cells) in grouped {
        let mut matches = regions.iter().enumerate().filter(|(_, bytes)| {
            bytes.len() >= 12
                && &bytes[..4] == b"OTRB"
                && bytes[5] == z
                && u16::from_le_bytes([bytes[6], bytes[7]]) == rx
                && u16::from_le_bytes([bytes[8], bytes[9]]) == ry
        });
        let existing = matches.next().map(|(index, _)| index);
        if matches.next().is_some() {
            return Err(invalid("duplicate source region"));
        }
        let region = match existing {
            Some(index) => b3::decode_region(&regions[index], bundle::TILE_LIMITS, &mut budget)?,
            None => Region {
                z,
                rx,
                ry,
                sectors: Vec::new(),
            },
        };
        let mut sectors = region
            .sectors
            .into_iter()
            .map(|(sx, sy, tiles)| ((sy % 8) * 8 + sx % 8, (sx, sy, tiles)))
            .collect::<BTreeMap<_, _>>();
        for (cell, palette) in cells {
            let (sx, sy) = (cell.x / 32, cell.y / 32);
            let (_, _, tiles) = sectors
                .entry((sy % 8) * 8 + sx % 8)
                .or_insert_with(|| (sx, sy, Vec::new()));
            if tiles
                .iter()
                .any(|tile| (tile.x, tile.y) == (cell.x, cell.y))
            {
                return Err(invalid("captured cell cannot be overwritten"));
            }
            tiles.push(Tile {
                x: cell.x,
                y: cell.y,
                flags: 0,
                house: 0,
                zones: Vec::new(),
                items: vec![Item {
                    palette,
                    depth: 0,
                    attrs: Attrs::default(),
                }],
            });
        }
        let mut sectors = sectors.into_values().collect::<Vec<_>>();
        for (_, _, tiles) in &mut sectors {
            tiles.sort_by_key(|tile| (tile.y, tile.x));
        }
        let encoded = encode(&Region { z, rx, ry, sectors })?;
        if let Some(index) = existing {
            regions[index] = encoded;
        } else {
            regions.push(encoded);
        }
    }
    Ok(())
}

/// Encode only in memory; the sector grammar is the existing shared checked codec.
fn encode(region: &Region) -> Result<Vec<u8>, Error> {
    let count = u16::try_from(region.sectors.len()).map_err(|_| invalid("sector count"))?;
    let mut frames = Vec::new();
    for (sx, sy, tiles) in &region.sectors {
        let payload = sector::encode(tiles)?;
        if payload.len() > b3::MAX_SECTOR_BYTES {
            return Err(invalid("sector size"));
        }
        let compressed =
            zstd::bulk::compress(&payload, 3).map_err(|error| invalid(&error.to_string()))?;
        frames.push((
            u8::try_from((sy % 8) * 8 + sx % 8).map_err(|_| invalid("sector position"))?,
            compressed,
        ));
    }
    let mut output = b"OTRB".to_vec();
    output.extend_from_slice(&[1, region.z]);
    output.extend_from_slice(&region.rx.to_le_bytes());
    output.extend_from_slice(&region.ry.to_le_bytes());
    output.extend_from_slice(&count.to_le_bytes());
    let mut offset = 12 + 9 * frames.len();
    for (local, frame) in &frames {
        output.push(*local);
        output.extend_from_slice(
            &u32::try_from(offset)
                .map_err(|_| invalid("region size"))?
                .to_le_bytes(),
        );
        output.extend_from_slice(
            &u32::try_from(frame.len())
                .map_err(|_| invalid("frame size"))?
                .to_le_bytes(),
        );
        offset = offset
            .checked_add(frame.len())
            .ok_or_else(|| invalid("region size"))?;
    }
    for (_, frame) in frames {
        output.extend_from_slice(&frame);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(x: u16, y: u16, z: u8) -> Cell {
        Cell {
            x,
            y,
            z,
            ground: "ground".into(),
        }
    }
    fn index() -> Value {
        serde_json::json!({"palette":[{"key":"ground","provisional":false}]})
    }
    fn original() -> Tile {
        Tile {
            x: 2,
            y: 3,
            flags: 17,
            house: 9,
            zones: vec![4],
            items: vec![Item {
                palette: 0,
                depth: 0,
                attrs: Attrs {
                    count: Some(0),
                    action: Some(123),
                    ..Attrs::default()
                },
            }],
        }
    }
    fn regions() -> Result<Vec<Vec<u8>>, Error> {
        Ok(vec![encode(&Region {
            z: 7,
            rx: 0,
            ry: 0,
            sectors: vec![(0, 0, vec![original()])],
        })?])
    }
    #[test]
    fn additions_preserve_source_flags_house_zones_and_exact_attributes() -> Result<(), Error> {
        let mut data = regions()?;
        apply_cells(
            vec![cell(5, 6, 7), cell(33, 40, 7)],
            None,
            &index(),
            &mut data,
        )?;
        let mut budget = bundle::BUNDLE_BUDGET;
        let decoded = b3::decode_region(&data[0], bundle::TILE_LIMITS, &mut budget)?;
        assert_eq!(decoded.sectors[0].2[0], original());
        assert_eq!(decoded.sectors.len(), 2);
        assert_eq!(decoded.sectors[0].2[1].items[0].palette, 0);
        Ok(())
    }
    #[test]
    fn an_authored_floor_cannot_replace_even_an_identical_captured_cell() -> Result<(), Error> {
        let mut data = regions()?;
        assert!(apply_cells(vec![cell(2, 3, 7)], None, &index(), &mut data).is_err());
        assert_eq!(data, regions()?);
        Ok(())
    }
    #[test]
    fn another_floor_and_region_selection_keep_existing_regions_intact() -> Result<(), Error> {
        let mut data = regions()?;
        let before = data.clone();
        apply_cells(vec![cell(5, 6, 11)], Some(&[(1, 1)]), &index(), &mut data)?;
        assert_eq!(data, before);
        apply_cells(vec![cell(5, 6, 11)], Some(&[(0, 0)]), &index(), &mut data)?;
        assert_eq!(data[0], before[0]);
        assert_eq!(data.len(), 2);
        Ok(())
    }
    #[test]
    fn duplicate_invalid_and_unadmitted_cells_fail_closed() {
        for cells in [vec![cell(1, 1, 7), cell(1, 1, 7)], vec![cell(1, 1, 16)]] {
            assert!(apply_cells(cells, None, &index(), &mut Vec::new()).is_err());
        }
        assert!(
            apply_cells(
                vec![cell(1, 1, 7)],
                None,
                &serde_json::json!({"palette":[]}),
                &mut Vec::new()
            )
            .is_err()
        );
    }
}
