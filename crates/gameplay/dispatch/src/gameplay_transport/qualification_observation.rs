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

pub(super) struct SessionView {
    session: [u8; 16],
    generation: u64,
}

impl SessionView {
    pub(super) fn new(session: [u8; 16], generation: u64) -> Self {
        Self {
            session,
            generation,
        }
    }

    pub(super) fn observe(&self, table: &ItemHandleTable) {
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

impl Drop for SessionView {
    fn drop(&mut self) {
        if let Some(views) = VIEWS.get()
            && let Ok(mut views) = views.lock()
            && views
                .get(&self.session)
                .is_some_and(|(generation, _)| *generation == self.generation)
        {
            views.remove(&self.session);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay_transport::item_view::{ItemKey, View};

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
        first.observe(&table);
        assert_eq!(instance([31; 16], original), Some([45; 16]));
        assert_eq!(instance([31; 16], other), Some([46; 16]));
        assert_eq!(instance([32; 16], original), None);
        table
            .replace(View::Map, &[ItemKey::Instance([46; 16])])
            .map_err(|error| format!("{error:?}"))?;
        let next = SessionView::new([31; 16], 2);
        next.observe(&table);
        drop(first);
        assert_eq!(instance([31; 16], original), None);
        assert_eq!(instance([31; 16], other), Some([46; 16]));
        drop(next);
        assert_eq!(instance([31; 16], other), None);
        Ok(())
    }
}
