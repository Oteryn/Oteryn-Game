//! Crystal 201189f8, data-global/scripts/quests/demon_helmet/movement-walls.lua.
use super::*;
use crate::map::floor::{SceneEvacuation, prepare_source_scene};
use crate::map::view::EntryFacts;

const PLATES: [Position; 2] = [position(33190, 31629), position(33191, 31629)];
const WALLS: [Position; 3] = [
    position(33210, 31630),
    position(33211, 31630),
    position(33212, 31630),
];
const DISPLACEMENT: Position = position(33211, 31631);

const fn position(x: u16, y: u16) -> Position {
    Position { x, y, floor: -13 }
}

fn source_entry(
    map: &BundleMap,
    at: Position,
    ordinal: u8,
    appearance: u16,
) -> Option<(u64, EntryFacts)> {
    let at = tile(at);
    if map.facts.house_tile(at) {
        return None;
    }
    let base = map.overlay.base().tile(at.x, at.y, at.floor)?;
    let (id, _) = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(usize::from(ordinal))?;
    let key = placement_key(at, ordinal)?;
    let entry = map.facts.base_entry(at, ordinal, *id)?;
    (entry.appearance_id == appearance && !entry.pickupable).then_some((key, entry))
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn step_demon_helmet_plates(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: MovementPositionSnapshot,
        entered: MovementPositionSnapshot,
    ) {
        if origin.position() == entered.position()
            || !PLATES.iter().any(|plate| {
                let at = local(*plate);
                at == origin.position() || at == entered.position()
            })
        {
            return;
        }
        let Some(map) = self.bundle_map() else { return };
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if runtime.read_actor_position(actor) == Ok(entered)
            && runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
            && !states.is_dead(actor)
        {
            self.update_demon_helmet_walls(&mut runtime, &mut states, map, false);
        }
    }

    pub(super) fn update_demon_helmet_walls(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
        closing_only: bool,
    ) {
        let [Some((left, wall)), Some((center, _)), Some((right, _))] =
            WALLS.map(|at| source_entry(map, at, 1, 1295))
        else {
            return;
        };
        let keys = [left, center, right];
        let hidden = keys.map(|key| map.facts.base_hidden(key));
        if (closing_only && hidden != [true; 3])
            || !PLATES.iter().all(|at| {
                source_entry(map, *at, 0, 10145).is_some_and(|(key, _)| !map.facts.base_hidden(key))
            })
            || runtime.owner_fence().is_err()
        {
            return;
        }
        let Ok(census) = runtime.positioned_melee_census() else {
            return;
        };
        let occupied = PLATES.iter().all(|at| {
            census.iter().any(|(actor, snapshot, session)| {
                snapshot.position() == local(*at) && session.is_some() && !states.is_dead(*actor)
            })
        });
        let publish = || {
            map.facts.apply_quest_callback(
                left,
                map.facts.object_revision(left),
                wall,
                &keys,
                &[],
                !occupied,
                None,
            )
        };
        if occupied {
            if !closing_only && hidden == [false; 3] {
                publish();
            }
            return;
        }
        if hidden != [true; 3] {
            return;
        }
        let landing = local(DISPLACEMENT);
        let moves = WALLS
            .iter()
            .map(|at| SceneEvacuation {
                from: local(*at),
                players_to: landing,
                creatures_to: landing,
                all: true,
            })
            .collect();
        let Ok(proof) =
            prepare_source_scene(runtime, &self.movement_cells, moves, map.facts.doors())
        else {
            return;
        };
        let observations = proof.observations();
        let Ok(committed) = runtime.commit_source_scene_relocations(proof, publish) else {
            return;
        };
        let now = self.owner_now().get();
        for ((actor, session, before), after) in observations.into_iter().zip(committed) {
            let Some(session) = session else { continue };
            states.stage_special_movement_step(
                runtime,
                map,
                actor,
                session,
                now,
                Some(before.position()),
                after.position(),
                None,
            );
            states.stage_underwater_step(
                runtime,
                actor,
                session,
                now,
                underwater(map, before.position()),
                underwater(map, after.position()),
            );
        }
    }
}
