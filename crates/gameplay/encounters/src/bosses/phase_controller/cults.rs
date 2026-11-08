//! Crystal 9f5a72c: cults_of_tibia/creaturescripts_destroyed_pillar.lua and
//! creaturescripts_essence_of_malice.lua. Five independent pillar/guardian branches
//! join before Essence. Spawn/removal positions remain with the source map hook.
use super::{Plan, Role};

const ROLES: [Role; 11] = [
    Role {
        creature: "oteryn:creature.pillar_of_summoning",
        after: 0,
    },
    Role {
        creature: "oteryn:creature.pillar_of_death",
        after: 0,
    },
    Role {
        creature: "oteryn:creature.pillar_of_protection",
        after: 0,
    },
    Role {
        creature: "oteryn:creature.pillar_of_healing",
        after: 0,
    },
    Role {
        creature: "oteryn:creature.pillar_of_draining",
        after: 0,
    },
    Role {
        creature: "oteryn:creature.eshtaba_the_conjurer",
        after: 1,
    },
    Role {
        creature: "oteryn:creature.malkhar_deathbringer",
        after: 2,
    },
    Role {
        creature: "oteryn:creature.eliz_the_unyielding",
        after: 4,
    },
    Role {
        creature: "oteryn:creature.mezlon_the_defiler",
        after: 8,
    },
    Role {
        creature: "oteryn:creature.dorokoll_the_mystic",
        after: 16,
    },
    Role {
        creature: "oteryn:creature.essence_of_malice",
        after: 992,
    },
];
pub fn plan() -> Plan {
    Plan {
        roles: &ROLES,
        terminal: 1 << 10,
    }
}

use super::NativePhaseOwner;
use oteryn_gameplay_entities::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
};

const STOPS: [&str; 5] = [
    "oteryn:creature.eshtaba_the_conjurer_stop",
    "oteryn:creature.malkhar_deathbringer_stop",
    "oteryn:creature.eliz_the_unyielding_stop",
    "oteryn:creature.mezlon_the_defiler_stop",
    "oteryn:creature.dorokoll_the_mystic_stop",
];

/// Small source hook on the shared phase owner: destroyed pillars replace the five
/// stationary guardians; only those five exact replacement deaths release Essence.
#[derive(Debug)]
pub struct CultsEncounter {
    phase: NativePhaseOwner,
    stops: [ExactActorRef; 5],
    pillars_at: [MovementLocalPosition; 5],
    guardians_at: [MovementLocalPosition; 5],
    essence_at: MovementLocalPosition,
}
impl CultsEncounter {
    pub fn bind(
        runtime: &mut ChannelRuntimeV1,
        participants: &[(ExactActorRef, GameSessionId)],
        pillars: [ExactActorRef; 5],
        stops: [ExactActorRef; 5],
        essence_at: MovementLocalPosition,
    ) -> Result<Self, CarrierError> {
        let mut phase = NativePhaseOwner::bind(runtime, plan(), participants)?;
        let mut pillars_at = [essence_at; 5];
        let mut guardians_at = [essence_at; 5];
        for i in 0..5 {
            phase.attach(runtime, i, pillars[i])?;
            if !runtime.matches_live_creature_identity(stops[i], STOPS[i].as_bytes())
                || stops[..i].contains(&stops[i])
                || pillars.contains(&stops[i])
            {
                return Err(CarrierError::CreatureTargetMismatch);
            }
            pillars_at[i] = runtime.read_actor_position(pillars[i])?.position();
            guardians_at[i] = runtime.read_actor_position(stops[i])?.position();
        }
        super::gates::protect(runtime, &stops)?;
        Ok(Self {
            phase,
            stops,
            pillars_at,
            guardians_at,
            essence_at,
        })
    }
    pub fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.phase.current(runtime).is_ok()
    }
    pub fn contains(&self, actor: ExactActorRef) -> bool {
        self.phase.actors.contains(&Some(actor))
    }
    pub fn projected_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        self.phase.projected_death(runtime, actor)
    }
    /// The caller keeps the current map/owner turn across this operation. Failed
    /// replacements remain pending and can retry without recounting any death.
    pub fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        map_admits: impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
    ) -> Result<(), CarrierError> {
        let pending: Vec<_> = self.phase.pending().map(|(role, _)| role).collect();
        for role in pending {
            match role {
                5..=9 => {
                    let i = role - 5;
                    // A recycled or unrelated actor must never be removed as the guardian.
                    if !runtime.matches_live_creature_identity(self.stops[i], STOPS[i].as_bytes()) {
                        return Err(CarrierError::StaleActorGeneration);
                    }
                    self.phase.spawn_role(
                        runtime,
                        role,
                        self.guardians_at[i],
                        Some(self.stops[i]),
                        Some(("oteryn:creature.destroyed_pillar", self.pillars_at[i])),
                        &map_admits,
                    )?;
                }
                10 => {
                    if !runtime.native_summon_cell_free(self.essence_at) {
                        return Err(CarrierError::PlanConflict);
                    }
                    self.phase.spawn_role(
                        runtime,
                        role,
                        self.essence_at,
                        None,
                        None,
                        &map_admits,
                    )?;
                }
                _ => return Err(CarrierError::PlanConflict),
            }
        }
        Ok(())
    }
    pub fn victors(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<&[(ExactActorRef, GameSessionId)]>, CarrierError> {
        self.phase.victors(runtime)
    }
}
