//! Crystal 201189f8 Grave Danger Azaram's soul and Splinter phases.
use super::{NativePhaseOwner, Plan, Role, gates};
use oteryn_gameplay_entities::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
};

const LORD: &str = "oteryn:creature.lord_azaram";
const SOUL: &str = "oteryn:creature.azaram_s_soul";
const SPLINTER: &str = "oteryn:creature.tainted_soul_splinter";
const SINS: [&str; 2] = [
    "oteryn:creature.condensed_sin",
    "oteryn:creature.condensed_sins",
];
const ROLES: [Role; 1] = [Role {
    creature: LORD,
    after: 0,
}];

#[derive(Debug)]
pub struct LordAzaramEncounter {
    owner: NativePhaseOwner,
    lord: ExactActorRef,
    expires: u64,
    last_health: i64,
    damage: i64,
    soul_at: Option<u64>,
    return_at: Option<u64>,
    lord_health: Option<i64>,
    positions: [MovementLocalPosition; 4],
    splinters: Vec<ExactActorRef>,
    cleansed: Vec<MovementLocalPosition>,
    sins: Vec<ExactActorRef>,
    absorbed: u8,
}

impl LordAzaramEncounter {
    pub fn bind(
        runtime: &ChannelRuntimeV1,
        players: &[(ExactActorRef, GameSessionId)],
        actors: &[ExactActorRef],
        positions: [MovementLocalPosition; 4],
        now: u64,
    ) -> Result<Self, CarrierError> {
        let [lord, sins @ ..] = actors else {
            return Err(CarrierError::PlanConflict);
        };
        if players.len() > 5
            || sins.iter().any(|&actor| {
                !SINS
                    .iter()
                    .any(|key| runtime.matches_live_creature_identity(actor, key.as_bytes()))
            })
        {
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
        owner.attach(runtime, 0, *lord)?;
        runtime.companion_policy(SOUL)?;
        runtime.companion_policy(SPLINTER)?;
        Ok(Self {
            owner,
            lord: *lord,
            expires: now.saturating_add(1_200_000_000),
            last_health: runtime.companion_snapshot(*lord)?.health,
            damage: 0,
            soul_at: None,
            return_at: None,
            lord_health: None,
            positions,
            splinters: Vec::new(),
            cleansed: Vec::new(),
            sins: sins.to_vec(),
            absorbed: 0,
        })
    }

    pub fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.owner.current(runtime).is_ok()
    }

    pub fn contains(&self, actor: ExactActorRef) -> bool {
        self.lord == actor || self.splinters.contains(&actor)
    }

    pub fn owns_scoped_credit(&self, actor: ExactActorRef) -> bool {
        self.lord == actor
    }

    pub fn projected_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        if actor == self.lord {
            return self.owner.projected_death(runtime, actor);
        }
        if let Some(index) = self
            .splinters
            .iter()
            .position(|&splinter| splinter == actor)
        {
            let (_, position) = runtime.borrow_combat_death().projected_death(actor)?;
            self.splinters.swap_remove(index);
            self.cleansed.push(position);
        }
        Ok(false)
    }

    pub fn victors(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<&[(ExactActorRef, GameSessionId)]>, CarrierError> {
        self.owner.victors(runtime)
    }

    pub fn next_due_us(&self) -> u64 {
        self.soul_at
            .or(self.return_at)
            .map_or(self.expires, |due| due.min(self.expires))
    }

    pub fn timeout_participants(&self, now: u64) -> Option<&[(ExactActorRef, GameSessionId)]> {
        (now >= self.expires).then_some(&self.owner.participants)
    }

    pub fn retire(&self, runtime: &mut ChannelRuntimeV1) -> Result<(), CarrierError> {
        self.owner.current(runtime)?;
        for actor in std::iter::once(self.lord)
            .chain(self.splinters.iter().copied())
            .chain(self.sins.iter().copied())
        {
            if runtime.contains_live_creature(actor) {
                runtime.remove_encounter_actor(actor)?;
            }
        }
        Ok(())
    }

    pub fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now: u64,
        place: impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
    ) -> Result<bool, CarrierError> {
        self.owner.current(runtime)?;
        if !runtime.contains_live_creature(self.lord) {
            return Ok(true);
        }
        let snapshot = runtime.companion_snapshot(self.lord)?;
        if let Some(health) = self.lord_health {
            if self.return_at.is_some_and(|due| now >= due) {
                let before = runtime.encounter_parts().0.clone();
                let result = (|| {
                    let change = runtime
                        .prepare_quest_creature_profile_with_health(&snapshot, LORD, health)?;
                    runtime.commit_creature_profile_transformation(change)?;
                    gates::unprotect(runtime, self.lord)?;
                    for &actor in &self.splinters {
                        if runtime.contains_live_creature(actor) {
                            runtime.remove_encounter_actor(actor)?;
                        }
                    }
                    Ok::<_, CarrierError>(())
                })();
                if let Err(error) = result {
                    *runtime.encounter_parts_mut().0 = before;
                    return Err(error);
                }
                self.lord_health = None;
                self.return_at = None;
                self.splinters.clear();
                self.cleansed.clear();
                self.last_health = health;
                self.damage = 0;
                self.absorbed = 0;
            } else if self.return_at.is_none() {
                let position = runtime.read_actor_position(self.lord)?.position();
                let mut full = snapshot.health >= snapshot.maximum_health;
                if let Some(index) = self.cleansed.iter().position(|&at| at == position) {
                    let maximum = u64::try_from(snapshot.maximum_health)
                        .map_err(|_| CarrierError::PlanConflict)?;
                    let healed = runtime.commit_source_creature_heal_batch(
                        runtime.content_pin().server_artifact_digest(),
                        &[(self.lord, SOUL.to_owned(), maximum, maximum / 3)],
                    )?;
                    self.cleansed.swap_remove(index);
                    self.absorbed += 1;
                    full |= healed
                        .iter()
                        .any(|result| result.health_after >= snapshot.maximum_health);
                }
                if full || self.absorbed >= 3 {
                    self.return_at = Some(now.saturating_add(500_000));
                }
            }
            return Ok(true);
        }

        self.damage = self
            .damage
            .saturating_add(self.last_health.saturating_sub(snapshot.health).max(0));
        self.last_health = snapshot.health;
        if self.soul_at.is_none()
            && i128::from(self.damage) * 5 >= i128::from(snapshot.maximum_health)
        {
            self.soul_at = Some(now.saturating_add(100_000));
        }
        if self.soul_at.is_some_and(|due| now >= due) {
            let soul_max = runtime.companion_policy(SOUL)?.maximum_health;
            let splinter_hp = runtime.companion_policy(SPLINTER)?.maximum_health;
            let spawns: Vec<_> = self
                .positions
                .into_iter()
                .filter(|&at| place(runtime, at) && runtime.native_summon_cell_free(at))
                .map(|at| (SPLINTER, splinter_hp, at))
                .collect();
            let before = runtime.encounter_parts().0.clone();
            let result = (|| {
                let health = i64::try_from(i128::from(soul_max) * 3 / 10)
                    .map_err(|_| CarrierError::PlanConflict)?
                    .max(1);
                let change =
                    runtime.prepare_quest_creature_profile_with_health(&snapshot, SOUL, health)?;
                runtime.commit_creature_profile_transformation(change)?;
                gates::protect(runtime, &[self.lord])?;
                if spawns.is_empty() {
                    return Ok(Vec::new());
                }
                runtime.commit_quest_creature_spawn(&spawns)
            })();
            match result {
                Ok(splinters) => {
                    self.splinters = splinters;
                    self.lord_health = Some(snapshot.health);
                    self.soul_at = None;
                }
                Err(error) => {
                    *runtime.encounter_parts_mut().0 = before;
                    return Err(error);
                }
            }
        }
        Ok(true)
    }
}
