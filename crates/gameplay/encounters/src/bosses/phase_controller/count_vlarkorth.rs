//! Shield and reinforcement timing from Crystal's Grave Danger Count Vlarkorth script.
use super::{NativePhaseOwner, Plan, Role, gates};
use oteryn_gameplay_entities::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
};

const COUNT: &str = "oteryn:creature.count_vlarkorth";
const MINION: &str = "oteryn:creature.soulless_minion";
const ROLES: [Role; 1] = [Role {
    creature: COUNT,
    after: 0,
}];
const LIMIT_US: u64 = 20 * 60 * 1_000_000;
const SHIELD_US: u64 = 120 * 1_000_000;

#[derive(Debug)]
pub struct CountVlarkorthEncounter {
    owner: NativePhaseOwner,
    count: ExactActorRef,
    health: i64,
    lost: i64,
    raise_at: Option<u64>,
    drop_at: Option<u64>,
    shield_remaining: usize,
    heal_at: Option<u64>,
    expires: u64,
    darks: Vec<(ExactActorRef, &'static str)>,
    minions: Vec<ExactActorRef>,
}

impl CountVlarkorthEncounter {
    pub fn bind(
        runtime: &ChannelRuntimeV1,
        players: &[(ExactActorRef, GameSessionId)],
        actors: &[ExactActorRef],
        now: u64,
    ) -> Result<Self, CarrierError> {
        let [count, minions @ ..] = actors else {
            return Err(CarrierError::PlanConflict);
        };
        if players.len() > 5
            || minions
                .iter()
                .any(|&actor| !runtime.matches_live_creature_identity(actor, MINION.as_bytes()))
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
        owner.attach(runtime, 0, *count)?;
        Ok(Self {
            owner,
            count: *count,
            health: runtime.companion_snapshot(*count)?.health,
            lost: 0,
            raise_at: None,
            drop_at: None,
            shield_remaining: 0,
            heal_at: None,
            expires: now.saturating_add(LIMIT_US),
            darks: Vec::new(),
            minions: minions.to_vec(),
        })
    }

    pub fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.owner.current(runtime).is_ok()
    }

    pub fn contains(&self, actor: ExactActorRef) -> bool {
        actor == self.count
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
        [self.raise_at, self.drop_at, self.heal_at]
            .into_iter()
            .flatten()
            .fold(self.expires, u64::min)
    }

    pub fn timeout_participants(&self, now: u64) -> Option<&[(ExactActorRef, GameSessionId)]> {
        (now >= self.expires).then_some(&self.owner.participants)
    }

    pub fn retire(&self, runtime: &mut ChannelRuntimeV1) -> Result<(), CarrierError> {
        self.owner.current(runtime)?;
        for actor in std::iter::once(self.count)
            .chain(self.minions.iter().copied())
            .chain(self.darks.iter().map(|&(actor, _)| actor))
        {
            if runtime.contains_live_creature(actor) {
                runtime.remove_encounter_actor(actor)?;
            }
        }
        Ok(())
    }

    pub fn accepts_remains(
        &self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        corpse: [u8; 16],
        dark: &str,
        now: u64,
    ) -> bool {
        self.count == target
            && self.is_current(runtime)
            && self.shield_remaining > 0
            && self.drop_at.is_some_and(|due| now < due)
            && self.darks.iter().any(|&(actor, key)| {
                key == dark
                    && actor.corpse_identity() == corpse
                    && !runtime.contains_live_creature(actor)
            })
    }

    pub fn consume_remains(&mut self, runtime: &mut ChannelRuntimeV1) -> Result<(), CarrierError> {
        self.owner.current(runtime)?;
        if self.shield_remaining == 0 {
            return Err(CarrierError::PlanConflict);
        }
        if self.shield_remaining == 1 {
            gates::unprotect(runtime, self.count)?;
            self.drop_at = None;
        }
        self.shield_remaining -= 1;
        Ok(())
    }

    pub fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now: u64,
        in_room: impl Fn(MovementLocalPosition) -> bool,
        place: impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
        dark: impl Fn(&ChannelRuntimeV1, ExactActorRef, GameSessionId) -> Option<&'static str>,
    ) -> Result<(), CarrierError> {
        self.owner.current(runtime)?;
        if !runtime.contains_live_creature(self.count) {
            return Ok(());
        }
        let snapshot = runtime.companion_snapshot(self.count)?;
        let damage = self.health.saturating_sub(snapshot.health).max(0);
        self.health = snapshot.health;
        if damage > 0 {
            self.heal_at.get_or_insert(now.saturating_add(2_000_000));
        }
        if self.drop_at.is_some_and(|due| now >= due) {
            gates::unprotect(runtime, self.count)?;
            self.drop_at = None;
            self.shield_remaining = 0;
        }
        if self.drop_at.is_none() && self.raise_at.is_none() {
            self.lost = self.lost.saturating_add(damage);
            if i128::from(self.lost) * 100 >= i128::from(snapshot.maximum_health) * 15 {
                self.lost = 0;
                self.raise_at = Some(now.saturating_add(100_000));
            }
        }
        if self.raise_at.is_some_and(|due| now >= due) {
            let centre = runtime.read_actor_position(self.count)?.position();
            let mut cells = (1..=2)
                .flat_map(|radius: i32| {
                    (-radius..=radius).flat_map(move |x| {
                        (-radius..=radius).filter_map(move |y| {
                            (x.abs().max(y.abs()) == radius).then_some(MovementLocalPosition {
                                x: centre.x + x,
                                y: centre.y + y,
                                ..centre
                            })
                        })
                    })
                })
                .filter(|&at| {
                    in_room(at) && place(runtime, at) && runtime.native_summon_cell_free(at)
                });
            let mut spawns = Vec::new();
            for &(actor, session) in &self.owner.participants {
                if !runtime
                    .read_actor_position(actor)
                    .is_ok_and(|pos| in_room(pos.position()))
                {
                    continue;
                }
                if let Some(key) = dark(runtime, actor, session)
                    && let Some(at) = cells.next()
                {
                    spawns.push((key, runtime.companion_policy(key)?.maximum_health, at));
                }
            }
            if !spawns.is_empty() {
                let before = runtime.encounter_parts().0.clone();
                let result = runtime
                    .commit_quest_creature_spawn(&spawns)
                    .and_then(|actors| {
                        gates::protect(runtime, &[self.count])?;
                        Ok(actors)
                    });
                match result {
                    Ok(actors) => {
                        self.shield_remaining = actors.len();
                        self.darks
                            .retain(|&(actor, _)| runtime.contains_live_creature(actor));
                        self.darks.extend(
                            actors
                                .into_iter()
                                .zip(spawns.iter().map(|&(key, _, _)| key)),
                        );
                        self.drop_at = Some(now.saturating_add(SHIELD_US));
                    }
                    Err(error) => {
                        *runtime.encounter_parts_mut().0 = before;
                        return Err(error);
                    }
                }
            }
            self.raise_at = None;
        }
        if self.heal_at.is_some_and(|due| now >= due) {
            for &minion in &self.minions {
                if !runtime.contains_live_creature(minion) {
                    continue;
                }
                let from = runtime.read_actor_position(minion)?.position();
                if !in_room(from) {
                    continue;
                }
                for (actor, key) in
                    std::iter::once((self.count, COUNT)).chain(self.darks.iter().copied())
                {
                    if !runtime.contains_live_creature(actor) {
                        continue;
                    }
                    let target = runtime.companion_snapshot(actor)?;
                    let to = runtime.read_actor_position(actor)?.position();
                    if target.health < target.maximum_health
                        && to.floor == from.floor
                        && to.x.abs_diff(from.x).max(to.y.abs_diff(from.y)) <= 1
                    {
                        let amount = 300
                            + (now
                                ^ u64::from_le_bytes(
                                    minion.placement_identity()[..8]
                                        .try_into()
                                        .map_err(|_| CarrierError::PlanConflict)?,
                                ))
                                % 246;
                        let max = u64::try_from(target.maximum_health)
                            .map_err(|_| CarrierError::PlanConflict)?;
                        runtime.commit_source_creature_heal_batch(
                            runtime.content_pin().server_artifact_digest(),
                            &[(actor, key.to_owned(), max, amount)],
                        )?;
                        break;
                    }
                }
            }
            self.health = runtime.companion_snapshot(self.count)?.health;
            self.heal_at = Some(now.saturating_add(2_000_000));
        }
        Ok(())
    }
}
