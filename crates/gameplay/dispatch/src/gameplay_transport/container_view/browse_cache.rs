//! Crystal's shared, weak browse-field cache: live viewers share one tile order.

use super::super::item_view::{ItemKey, ViewItem};
use crate::map::overlay::TilePos;
use oteryn_protocol_oteryn::item_view::WorldTilePosition;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, Weak};

/// Retained only while a connection has the field open. It never enters session continuity.
#[derive(Debug, Clone)]
pub struct BrowseLease(Arc<()>);

impl PartialEq for BrowseLease {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for BrowseLease {}

type FieldKey = ([u8; 32], u64, u16, u16, i8);

#[derive(Debug)]
struct CachedField {
    order: Vec<ItemKey>,
    lease: Weak<()>,
}

/// Channel-owned ephemeral metadata; expired entries are removed on the next observation.
#[derive(Debug, Default)]
pub(in crate::gameplay_transport) struct BrowseFields {
    fields: Mutex<BTreeMap<FieldKey, CachedField>>,
}

impl BrowseFields {
    /// A successful ordinary move supplies the tile notifications that polling could miss.
    /// In particular, pickup and redrop of the same instance must append it even when no
    /// viewer observed its absence (Crystal `tile.cpp:379-393`, `:504-518`). Call while the
    /// existing move lane is retained; this leaf metadata update never writes custody.
    pub(in crate::gameplay_transport) fn committed_move(
        &self,
        source: ItemKey,
        destination: Option<WorldTilePosition>,
    ) {
        let Ok(mut fields) = self.fields.lock() else {
            return;
        };
        fields.retain(|_, field| field.lease.strong_count() > 0);
        for (key, field) in fields.iter_mut() {
            field.order.retain(|item| *item != source);
            if matches!(source, ItemKey::Instance(_))
                && destination.is_some_and(|to| {
                    to.x == i32::from(key.2)
                        && to.y == i32::from(key.3)
                        && to.floor == i16::from(key.4)
                })
            {
                field.order.push(source);
            }
        }
    }

    /// `fresh` is the complete initial browse order, after filtering and reversal.
    /// Crystal `game.cpp:5064-5072` reuses a live tile container; `tile.cpp:379-393`
    /// appends later additions while retaining the order of surviving entries.
    pub(in crate::gameplay_transport) fn observe(
        &self,
        bundle_digest: [u8; 32],
        reset_epoch: u64,
        tile: TilePos,
        fresh: Vec<ViewItem>,
    ) -> Option<(Vec<ViewItem>, BrowseLease)> {
        // The full item count has the same u16 bound as browse page metadata.
        u16::try_from(fresh.len()).ok()?;
        let mut remaining = BTreeMap::new();
        for entry in &fresh {
            if remaining.insert(entry.key, *entry).is_some() {
                return None;
            }
        }
        let key = (bundle_digest, reset_epoch, tile.x, tile.y, tile.floor);
        let mut fields = self.fields.lock().ok()?;
        fields.retain(|_, field| field.lease.strong_count() > 0);
        if let Some(field) = fields.get_mut(&key)
            && let Some(lease) = field.lease.upgrade()
        {
            let mut ordered = Vec::with_capacity(fresh.len());
            for previous in &field.order {
                if let Some(current) = remaining.remove(previous) {
                    ordered.push(current);
                }
            }
            // Reverse the initial projection back to tile insertion order for new entries.
            for entry in fresh.iter().rev() {
                if remaining.remove(&entry.key).is_some() {
                    ordered.push(*entry);
                }
            }
            field.order = ordered.iter().map(|entry| entry.key).collect();
            return Some((ordered, BrowseLease(lease)));
        }
        let lease = Arc::new(());
        fields.insert(
            key,
            CachedField {
                order: fresh.iter().map(|entry| entry.key).collect(),
                lease: Arc::downgrade(&lease),
            },
        );
        Some((fresh, BrowseLease(lease)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU32;

    type TestResult = Result<(), &'static str>;

    fn tile() -> TilePos {
        TilePos {
            x: 300,
            y: 301,
            floor: -7,
        }
    }

    fn item(id: u8) -> ViewItem {
        ViewItem {
            key: ItemKey::Instance([id; 16]),
            item_definition_ref: NonZeroU32::MIN,
            count: NonZeroU32::MIN,
            sub_type: 0,
        }
    }

    fn items(ids: &[u8]) -> Vec<ViewItem> {
        ids.iter().copied().map(item).collect()
    }

    #[test]
    fn browse_cache_second_viewer_reuses_live_order_and_updates_current_values() -> TestResult {
        let cache = BrowseFields::default();
        let (first, lease_a) = cache
            .observe([1; 32], 0, tile(), items(&[3, 2, 1]))
            .ok_or("first")?;
        assert_eq!(first, items(&[3, 2, 1]));
        let mut changed = items(&[5, 4, 3, 1]);
        changed[2].count = NonZeroU32::new(7).ok_or("count")?;
        let (second, lease_b) = cache
            .observe([1; 32], 0, tile(), changed.clone())
            .ok_or("second")?;
        let mut expected = items(&[3, 1, 4, 5]);
        expected[0].count = NonZeroU32::new(7).ok_or("count")?;
        assert_eq!(second, expected);
        assert_eq!(lease_a, lease_b);
        let (refreshed, lease_c) = cache
            .observe([1; 32], 0, tile(), changed)
            .ok_or("refresh")?;
        assert_eq!(refreshed, second);
        assert_eq!(lease_a, lease_c);
        Ok(())
    }

    #[test]
    fn browse_cache_last_close_recreates_initial_reverse_order() -> TestResult {
        let cache = BrowseFields::default();
        let (_, lease_a) = cache
            .observe([1; 32], 0, tile(), items(&[2, 1]))
            .ok_or("first")?;
        let lease_b = lease_a.clone();
        let expired = Arc::downgrade(&lease_a.0);
        drop(lease_a);
        let (while_open, live) = cache
            .observe([1; 32], 0, tile(), items(&[3, 2, 1]))
            .ok_or("live")?;
        assert_eq!(while_open, items(&[2, 1, 3]));
        assert_eq!(lease_b, live);
        drop(lease_b);
        drop(live);
        assert!(expired.upgrade().is_none());
        let (reopened, _) = cache
            .observe([1; 32], 0, tile(), items(&[3, 2, 1]))
            .ok_or("reopen")?;
        assert_eq!(reopened, items(&[3, 2, 1]));
        Ok(())
    }

    #[test]
    fn browse_cache_isolates_bundle_epoch_and_tile_while_views_remain_open() -> TestResult {
        let cache = BrowseFields::default();
        let (_, original) = cache
            .observe([1; 32], 0, tile(), items(&[2, 1]))
            .ok_or("original")?;
        let alternatives = [
            ([2; 32], 0, tile()),
            ([1; 32], 1, tile()),
            ([1; 32], 0, TilePos { x: 301, ..tile() }),
            (
                [1; 32],
                0,
                TilePos {
                    floor: -6,
                    ..tile()
                },
            ),
        ];
        let mut leases = Vec::new();
        for (digest, epoch, tile) in alternatives {
            let (separate, lease) = cache
                .observe(digest, epoch, tile, items(&[3, 2, 1]))
                .ok_or("separate")?;
            assert_eq!(separate, items(&[3, 2, 1]));
            assert_ne!(original, lease);
            leases.push(lease);
        }
        let (updated, lease) = cache
            .observe([1; 32], 0, tile(), items(&[3, 2, 1]))
            .ok_or("updated")?;
        assert_eq!(updated, items(&[2, 1, 3]));
        assert_eq!(original, lease);
        assert_eq!(leases.len(), alternatives.len());
        Ok(())
    }

    #[test]
    fn browse_cache_rejects_duplicate_keys_without_changing_the_live_order() -> TestResult {
        let cache = BrowseFields::default();
        let (_, original) = cache
            .observe([1; 32], 0, tile(), items(&[2, 1]))
            .ok_or("original")?;
        assert!(cache.observe([1; 32], 0, tile(), items(&[1, 1])).is_none());
        let (updated, lease) = cache
            .observe([1; 32], 0, tile(), items(&[3, 2, 1]))
            .ok_or("updated")?;
        assert_eq!(updated, items(&[2, 1, 3]));
        assert_eq!(original, lease);
        Ok(())
    }

    #[test]
    fn browse_cache_pickup_and_redrop_appends_without_an_intermediate_observation() -> TestResult {
        let cache = BrowseFields::default();
        let (_, original) = cache
            .observe([1; 32], 0, tile(), items(&[2, 1]))
            .ok_or("original")?;
        let other = TilePos { x: 302, ..tile() };
        let (_, other_lease) = cache
            .observe([1; 32], 0, other, items(&[4, 3]))
            .ok_or("other")?;
        let destination = WorldTilePosition {
            x: i32::from(tile().x),
            y: i32::from(tile().y),
            floor: i16::from(tile().floor),
        };
        cache.committed_move(item(2).key, None);
        cache.committed_move(item(2).key, Some(destination));
        let (redropped, lease) = cache
            .observe([1; 32], 0, tile(), items(&[2, 1]))
            .ok_or("redropped")?;
        assert_eq!(redropped, items(&[1, 2]));
        assert_eq!(original, lease);
        let (unchanged, lease) = cache
            .observe([1; 32], 0, other, items(&[4, 3]))
            .ok_or("unchanged")?;
        assert_eq!(unchanged, items(&[4, 3]));
        assert_eq!(other_lease, lease);
        Ok(())
    }
}
