//! Source-declared stock occupies a reserved identity, never a loose map entry.
use super::{PaletteGround, overlay::TilePos};
use oteryn_world_bundle::sector::Tile;
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy)]
pub struct ContainerStock {
    pub position: TilePos,
    pub parent_ordinal: u8,
    pub stock_ordinal: u8,
    pub item: u16,
    pub count: u32,
    pub key_number: Option<u16>,
    pub capacity: u8,
}
#[derive(Deserialize)]
struct Capture {
    schema: u8,
    bindings: Vec<Binding>,
}
#[derive(Deserialize)]
struct Binding {
    position: [u16; 3],
    container: u16,
    ordinal: u8,
    stock_ordinal: u8,
    item: u16,
    count: u32,
    key_number: Option<u16>,
    capacity: u8,
}
fn bindings() -> &'static [Binding] {
    static CAPTURE: OnceLock<Vec<Binding>> = OnceLock::new();
    CAPTURE.get_or_init(|| {
        let Ok(capture) = serde_json::from_str::<Capture>(include_str!(
            "../../../../../tools/content-schema/world-authoring/samples/quest-container-stock.json"
        )) else {
            return Vec::new();
        };
        let unique: std::collections::BTreeSet<_> =
            capture.bindings.iter().map(|b| b.position).collect();
        if capture.schema != 1
            || capture.bindings.len() > 4096
            || unique.len() != capture.bindings.len()
            || capture.bindings.iter().any(|b| {
                b.position[2] > 15
                    || b.container == 0
                    || b.item == 0
                    || b.ordinal >= b.stock_ordinal
                    || b.stock_ordinal > 63
                    || b.capacity == 0
                    || b.capacity > 40
                    || !(1..=100).contains(&b.count)
                    || b.key_number.is_some_and(|key| key == 0 || b.count != 1)
            })
        {
            return Vec::new();
        }
        capture.bindings
    })
}
/// The original native root must exist, be empty and have no conflicting script attributes.
/// The reserved stock ordinal must be absent from the original tile.
pub(super) fn select(tile: &Tile, floor: i8, palette: &[PaletteGround]) -> Option<ContainerStock> {
    if tile.house != 0 {
        return None;
    }
    let binding = bindings().iter().find(|b| {
        b.position[0] == tile.x
            && b.position[1] == tile.y
            && i32::from(b.position[2]) == -i32::from(floor)
    })?;
    let roots: Vec<_> = tile
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.depth == 0)
        .collect();
    if roots.len() > usize::from(binding.stock_ordinal) {
        return None;
    }
    let (at, parent) = *roots.get(usize::from(binding.ordinal))?;
    if palette.get(parent.palette as usize)?.appearance != Some(binding.container)
        || parent.attrs != Default::default()
        || tile.items.get(at + 1).is_some_and(|item| item.depth > 0)
    {
        return None;
    }
    Some(ContainerStock {
        position: TilePos {
            x: tile.x,
            y: tile.y,
            floor,
        },
        parent_ordinal: binding.ordinal,
        stock_ordinal: binding.stock_ordinal,
        item: binding.item,
        count: binding.count,
        key_number: binding.key_number,
        capacity: binding.capacity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_world_bundle::sector::{Attrs, Item};
    fn scene() -> (Tile, Vec<PaletteGround>) {
        (
            Tile {
                x: 32794,
                y: 31572,
                flags: 0,
                house: 0,
                zones: vec![],
                items: vec![
                    Item {
                        palette: 0,
                        depth: 0,
                        attrs: Attrs::default(),
                    },
                    Item {
                        palette: 1,
                        depth: 0,
                        attrs: Attrs::default(),
                    },
                ],
            },
            vec![
                PaletteGround {
                    id: 0,
                    ground: Some((true, 150)),
                    appearance: Some(410),
                },
                PaletteGround {
                    id: 1,
                    ground: None,
                    appearance: Some(5972),
                },
            ],
        )
    }
    #[test]
    fn exact_empty_native_container_gets_source_stock() -> Result<(), &'static str> {
        let (tile, palette) = scene();
        let stock = select(&tile, -7, &palette).ok_or("exact source container")?;
        assert_eq!(
            (
                stock.parent_ordinal,
                stock.stock_ordinal,
                stock.item,
                stock.count,
                stock.key_number,
                stock.capacity
            ),
            (1, 63, 2969, 1, Some(3001), 10)
        );
        let mut wrong = tile.clone();
        wrong.x += 1;
        assert!(select(&wrong, -7, &palette).is_none());
        assert!(select(&tile, -6, &palette).is_none());
        Ok(())
    }
    #[test]
    fn nonempty_house_bound_or_wrong_container_is_never_filled() {
        let (tile, palette) = scene();
        let mut nonempty = tile.clone();
        nonempty.items.push(Item {
            palette: 0,
            depth: 1,
            attrs: Attrs::default(),
        });
        assert!(select(&nonempty, -7, &palette).is_none());
        let mut house = tile.clone();
        house.house = 1;
        assert!(select(&house, -7, &palette).is_none());
        let mut bound = tile.clone();
        bound.items[1].attrs.action = Some(42);
        assert!(select(&bound, -7, &palette).is_none());
        let mut wrong = tile;
        wrong.items[1].palette = 0;
        assert!(select(&wrong, -7, &palette).is_none());
    }
    #[test]
    fn stock_identity_never_aliases_a_real_base_entry() {
        let (mut tile, palette) = scene();
        while tile.items.len() < 63 {
            tile.items.push(Item {
                palette: 0,
                depth: 0,
                attrs: Attrs::default(),
            });
        }
        assert!(select(&tile, -7, &palette).is_some());
        tile.items.push(Item {
            palette: 0,
            depth: 0,
            attrs: Attrs::default(),
        });
        assert!(select(&tile, -7, &palette).is_none());
    }
}
