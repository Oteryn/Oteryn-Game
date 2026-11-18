//! Crystal Faceless Bane's health resets and timed activation tiles.
use super::{NativePhaseOwner, Plan, Role, gates};
use oteryn_gameplay_entities::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use std::collections::BTreeMap;

const BOSS: &str = "oteryn:creature.faceless_bane";
const SPECTRES: [&str; 3] = [
    "oteryn:creature.burster_spectre",
    "oteryn:creature.gazer_spectre",
    "oteryn:creature.ripper_spectre",
];
const ROLES: [Role; 1] = [Role {
    creature: BOSS,
    after: 0,
}];
const REARM_US: u64 = 10_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileEffect {
    None,
    Charged,
    Released,
}

#[derive(Default, Debug)]
struct Tiles {
    charges: Option<u8>,
    rearm: BTreeMap<u64, u64>,
}

impl Tiles {
    fn step(&mut self, key: u64, now: u64) -> TileEffect {
        self.rearm.retain(|_, deadline| *deadline > now);
        if self.rearm.contains_key(&key) {
            return TileEffect::None;
        }
        match self.charges {
            None => {
                self.charges = Some(0);
                TileEffect::None
            }
            Some(5) => {
                self.charges = Some(0);
                TileEffect::Released
            }
            Some(charges) => {
                self.charges = Some(charges + 1);
                self.rearm.insert(key, now.saturating_add(REARM_US));
                TileEffect::Charged
            }
        }
    }
}

#[derive(Debug)]
pub struct FacelessBane {
    phase: NativePhaseOwner,
    boss: ExactActorRef,
    max_health: u64,
    resets: u8,
    shielded: bool,
    tiles: Tiles,
}

impl FacelessBane {
    pub fn bind(
        runtime: &ChannelRuntimeV1,
        participants: &[(ExactActorRef, GameSessionId)],
        boss: ExactActorRef,
    ) -> Result<Self, CarrierError> {
        let mut phase = NativePhaseOwner::bind(
            runtime,
            Plan {
                roles: &ROLES,
                terminal: 1,
            },
            participants,
        )?;
        phase.attach(runtime, 0, boss)?;
        let max_health = u64::try_from(runtime.companion_policy(BOSS)?.maximum_health)
            .map_err(|_| CarrierError::InvalidCreatureHealth)?;
        Ok(Self {
            phase,
            boss,
            max_health,
            resets: 0,
            shielded: false,
            tiles: Tiles::default(),
        })
    }

    pub fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.phase.current(runtime).is_ok()
            && (runtime.contains_live_creature(self.boss)
                || runtime.retains_corpse_projection(self.boss))
    }

    pub fn contains(&self, actor: ExactActorRef) -> bool {
        self.boss == actor
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

    pub fn step_tile(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        key: u64,
        now: u64,
    ) -> Result<TileEffect, CarrierError> {
        self.phase.current(runtime)?;
        if !self.phase.participants.contains(&(actor, session))
            || !runtime.matches_live_creature_identity(self.boss, BOSS.as_bytes())
        {
            return Ok(TileEffect::None);
        }
        let effect = self.tiles.step(key, now);
        if effect == TileEffect::Released && self.shielded {
            gates::unprotect(runtime, self.boss)?;
            self.shielded = false;
        }
        Ok(effect)
    }

    pub fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        map_admits: impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
    ) -> Result<(), CarrierError> {
        self.phase.current(runtime)?;
        if self.resets >= 4 || self.shielded || self.phase.progress.completed() {
            return Ok(());
        }
        let Some(health) = runtime.live_creature_health(self.boss) else {
            return Ok(());
        };
        if health > self.max_health / 5 {
            return Ok(());
        }
        let at = runtime.read_actor_position(self.boss)?.position();
        let content = runtime.content_pin().server_artifact_digest();
        gates::protect(runtime, &[self.boss])?;
        if let Err(error) = runtime.commit_source_creature_heal_batch(
            content,
            &[(self.boss, BOSS.into(), self.max_health, self.max_health)],
        ) {
            gates::unprotect(runtime, self.boss)?;
            return Err(error);
        }
        self.shielded = true;
        self.resets += 1;
        if map_admits(runtime, at) {
            // Crystal attempts each summon once at the boss's position.
            for key in SPECTRES {
                if let Ok(policy) = runtime.companion_policy(key) {
                    let _ =
                        runtime.commit_quest_creature_spawn(&[(key, policy.maximum_health, at)]);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faceless_tiles_rearm_after_ten_seconds_and_release_after_five_charges() {
        let mut tiles = Tiles::default();
        assert_eq!(tiles.step(1, 0), TileEffect::None);
        assert_eq!(tiles.step(1, 1), TileEffect::Charged);
        assert_eq!(tiles.step(1, REARM_US), TileEffect::None);
        assert_eq!(tiles.step(1, REARM_US + 1), TileEffect::Charged);
        for key in 2..=4 {
            assert_eq!(tiles.step(key, REARM_US + 1), TileEffect::Charged);
        }
        assert_eq!(tiles.step(5, REARM_US + 1), TileEffect::Released);
        assert_eq!(tiles.step(5, REARM_US + 1), TileEffect::Charged);
    }
}
