//! Pin start validation against the same native Item solidity the compiler already uses.
use super::*;
use oteryn_world_bundle_compiler::compile::KeyResolver;

pub(super) fn check(bytes: &[u8], at: EntryStart, registry: &Registry) -> Result<(), Error> {
    let mut cells = std::collections::BTreeMap::new();
    bundle::visit(bytes, |manifest: &Manifest, sector| {
        if sector.floor != at.floor {
            return Ok(());
        }
        for tile in &sector.tiles {
            if u32::from(tile.x.abs_diff(at.x)) + u32::from(tile.y.abs_diff(at.y)) > 1 {
                continue;
            }
            let (mut ground, mut solid) = (None, false);
            for item in tile.items.iter().filter(|item| item.depth == 0) {
                let entry = &manifest.palette[item.palette as usize];
                if ground.is_none()
                    && let Some(terrain) = entry
                        .terrain
                        .filter(|terrain| terrain.kind == TerrainKind::Ground)
                {
                    ground = terrain.walkable;
                }
                solid |= entry
                    .terrain
                    .is_some_and(|terrain| terrain.kind == TerrainKind::Wall)
                    || (entry.family == Family::Item
                        && entry.terrain.is_none()
                        && registry.solid(&entry.key));
            }
            cells.insert((tile.x, tile.y), ground == Some(true) && !solid);
        }
        Ok(())
    })?;
    check_cells(at, &cells)
}

fn check_cells(
    at: EntryStart,
    cells: &std::collections::BTreeMap<(u16, u16), bool>,
) -> Result<(), Error> {
    if cells.get(&(at.x, at.y)) != Some(&true) {
        return Err(Error::Format(
            "entry_start: the cell is absent, solid or has no walkable ground".into(),
        ));
    }
    if !cells.iter().any(|(cell, walkable)| {
        u32::from(cell.0.abs_diff(at.x)) + u32::from(cell.1.abs_diff(at.y)) == 1 && *walkable
    }) {
        return Err(Error::Format(
            "entry_start: no steppable orthogonal neighbour".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "entry_start_tests.rs"]
mod tests;
