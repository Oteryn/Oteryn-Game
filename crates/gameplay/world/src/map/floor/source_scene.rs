//! Complete source census and landing proof for the existing scene commit owner.
use super::*;
use crate::map::view::MapFacts;
use oteryn_gameplay_entities::{
    MAX_SOURCE_SCENE_ACTORS, SceneActorKind, SceneActorSnapshot, SourceSceneMove,
    SourceSceneRelocationProof,
};

#[derive(Clone, Copy, Debug)]
pub struct SceneEvacuation {
    pub from: MovementLocalPosition,
    pub players_to: MovementLocalPosition,
    pub creatures_to: MovementLocalPosition,
    pub all: bool,
}
pub struct SourceSceneRelocation<'a> {
    cells: &'a NativeEntryMovementCells,
    closed: &'a dyn CellBlocker,
    sources: Vec<SceneEvacuation>,
    census: Vec<SceneActorSnapshot>,
    moves: Vec<SourceSceneMove>,
}
pub fn prepare_source_scene<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    sources: Vec<SceneEvacuation>,
    closed: &'a dyn CellBlocker,
) -> Result<SourceSceneRelocation<'a>, MovementError> {
    if sources.len() > 32
        || sources.iter().enumerate().any(|(i, s)| {
            sources[..i].iter().any(|p| p.from == s.from)
                || sources
                    .iter()
                    .any(|p| p.from == s.players_to || p.from == s.creatures_to)
        })
    {
        return Err(MovementError::NotQualified);
    }
    let census = current_census(runtime, &sources)?;
    if census.len() > MAX_SOURCE_SCENE_ACTORS
        || sources.iter().any(|s| {
            !s.all
                && census
                    .iter()
                    .filter(|a| a.position.position() == s.from)
                    .count()
                    > 1
        })
    {
        return Err(MovementError::NotQualified);
    }
    let moves = census
        .iter()
        .map(|entry| {
            let source = sources
                .iter()
                .find(|s| s.from == entry.position.position())
                .ok_or(MovementError::NotQualified)?;
            Ok(SourceSceneMove {
                actor: entry.actor,
                expected: entry.position,
                destination: if matches!(entry.kind, SceneActorKind::Creature) {
                    source.creatures_to
                } else {
                    source.players_to
                },
            })
        })
        .collect::<Result<Vec<_>, MovementError>>()?;
    let proof = SourceSceneRelocation {
        cells,
        closed,
        sources,
        census,
        moves,
    };
    proof.check(runtime)?;
    Ok(proof)
}
fn current_census(
    runtime: &ChannelRuntimeV1,
    sources: &[SceneEvacuation],
) -> Result<Vec<SceneActorSnapshot>, MovementError> {
    Ok(runtime
        .scene_actor_census()
        .map_err(MovementError::Actor)?
        .into_iter()
        .filter(|entry| sources.iter().any(|s| s.from == entry.position.position()))
        .collect())
}
impl SourceSceneRelocation<'_> {
    pub fn moves(&self) -> &[SourceSceneMove] {
        &self.moves
    }
    pub fn observations(
        &self,
    ) -> Vec<(
        ExactActorRef,
        Option<GameSessionId>,
        MovementPositionSnapshot,
    )> {
        self.census
            .iter()
            .map(|entry| {
                (
                    entry.actor,
                    if let SceneActorKind::Player(session) = entry.kind {
                        Some(session)
                    } else {
                        None
                    },
                    entry.position,
                )
            })
            .collect()
    }
    fn check(&self, runtime: &ChannelRuntimeV1) -> Result<(), MovementError> {
        if self.cells.scope().world_id != runtime.binding().world_id()
            || self.cells.scope().generation_digest
                != runtime.content_pin().server_artifact_digest()
            || current_census(runtime, &self.sources)? != self.census
        {
            return Err(MovementError::NotQualified);
        }
        for source in &self.sources {
            if runtime
                .scene_destination_reserved(source.from)
                .map_err(MovementError::Actor)?
            {
                return Err(MovementError::NotQualified);
            }
        }
        for (entry, change) in self.census.iter().zip(&self.moves) {
            if (!matches!(entry.kind, SceneActorKind::Npc)
                && runtime.actor_spell_reserved(entry.actor))
                || entry.position.context() != runtime.pinned_movement_context()
            {
                return Err(MovementError::NotQualified);
            }
            if let SceneActorKind::Player(session) = entry.kind {
                qualify_origin(runtime, self.cells, entry.actor, session, entry.position)?;
            }
            let to = change.destination;
            let cell = LogicalCell {
                x: to.x,
                y: to.y,
                z: i32::from(to.floor),
            };
            if entry.position.position().floor != to.floor
                || ground_of(self.cells)((to.x, to.y, i32::from(to.floor))) != Ground::Walkable
                || !landable_of(self.cells, true, self.closed)((to.x, to.y, i32::from(to.floor)))
                || is_teleport(self.cells, cell)
                || !self
                    .cells
                    .spell_tiles()
                    .lookup(self.cells.scope(), cell)
                    .is_ok_and(|facts| {
                        !facts.flags().protection_zone && !facts.flags().floor_change
                    })
                || runtime
                    .scene_destination_reserved(to)
                    .map_err(MovementError::Actor)?
            {
                return Err(MovementError::Blocked);
            }
            if let NativeMovementCollisionIndex::Bundle(index) = self.cells.index() {
                let at = pos(to.x, to.y, i32::from(to.floor)).ok_or(MovementError::Blocked)?;
                if index.map().facts.house_tile(at) {
                    return Err(MovementError::Blocked);
                }
            }
        }
        Ok(())
    }
}
impl crate::foundation::source_step_seal::Sealed for SourceSceneRelocation<'_> {}
impl SourceSceneRelocationProof for SourceSceneRelocation<'_> {
    fn validate_current(&self, runtime: &ChannelRuntimeV1) -> Result<(), CarrierError> {
        self.check(runtime).map_err(|_| CarrierError::PlanConflict)
    }
    fn moves(&self) -> &[SourceSceneMove] {
        &self.moves
    }
}
