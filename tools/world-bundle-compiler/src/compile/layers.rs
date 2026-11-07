//! Resolve retained source tile facts through the native World Project compiler.
use super::{Input, KeyResolver, Resolution};
use crate::{
    Error,
    bundle::{Family, PaletteEntry, Sector},
    sector::{Attrs, Item, Tile},
};
use oteryn_world_bundle::layers::Layer;
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Source {
    #[serde(default)]
    schema: String,
    #[serde(default)]
    source_revision: String,
    #[serde(default)]
    layers: Vec<RawLayer>,
}
#[derive(Clone, Debug, Deserialize)]
struct RawLayer {
    key: String,
    tiles: Vec<RawTile>,
}
#[derive(Clone, Debug, Deserialize)]
struct RawTile {
    position: (u16, u16, u8),
    flags: Option<u32>,
    entries: Vec<(u16, Option<u16>, Option<u16>)>,
}
impl Source {
    pub fn load(root: &Path) -> Result<Self, Error> {
        let bytes = std::fs::read(root.join("content/encounters/world-changes/layers.json"))
            .map_err(|e| Error::Format(e.to_string()))?;
        if bytes.len() > 1 << 20 {
            return Err(Error::Limit("world change layer input".into()));
        }
        let source: Self =
            serde_json::from_slice(&bytes).map_err(|e| Error::Format(e.to_string()))?;
        if source.schema != "OTERYN_WORLD_CHANGE_LAYERS/v1"
            || source.source_revision != "201189f871d23597b82caf88fd611454a2ddefb8"
            || source.layers.len() > 16
            || source.layers.iter().any(|layer| {
                layer.tiles.is_empty()
                    || layer.tiles.len() > 4096
                    || layer
                        .tiles
                        .iter()
                        .any(|tile| tile.position.2 > 15 || tile.entries.len() > 64)
            })
        {
            return Err(Error::Format("world change layer source".into()));
        }
        Ok(source)
    }
}

pub fn realize(
    input: &Input<'_>,
    resolver: &dyn KeyResolver,
    base: &[Sector],
) -> Result<Vec<Layer>, Error> {
    let houses: std::collections::BTreeSet<_> = base
        .iter()
        .flat_map(|s| {
            s.tiles
                .iter()
                .filter(|t| t.house != 0)
                .map(move |t| (t.x, t.y, s.floor))
        })
        .collect();
    let regions: std::collections::BTreeSet<_> =
        base.iter().map(|s| (s.sx / 8, s.sy / 8)).collect();
    let mut layers = Vec::new();
    for source in &input.families.layers.layers {
        // Regional bundles admit a layer only when every replacement tile belongs to them.
        if !source.tiles.iter().all(|t| {
            input
                .world
                .contains(t.position.0, t.position.1, -(t.position.2 as i8))
                && regions.contains(&(t.position.0 / 256, t.position.1 / 256))
        }) {
            continue;
        }
        let mut palette = Vec::new();
        let mut entries = BTreeMap::new();
        let mut sectors: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for tile in &source.tiles {
            let (x, y, z) = tile.position;
            let floor = super::native_floor(z)?;
            if houses.contains(&(x, y, floor)) {
                return Err(Error::Family(format!(
                    "layer {} intersects house",
                    source.key
                )));
            }
            let mut items = Vec::new();
            for &(appearance, action, unique) in &tile.entries {
                let at = if let Some(at) = entries.get(&appearance) {
                    *at
                } else {
                    let mut resolved = None;
                    for family in ["terrain", "item"] {
                        let key = format!("oteryn:{family}.tibia.i{appearance}");
                        if let Resolution::Resolved(family, id) = resolver.resolve(&key) {
                            let terrain = resolver.terrain(&key)?;
                            if family == Family::Terrain && terrain.is_none() {
                                return Err(Error::Family(format!(
                                    "unclassified layer terrain {key}"
                                )));
                            }
                            resolved = Some(PaletteEntry {
                                key,
                                family,
                                id,
                                terrain,
                            });
                            break;
                        }
                    }
                    let entry = resolved
                        .ok_or_else(|| Error::Key(format!("layer appearance {appearance}")))?;
                    let at = u32::try_from(palette.len())
                        .map_err(|_| Error::Limit("layer palette".into()))?;
                    palette.push(entry);
                    entries.insert(appearance, at);
                    at
                };
                items.push(Item {
                    palette: at,
                    depth: 0,
                    attrs: Attrs {
                        action,
                        unique,
                        ..Attrs::default()
                    },
                });
            }
            sectors
                .entry((floor, y / 32, x / 32))
                .or_default()
                .push(Tile {
                    x,
                    y,
                    flags: tile.flags.unwrap_or(0),
                    house: 0,
                    zones: Vec::new(),
                    items,
                });
        }
        let sectors = sectors
            .into_iter()
            .map(|((floor, sy, sx), mut tiles)| {
                tiles.sort_by_key(|t| (t.y, t.x));
                Sector {
                    floor,
                    sy,
                    sx,
                    tiles,
                }
            })
            .collect();
        layers.push(Layer {
            key: source.key.clone(),
            palette,
            sectors,
        });
    }
    layers.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(layers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bundle::{BuildClass, Extent, Identity, Terrain, TerrainKind},
        project::Families,
    };
    struct Resolver;
    impl KeyResolver for Resolver {
        fn resolve(&self, key: &str) -> Resolution {
            if key == "oteryn:terrain.tibia.i1" {
                Resolution::Resolved(Family::Terrain, 42)
            } else {
                Resolution::Unknown
            }
        }
        fn terrain(&self, _: &str) -> Result<Option<Terrain>, Error> {
            Ok(Some(Terrain {
                kind: TerrainKind::Ground,
                walkable: Some(true),
                ground_speed: Some(150),
                blocks_projectile: None,
            }))
        }
        fn floor_change(&self, _: &str) -> bool {
            false
        }
    }
    #[test]
    fn w05b_compiler_resolves_native_layer_and_refuses_house_or_missing_reference()
    -> Result<(), Error> {
        let mut families = Families {
            layers: Source {
                schema: "OTERYN_WORLD_CHANGE_LAYERS/v1".into(),
                source_revision: "201189f871d23597b82caf88fd611454a2ddefb8".into(),
                layers: vec![RawLayer {
                    key: "fury_gates/test".into(),
                    tiles: vec![RawTile {
                        position: (1, 1, 7),
                        flags: Some(0),
                        entries: vec![(1, Some(9710), None)],
                    }],
                }],
            },
            ..Families::default()
        };
        let base_tile = Tile {
            x: 1,
            y: 1,
            flags: 0,
            house: 0,
            zones: Vec::new(),
            items: Vec::new(),
        };
        let mut base = vec![Sector {
            floor: -7,
            sx: 0,
            sy: 0,
            tiles: vec![base_tile],
        }];
        fn input(families: &Families) -> Input<'_> {
            Input {
                regions: &[],
                palette: &[],
                identity: Identity::default(),
                world: Extent {
                    min_x: 0,
                    min_y: 0,
                    max_x: 32,
                    max_y: 32,
                    floors: vec![-7],
                },
                build_class: BuildClass::Production,
                draft_areas: Vec::new(),
                families,
            }
        }
        let layers = realize(&input(&families), &Resolver, &base)?;
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].palette[0].id, 42);
        assert_eq!(
            layers[0].sectors[0].tiles[0].items[0].attrs.action,
            Some(9710)
        );
        base[0].tiles[0].house = 1;
        assert!(matches!(
            realize(&input(&families), &Resolver, &base),
            Err(Error::Family(_))
        ));
        base[0].tiles[0].house = 0;
        families.layers.layers[0].tiles[0].entries[0].0 = 2;
        assert!(matches!(
            realize(&input(&families), &Resolver, &base),
            Err(Error::Key(_))
        ));
        Ok(())
    }
}
