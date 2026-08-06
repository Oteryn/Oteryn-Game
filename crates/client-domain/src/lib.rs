//! Protocol-neutral, non-authoritative client projection model.

use oteryn_foundation::ClientSessionEpoch;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientEntityRef(u64);

impl ClientEntityRef {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityProjection {
    pub entity_ref: ClientEntityRef,
    pub position: Position,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientWorldProjection {
    pub session_epoch: ClientSessionEpoch,
    pub revision: u64,
    pub entities: BTreeMap<ClientEntityRef, EntityProjection>,
}

impl ClientWorldProjection {
    #[must_use]
    pub fn empty(session_epoch: ClientSessionEpoch) -> Self {
        Self {
            session_epoch,
            revision: 0,
            entities: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionEvent {
    Upsert(EntityProjection),
    Remove(ClientEntityRef),
}
