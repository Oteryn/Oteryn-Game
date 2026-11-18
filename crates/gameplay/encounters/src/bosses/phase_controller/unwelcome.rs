//! The Unwelcome's source room links only the two native actors from its admission.
use super::{NativePhaseOwner, Plan, Role};
use oteryn_gameplay_entities::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
    OwnerDamageCommand,
};

pub const UNWELCOME: &str = "oteryn:creature.the_unwelcome";
pub const BROTHER_WORM: &str = "oteryn:creature.brother_worm";
const ROLES: &[Role] = &[
    Role {
        creature: UNWELCOME,
        after: 0,
    },
    Role {
        creature: BROTHER_WORM,
        after: 0,
    },
];

#[derive(Debug)]
pub struct UnwelcomeEncounter {
    phase: NativePhaseOwner,
    brother_at: MovementLocalPosition,
    brother_due_us: u64,
}

impl UnwelcomeEncounter {
    pub fn bind(
        runtime: &ChannelRuntimeV1,
        players: &[(ExactActorRef, GameSessionId)],
        boss: ExactActorRef,
        brother_at: MovementLocalPosition,
        now_us: u64,
    ) -> Result<Self, CarrierError> {
        let mut phase = NativePhaseOwner::bind(
            runtime,
            Plan {
                roles: ROLES,
                terminal: 0b11,
            },
            players,
        )?;
        phase.attach(runtime, 0, boss)?;
        Ok(Self {
            phase,
            brother_at,
            brother_due_us: now_us
                .checked_add(20_000_000)
                .ok_or(CarrierError::PlanConflict)?,
        })
    }

    pub fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.phase.current(runtime).is_ok()
    }

    pub fn contains(&self, actor: ExactActorRef) -> bool {
        self.phase.actors.contains(&Some(actor))
    }

    pub fn victors(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<&[(ExactActorRef, GameSessionId)]>, CarrierError> {
        self.phase.victors(runtime)
    }

    pub fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now_us: u64,
        map_admits: impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
    ) -> Result<(), CarrierError> {
        self.phase.current(runtime)?;
        if now_us >= self.brother_due_us && self.phase.actors[1].is_none() {
            if !runtime.native_summon_cell_free(self.brother_at) {
                return Err(CarrierError::PlanConflict);
            }
            self.phase
                .spawn_role(runtime, 1, self.brother_at, None, None, map_admits)?;
        }
        Ok(())
    }

    /// Crystal's Brother Worm death closes the remaining life of its peer. Neutral
    /// damage preserves The Unwelcome's own player contribution ledger.
    pub fn projected_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        now_ms: u64,
    ) -> Result<Option<ExactActorRef>, CarrierError> {
        self.phase.projected_death(runtime, actor)?;
        if self.phase.actors[1] != Some(actor) {
            return Ok(None);
        }
        let boss = self.phase.actors[0].ok_or(CarrierError::CreatureTargetMismatch)?;
        if !runtime.contains_live_creature(boss) {
            return Ok(None);
        }
        let snapshot = runtime.companion_snapshot(boss)?;
        let identity = actor.placement_identity();
        runtime
            .borrow_exact_actor_commit_at(now_ms)
            .commit_raw_creature_damage_batch(
                &[boss],
                OwnerDamageCommand {
                    target: UNWELCOME.as_bytes(),
                    occurrence: &identity,
                    binding: b"oteryn:unwelcome-linked-death",
                    damage: snapshot.health,
                },
                |results| {
                    if results[0].applied && results[0].health_after == 0 {
                        Ok(Some(boss))
                    } else {
                        Err(CarrierError::PlanConflict)
                    }
                },
            )
    }
}
