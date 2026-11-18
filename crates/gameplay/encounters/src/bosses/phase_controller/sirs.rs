//! Crystal 201189f actions_sir_baeloc_sir_nictros_lever.lua and creaturescripts_sirs.lua.
use super::{NativePhaseOwner, Plan, Role, gates};
use oteryn_gameplay_entities::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
};

const NICTROS: &str = "oteryn:creature.sir_nictros";
const BAELOC: &str = "oteryn:creature.sir_baeloc";
const ROLES: [Role; 2] = [
    Role {
        creature: NICTROS,
        after: 0,
    },
    Role {
        creature: BAELOC,
        after: 0,
    },
];
const INTRO_US: u64 = 5_000_000;
const BALANCE_US: u64 = 2_000_000;
const LIMIT_US: u64 = 20 * 60 * 1_000_000;

#[derive(Debug)]
enum Stage {
    Introduction { due: u64 },
    Nictros,
    Baeloc,
    Together { due: u64 },
}

#[derive(Debug)]
pub struct SirsEncounter {
    owner: NativePhaseOwner,
    nictros: ExactActorRef,
    baeloc: ExactActorRef,
    nictros_hold: MovementLocalPosition,
    fight: [MovementLocalPosition; 3],
    expires: u64,
    stage: Stage,
}

impl SirsEncounter {
    pub fn bind(
        runtime: &mut ChannelRuntimeV1,
        players: &[(ExactActorRef, GameSessionId)],
        actors: &[ExactActorRef],
        fight: [MovementLocalPosition; 3],
        now: u64,
    ) -> Result<Self, CarrierError> {
        if actors.len() != 2 || players.len() > 5 {
            return Err(CarrierError::PlanConflict);
        }
        let mut owner = NativePhaseOwner::bind(
            runtime,
            Plan {
                roles: &ROLES,
                terminal: 3,
            },
            players,
        )?;
        for (i, role) in ROLES.iter().enumerate() {
            let actor = actors
                .iter()
                .copied()
                .find(|&actor| {
                    runtime.matches_live_creature_identity(actor, role.creature.as_bytes())
                })
                .ok_or(CarrierError::CreatureTargetMismatch)?;
            owner.attach(runtime, i, actor)?;
        }
        let (Some(nictros), Some(baeloc)) = (owner.actors[0], owner.actors[1]) else {
            return Err(CarrierError::PlanConflict);
        };
        let nictros_hold = runtime.read_actor_position(nictros)?.position();
        gates::protect(runtime, &[nictros, baeloc])?;
        Ok(Self {
            owner,
            nictros,
            baeloc,
            nictros_hold,
            fight,
            expires: now.saturating_add(LIMIT_US),
            stage: Stage::Introduction {
                due: now.saturating_add(INTRO_US),
            },
        })
    }

    pub fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.owner.current(runtime).is_ok()
    }

    pub fn contains(&self, actor: ExactActorRef) -> bool {
        [self.nictros, self.baeloc].contains(&actor)
    }

    pub fn projected_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        self.owner.projected_death(runtime, actor)
    }

    pub fn victors(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<&[(ExactActorRef, GameSessionId)]>, CarrierError> {
        self.owner.victors(runtime)
    }

    pub fn next_due_us(&self) -> u64 {
        match self.stage {
            Stage::Introduction { due } | Stage::Together { due } => due.min(self.expires),
            Stage::Nictros | Stage::Baeloc => self.expires,
        }
    }

    pub fn timeout_participants(&self, now: u64) -> Option<&[(ExactActorRef, GameSessionId)]> {
        (now >= self.expires).then_some(&self.owner.participants)
    }

    pub fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now: u64,
        relocate: impl Fn(
            &mut ChannelRuntimeV1,
            ExactActorRef,
            MovementLocalPosition,
        ) -> Result<(), CarrierError>,
    ) -> Result<bool, CarrierError> {
        self.owner.current(runtime)?;
        if self.timeout_participants(now).is_some() {
            for actor in [self.nictros, self.baeloc] {
                if runtime.contains_live_creature(actor) {
                    runtime.remove_encounter_actor(actor)?;
                }
            }
            return Ok(false);
        }
        let below_sixty = |actor| {
            runtime
                .companion_snapshot(actor)
                .is_ok_and(|p| i128::from(p.health) * 5 < i128::from(p.maximum_health) * 3)
        };
        let next = match self.stage {
            Stage::Introduction { due } if now >= due => {
                let before = runtime.encounter_parts().0.clone();
                if let Err(error) = relocate(runtime, self.nictros, self.fight[0])
                    .and_then(|()| gates::unprotect(runtime, self.nictros))
                {
                    *runtime.encounter_parts_mut().0 = before;
                    return Err(error);
                }
                Stage::Nictros
            }
            Stage::Nictros if below_sixty(self.nictros) => {
                let before = runtime.encounter_parts().0.clone();
                if let Err(error) = (|| {
                    relocate(runtime, self.nictros, self.nictros_hold)?;
                    relocate(runtime, self.baeloc, self.fight[1])?;
                    gates::protect(runtime, &[self.nictros])?;
                    gates::unprotect(runtime, self.baeloc)
                })() {
                    *runtime.encounter_parts_mut().0 = before;
                    return Err(error);
                }
                Stage::Baeloc
            }
            Stage::Baeloc if below_sixty(self.baeloc) => {
                let before = runtime.encounter_parts().0.clone();
                if let Err(error) = relocate(runtime, self.nictros, self.fight[2])
                    .and_then(|()| gates::unprotect(runtime, self.nictros))
                {
                    *runtime.encounter_parts_mut().0 = before;
                    return Err(error);
                }
                Stage::Together {
                    due: now.saturating_add(BALANCE_US),
                }
            }
            Stage::Together { due } if now >= due => {
                self.balance(runtime)?;
                Stage::Together {
                    due: now.saturating_add(BALANCE_US),
                }
            }
            _ => return Ok(true),
        };
        self.stage = next;
        Ok(true)
    }

    fn balance(&self, runtime: &mut ChannelRuntimeV1) -> Result<(), CarrierError> {
        let (Ok(nictros), Ok(baeloc)) = (
            runtime.companion_snapshot(self.nictros),
            runtime.companion_snapshot(self.baeloc),
        ) else {
            return Ok(());
        };
        let left = i128::from(nictros.health) * i128::from(baeloc.maximum_health);
        let right = i128::from(baeloc.health) * i128::from(nictros.maximum_health);
        let scale = i128::from(nictros.maximum_health) * i128::from(baeloc.maximum_health);
        if (left - right).abs() <= scale / 100 * 15 + scale % 100 * 15 / 100 {
            return Ok(());
        }
        let lower = if left < right { nictros } else { baeloc };
        runtime.commit_source_creature_heal_batch(
            runtime.content_pin().server_artifact_digest(),
            &[(
                lower.actor,
                lower.state.policy.definition_key.clone(),
                u64::try_from(lower.maximum_health).map_err(|_| CarrierError::PlanConflict)?,
                18_000,
            )],
        )?;
        Ok(())
    }
}
