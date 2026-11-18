//! Crystal 201189f grave_danger_earl_osam.lua:67–144.
use super::{NativePhaseOwner, Plan, Role, gates};
use oteryn_gameplay_entities::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
};

const EARL: &str = "oteryn:creature.earl_osam";
const CHANNELING: &str = "oteryn:creature.channeling_earl_osam";
const SPHERE: &str = "oteryn:creature.magical_sphere";
const ROLES: [Role; 1] = [Role {
    creature: EARL,
    after: 0,
}];
const LIMIT_US: u64 = 20 * 60 * 1_000_000;
const CHANNEL_DELAY_US: u64 = 100_000;
const SPHERE_STEP_US: u64 = 4_000_000;
const CHANNEL_END_US: u64 = 3_000_000;

#[derive(Debug)]
enum Stage {
    Combat { previous_health: i64, lost: u64 },
    Start { due: u64 },
    Spheres { due: u64 },
    End { due: u64 },
}

#[derive(Debug)]
pub struct EarlOsamEncounter {
    owner: NativePhaseOwner,
    earl: ExactActorRef,
    centre: MovementLocalPosition,
    spheres: Vec<ExactActorRef>,
    stage: Stage,
    expires: u64,
}

impl EarlOsamEncounter {
    pub fn bind(
        runtime: &ChannelRuntimeV1,
        players: &[(ExactActorRef, GameSessionId)],
        actors: &[ExactActorRef],
        centre: MovementLocalPosition,
        now: u64,
    ) -> Result<Self, CarrierError> {
        let [earl] = actors else {
            return Err(CarrierError::PlanConflict);
        };
        if players.len() > 5 {
            return Err(CarrierError::PlanConflict);
        }
        let mut owner = NativePhaseOwner::bind(
            runtime,
            Plan {
                roles: &ROLES,
                terminal: 1,
            },
            players,
        )?;
        owner.attach(runtime, 0, *earl)?;
        runtime.companion_policy(CHANNELING)?;
        runtime.companion_policy(SPHERE)?;
        let previous_health = runtime.companion_snapshot(*earl)?.health;
        Ok(Self {
            owner,
            earl: *earl,
            centre,
            spheres: Vec::with_capacity(4),
            stage: Stage::Combat {
                previous_health,
                lost: 0,
            },
            expires: now.saturating_add(LIMIT_US),
        })
    }

    pub fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.owner.current(runtime).is_ok()
    }

    pub fn contains(&self, actor: ExactActorRef) -> bool {
        self.earl == actor
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
            Stage::Start { due } | Stage::Spheres { due } | Stage::End { due } => {
                self.expires.min(due)
            }
            Stage::Combat { .. } => self.expires,
        }
    }

    pub fn timeout_participants(&self, now: u64) -> Option<&[(ExactActorRef, GameSessionId)]> {
        (now >= self.expires).then_some(&self.owner.participants)
    }

    pub fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now: u64,
        place: impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
        relocate: impl Fn(
            &mut ChannelRuntimeV1,
            ExactActorRef,
            MovementLocalPosition,
        ) -> Result<(), CarrierError>,
    ) -> Result<bool, CarrierError> {
        self.owner.current(runtime)?;
        if self.timeout_participants(now).is_some() {
            for actor in std::iter::once(self.earl).chain(self.spheres.iter().copied()) {
                if runtime.contains_live_creature(actor) {
                    runtime.remove_encounter_actor(actor)?;
                }
            }
            return Ok(false);
        }
        if !runtime.contains_live_creature(self.earl) {
            return Ok(true);
        }
        self.stage = match self.stage {
            Stage::Combat {
                previous_health,
                lost,
            } => {
                let snapshot = runtime.companion_snapshot(self.earl)?;
                let damage =
                    u64::try_from(previous_health.saturating_sub(snapshot.health)).unwrap_or(0);
                let lost = lost.saturating_add(damage);
                let threshold = i128::from(snapshot.maximum_health) * 15;
                if i128::from(lost) * 100 >= threshold {
                    Stage::Start {
                        due: now.saturating_add(CHANNEL_DELAY_US),
                    }
                } else {
                    Stage::Combat {
                        previous_health: snapshot.health,
                        lost,
                    }
                }
            }
            Stage::Start { due } if now >= due => {
                let before = runtime.encounter_parts().0.clone();
                let result = (|| {
                    if runtime.read_actor_position(self.earl)?.position() != self.centre {
                        relocate(runtime, self.earl, self.centre)?;
                    }
                    let snapshot = runtime.companion_snapshot(self.earl)?;
                    let next =
                        runtime.prepare_creature_profile_transformation(&snapshot, CHANNELING)?;
                    runtime.commit_creature_profile_transformation(next)?;
                    gates::protect(runtime, &[self.earl])?;
                    let health = runtime.companion_policy(SPHERE)?.maximum_health;
                    let spawns = sphere_positions(self.centre).map(|at| (SPHERE, health, at));
                    if spawns.iter().any(|&(_, _, at)| {
                        !place(runtime, at) || !runtime.native_summon_cell_free(at)
                    }) {
                        return Err(CarrierError::PlanConflict);
                    }
                    runtime.commit_quest_creature_spawn(&spawns)
                })();
                match result {
                    Ok(spheres) => self.spheres = spheres,
                    Err(error) => {
                        *runtime.encounter_parts_mut().0 = before;
                        return Err(error);
                    }
                }
                Stage::Spheres {
                    due: now.saturating_add(SPHERE_STEP_US),
                }
            }
            Stage::Spheres { due } if now >= due => {
                self.advance_spheres(runtime, &relocate)?;
                if self.spheres.is_empty() {
                    Stage::End {
                        due: now.saturating_add(CHANNEL_END_US),
                    }
                } else {
                    Stage::Spheres {
                        due: now.saturating_add(SPHERE_STEP_US),
                    }
                }
            }
            Stage::End { due } if now >= due => {
                let before = runtime.encounter_parts().0.clone();
                let result = (|| {
                    let snapshot = runtime.companion_snapshot(self.earl)?;
                    let next = runtime.prepare_creature_profile_transformation(&snapshot, EARL)?;
                    let normal = runtime.commit_creature_profile_transformation(next)?;
                    gates::unprotect(runtime, self.earl)?;
                    Ok(normal.health)
                })();
                match result {
                    Ok(health) => Stage::Combat {
                        previous_health: health,
                        lost: 0,
                    },
                    Err(error) => {
                        *runtime.encounter_parts_mut().0 = before;
                        return Err(error);
                    }
                }
            }
            _ => return Ok(true),
        };
        Ok(true)
    }

    fn advance_spheres(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        relocate: &impl Fn(
            &mut ChannelRuntimeV1,
            ExactActorRef,
            MovementLocalPosition,
        ) -> Result<(), CarrierError>,
    ) -> Result<(), CarrierError> {
        if runtime.read_actor_position(self.earl)?.position() != self.centre {
            relocate(runtime, self.earl, self.centre)?;
        }
        self.spheres
            .retain(|&sphere| runtime.contains_live_creature(sphere));
        let mut i = 0;
        while i < self.spheres.len() {
            let sphere = self.spheres[i];
            let from = runtime.read_actor_position(sphere)?.position();
            let mut next = from;
            if next.x != self.centre.x {
                next.x += if next.x < self.centre.x { 1 } else { -1 };
            } else if next.y != self.centre.y {
                next.y += if next.y < self.centre.y { 1 } else { -1 };
            }
            if next == self.centre {
                let before = runtime.encounter_parts().0.clone();
                let result = (|| {
                    let earl = runtime.companion_snapshot(self.earl)?;
                    runtime.remove_encounter_actor(sphere)?;
                    runtime.commit_source_creature_heal_batch(
                        runtime.content_pin().server_artifact_digest(),
                        &[(
                            self.earl,
                            CHANNELING.to_owned(),
                            u64::try_from(earl.maximum_health)
                                .map_err(|_| CarrierError::PlanConflict)?,
                            80_000,
                        )],
                    )?;
                    Ok(())
                })();
                if result.is_err() {
                    *runtime.encounter_parts_mut().0 = before;
                    return result;
                }
                self.spheres.remove(i);
            } else {
                if runtime.native_summon_cell_free(next) {
                    relocate(runtime, sphere, next)?;
                }
                i += 1;
            }
        }
        Ok(())
    }
}

pub fn sphere_positions(centre: MovementLocalPosition) -> [MovementLocalPosition; 4] {
    [(-8, 0), (0, -8), (8, 0), (0, 8)].map(|(dx, dy)| MovementLocalPosition {
        x: centre.x.saturating_add(dx),
        y: centre.y.saturating_add(dy),
        ..centre
    })
}
