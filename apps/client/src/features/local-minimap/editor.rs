//! Atomic portable presentation access to the existing local minimap owner.
use super::{Binding, Marker, Minimap, lock};
use crate::play::mailbox::Mailbox;
use crate::play::{PlayLink, PlayView};
use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicU64, Ordering},
};
#[derive(Clone, Debug)]
struct Session(Weak<Mutex<Mailbox>>);
impl PartialEq for Session {
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}
impl Eq for Session {}

static NEXT_EDITOR: AtomicU64 = AtomicU64::new(1);
#[derive(Debug)]
pub(super) struct Editor {
    identity: u64,
    revision: u64,
    session: Option<Session>,
}
impl Default for Editor {
    fn default() -> Self {
        Self {
            identity: NEXT_EDITOR.fetch_add(1, Ordering::Relaxed),
            revision: 0,
            session: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Snapshot {
    identity: u64,
    revision: u64,
    session: Option<Session>,
    world: Option<String>,
    binding: Option<Binding>,
    pub markers: Vec<Marker>,
    pub centre: (i32, i32, i16),
    pub own: (i32, i32, i16),
    pub zoom: i32,
    pub tracking: bool,
    pub feedback: String,
}
impl Snapshot {
    pub(crate) fn same_scope(&self, other: &Self) -> bool {
        self.identity == other.identity
            && self.session == other.session
            && self.world == other.world
            && self.binding == other.binding
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Close,
    ZoomIn,
    ZoomOut,
    FloorUp,
    FloorDown,
    Recenter,
    Pan(i32, i32),
    Save(Marker),
    Delete(Marker),
    GoTo(Marker),
}
impl Minimap {
    fn editor_snapshot(&self, own: (i32, i32, i16)) -> Snapshot {
        Snapshot {
            identity: self.editor.identity,
            revision: self.editor.revision,
            session: self.editor.session.clone(),
            world: self.world.clone(),
            binding: self.binding,
            markers: self.markers.clone(),
            centre: (self.x, self.y, self.floor),
            own,
            zoom: self.zoom,
            tracking: self.tracking,
            feedback: self.feedback.clone(),
        }
    }
}
impl PlayView {
    pub(crate) fn minimap_editor_snapshot(&self) -> Option<Snapshot> {
        let map = lock(&self.minimap);
        map.editor_open
            .then(|| map.editor_snapshot((self.own.x, self.own.y, self.floor)))
    }
    pub(crate) fn minimap_editor_open(&self, link: &PlayLink, admitted: bool) -> bool {
        let blocked = self.gameplay_input_blocked(link);
        let mut map = lock(&self.minimap);
        let mailbox = crate::play::mailbox::lock(&link.mailbox);
        if !admitted
            || link.commands.is_closed()
            || mailbox.depot_shutdown
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || blocked
        {
            return false;
        }
        let session = Some(Session(Arc::downgrade(&link.mailbox)));
        if !map.editor_open || map.editor.session != session {
            map.editor.session = session;
            map.editor_open = true;
            map.editor.revision = map.editor.revision.wrapping_add(1);
        }
        true
    }
    pub(crate) fn minimap_editor_create(&self, link: &PlayLink, admitted: bool) -> bool {
        let blocked = self.gameplay_input_blocked(link);
        let mut map = lock(&self.minimap);
        let mailbox = crate::play::mailbox::lock(&link.mailbox);
        if !admitted
            || link.commands.is_closed()
            || mailbox.depot_shutdown
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || blocked
        {
            return false;
        }
        match map.create_mark_at((self.own.x, self.own.y, self.floor)) {
            Ok(false) => {
                map.editor_open = true;
                map.editor.session = Some(Session(Arc::downgrade(&link.mailbox)));
            }
            Ok(true) => map.feedback = tr!("Marker saved locally").into(),
            Err(error) => map.feedback = error.to_string(),
        }
        map.editor.revision = map.editor.revision.wrapping_add(1);
        true
    }
    /// Close checks identity only, so stale draft, input gates and failed saves cannot trap UI.
    /// All other operations compare and consume the exact projection under the owner lock.
    pub(crate) fn minimap_editor_bound(
        &self,
        link: &PlayLink,
        expected: &Snapshot,
        action: Action,
        admitted: bool,
    ) -> bool {
        let blocked = action != Action::Close && self.gameplay_input_blocked(link);
        let mut map = lock(&self.minimap);
        let mailbox = crate::play::mailbox::lock(&link.mailbox);
        if map.editor.identity != expected.identity
            || !map.editor_open
            || expected.session != Some(Session(Arc::downgrade(&link.mailbox)))
            || map.editor.session != expected.session
            || link.commands.is_closed()
            || mailbox.depot_shutdown
        {
            return false;
        }
        if action == Action::Close {
            map.editor_open = false;
            map.editor.revision = map.editor.revision.wrapping_add(1);
            return true;
        }
        if !admitted
            || blocked
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || map.editor_snapshot((self.own.x, self.own.y, self.floor)) != *expected
        {
            return false;
        }
        if let Action::Delete(marker) | Action::GoTo(marker) = &action
            && !map.markers.contains(marker)
        {
            return false;
        }
        map.editor.revision = map.editor.revision.wrapping_add(1);
        match action {
            Action::Close => return false,
            Action::ZoomIn => map.zoom = oteryn_client_ui::minimap::zoom_in(map.zoom),
            Action::ZoomOut => map.zoom = oteryn_client_ui::minimap::zoom_out(map.zoom),
            Action::FloorUp => {
                map.tracking = false;
                map.floor = oteryn_client_ui::minimap::floor_up(map.floor);
            }
            Action::FloorDown => {
                map.tracking = false;
                map.floor = oteryn_client_ui::minimap::floor_down(map.floor);
            }
            Action::Recenter => {
                map.tracking = true;
                let (x, y, floor) = map.own;
                map.follow(x, y, floor);
            }
            Action::Pan(x, y) => map.pan(x, y),
            Action::Save(marker) => {
                map.feedback = match map.edit_marker(
                    marker.x,
                    marker.y,
                    marker.floor,
                    marker.label,
                    marker.color,
                ) {
                    Ok(()) => tr!("Marker saved locally").into(),
                    Err(error) => error.to_string(),
                };
            }
            Action::Delete(marker) => {
                map.feedback = match map.remove_marker(marker.x, marker.y, marker.floor) {
                    Ok(()) => tr!("Marker deleted").into(),
                    Err(error) => error.to_string(),
                };
            }
            Action::GoTo(marker) => {
                map.tracking = false;
                map.x = marker.x;
                map.y = marker.y;
                map.floor = marker.floor;
            }
        }
        true
    }
}
