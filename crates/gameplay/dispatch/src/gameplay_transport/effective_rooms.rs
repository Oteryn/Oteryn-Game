//! Current source Instance geometry comes from its independently owned map.
impl super::ComposedFreshAdmission<'_, '_, '_> {
    /// A source Instance owns its mutable map; ordinary boot owners retain their
    /// existing borrowed cells. Neither branch obtains scope or placement authority.
    pub(super) fn effective_movement_cells(&self) -> &crate::content::NativeEntryMovementCells {
        self.states
            .activity_room
            .as_ref()
            .map_or(self.movement_cells, |room| room.movement_cells())
    }

    /// Bundle source rooms open combat only. Generic entry-world casting, House
    /// and source-map initialization gates remain closed on every Bundle owner.
    pub(super) fn effective_rooms(&self) -> super::SeamRooms<'_> {
        match &self.states.activity_room {
            Some(room) => super::SeamRooms::new(None, Some(room)),
            None => self.rooms,
        }
    }
}
