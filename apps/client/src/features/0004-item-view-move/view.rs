#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
#[cfg(windows)]
#[path = "view/drag.rs"]
mod drag;
#[cfg(windows)]
pub(in crate::play) use drag::{DragNative, clear_surfaces, list_surface, view_surfaces};

impl crate::play::PlayLink {
    pub fn item_movement_status(&self) -> &'static str {
        let movement = self.item_movement();
        if movement.dragged.is_some() {
            tr!("Dragging item: release over inventory, an equipment slot, a container or map tile")
        } else if movement.prompt.is_some() {
            tr!("Choose the item count in the Move stack window")
        } else if movement.pending {
            tr!("Moving item…")
        } else {
            movement.feedback.unwrap_or(tr!(
                "Drag items between inventory, equipment, containers and map"
            ))
        }
    }
}
