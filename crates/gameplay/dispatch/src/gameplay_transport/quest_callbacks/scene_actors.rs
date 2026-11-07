use super::*;
use crate::foundation::SourceSceneMove;
use crate::map::floor::PreparedSceneRelocations;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn prepare_quest_scene_moves<'a>(
        &'a self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        map: &'a BundleMap,
        callback: &Callback,
    ) -> Result<Option<PreparedSceneRelocations<'a>>, UseDisposition> {
        let Some(scene) = callback
            .effects
            .scene
            .as_ref()
            .filter(|scene| !scene.relocations.is_empty())
        else {
            return Ok(None);
        };
        let actors = runtime
            .positioned_actor_census()
            .map_err(|_| UseDisposition::Rejected)?;
        let mut moves = Vec::new();
        let mut sessions = Vec::new();
        for (actor, _, session) in actors {
            let expected = runtime
                .read_actor_position(actor)
                .map_err(|_| UseDisposition::Rejected)?;
            let Some(relocation) = scene.relocations.iter().find(|relocation| {
                relocation
                    .from
                    .iter()
                    .any(|position| local(*position) == expected.position())
            }) else {
                continue;
            };
            if states.is_dead(actor) || moves.len() == 32 {
                return Err(UseDisposition::Rejected);
            }
            moves.push(SourceSceneMove {
                actor,
                expected,
                destination: local(relocation.to),
            });
            sessions.push(session);
        }
        crate::map::floor::prepare_scene_relocations(
            runtime,
            self.movement_cells,
            map,
            moves,
            sessions,
        )
        .map(Some)
        .map_err(|error| {
            #[cfg(test)]
            eprintln!("QUEST_SCENE_WORLD_REFUSED {error:?}");
            let _ = error;
            UseDisposition::NotPossible
        })
    }
}
