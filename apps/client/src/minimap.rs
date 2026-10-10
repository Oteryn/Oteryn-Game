//! Read-only overview of the same locally loaded map and atlas that the scene draws.
//! Colors come from rendered artwork, not inferred walkability. No explored state, marks,
//! server entities or other floors are invented. Rebuild once when the loaded world changes.
use crate::world::{RADIUS, START, START_FLOOR, World};
use oteryn_renderer::TileCoord;
use oteryn_session::ActorPosition;
pub const MINIMAP_SIDE: u16 = (2 * RADIUS + 1) as u16;
const ORIGIN: (i32, i32) = (START.0 - RADIUS, START.1 - RADIUS);
const BACKGROUND: [u8; 4] = [9, 14, 18, 255];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinimapZoom {
    Nearby,
    Region,
    LoadedArea,
}

impl MinimapZoom {
    const fn radius(self) -> i32 {
        match self {
            Self::Nearby => 16,
            Self::Region => 32,
            Self::LoadedArea => RADIUS,
        }
    }
}

/// The source map position of the authoritative session position, using the scene's exact
/// wrapping translation. The source map contains only START_FLOOR; a session floor change
/// invalidates its mapping until the scene has a map for that floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapLocation {
    pub position: TileCoord,
    pub source_floor: u8,
}

impl MapLocation {
    #[must_use]
    pub fn from_session(
        position: ActorPosition,
        anchor: TileCoord,
        mapped_floor: Option<i16>,
    ) -> Option<Self> {
        (mapped_floor == Some(position.floor)).then_some(Self {
            position: TileCoord::new(
                position.x.wrapping_add(anchor.x),
                position.y.wrapping_add(anchor.y),
            ),
            source_floor: START_FLOOR,
        })
    }
}

/// Integer texture crop and player pixel relative to it. Presentation can fit this crop to
/// any panel size without changing the source coordinates or copying the whole map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MinimapViewport {
    pub origin: [u16; 2],
    pub size: [u16; 2],
    pub player: [u16; 2],
}

#[derive(Debug)]
pub struct LoadedMinimap {
    pixels: Vec<[u8; 4]>,
    known: Vec<bool>,
}

impl LoadedMinimap {
    /// At most 129×129 colors and presence bits. It borrows atlas bytes during sampling and
    /// never copies the atlas, redistributes asset files or treats sprite colors as collision.
    #[must_use]
    pub fn from_world(world: &World) -> Option<Self> {
        if world.tile_count() == 0 {
            return None;
        }
        let side = usize::from(MINIMAP_SIDE);
        let mut pixels = vec![BACKGROUND; side * side];
        let mut known = vec![false; side * side];
        let atlas = world.atlas();
        for row in 0..side {
            for column in 0..side {
                let Some(tile) = world.tile(ORIGIN.0 + column as i32, ORIGIN.1 + row as i32) else {
                    continue;
                };
                let at = row * side + column;
                known[at] = true;
                let mut color = [0, 0, 0, 255];
                for draw in tile.under.iter().chain(&tile.over) {
                    let center = i64::from(atlas.cell_px() / 2);
                    let x = center - i64::from(draw.offset[0]);
                    let y = center - i64::from(draw.offset[1]);
                    if x < 0
                        || y < 0
                        || x >= i64::from(atlas.cell_px())
                        || y >= i64::from(atlas.cell_px())
                        || u32::from(draw.cell) >= atlas.cell_count()
                    {
                        continue;
                    }
                    let columns = atlas.width() / atlas.cell_px();
                    let left = u32::from(draw.cell) % columns * atlas.cell_px();
                    let top = u32::from(draw.cell) / columns * atlas.cell_px();
                    let index = ((u64::from(top) + y as u64) * u64::from(atlas.width())
                        + u64::from(left)
                        + x as u64) as usize
                        * 4;
                    let source = &atlas.rgba()[index..index + 4];
                    let alpha = u32::from(source[3]);
                    for channel in 0..3 {
                        color[channel] = ((u32::from(source[channel]) * alpha
                            + u32::from(color[channel]) * (255 - alpha))
                            / 255) as u8;
                    }
                }
                pixels[at] = color;
            }
        }
        known
            .iter()
            .any(|known| *known)
            .then_some(Self { pixels, known })
    }
    #[must_use]
    pub fn pixels(&self) -> &[[u8; 4]] {
        &self.pixels
    }
    /// Only the actually loaded floor and tile can position the player. Unknown mapping,
    /// unloaded tiles and unsupported floor requests return unavailable, not a fake map.
    #[must_use]
    pub fn viewport(
        &self,
        location: Option<MapLocation>,
        floor: u8,
        zoom: MinimapZoom,
    ) -> Option<MinimapViewport> {
        let location = location?;
        if floor != START_FLOOR || location.source_floor != floor {
            return None;
        }
        let x = location.position.x.checked_sub(ORIGIN.0)?;
        let y = location.position.y.checked_sub(ORIGIN.1)?;
        let side = i32::from(MINIMAP_SIDE);
        if !(0..side).contains(&x)
            || !(0..side).contains(&y)
            || !self.known[y as usize * usize::from(MINIMAP_SIDE) + x as usize]
        {
            return None;
        }
        let radius = zoom.radius();
        let (left, top, right, bottom) = if zoom == MinimapZoom::LoadedArea {
            (0, 0, side, side)
        } else {
            (
                (x - radius).max(0),
                (y - radius).max(0),
                (x + radius + 1).min(side),
                (y + radius + 1).min(side),
            )
        };
        Some(MinimapViewport {
            origin: [left as u16, top as u16],
            size: [(right - left) as u16, (bottom - top) as u16],
            player: [(x - left) as u16, (y - top) as u16],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Draw, MARKER_CELL, MapTile, TARGET_CELL};
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn overview() -> Result<LoadedMinimap, Box<dyn std::error::Error>> {
        let tile = MapTile {
            under: vec![Draw {
                cell: MARKER_CELL,
                offset: [0, 0],
            }],
            over: vec![Draw {
                cell: TARGET_CELL,
                offset: [0, 0],
            }],
            elevation: 0,
        };
        let world = World::builtin_with([(START, tile.clone()), (ORIGIN, tile)])?;
        LoadedMinimap::from_world(&world).ok_or_else(|| "test map unavailable".into())
    }

    #[test]
    fn source_artwork_is_composited_without_collision_inference() -> TestResult {
        let overview = overview()?;
        assert_eq!(overview.pixels()[0], [220, 40, 40, 255]);
        assert_eq!(overview.pixels()[1], BACKGROUND);
        assert!(LoadedMinimap::from_world(&World::builtin()?).is_none());
        Ok(())
    }

    #[test]
    fn scene_translation_floor_change_and_unloaded_tiles_are_respected() -> TestResult {
        let own = ActorPosition {
            x: i32::MAX,
            y: i32::MIN,
            floor: 0,
        };
        let anchor = TileCoord::new(START.0.wrapping_sub(own.x), START.1.wrapping_sub(own.y));
        let location = MapLocation::from_session(own, anchor, Some(0));
        assert_eq!(
            location.map(|location| location.position),
            Some(TileCoord::new(START.0, START.1))
        );
        let overview = overview()?;
        let floor = START_FLOOR;
        let near = MinimapZoom::Nearby;
        let view = overview
            .viewport(location, floor, near)
            .ok_or("viewport unavailable")?;
        assert_eq!(view.player, [16, 16]);
        assert_eq!(view.size, [33, 33]);
        assert!(MapLocation::from_session(own, anchor, Some(1)).is_none());
        assert!(overview.viewport(None, floor, near).is_none());
        assert!(overview.viewport(location, floor + 1, near).is_none());
        let unloaded = MapLocation {
            position: TileCoord::new(START.0 + 1, START.1),
            source_floor: START_FLOOR,
        };
        assert!(overview.viewport(Some(unloaded), floor, near).is_none());
        Ok(())
    }

    #[test]
    fn edge_crop_keeps_marker_inside_and_overview_fits_loaded_bounds() -> TestResult {
        let overview = overview()?;
        let location = Some(MapLocation {
            position: TileCoord::new(ORIGIN.0, ORIGIN.1),
            source_floor: START_FLOOR,
        });
        let edge = overview
            .viewport(location, START_FLOOR, MinimapZoom::Region)
            .ok_or("edge unavailable")?;
        assert_eq!(edge.origin, [0, 0]);
        assert_eq!(edge.player, [0, 0]);
        assert_eq!(edge.size, [33, 33]);
        let full = overview
            .viewport(location, START_FLOOR, MinimapZoom::LoadedArea)
            .ok_or("overview unavailable")?;
        assert_eq!(full.size, [MINIMAP_SIDE; 2]);
        Ok(())
    }
}
