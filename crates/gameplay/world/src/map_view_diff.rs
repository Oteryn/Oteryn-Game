//! Pure map-view delta planning over the previously sent and current wire tiles.

use oteryn_protocol_oteryn::world_map::{
    MAX_DELTA_ENTRIES, MapTile, MapViewHeader, WorldMapViewDelta, WorldMapViewSnapshot,
    WorldMapWireError, encode_world_map_delta, encode_world_map_snapshot, in_window, tile_order,
};
use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub struct SentView {
    pub header: MapViewHeader,
    /// In wire order.
    pub tiles: Vec<MapTile>,
}

pub enum Planned {
    Snapshot(Vec<u8>),
    Delta(Vec<u8>),
    Nothing,
}

/// Whether a delta from `sent` can carry the move to `header`: the same binding, the same floor
/// and at most one step (contract §3 Origin).
fn delta_applies(sent: &MapViewHeader, header: &MapViewHeader) -> bool {
    let (from, to) = (sent.origin, header.origin);
    sent.content_generation == header.content_generation
        && sent.bundle_digest == header.bundle_digest
        && sent.reset_epoch == header.reset_epoch
        && from.floor == to.floor
        && i64::from(from.x).abs_diff(i64::from(to.x)) <= 1
        && i64::from(from.y).abs_diff(i64::from(to.y)) <= 1
}

/// The update from `sent` to `tiles`, which it hands back.
pub fn encode(
    sent: Option<&SentView>,
    header: &MapViewHeader,
    tiles: Vec<MapTile>,
) -> Result<(Planned, Vec<MapTile>), WorldMapWireError> {
    if let Some(sent) = sent.filter(|sent| delta_applies(&sent.header, header))
        && let Some(planned) = delta(sent, header, &tiles)?
    {
        return Ok((planned, tiles));
    }
    let snapshot = WorldMapViewSnapshot {
        header: *header,
        tiles,
    };
    let payload = encode_world_map_snapshot(&snapshot)?;
    Ok((Planned::Snapshot(payload), snapshot.tiles))
}

/// The delta or nothing from `sent` to `tiles`; `None` when only a snapshot carries it.
fn delta(
    sent: &SentView,
    header: &MapViewHeader,
    tiles: &[MapTile],
) -> Result<Option<Planned>, WorldMapWireError> {
    // Both tile lists are in wire order: one merge pass finds the changed and the cleared tiles.
    let mut changed = Vec::new();
    let mut cleared = Vec::new();
    let (mut now, mut before) = (tiles.iter().peekable(), sent.tiles.iter().peekable());
    loop {
        match (now.peek(), before.peek()) {
            (None, None) => break,
            (Some(tile), None) => {
                if changed.len() == MAX_DELTA_ENTRIES {
                    return Ok(None);
                }
                changed.push((*tile).clone());
                now.next();
            }
            (None, Some(tile)) => {
                if in_window(header.origin, tile.position) {
                    cleared.push(tile.position);
                }
                before.next();
            }
            (Some(new), Some(old)) => {
                match tile_order(&new.position).cmp(&tile_order(&old.position)) {
                    Ordering::Less => {
                        if changed.len() == MAX_DELTA_ENTRIES {
                            return Ok(None);
                        }
                        changed.push((*new).clone());
                        now.next();
                    }
                    Ordering::Greater => {
                        if in_window(header.origin, old.position) {
                            cleared.push(old.position);
                        }
                        before.next();
                    }
                    Ordering::Equal => {
                        if new != old {
                            if changed.len() == MAX_DELTA_ENTRIES {
                                return Ok(None);
                            }
                            changed.push((*new).clone());
                        }
                        now.next();
                        before.next();
                    }
                }
            }
        }
    }
    if changed.len() + cleared.len() > MAX_DELTA_ENTRIES {
        return Ok(None);
    }
    if changed.is_empty()
        && cleared.is_empty()
        && sent.header.origin == header.origin
        && sent.header.first_visible_floor == header.first_visible_floor
    {
        return Ok(Some(Planned::Nothing));
    }
    encode_world_map_delta(
        &sent.header,
        &WorldMapViewDelta {
            header: *header,
            tiles: changed,
            cleared,
        },
    )
    .map(|payload| Some(Planned::Delta(payload)))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::world_map::{
        MapDefinition, MapItem, MapOrigin, decode_world_map_delta, decode_world_map_snapshot,
    };
    use oteryn_protocol_oteryn::world_spatial::ActorPosition;
    use std::num::NonZeroU32;

    fn header() -> MapViewHeader {
        MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 1,
            origin: ActorPosition {
                x: 20,
                y: 20,
                floor: -7,
            },
            first_visible_floor: -7,
        }
    }

    fn tile(x: i32) -> MapTile {
        MapTile {
            position: ActorPosition {
                x,
                ..header().origin
            },
            items: vec![MapItem {
                definition: MapDefinition::Item(NonZeroU32::new(1).expect("definition")),
                count: 1,
                sub_type: 0,
                origin: MapOrigin::DisplayOnly,
                appearance_id: 1,
            }],
            more: false,
            ground_speed: 150,
        }
    }

    #[test]
    fn merge_resends_changed_tiles_and_clears_only_tiles_still_in_view() {
        let sent = SentView {
            header: header(),
            tiles: vec![tile(12), tile(14), tile(16), tile(18), tile(20)],
        };
        let mut next = sent.header;
        next.origin.x += 1;
        let mut changed = tile(16);
        changed.items[0].count = 2;
        let tiles = vec![tile(13), changed.clone(), tile(18), tile(21)];
        let (planned, retained) = encode(Some(&sent), &next, tiles.clone()).expect("plan");
        assert_eq!(retained, tiles);
        let Planned::Delta(payload) = planned else {
            panic!("one-step tile changes need a delta");
        };
        let delta = decode_world_map_delta(&sent.header, &payload).expect("delta");
        assert_eq!(delta.header, next);
        assert_eq!(delta.tiles, vec![tile(13), changed, tile(21)]);
        assert_eq!(delta.cleared, vec![tile(14).position, tile(20).position]);
    }

    #[test]
    fn unchanged_views_send_nothing_and_header_changes_send_empty_deltas() {
        let sent = SentView {
            header: header(),
            tiles: vec![tile(20)],
        };
        assert!(matches!(
            encode(Some(&sent), &sent.header, sent.tiles.clone())
                .expect("unchanged")
                .0,
            Planned::Nothing
        ));
        for next in [
            MapViewHeader {
                origin: ActorPosition {
                    x: 21,
                    ..sent.header.origin
                },
                ..sent.header
            },
            MapViewHeader {
                first_visible_floor: 0,
                ..sent.header
            },
        ] {
            let (planned, _) = encode(Some(&sent), &next, sent.tiles.clone()).expect("plan");
            let Planned::Delta(payload) = planned else {
                panic!("header changes need a delta");
            };
            let delta = decode_world_map_delta(&sent.header, &payload).expect("delta");
            assert_eq!(delta.header, next);
            assert!(delta.tiles.is_empty() && delta.cleared.is_empty());
        }
    }

    #[test]
    fn joins_binding_changes_teleports_and_floor_changes_send_snapshots() {
        let sent = SentView {
            header: header(),
            tiles: vec![tile(20)],
        };
        for (previous, next, tiles) in [
            (None, sent.header, sent.tiles.clone()),
            (
                Some(&sent),
                MapViewHeader {
                    reset_epoch: 2,
                    ..sent.header
                },
                sent.tiles.clone(),
            ),
            (
                Some(&sent),
                MapViewHeader {
                    content_generation: [3; 32],
                    ..sent.header
                },
                sent.tiles.clone(),
            ),
            (
                Some(&sent),
                MapViewHeader {
                    bundle_digest: [4; 32],
                    ..sent.header
                },
                sent.tiles.clone(),
            ),
            (
                Some(&sent),
                MapViewHeader {
                    origin: ActorPosition {
                        x: 22,
                        ..sent.header.origin
                    },
                    ..sent.header
                },
                sent.tiles.clone(),
            ),
            (
                Some(&sent),
                MapViewHeader {
                    origin: ActorPosition {
                        floor: -6,
                        ..sent.header.origin
                    },
                    first_visible_floor: -6,
                    ..sent.header
                },
                vec![],
            ),
        ] {
            let (planned, retained) = encode(previous, &next, tiles.clone()).expect("plan");
            assert_eq!(retained, tiles);
            let Planned::Snapshot(payload) = planned else {
                panic!("view changes need a snapshot");
            };
            assert_eq!(
                decode_world_map_snapshot(&payload).expect("snapshot"),
                WorldMapViewSnapshot {
                    header: next,
                    tiles
                }
            );
        }
    }
}
