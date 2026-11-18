//! Crystal 201189f8, the_order_of_lion_quest/creaturescripts_commander_kills.lua:1-50.
use super::*;
use crate::gameplay_transport::chat_intent::QuestFeedback;
use oteryn_gameplay_encounters::bosses::phase_controller::{
    NativePhaseOwner, PhaseDecoration, Plan, Role,
};
use oteryn_gameplay_quests::quest::world_callbacks::{Battlefield, Encounter};

const LION: &str = "oteryn:creature.lion_commander";
const USURPER: &str = "oteryn:creature.usurper_commander";
const KESAR: &str = "oteryn:creature.kesar";
const ALL_LIONS: u64 = 0b000_0111;
const ALL_USURPERS: u64 = 0b011_1000;
const DRUME: usize = 6;
const TELEPORT_EFFECT: u16 = 11;

#[derive(Clone, Copy)]
enum Ejection {
    Defeat,
    Timeout,
}

const ROLES: &[Role] = &[
    Role {
        creature: LION,
        after: 0,
    },
    Role {
        creature: LION,
        after: 0,
    },
    Role {
        creature: LION,
        after: 0,
    },
    Role {
        creature: USURPER,
        after: 0,
    },
    Role {
        creature: USURPER,
        after: 0,
    },
    Role {
        creature: USURPER,
        after: 0,
    },
    Role {
        creature: "oteryn:creature.drume",
        after: ALL_USURPERS,
    },
];

#[derive(Debug)]
pub(super) struct Skirmish {
    pub(super) phase: NativePhaseOwner,
    battlefield: Battlefield,
    feedback: QuestFeedback,
    pub(super) ejected: bool,
    deadline: u64,
}

impl Skirmish {
    pub(super) fn bind(
        runtime: &ChannelRuntimeV1,
        participants: &[(ExactActorRef, GameSessionId)],
        actors: &[ExactActorRef],
        battlefield: Battlefield,
        feedback: QuestFeedback,
        now_us: u64,
    ) -> Result<Self, CarrierError> {
        let mut phase = NativePhaseOwner::bind(
            runtime,
            Plan {
                roles: ROLES,
                terminal: 1 << DRUME,
            },
            participants,
        )?;
        for (species, start) in [(LION, 0), (USURPER, 3)] {
            let mut squad = actors
                .iter()
                .copied()
                .filter(|actor| runtime.matches_live_creature_identity(*actor, species.as_bytes()));
            for role in start..start + 3 {
                phase.attach(
                    runtime,
                    role,
                    squad.next().ok_or(CarrierError::CreatureTargetMismatch)?,
                )?;
            }
            if squad.next().is_some() {
                return Err(CarrierError::CreatureTargetMismatch);
            }
        }
        Ok(Self {
            phase,
            battlefield,
            feedback,
            ejected: false,
            deadline: now_us
                .checked_add(u64::from(battlefield.timeout_ms) * 1000)
                .ok_or(CarrierError::PlanConflict)?,
        })
    }

    pub(super) fn contains_terminal(&self, actor: ExactActorRef) -> bool {
        self.phase.role_actor(DRUME) == Some(actor)
    }

    pub(super) fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        room: &Encounter,
        cells: &crate::content::NativeEntryMovementCells,
    ) -> Result<(), CarrierError> {
        if self.ejected {
            return Ok(());
        }
        if self.phase.dead_roles() & ALL_LIONS == ALL_LIONS {
            self.eject(runtime, room, cells, Ejection::Defeat)?;
            self.ejected = true;
            return Ok(());
        }
        if self.phase.pending().any(|(role, _)| role == DRUME) {
            self.phase.spawn_role(
                runtime,
                DRUME,
                local(self.battlefield.boss),
                None,
                // Crystal commander_kills.lua:45-50 makes Kesar immune from spawn.
                Some(PhaseDecoration {
                    creature: KESAR,
                    position: local(self.battlefield.captain),
                    damage_protected: true,
                }),
                |runtime, at| placement(cells, runtime, at),
            )?;
        }
        Ok(())
    }

    pub(super) fn maintain(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        room: &Encounter,
        cells: &crate::content::NativeEntryMovementCells,
        now_us: u64,
    ) -> Result<(), CarrierError> {
        // Crystal action_drume.lua:16-17,21-32,115-120 schedules physical teardown
        // even when the boss has fallen and normal terminal credit already settled.
        if now_us >= self.deadline {
            self.eject(runtime, room, cells, Ejection::Timeout)?;
            self.ejected = true;
            return Ok(());
        }
        self.advance(runtime, room, cells)
    }

    fn eject(
        &self,
        runtime: &mut ChannelRuntimeV1,
        room: &Encounter,
        cells: &crate::content::NativeEntryMovementCells,
        reason: Ejection,
    ) -> Result<(), CarrierError> {
        use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
        let NativeMovementCollisionIndex::Bundle(index) = cells.index() else {
            return Err(CarrierError::PlanConflict);
        };
        let map = index.map();
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let exit = local(self.battlefield.exit);
        let census = runtime.positioned_melee_census()?;
        let mut proofs = Vec::new();
        let mut recipients = Vec::new();
        for (actor, position, session) in census {
            if let Some(session) = session
                && super::super::quest_callbacks::room_contains(room, position.position())
            {
                proofs.push(
                    crate::map::floor::prepare_quest_encounter_ejection(
                        runtime, cells, actor, session, position, exit, &closed,
                    )
                    .map_err(|_| CarrierError::PlanConflict)?,
                );
                recipients.push(session);
            }
        }
        if !proofs.is_empty() {
            runtime.commit_quest_relocation(proofs, || true)?;
        }
        // The successful move consumes this player's room presence. If cleanup
        // retries after a creature-removal error, these sessions are already outside.
        // Crystal commander_kills.lua:19 and action_drume.lua:31.
        let notice = match reason {
            Ejection::Defeat => "You lost the skirmish.",
            Ejection::Timeout => "Your time is over.",
        };
        for session in recipients {
            self.feedback.push(session, notice);
        }
        // Crystal commander_kills.lua:14-19 preserves monsters with a master.
        // The timeout in action_drume.lua:23-32 removes every monster in the room.
        for actor in runtime.current_live_creature_census()? {
            // An earlier removal can also retire the removed master's children.
            if !runtime.contains_live_creature(actor)
                || !super::super::quest_callbacks::room_contains(
                    room,
                    runtime.read_actor_position(actor)?.position(),
                )
                || (matches!(reason, Ejection::Defeat)
                    && (runtime.native_summon_master(actor).is_some()
                        || runtime
                            .companion_snapshot(actor)
                            .is_ok_and(|snapshot| snapshot.state.master.is_some())))
            {
                continue;
            }
            runtime.remove_encounter_actor(actor)?;
        }
        // Crystal commander_kills.lua:23 emits a single effect at the exit.
        if matches!(reason, Ejection::Defeat) {
            map.facts.emit_movement_effect(
                crate::map::overlay::TilePos {
                    x: self.battlefield.exit.x,
                    y: self.battlefield.exit.y,
                    floor: self.battlefield.exit.floor,
                },
                TELEPORT_EFFECT,
            );
        }
        Ok(())
    }
}

pub(super) fn local(
    position: oteryn_gameplay_quests::quest::world_callbacks::Position,
) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(position.x),
        y: i32::from(position.y),
        floor: -i16::from(position.floor),
    }
}
