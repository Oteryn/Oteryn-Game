
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
