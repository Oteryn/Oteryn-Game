//! First-fight Mimar adapter for the shared native boss phase owner.
use super::{NativePhaseOwner, Plan, Role};
pub use oteryn_gameplay_entities::carrier::mimar::{BOSS, Cannon};
use oteryn_gameplay_entities::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
};

const MAX_PLAYERS: usize = 5;
const ROLES: &[Role] = &[Role {
    creature: BOSS,
    after: 0,
}];

#[derive(Debug)]
pub struct MimarEncounter {
    phase: NativePhaseOwner,
    boss: ExactActorRef,
    center: MovementLocalPosition,
}

impl MimarEncounter {
    /// Bind the boss already placed by the encounter's qualified spawn owner.
    pub fn bind(
        runtime: &mut ChannelRuntimeV1,
        players: &[(ExactActorRef, GameSessionId)],
        boss: ExactActorRef,
        now_ms: u64,
    ) -> Result<Self, CarrierError> {
        if players.len() > MAX_PLAYERS {
            return Err(CarrierError::PlanConflict);
        }
        let mut phase = NativePhaseOwner::bind(
            runtime,
            Plan {
                roles: ROLES,
                terminal: 1,
            },
            players,
        )?;
        phase.attach(runtime, 0, boss)?;
        let center = runtime.read_actor_position(boss)?.position();
        runtime.bind_mimar_shield(boss, now_ms)?;
        Ok(Self {
            phase,
            boss,
            center,
        })
    }

    /// The item/map owner resolves the exact cannon in the encounter room and
    /// provides its already-fenced consumption operation in this owner turn.
    pub fn load_cannon(
        &self,
        runtime: &mut ChannelRuntimeV1,
        player: (ExactActorRef, GameSessionId),
        cannon: Cannon,
        now_ms: u64,
        consume: impl FnOnce(&str) -> bool,
    ) -> Result<bool, CarrierError> {
        self.phase.current(runtime)?;
        if !self.phase.participants.contains(&player)
            || runtime
                .player_control_facts(player.0, player.1)?
                .control_loss
                .is_some()
        {
            return Err(CarrierError::PlanConflict);
        }
        runtime.load_mimar_cannon(self.boss, cannon, now_ms, consume)
    }

    /// Keep the exact boss incarnation and its remaining HP through relocking.
    pub fn advance(
        &self,
        runtime: &mut ChannelRuntimeV1,
        now_ms: u64,
        map_admits: impl FnOnce(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
    ) -> Result<(), CarrierError> {
        self.phase.current(runtime)?;
        if runtime.contains_live_creature(self.boss) {
            runtime.advance_mimar_shield(self.boss, self.center, now_ms, map_admits)?;
        }
        Ok(())
    }

    pub fn projected_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        self.phase.projected_death(runtime, actor)
    }

    pub fn victors<'a>(
        &'a self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<&'a [(ExactActorRef, GameSessionId)]>, CarrierError> {
        self.phase.victors(runtime)
    }

    pub fn retire(&self, runtime: &mut ChannelRuntimeV1) -> Result<(), CarrierError> {
        self.phase.current(runtime)?;
        if runtime.contains_live_creature(self.boss) {
            runtime.remove_encounter_actor(self.boss)?;
        }
        Ok(())
    }
}
