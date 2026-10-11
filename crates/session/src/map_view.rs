//! Bounded MAP-WIRE-1 domain store. Only the server composes tile stacks and visible floors.
use crate::{ActorPosition, ItemHandle};
pub use oteryn_protocol_oteryn::world_map::{
    MAX_MAP_VIEW_HANDLES, MAX_SNAPSHOT_TILES, MapDefinition, MapItem, MapOrigin, MapTile,
    MapViewHeader, WorldMapViewDelta, WorldMapViewSnapshot, WorldMapWireError,
    decode_world_map_delta, decode_world_map_snapshot, encode_world_map_delta,
    encode_world_map_snapshot, in_window, tile_order,
};
use std::collections::BTreeMap;

type TileKey = (i16, i32, i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapViewError {
    AwaitingSnapshot,
    ContentGeneration,
    Limit,
    Wire(WorldMapWireError),
}

/// A target obtained from the current map, never from its appearance or definition number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapTarget {
    Item(ItemHandle),
    Base {
        placement: [u8; 40],
        expected_revision: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapView {
    generation: [u8; 32],
    header: Option<MapViewHeader>,
    tiles: BTreeMap<TileKey, MapTile>,
}

impl MapView {
    pub fn new(generation: [u8; 32]) -> Result<Self, MapViewError> {
        if generation == [0; 32] {
            return Err(MapViewError::ContentGeneration);
        }
        Ok(Self {
            generation,
            header: None,
            tiles: BTreeMap::new(),
        })
    }

    /// Any refused update invalidates drawing and targets until a new valid snapshot arrives.
    /// The transport consumer must request resync; an old handle is not a fallback.
    fn finish(
        &mut self,
        result: Result<WorldMapViewSnapshot, MapViewError>,
    ) -> Result<(), MapViewError> {
        match result {
            Ok(snapshot) => {
                self.header = Some(snapshot.header);
                self.tiles = snapshot
                    .tiles
                    .into_iter()
                    .map(|tile| (tile_order(&tile.position), tile))
                    .collect();
                Ok(())
            }
            Err(error) => {
                self.header = None;
                self.tiles.clear();
                Err(error)
            }
        }
    }

    fn check(&self, snapshot: WorldMapViewSnapshot) -> Result<WorldMapViewSnapshot, MapViewError> {
        if snapshot.header.content_generation != self.generation {
            return Err(MapViewError::ContentGeneration);
        }
        let handles = snapshot
            .tiles
            .iter()
            .flat_map(|tile| &tile.items)
            .filter(|item| matches!(item.origin, MapOrigin::Handle(_)))
            .count();
        if snapshot.tiles.len() > MAX_SNAPSHOT_TILES || handles > MAX_MAP_VIEW_HANDLES {
            return Err(MapViewError::Limit);
        }
        Ok(snapshot)
    }

    pub fn replace_snapshot(&mut self, payload: &[u8]) -> Result<(), MapViewError> {
        let result = decode_world_map_snapshot(payload)
            .map_err(MapViewError::Wire)
            .and_then(|snapshot| self.check(snapshot));
        self.finish(result)
    }

    pub fn apply_delta(&mut self, payload: &[u8]) -> Result<(), MapViewError> {
        let result = (|| {
            let header = self.header.ok_or(MapViewError::AwaitingSnapshot)?;
            let delta = decode_world_map_delta(&header, payload).map_err(MapViewError::Wire)?;
            // Stage the entire bounded state; no intermediate window or stack is observable.
            let mut tiles = self.tiles.clone();
            tiles.retain(|_, tile| in_window(delta.header.origin, tile.position));
            for position in delta.cleared {
                tiles.remove(&tile_order(&position));
            }
            for tile in delta.tiles {
                tiles.insert(tile_order(&tile.position), tile);
            }
            self.check(WorldMapViewSnapshot {
                header: delta.header,
                tiles: tiles.into_values().collect(),
            })
        })();
        self.finish(result)
    }

    /// Accepts an already canonically decoded snapshot without a second wire decode.
    pub(crate) fn replace_decoded(
        &mut self,
        snapshot: WorldMapViewSnapshot,
    ) -> Result<(), MapViewError> {
        let result = self.check(snapshot);
        self.finish(result)
    }

    /// A bounded latest-state projection; coalescing it cannot lose clears or origin shifts.
    #[must_use]
    pub fn snapshot(&self) -> Option<WorldMapViewSnapshot> {
        Some(WorldMapViewSnapshot {
            header: self.header?,
            tiles: self.tiles.values().cloned().collect(),
        })
    }

    #[must_use]
    pub fn header(&self) -> Option<MapViewHeader> {
        self.header
    }

    pub fn visible_tiles(&self) -> impl Iterator<Item = &MapTile> {
        self.tiles.values().filter(|tile| {
            self.header
                .is_some_and(|header| tile.position.floor <= header.first_visible_floor)
        })
    }

    #[must_use]
    pub fn tile(&self, position: ActorPosition) -> Option<&MapTile> {
        self.tiles.get(&tile_order(&position))
    }

    #[must_use]
    pub fn ground_speed(&self, position: ActorPosition) -> Option<u16> {
        self.tile(position)
            .map(|tile| tile.ground_speed)
            .filter(|speed| *speed != 0)
    }

    #[must_use]
    pub fn target(&self, position: ActorPosition, stack_index: usize) -> Option<MapTarget> {
        let header = self.header?;
        // Hidden floors are not interactable through this rendered-view accessor.
        if position.floor > header.first_visible_floor {
            return None;
        }
        let item = self.tile(position)?.items.get(stack_index)?;
        match item.origin {
            MapOrigin::DisplayOnly => None,
            MapOrigin::Handle(handle) => Some(MapTarget::Item(handle)),
            MapOrigin::BaseOrdinal {
                ordinal,
                object_revision,
            } => {
                let x = u16::try_from(position.x).ok()?;
                let y = u16::try_from(position.y).ok()?;
                let floor = u8::try_from(position.floor.checked_neg()?).ok()?;
                let key = u64::from(x) << 32
                    | u64::from(y) << 16
                    | u64::from(floor) << 8
                    | u64::from(ordinal);
                let mut placement = [0; 40];
                placement[..32].copy_from_slice(&header.bundle_digest);
                placement[32..].copy_from_slice(&key.to_be_bytes());
                Some(MapTarget::Base {
                    placement,
                    expected_revision: object_revision,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests;
