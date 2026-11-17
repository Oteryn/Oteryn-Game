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

/// A replacement discards every tile from the old window, including overlapping coordinates.
pub(in crate::play) fn world_map_snapshot(
    pushed: &mut Pushed,
    snapshot: AppliedDelta<WorldMapViewSnapshot>,
) {
    pushed.own = Some(snapshot.value.header.origin);
    pushed.map = Some(Box::new(snapshot.value));
}

impl crate::play::mailbox::Mailbox {
    /// Retain the current window across polls, while reducing replacements in wire order.
    pub(in crate::play) fn coalesce_events(&mut self, events: Vec<oteryn_session::SessionEvent>) {
        crate::play::feature_0023_presentation::state::ui_sound::observe(self, &events);
        use oteryn_session::SessionEvent;
        let has_map = events.iter().any(|event| {
            matches!(
                event,
                SessionEvent::WorldMap(_) | SessionEvent::WorldMapSnapshot(_)
            )
        });
        if has_map && self.pushed.map.is_none() {
            self.pushed.map = self.map.clone();
        }
        crate::play::coalesce_pushed(&mut self.pushed, events);
        if has_map {
            self.map = self.pushed.map.clone();
        }
    }
}

/// A scope barrier discards coalesced source projections before fresh destination deltas.
pub(in crate::play) fn activity_scope(
    pushed: &mut Pushed,
    scope: AppliedDelta<oteryn_session::activity_scope::ActivityScopeSnapshot>,
) {
    *pushed = Pushed::default();
    pushed.activity_scope = Some(scope.value);
}
