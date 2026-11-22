//! Crystal 201189f data-global/scripts/raids/bosses/man_in_the_cave.lua.
use super::*;
use crate::foundation::{MovementLocalPosition, MovementPositionSnapshot};
use crate::map::floor::CellBlocker;

const HOLE: MovementLocalPosition = MovementLocalPosition {
    x: 32131,
    y: 31147,
    floor: 3,
};
const LANDING: MovementLocalPosition = MovementLocalPosition {
    x: 32131,
    y: 31146,
    floor: 2,
};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn man_in_the_cave_step(
        &self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        step: MovementPositionSnapshot,
        blocking: &dyn CellBlocker,
    ) -> Option<MovementPositionSnapshot> {
        if step.position() != HOLE {
            return None;
        }
        let map = self.bundle_map()?;
        if !super::special_movement::tile_entries(map, HOLE)?
            .entries
            .iter()
            .any(|(_, entry)| matches!(entry.appearance_id, 799 | 6594))
        {
            return None;
        }
        let boss_present =
            runtime
                .positioned_melee_census()
                .ok()?
                .into_iter()
                .any(|(candidate, at, session)| {
                    let at = at.position();
                    session.is_none()
                        && at.floor == LANDING.floor
                        && (32124..=32142).contains(&at.x)
                        && (31141..=31153).contains(&at.y)
                        && runtime.matches_live_creature_identity(
                            candidate,
                            b"oteryn:creature.man_in_the_cave",
                        )
                });
        if !boss_present {
            return None;
        }

        let proof = crate::map::floor::prepare_quest_callback_teleport(
            runtime,
            self.movement_cells(),
            actor,
            session,
            step,
            LANDING,
            false,
            blocking,
        )
        .ok()?;
        let arrival = runtime.commit_source_step(proof).ok()?;
        let position = crate::map::overlay::TilePos {
            x: 32131,
            y: 31146,
            floor: -2,
        };
        map.facts.emit_movement_effect(position, 11);
        let captured_name = self.states.chat.speaker_name(session).and_then(|name| {
            oteryn_protocol_oteryn::entity_names::EntityDisplayName::new(&name).ok()
        });
        map.facts
            .emit_movement_speech(crate::map::facts::MovementSpeech {
                speaker: actor,
                position,
                talk_type: 36,
                text: "You have been roped up!".into(),
                captured_name,
            });
        Some(arrival)
    }
}
