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

#[derive(Clone, Copy, Debug)]
enum Ejection {
    Defeat,
    Timeout,
}

#[derive(Clone, Copy, Debug)]
enum BattlefieldState {
    Fighting,
    Exiting(Ejection),
    AwaitingTimeout,
    Finished,
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
    state: BattlefieldState,
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
            state: BattlefieldState::Fighting,
            deadline: now_us
                .checked_add(u64::from(battlefield.timeout_ms) * 1000)
                .ok_or(CarrierError::PlanConflict)?,
        })
    }

    pub(super) fn contains_terminal(&self, actor: ExactActorRef) -> bool {
        self.phase.role_actor(DRUME) == Some(actor)
    }

    pub(super) fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        !matches!(self.state, BattlefieldState::Finished) && self.phase.is_current(runtime)
    }

    pub(super) fn blocks_channel_switch(&self) -> bool {
        matches!(
            self.state,
            BattlefieldState::Fighting | BattlefieldState::Exiting(_)
        )
    }

    pub(super) fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        room: &Encounter,
        cells: &crate::content::NativeEntryMovementCells,
    ) -> Result<(), CarrierError> {
        if matches!(self.state, BattlefieldState::Fighting)
            && self.phase.dead_roles() & ALL_LIONS == ALL_LIONS
        {
            // Crystal commander_kills.lua:8-25 selects defeat when the last
            // Lion dies. An obstructed exit must retain the selected outcome.
            self.state = BattlefieldState::Exiting(Ejection::Defeat);
        }
        if let BattlefieldState::Exiting(reason) = self.state {
            self.eject(runtime, room, cells, reason)?;
            self.state = match reason {
                Ejection::Defeat => BattlefieldState::AwaitingTimeout,
                Ejection::Timeout => BattlefieldState::Finished,
            };
            return Ok(());
        }
        if !matches!(self.state, BattlefieldState::Fighting) {
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
        // Crystal action_drume.lua:23-34,119-122 keeps the timer after defeat.
        // A subsequent successful lever admission replaces the scheduled clear.
        if now_us >= self.deadline {
            if matches!(self.state, BattlefieldState::AwaitingTimeout) {
                self.eject(runtime, room, cells, Ejection::Timeout)?;
                self.state = BattlefieldState::Finished;
                return Ok(());
            }
            if matches!(self.state, BattlefieldState::Fighting) {
                self.state = BattlefieldState::Exiting(Ejection::Timeout);
            }
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
        // Crystal commander_kills.lua:14-19 preserves monsters with a master.
        // The timeout in action_drume.lua:23-32 removes every monster in the room.
        let mut retired = Vec::new();
        for actor in runtime.current_live_creature_census()? {
            if !super::super::quest_callbacks::room_contains(
                room,
                runtime.read_actor_position(actor)?.position(),
            ) || (matches!(reason, Ejection::Defeat)
                && (runtime.native_summon_master(actor).is_some()
                    || runtime
                        .companion_snapshot(actor)
                        .is_ok_and(|snapshot| snapshot.state.master.is_some())))
            {
                continue;
            }
            retired.push(actor);
        }
        if !proofs.is_empty() || !retired.is_empty() {
            runtime.commit_quest_relocation_with_retirements(proofs, &retired, || true)?;
        }
        // Crystal commander_kills.lua:19 and action_drume.lua:31. Only
        // publish feedback after the whole battlefield exit commits.
        let notice = match reason {
            Ejection::Defeat => "You lost the skirmish.",
            Ejection::Timeout => "Your time is over.",
        };
        for session in recipients {
            self.feedback.push(session, notice);
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
