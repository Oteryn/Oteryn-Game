use crate::play::Pushed;
use oteryn_session::AppliedDelta;
use oteryn_session::world_map::{WorldMapViewDelta, WorldMapViewSnapshot, in_window, tile_order};

/// Reduce every delta before the shell polls; publish only the latest bounded window.
pub(in crate::play) fn world_map(pushed: &mut Pushed, delta: AppliedDelta<WorldMapViewDelta>) {
    let delta = delta.value;
    let map = pushed.map.get_or_insert_with(|| {
        Box::new(WorldMapViewSnapshot {
            header: delta.header,
            tiles: Vec::new(),
        })
    });
    map.header = delta.header;
    map.tiles.retain(|tile| {
        in_window(map.header.origin, tile.position)
            && !delta.cleared.contains(&tile.position)
            && !delta.tiles.iter().any(|new| new.position == tile.position)
    });
    map.tiles.extend(delta.tiles);
    map.tiles.sort_by_key(|tile| tile_order(&tile.position));
}

impl crate::play::Outbox {
    pub(in crate::play) fn map_initialize(&self, payload: Option<Vec<u8>>) {
        crate::play::mailbox::lock(&self.0).map = payload
            .and_then(|bytes| oteryn_session::world_map::decode_world_map_snapshot(&bytes).ok())
            .map(Box::new);
    }
}
