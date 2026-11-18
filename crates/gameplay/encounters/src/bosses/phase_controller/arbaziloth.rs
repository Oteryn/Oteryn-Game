//! Arbaziloth encounter ownership and timed replacement.
use super::{NativePhaseOwner, Plan, Role};
pub use oteryn_gameplay_entities::carrier::arbaziloth::ShieldSequence;
use oteryn_gameplay_entities::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
};

pub const BOSS: &str = "oteryn:creature.arbaziloth";
pub const FORGEMASTER: &str = "oteryn:creature.the_forgemaster";
const ENRAGED: &str = "oteryn:creature.enraged_arbaziloth";
const ENRAGED_ROLES: &[Role] = &[Role {
    creature: ENRAGED,
    after: 0,
}];

/// The replacement remains bound to the exact actors admitted for this attempt.
#[derive(Debug)]
pub struct ArbazilothEncounter {
    phase: NativePhaseOwner,
    boss: ExactActorRef,
    forgemaster: ExactActorRef,
    enraged: Option<ExactActorRef>,
}

impl ArbazilothEncounter {
    pub fn bind(
        runtime: &mut ChannelRuntimeV1,
        players: &[(ExactActorRef, GameSessionId)],
        boss: ExactActorRef,
        forgemaster: ExactActorRef,
    ) -> Result<Self, CarrierError> {
        if !runtime.matches_live_creature_identity(boss, BOSS.as_bytes())
            || !runtime.matches_live_creature_identity(forgemaster, FORGEMASTER.as_bytes())
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        runtime.companion_policy(ENRAGED)?;
        let phase = NativePhaseOwner::bind(
            runtime,
            Plan {
                roles: ENRAGED_ROLES,
                terminal: 1,
            },
            players,
        )?;
        runtime.bind_arbaziloth_shields(boss)?;
        Ok(Self {
            phase,
            boss,
            forgemaster,
            enraged: None,
        })
    }

    pub fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.phase.current(runtime).is_ok()
            && self.enraged.map_or_else(
                || runtime.contains_live_creature(self.boss),
                |enraged| {
                    runtime.contains_live_creature(enraged)
                        || runtime.retains_corpse_projection(enraged)
                },
            )
    }

    pub fn contains_terminal(&self, actor: ExactActorRef) -> bool {
        self.enraged == Some(actor)
    }

    pub fn projected_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        let recorded = self.phase.projected_death(runtime, actor)?;
        if recorded {
            runtime.open_arbaziloth_doomforge(actor)?;
        }
        Ok(recorded)
    }

    pub fn victors<'a>(
        &'a self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<&'a [(ExactActorRef, GameSessionId)]>, CarrierError> {
        self.phase.victors(runtime)
    }

    pub fn normal_boss(&self) -> Option<ExactActorRef> {
        self.enraged.is_none().then_some(self.boss)
    }

    /// Crystal removes the Forgemaster and normal boss when the delayed event
    /// fires, then puts the enraged form at the original boss's current tile.
    pub fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now_ms: u64,
        map_admits: impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
    ) -> Result<Option<ExactActorRef>, CarrierError> {
        if self.enraged.is_some()
            || !runtime
                .arbaziloth_shields(self.boss)
                .is_some_and(|shields| shields.forgemaster_departure_due(now_ms))
            || !runtime.matches_live_creature_identity(self.boss, BOSS.as_bytes())
            || !runtime.matches_live_creature_identity(self.forgemaster, FORGEMASTER.as_bytes())
        {
            return Ok(None);
        }
        let position = runtime.read_actor_position(self.boss)?.position();
        let enraged = self.phase.spawn_role(
            runtime,
            0,
            position,
            &[self.boss, self.forgemaster],
            None,
            map_admits,
        )?;
        self.enraged = Some(enraged);
        Ok(Some(enraged))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arbaziloth_four_distinct_shields_qualify_timed_departure() {
        let mut sequence = ShieldSequence::default();
        for health in [100, 80, 60, 40] {
            assert_eq!(sequence.limit_damage(health, 100, 100), 20);
            assert_eq!(sequence.limit_damage(health - 20, 100, 100), 0);
            assert!(!sequence.use_rod(51, 100));
            assert!(sequence.step_on_fire());
            assert!(!sequence.step_on_fire());
            assert!(!sequence.use_rod(50, 100));
            assert!(sequence.use_rod(51, 100));
        }
        assert_eq!(sequence.limit_damage(20, 100, 20), 20);
        assert!(!sequence.forgemaster_departure_due(3_099));
        assert!(sequence.forgemaster_departure_due(3_100));
    }
}
