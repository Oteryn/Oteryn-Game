//! Compatibility names delegate to the canonical shared activity room owner.
impl super::ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn effective_movement_cells(&self) -> &crate::content::NativeEntryMovementCells {
        self.movement_cells()
    }
    pub(super) fn effective_rooms(&self) -> &super::SeamRooms<'_> {
        self.rooms()
    }
}
