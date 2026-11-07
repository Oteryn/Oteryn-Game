//! Canonical tile insertion order owned by physical position commits.
//! Canary47dfd51f tile.cpp336-340,1131-1143,1764-1774: insertion
//! prepends and getTopCreature reads the first entry. No actor-ID ordering.
use super::*;
impl Slot {
    pub(super) fn tile_entry_order(&self) -> Option<u64> {
        match self {
            Self::Occupied {
                committed: true,
                position: Some(_),
                lifecycle,
                ..
            } => Some(lifecycle.tile_entry_order),
            Self::CreatureOccupied {
                position: Some(_),
                health,
                committed,
                spell_combat,
                ..
            } if *health > 0
                || (spell_combat.native_death_events.is_some()
                    && !spell_combat.source_death_completed
                    && committed.lethal().is_some()) =>
            {
                Some(committed.tile_entry_order)
            }
            Self::NpcOccupied {
                tile_entry_order, ..
            } => Some(*tile_entry_order),
            _ => None,
        }
    }
    fn install_tile_entry_order(&mut self, order: u64) {
        match self {
            Self::Occupied { lifecycle, .. } => lifecycle.tile_entry_order = order,
            Self::CreatureOccupied { committed, .. } => committed.tile_entry_order = order,
            Self::NpcOccupied {
                tile_entry_order, ..
            } => *tile_entry_order = order,
            _ => unreachable!("position owner preflight proved an actual actor slot"),
        }
    }
}
impl ChannelActorCarrier {
    pub(super) fn next_tile_entry_order(&self) -> Result<u64, CarrierError> {
        self.tile_entry_counter
            .checked_add(1)
            .ok_or(CarrierError::PositionRevisionExhausted)
    }
    /// Infallible publication after all position owner preflight, under the same
    /// exclusive carrier borrow. This stores ordering only, never coordinates.
    pub(super) fn install_tile_entry_order(&mut self, index: usize, order: u64) {
        self.slots[index].install_tile_entry_order(order);
        self.tile_entry_counter = order;
    }
    pub(super) fn install_next_tile_entry_order(&mut self, index: usize) {
        let Some(order) = self.tile_entry_counter.checked_add(1) else {
            unreachable!("held owner preflight reserved tile entry capacity")
        };
        self.install_tile_entry_order(index, order);
    }
    pub(super) fn top_actor_on_cell(
        &self,
        continuity: &NamespaceContinuityGuard,
        context: PreProductionPositionContext,
        cell: LocalPosition,
    ) -> Result<Option<ActorRef>, CarrierError> {
        self.validate_current_continuity(continuity)?;
        self.validate_position_context(context)?;
        let mut top = None;
        let mut latest = 0;
        for (index, slot) in self.slots.iter().enumerate() {
            let position = match slot {
                Slot::Occupied {
                    committed: true,
                    position: Some(position),
                    ..
                }
                | Slot::NpcOccupied { position, .. } => position,
                Slot::CreatureOccupied {
                    position: Some(position),
                    ..
                } if slot.tile_entry_order().is_some() => position,
                _ => continue,
            };
            if position.position != cell {
                continue;
            }
            if position.context != context {
                return Err(CarrierError::PositionContextMismatch);
            }
            let actor = ActorRef::from_scoped_owner_parts(
                self.runtime_scope,
                self.scope_generation,
                position.actor_local_id,
                position.actor_local_generation,
            );
            if self.validate_ref(continuity, actor)? != index {
                return Err(CarrierError::PositionSnapshotMismatch);
            }
            let order = slot
                .tile_entry_order()
                .ok_or(CarrierError::PositionSnapshotMismatch)?;
            if order == 0 || (order == latest && top.is_some()) {
                return Err(CarrierError::PositionSnapshotMismatch);
            }
            if order > latest {
                latest = order;
                top = Some(actor);
            }
        }
        Ok(top)
    }
}
impl ChannelRuntimeV1 {
    /// Current source `Tile:getTopCreature` including Player and NPC subclasses.
    /// A source-qualified HP0 creature stays registered through its queued
    /// onDeath. Completed deaths retained only for corpse settlement are excluded.
    pub fn current_top_actor_on_cell(
        &self,
        cell: MovementLocalPosition,
    ) -> Result<Option<ExactActorRef>, CarrierError> {
        self.owner_fence()?;
        self.carrier
            .top_actor_on_cell(
                &self.continuity,
                self.pinned_position_context(),
                LocalPosition {
                    x: cell.x,
                    y: cell.y,
                    floor: cell.floor,
                },
            )
            .map(|actor| actor.map(ExactActorRef))
    }
    pub fn is_current_top_actor(&self, actor: ExactActorRef) -> Result<bool, CarrierError> {
        let position = self.read_actor_position(actor)?;
        if position.context() != self.pinned_movement_context() {
            return Err(CarrierError::PositionContextMismatch);
        }
        Ok(self.current_top_actor_on_cell(position.position())? == Some(actor))
    }
}
