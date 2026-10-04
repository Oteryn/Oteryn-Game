//! BAGS-WIRE-1 (BAGS-0 §5): the server side of capability 14 `CONTAINER_TREE_V1` for one
//! connection.
//!
//! - **Views.** Domain 14 shows up to 16 open containers (`BAGS0-RL-03`), each with its direct
//!   entries and, when visible, its parent. Each delta carries every open view. Views live only on
//!   the connection: every admission, reconnect, resume and transfer starts with none.
//! - **Handles.** A view's container and entries take handles from the session's ITEM-VIEW-1b
//!   table ([`super::item_view`]) as its fourth view, under `ITEMV0-RL-03-CONTAINER-TREE`.
//! - **Command 21.** Opens a container in a new view or in place of an open one, closes a view,
//!   or goes up to the parent. A handle or view that no longer resolves, or a parent that is not
//!   visible, is `STALE`; the Channel owner's observation decides reach and what the item is.
//!   At most 10 view commands per second on a sliding window (`BAGS0-RL-04`): one over the rate
//!   is `REJECTED` with an empty payload before decoding. The window ([`ViewCommandWindow`]) is
//!   per GameSession: it travels in the session continuity across reconnect, resume and transfer.
//! - **`USE`.** With the capability, `USE` on a container handle opens it in a new view; a corpse
//!   stays a domain 11 view at depth 1.
//!
//! Nothing here writes: moving an item in the tree is BAGS-1. Capability 14 stays
//! `offered: false` until BAGS-1, so only the tests select it.

#![cfg_attr(not(test), allow(dead_code))]

use super::item_view::{
    ItemHandleTable, ItemKey, ItemViewDelta, ItemViewError, SessionItemView, ViewItem,
};
use oteryn_protocol_oteryn::container_tree::{
    ContainerView, ContainerViewIntent, ContainerViewOutcome, ContainerViews, MAX_CONTAINER_VIEWS,
    encode_container_views,
};
use oteryn_protocol_oteryn::item_view::ItemEntry;
use std::collections::BTreeMap;
use std::iter;
use tokio::time::{Duration, Instant};

/// `BAGS0-RL-04`: view commands per [`VIEW_COMMAND_WINDOW`].
pub(crate) const MAX_VIEW_COMMANDS_PER_WINDOW: usize = 10;
pub(crate) const VIEW_COMMAND_WINDOW: Duration = Duration::from_secs(1);

/// What the Channel owner observes for a container a view command or `USE` names, with reach
/// already decided against the actor's position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ContainerObservation {
    /// The item is gone or no longer visible to this session.
    Gone,
    /// A visible item that is not a container, or a corpse (domain 11).
    NotAContainer,
    /// A container out of reach.
    TooFar,
    /// A container in reach: its parent, its entry count and its direct entries in display order.
    Container {
        parent: Option<ItemKey>,
        capacity: u8,
        entries: Vec<ViewItem>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OpenView {
    container: ItemKey,
    parent: Option<ItemKey>,
    capacity: u8,
    entries: Vec<ViewItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    New,
    View(u8),
}

/// An open or up that waits for the Channel owner's observation of `key`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingOpen {
    key: ItemKey,
    slot: Slot,
}

impl PendingOpen {
    /// `USE` on a container handle: a new view.
    pub(crate) const fn new_view(key: ItemKey) -> Self {
        Self {
            key,
            slot: Slot::New,
        }
    }

    pub(crate) const fn key(&self) -> ItemKey {
        self.key
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ContainerPlan {
    /// The command is decided; the domain 14 delta, if any, follows the result.
    Decided(ContainerViewOutcome, Option<ItemViewDelta>),
    /// Observe the container, then [`ContainerViewState::apply`].
    Observe(PendingOpen),
}

/// `BAGS0-RL-04`: the instants of one GameSession's view commands still inside the sliding
/// window. It is carried in [`super::item_view::ItemViewContinuity`], so a reconnect, resume or
/// transfer keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ViewCommandWindow([Option<Instant>; MAX_VIEW_COMMANDS_PER_WINDOW]);

impl ViewCommandWindow {
    pub(crate) const EMPTY: Self = Self([None; MAX_VIEW_COMMANDS_PER_WINDOW]);

    /// Whether one more view command fits the sliding window ending at `now`; if so it is
    /// recorded in a place that is free or has left the window.
    pub(crate) fn admit(&mut self, now: Instant) -> bool {
        let free = self.0.iter_mut().find(|slot| {
            !matches!(slot, Some(at) if now.saturating_duration_since(*at) < VIEW_COMMAND_WINDOW)
        });
        let Some(slot) = free else {
            return false;
        };
        *slot = Some(now);
        true
    }
}

impl Default for ViewCommandWindow {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// One connection's open container views.
#[derive(Debug, Default)]
pub(crate) struct ContainerViewState {
    views: BTreeMap<u8, OpenView>,
}

impl ContainerViewState {
    pub(crate) fn open_views(&self) -> usize {
        self.views.len()
    }

    /// The first step of one command 21: a close is decided here; an open or up names the
    /// container to observe.
    pub(crate) fn plan(
        &mut self,
        item_view: &mut SessionItemView,
        intent: ContainerViewIntent,
    ) -> Result<ContainerPlan, ItemViewError> {
        let stale = Ok(ContainerPlan::Decided(ContainerViewOutcome::Stale, None));
        match intent {
            ContainerViewIntent::Open {
                handle,
                replace_view,
            } => {
                let Some(key) = item_view.resolve(handle) else {
                    return stale;
                };
                let slot = match replace_view {
                    Some(id) if self.views.contains_key(&id) => Slot::View(id),
                    Some(_) => return stale,
                    None => Slot::New,
                };
                Ok(ContainerPlan::Observe(PendingOpen { key, slot }))
            }
            ContainerViewIntent::Close { view_id } => {
                let mut next = self.views.clone();
                if next.remove(&view_id).is_none() {
                    return stale;
                }
                Ok(match self.commit(item_view, next)? {
                    Some(delta) => {
                        ContainerPlan::Decided(ContainerViewOutcome::Closed, Some(delta))
                    }
                    None => ContainerPlan::Decided(ContainerViewOutcome::Stale, None),
                })
            }
            ContainerViewIntent::Up { view_id } => {
                let parent = self
                    .views
                    .get(&view_id)
                    .and_then(|view| view.parent)
                    .filter(|parent| item_view.table().handle(parent).is_some());
                match parent {
                    Some(key) => Ok(ContainerPlan::Observe(PendingOpen {
                        key,
                        slot: Slot::View(view_id),
                    })),
                    None => stale,
                }
            }
        }
    }

    /// The second step of an open, up or container `USE`, on the Channel owner's observation
    /// (`None` when it cannot be observed: `STALE`). A container shows in one view only: opening
    /// one already shown elsewhere moves it. Views that would not encode leave everything
    /// unchanged and are `STALE`.
    pub(crate) fn apply(
        &mut self,
        item_view: &mut SessionItemView,
        pending: PendingOpen,
        observation: Option<ContainerObservation>,
    ) -> Result<(ContainerViewOutcome, Option<ItemViewDelta>), ItemViewError> {
        let (parent, capacity, entries) = match observation {
            None | Some(ContainerObservation::Gone) => {
                return Ok((ContainerViewOutcome::Stale, None));
            }
            Some(ContainerObservation::NotAContainer) => {
                return Ok((ContainerViewOutcome::NotAContainer, None));
            }
            Some(ContainerObservation::TooFar) => return Ok((ContainerViewOutcome::TooFar, None)),
            Some(ContainerObservation::Container {
                parent,
                capacity,
                entries,
            }) => (parent, capacity, entries),
        };
        let mut next = self.views.clone();
        let shown = next
            .iter()
            .find(|(_, view)| view.container == pending.key)
            .map(|(id, _)| *id);
        let id = match (pending.slot, shown) {
            (Slot::View(id), _) | (Slot::New, Some(id)) => id,
            (Slot::New, None) => {
                match (0..MAX_CONTAINER_VIEWS as u8).find(|id| !next.contains_key(id)) {
                    Some(id) => id,
                    None => return Ok((ContainerViewOutcome::TooManyViews, None)),
                }
            }
        };
        if let Some(other) = shown.filter(|other| *other != id) {
            next.remove(&other);
        }
        next.insert(
            id,
            OpenView {
                container: pending.key,
                parent,
                capacity,
                entries,
            },
        );
        Ok(match self.commit(item_view, next)? {
            Some(delta) => (ContainerViewOutcome::Opened, Some(delta)),
            None => (ContainerViewOutcome::Stale, None),
        })
    }

    /// Makes `next` the open views and returns the domain 14 delta, or `None` (nothing changed)
    /// when its handles are over the bound or it does not encode.
    fn commit(
        &mut self,
        item_view: &mut SessionItemView,
        next: BTreeMap<u8, OpenView>,
    ) -> Result<Option<ItemViewDelta>, ItemViewError> {
        let keys: Vec<ItemKey> = next
            .values()
            .flat_map(|view| iter::once(view.container).chain(view.entries.iter().map(|e| e.key)))
            .collect();
        let encode = |table: &ItemHandleTable| {
            let views = next
                .iter()
                .map(|(id, view)| encode_view(table, *id, view))
                .collect::<Result<_, _>>()?;
            encode_container_views(&ContainerViews { views }).map_err(|_| ItemViewError::Encode)
        };
        match item_view.tree_delta(&keys, encode) {
            Ok(delta) => {
                self.views = next;
                Ok(Some(delta))
            }
            Err(ItemViewError::Encode | ItemViewError::LimitExceeded) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

fn encode_view(
    table: &ItemHandleTable,
    view_id: u8,
    view: &OpenView,
) -> Result<ContainerView, ItemViewError> {
    Ok(ContainerView {
        view_id,
        container_handle: table.handle(&view.container).ok_or(ItemViewError::Encode)?,
        parent_handle: view.parent.and_then(|parent| table.handle(&parent)),
        capacity: view.capacity,
        entries: view
            .entries
            .iter()
            .map(|item| {
                Ok(ItemEntry {
                    handle: table.handle(&item.key).ok_or(ItemViewError::Encode)?,
                    item_definition_ref: item.item_definition_ref,
                    count: item.count,
                    sub_type: item.sub_type,
                })
            })
            .collect::<Result<_, ItemViewError>>()?,
    })
}

#[cfg(test)]
#[path = "container_view_tests.rs"]
mod tests;
