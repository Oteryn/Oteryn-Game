use super::*;
use crate::map::floor::{PreparedSceneRelocations, SceneEvacuation, prepare_source_scene};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn prepare_quest_scene_moves<'a>(
        &'a self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        map: &'a BundleMap,
        callback: &Callback,
        close: bool,
    ) -> Result<Option<PreparedSceneRelocations<'a>>, UseDisposition> {
        let Some(scene) = callback.effects.scene.as_ref() else {
            return Ok(None);
        };
        let mut sources = scene
            .relocations
            .iter()
            .flat_map(|m| {
                m.from.iter().map(|from| SceneEvacuation {
                    from: local(*from),
                    players_to: local(m.to),
                    creatures_to: local(m.to),
                    all: true,
                })
            })
            .collect::<Vec<_>>();
        sources.extend(
            (if close {
                &scene.evacuate_close
            } else {
                &scene.evacuate_open
            })
            .iter()
            .map(|m| SceneEvacuation {
                from: local(m.from),
                players_to: local(m.players_to),
                creatures_to: local(m.creatures_to),
                all: m.all,
            }),
        );
        if sources.is_empty() {
            return Ok(None);
        }
        if scene
            .relocations
            .iter()
            .any(|m| map.facts.house_tile(tile(m.to)))
        {
            return Err(UseDisposition::Rejected);
        }
        let proof = prepare_source_scene(runtime, &self.movement_cells, sources, map.facts.doors())
            .map_err(|_| UseDisposition::NotPossible)?;
        if proof.observations().iter().any(|(actor, session, _)| {
            session.is_some_and(|session| {
                states.is_dead(*actor) || !states.spell_idle(runtime, *actor, session)
            })
        }) {
            return Err(UseDisposition::NotPossible);
        }
        Ok(Some(proof))
    }
}
