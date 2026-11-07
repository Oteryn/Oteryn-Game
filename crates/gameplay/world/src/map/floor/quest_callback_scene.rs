//! Bounded forced scene transport; an occupied safe landing is source-authorized.
use super::*;
use crate::foundation::{SourceSceneMove, SourceSceneRelocationProof};
use crate::map::view::MapFacts;

pub struct PreparedSceneRelocations<'a> {
    cells: &'a NativeEntryMovementCells,
    map: &'a crate::map::boot::BundleMap,
    moves: Vec<SourceSceneMove>,
    sessions: Vec<Option<GameSessionId>>,
}

impl PreparedSceneRelocations<'_> {
    pub fn observations(
        &self,
    ) -> Vec<(
        ExactActorRef,
        Option<GameSessionId>,
        MovementPositionSnapshot,
    )> {
        self.moves
            .iter()
            .zip(&self.sessions)
            .map(|(change, session)| (change.actor, *session, change.expected))
            .collect()
    }
    fn check(&self, runtime: &ChannelRuntimeV1) -> Result<(), MovementError> {
        if self.moves.len() > 32 || self.moves.len() != self.sessions.len() {
            return Err(MovementError::CapacityExceeded);
        }
        for (change, session) in self.moves.iter().zip(&self.sessions) {
            if runtime
                .read_actor_position(change.actor)
                .map_err(MovementError::Actor)?
                != change.expected
            {
                return Err(MovementError::Blocked);
            }
            if let Some(session) = session {
                qualify_origin(runtime, self.cells, change.actor, *session, change.expected)?;
            }
            if self.cells.scope().world_id != change.expected.world_id()
                || self.cells.scope().generation_digest
                    != runtime.content_pin().server_artifact_digest()
            {
                return Err(MovementError::Blocked);
            }
            let to = change.destination;
            let cell = LogicalCell {
                x: to.x,
                y: to.y,
                z: i32::from(to.floor),
            };
            let tile = TilePos {
                x: u16::try_from(to.x).map_err(|_| MovementError::Blocked)?,
                y: u16::try_from(to.y).map_err(|_| MovementError::Blocked)?,
                floor: i8::try_from(to.floor.checked_neg().ok_or(MovementError::Blocked)?)
                    .map_err(|_| MovementError::Blocked)?,
            };
            if ground_of(self.cells)((to.x, to.y, i32::from(to.floor))) != Ground::Walkable
                || self.map.facts.house_tile(tile)
                || is_teleport(self.cells, cell)
                || !self
                    .cells
                    .spell_tiles()
                    .lookup(self.cells.scope(), cell)
                    .is_ok_and(|facts| {
                        !facts.flags().protection_zone && !facts.flags().floor_change
                    })
            {
                return Err(MovementError::Blocked);
            }
        }
        Ok(())
    }
}

pub fn prepare_scene_relocations<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    map: &'a crate::map::boot::BundleMap,
    moves: Vec<SourceSceneMove>,
    sessions: Vec<Option<GameSessionId>>,
) -> Result<PreparedSceneRelocations<'a>, MovementError> {
    let proof = PreparedSceneRelocations {
        cells,
        map,
        moves,
        sessions,
    };
    proof.check(runtime)?;
    Ok(proof)
}
impl crate::foundation::source_step_seal::Sealed for PreparedSceneRelocations<'_> {}
impl SourceSceneRelocationProof for PreparedSceneRelocations<'_> {
    fn validate_current(&self, runtime: &ChannelRuntimeV1) -> Result<(), CarrierError> {
        self.check(runtime).map_err(|_| CarrierError::PlanConflict)
    }
    fn moves(&self) -> &[SourceSceneMove] {
        &self.moves
    }
}
