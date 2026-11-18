//! Crystal 201189f grave_danger_king_zelos.lua:105–160. The north
//! encounter becomes available only after all four knights and their successors die.
use super::{NativePhaseOwner, Plan, Role};
use oteryn_gameplay_entities::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
};

const MAGNOR: usize = 0;
const NARGOL: usize = 1;
const MASS: usize = 8;
const KING: usize = 9;
const REGENERATION_US: u64 = 30_000_000;
const HALF_AFTER_US: u64 = 6 * 60 * 1_000_000;
const LIMIT_US: u64 = 24 * 60 * 1_000_000;

const ROLES: [Role; 10] = [
    Role {
        creature: "oteryn:creature.magnor_mournbringer",
        after: 0,
    },
    Role {
        creature: "oteryn:creature.nargol_the_impaler",
        after: 0,
    },
    Role {
        creature: "oteryn:creature.rewar_the_bloody",
        after: 0,
    },
    Role {
        creature: "oteryn:creature.the_red_knight",
        after: 0,
    },
    Role {
        creature: "oteryn:creature.shard_of_magnor",
        after: 1,
    },
    Role {
        creature: "oteryn:creature.shard_of_magnor",
        after: 1,
    },
    Role {
        creature: "oteryn:creature.shard_of_magnor",
        after: 1,
    },
    Role {
        creature: "oteryn:creature.shard_of_magnor",
        after: 1,
    },
    Role {
        creature: "oteryn:creature.regenerating_mass",
        after: 1 << NARGOL,
    },
    Role {
        creature: "oteryn:creature.king_zelos",
        after: (1 << KING) - 1,
    },
];

#[derive(Debug)]
pub struct ZelosEncounter {
    phase: NativePhaseOwner,
    start_us: u64,
    mass_due_us: Option<u64>,
    ritual_completed_us: Option<u64>,
    magnor_at: MovementLocalPosition,
    nargol_at: MovementLocalPosition,
    king_at: MovementLocalPosition,
}
impl ZelosEncounter {
    pub fn bind(
        runtime: &ChannelRuntimeV1,
        players: &[(ExactActorRef, GameSessionId)],
        actors: &[ExactActorRef],
        king_at: MovementLocalPosition,
        now: u64,
    ) -> Result<Self, CarrierError> {
        if actors.len() != 4 || players.len() > 10 {
            return Err(CarrierError::PlanConflict);
        }
        let mut phase = NativePhaseOwner::bind(
            runtime,
            Plan {
                roles: &ROLES,
                terminal: 1 << KING,
            },
            players,
        )?;
        for (role, def) in ROLES[..4].iter().enumerate() {
            let actor = actors
                .iter()
                .copied()
                .find(|a| runtime.matches_live_creature_identity(*a, def.creature.as_bytes()))
                .ok_or(CarrierError::CreatureTargetMismatch)?;
            phase.attach(runtime, role, actor)?;
        }
        for role in &ROLES[4..] {
            runtime.companion_policy(role.creature)?;
        }
        let position = |role: usize| {
            runtime
                .read_actor_position(phase.actors[role].ok_or(CarrierError::PlanConflict)?)
                .map(|snapshot| snapshot.position())
        };
        Ok(Self {
            magnor_at: position(MAGNOR)?,
            nargol_at: position(NARGOL)?,
            king_at,
            phase,
            start_us: now,
            mass_due_us: None,
            ritual_completed_us: None,
        })
    }

    pub fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.phase.current(runtime).is_ok()
    }

    pub fn contains(&self, actor: ExactActorRef) -> bool {
        self.phase.actors.contains(&Some(actor))
    }

    pub fn contains_terminal(&self, actor: ExactActorRef) -> bool {
        self.phase.actors[KING] == Some(actor)
    }

    pub fn projected_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        self.phase.projected_death(runtime, actor)
    }

    pub fn victors(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<&[(ExactActorRef, GameSessionId)]>, CarrierError> {
        self.phase.victors(runtime)
    }

    pub fn timeout_participants(&self, now: u64) -> Option<&[(ExactActorRef, GameSessionId)]> {
        (now.saturating_sub(self.start_us) >= LIMIT_US).then_some(&self.phase.participants)
    }

    pub fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now: u64,
        place: impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
    ) -> Result<bool, CarrierError> {
        self.phase.current(runtime)?;
        if self.timeout_participants(now).is_some() {
            for actor in self.phase.actors.iter().copied().flatten() {
                if runtime.contains_live_creature(actor) {
                    runtime.remove_encounter_actor(actor)?;
                }
            }
            return Ok(false);
        }
        if let Some(deadline) = self.mass_due_us
            && now >= deadline
            && self.phase.progress.dead() & (1 << MASS) == 0
        {
            let mut retry = self.phase.clone();
            retry.actors[NARGOL] = None;
            retry.actors[MASS] = None;
            retry.progress.dead &= !(1 << NARGOL);
            retry.spawn_role(
                runtime,
                NARGOL,
                self.nargol_at,
                self.phase.actors[MASS],
                None,
                &place,
            )?;
            self.phase = retry;
            self.mass_due_us = None;
        }
        loop {
            let Some(role) = self.phase.pending().next().map(|(role, _)| role) else {
                break;
            };
            let at = match role {
                4..=7 => {
                    let (dx, dy) = [(1, 0), (0, 1), (-1, 0), (0, -1)][role - 4];
                    MovementLocalPosition {
                        x: self.magnor_at.x + dx,
                        y: self.magnor_at.y + dy,
                        ..self.magnor_at
                    }
                }
                MASS => self.nargol_at,
                KING => {
                    let completed = *self.ritual_completed_us.get_or_insert(now);
                    if now < completed.saturating_add(300_000) {
                        break;
                    }
                    self.king_at
                }
                _ => return Err(CarrierError::PlanConflict),
            };
            self.phase
                .spawn_role(runtime, role, at, None, None, &place)?;
            if role == MASS {
                self.mass_due_us = Some(now.saturating_add(REGENERATION_US));
            }
        }
        Ok(true)
    }
}
