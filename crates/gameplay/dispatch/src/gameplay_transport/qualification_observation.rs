//! Read-only observations for the disposable qualification node's root control socket.
use super::item_view::ItemHandleTable;
use crate::foundation::RuntimeScopeRefV1;
use std::{
    collections::BTreeMap,
    sync::{Mutex, OnceLock},
};

use crate::durability::spell_owner_commit::SpellLane;
use oteryn_protocol_oteryn::item_view::ItemHandle;

type SessionViews = BTreeMap<[u8; 16], (u64, ItemHandleTable)>;
static VIEWS: OnceLock<Mutex<SessionViews>> = OnceLock::new();
type ScopeKey = (bool, [u8; 16], [u8; 16]);
static LANES: OnceLock<Mutex<BTreeMap<ScopeKey, SpellLane>>> = OnceLock::new();

pub fn enable() {
    VIEWS.get_or_init(Default::default);
    LANES.get_or_init(Default::default);
}

pub fn scope_lane(scope: RuntimeScopeRefV1) -> SpellLane {
    if let Some(lanes) = LANES.get()
        && let Ok(mut lanes) = lanes.lock()
    {
        let key = match scope {
            RuntimeScopeRefV1::Channel {
                world_id,
                channel_id,
            } => (false, *world_id.as_bytes(), *channel_id.as_bytes()),
            RuntimeScopeRefV1::Instance {
                world_id,
                instance_id,
            } => (true, *world_id.as_bytes(), instance_id),
        };
        return lanes
            .entry(key)
            .or_insert_with(|| SpellLane::for_scope(scope))
            .clone();
    }
    SpellLane::for_scope(scope)
}

pub fn instance(session: [u8; 16], handle: ItemHandle) -> Option<[u8; 16]> {
    let views = VIEWS.get()?.lock().ok()?;
    match views.get(&session)?.1.resolve(handle)? {
        super::item_view::ItemKey::Instance(item) => Some(item),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ObservationKey {
    session: [u8; 16],
    generation: u64,
}

impl ObservationKey {
    pub(super) fn publish(self, table: &ItemHandleTable) {
        if let Some(views) = VIEWS.get()
            && let Ok(mut views) = views.lock()
            && views
                .get(&self.session)
                .is_none_or(|(generation, _)| *generation <= self.generation)
        {
            views.insert(self.session, (self.generation, table.clone()));
        }
    }
}

pub(super) struct SessionView(ObservationKey);

impl SessionView {
    pub(super) fn new(session: [u8; 16], generation: u64) -> Self {
        Self(ObservationKey {
            session,
            generation,
        })
    }

    pub(super) fn key(&self) -> Option<ObservationKey> {
        VIEWS.get().map(|_| self.0)
    }
}

impl Drop for SessionView {
    fn drop(&mut self) {
        if let Some(views) = VIEWS.get()
            && let Ok(mut views) = views.lock()
            && views
                .get(&self.0.session)
                .is_some_and(|(generation, _)| *generation == self.0.generation)
        {
            views.remove(&self.0.session);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay_transport::item_view::{ItemKey, View};

    #[test]
    fn issued_handle_is_observable_before_the_next_serve_iteration()
    -> Result<(), Box<dyn std::error::Error>> {
        enable();
        use crate::gameplay_transport::item_view::{InventoryItems, SessionItemView, ViewItem};
        use oteryn_protocol_oteryn::item_view::decode_character_inventory;
        use std::num::NonZeroU32;

        let guard = SessionView::new([51; 16], 1);
        let mut view = SessionItemView::resume(Default::default());
        view.observe_qualification(guard.key());
        let one = NonZeroU32::MIN;
        let item = |instance| ViewItem {
            appearance_id: 0,
            key: ItemKey::Instance([instance; 16]),
            item_definition_ref: one,
            count: one,
            sub_type: 0,
        };
        let update = view
            .inventory_committed(InventoryItems {
                main_backpack: Some(item(53)),
                entries: vec![item(52)],
                equipment: Vec::new(),
            })
            .map_err(|error| format!("{error:?}"))?
            .ok_or("missing inventory delta")?;
        let wire =
            decode_character_inventory(&update.payload).map_err(|error| format!("{error:?}"))?;
        let handle = wire.entries.first().ok_or("inventory handle")?.handle;
        assert_eq!(instance([51; 16], handle), Some([52; 16]));
        Ok(())
    }

    #[test]
    fn qualification_instance_lookup_is_session_local_and_tracks_live_handles()
    -> Result<(), Box<dyn std::error::Error>> {
        enable();
        let first = SessionView::new([31; 16], 1);
        let mut table = ItemHandleTable::default();
        table
            .replace(
                View::Map,
                &[ItemKey::Instance([45; 16]), ItemKey::Instance([46; 16])],
            )
            .map_err(|error| format!("{error:?}"))?;
        let original = table
            .handle(&ItemKey::Instance([45; 16]))
            .ok_or("original handle")?;
        let other = table
            .handle(&ItemKey::Instance([46; 16]))
            .ok_or("other handle")?;
        table.observe_qualification(first.key());
        assert_eq!(instance([31; 16], original), Some([45; 16]));
        assert_eq!(instance([31; 16], other), Some([46; 16]));
        assert_eq!(instance([32; 16], original), None);
        table
            .replace(View::Map, &[ItemKey::Instance([46; 16])])
            .map_err(|error| format!("{error:?}"))?;
        let next = SessionView::new([31; 16], 2);
        table.observe_qualification(next.key());
        drop(first);
        assert_eq!(instance([31; 16], original), None);
        assert_eq!(instance([31; 16], other), Some([46; 16]));
        drop(next);
        assert_eq!(instance([31; 16], other), None);
        Ok(())
    }
}
