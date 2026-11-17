//! Compatibility factory for callers holding already enumerated exact scene moves.
use super::*;
use crate::foundation::SourceSceneMove;

pub type PreparedSceneRelocations<'a> = SourceSceneRelocation<'a>;

pub fn prepare_scene_relocations<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    map: &'a crate::map::boot::BundleMap,
    moves: Vec<SourceSceneMove>,
    sessions: Vec<Option<GameSessionId>>,
) -> Result<PreparedSceneRelocations<'a>, MovementError> {
    if moves.len() != sessions.len() || moves.len() > 32 {
        return Err(MovementError::CapacityExceeded);
    }
    let mut sources = Vec::<SceneEvacuation>::new();
    for change in &moves {
        let from = change.expected.position();
        if let Some(prior) = sources.iter().find(|s| s.from == from) {
            if prior.players_to != change.destination {
                return Err(MovementError::NotQualified);
            }
        } else {
            sources.push(SceneEvacuation {
                from,
                players_to: change.destination,
                creatures_to: change.destination,
                all: true,
            });
        }
    }
    // The Bundle's own dynamic door blocker stays live through commit.
    let proof = prepare_source_scene(runtime, cells, sources, map.facts.doors())?;
    let observed = proof.observations();
    if proof.moves().len() != moves.len()
        || moves.iter().zip(sessions).any(|(change, session)| {
            !observed.iter().any(|(actor, current, expected)| {
                *actor == change.actor && *current == session && *expected == change.expected
            })
        })
    {
        return Err(MovementError::NotQualified);
    }
    Ok(proof)
}
