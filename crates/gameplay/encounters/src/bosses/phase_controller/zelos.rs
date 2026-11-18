//! Crystal 201189f grave_danger_king_zelos.lua:105–160. The north
//! encounter becomes available only after all four knights and their successors die.
use super::{NativePhaseOwner, Plan, Role};
use oteryn_gameplay_entities::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
    OwnerDamageCommand, project_fixed_one_creature_death,
};

const MAGNOR: usize = 0;
const NARGOL: usize = 1;
const REWAR: usize = 2;
const RED: usize = 3;
const FIRST_SHARD: usize = 4;
const MASS: usize = 8;
const KING: usize = 9;
const KNIGHTS: u64 = (1 << KING) - 1;
const REGENERATION_US: u64 = 30_000_000;
const BLOOD: &str = "oteryn:creature.vampiric_blood";
const SOLDIER: &str = "oteryn:creature.risen_soldier";
const REFILL_US: u64 = 10_000_000;
const FETTER: &str = "oteryn:creature.fetter";
const REWAR_IMMUNE: &str = "oteryn:creature.rewar_the_bloody_inv";
const FETTER_WAVE_US: u64 = 20_000_000;
const FETTER_LIMIT: usize = 6;
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
        after: 0,
    },
];

#[derive(Clone, Copy, Debug)]
pub struct MagnorBurst {
    pub shard: ExactActorRef,
    pub position: MovementLocalPosition,
}

#[derive(Debug)]
pub struct ZelosEncounter {
    phase: NativePhaseOwner,
    start_us: u64,
    mass_due_us: Option<u64>,
    ritual_completed_us: Option<u64>,
    magnor_at: MovementLocalPosition,
    nargol_at: MovementLocalPosition,
    rewar_at: MovementLocalPosition,
    fetters: Vec<ExactActorRef>,
    fetter_due_us: Option<u64>,
    rewar_immune: bool,
    king_gate_released: bool,
    king_at: MovementLocalPosition,
    red_at: MovementLocalPosition,
    soldiers: Vec<ExactActorRef>,
    blood: Vec<ExactActorRef>,
    refill_due_us: u64,
    magnor_bursts: Vec<MagnorBurst>,
}
impl ZelosEncounter {
    pub fn bind(
        runtime: &mut ChannelRuntimeV1,
        players: &[(ExactActorRef, GameSessionId)],
        actors: &[ExactActorRef],
        now: u64,
    ) -> Result<Self, CarrierError> {
        if !(5..=16).contains(&actors.len()) || players.len() > 10 {
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
        for role in [MAGNOR, NARGOL, REWAR, RED, KING] {
            let def = ROLES[role];
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
        for key in [BLOOD, FETTER, REWAR_IMMUNE, SOLDIER] {
            runtime.companion_policy(key)?;
        }
        let mut soldiers = Vec::new();
        for &actor in actors {
            if !phase.actors.contains(&Some(actor)) {
                if !runtime.matches_live_creature_identity(actor, SOLDIER.as_bytes()) {
                    return Err(CarrierError::CreatureTargetMismatch);
                }
                soldiers.push(actor);
            }
        }
        let position = |role: usize| {
            runtime
                .read_actor_position(phase.actors[role].ok_or(CarrierError::PlanConflict)?)
                .map(|snapshot| snapshot.position())
        };
        let encounter = Self {
            magnor_at: position(MAGNOR)?,
            nargol_at: position(NARGOL)?,
            rewar_at: position(REWAR)?,
            fetters: Vec::with_capacity(FETTER_LIMIT),
            fetter_due_us: None,
            rewar_immune: false,
            king_gate_released: false,
            king_at: position(KING)?,
            red_at: position(RED)?,
            soldiers,
            blood: Vec::new(),
            refill_due_us: now.saturating_add(REFILL_US),
            magnor_bursts: Vec::new(),
            phase,
            start_us: now,
            mass_due_us: None,
            ritual_completed_us: None,
        };
        super::gates::protect(runtime, &[encounter.king()?])?;
        Ok(encounter)
    }

    pub fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.phase.current(runtime).is_ok()
    }

    pub fn contains(&self, actor: ExactActorRef) -> bool {
        self.phase.actors.contains(&Some(actor)) || self.blood.contains(&actor)
    }

    pub fn contains_terminal(&self, actor: ExactActorRef) -> bool {
        self.phase.actors[KING] == Some(actor)
    }

    pub fn projected_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Result<bool, CarrierError> {
        if let Some(index) = self.blood.iter().position(|&blood| blood == actor) {
            self.vampiric_blood_death(runtime, actor)?;
            self.blood.swap_remove(index);
            return Ok(true);
        }
        if self.contains_terminal(actor) && !self.king_gate_released {
            return Err(CarrierError::PlanConflict);
        }
        if self.phase.actors[FIRST_SHARD..MASS].contains(&Some(actor)) {
            if self.phase.progress.dead() & (1 << FIRST_SHARD) != 0 {
                return Ok(false);
            }
            let mut bursts = Vec::with_capacity(MASS - FIRST_SHARD);
            for role in FIRST_SHARD..MASS {
                let shard = self.phase.actors[role].ok_or(CarrierError::PlanConflict)?;
                // The lethal projection may already have detached the live position
                // for the shard actually struck. All four occupy this encounter room.
                let position = runtime
                    .read_actor_position(shard)
                    .map_or(self.magnor_at, |snapshot| snapshot.position());
                bursts.push(MagnorBurst { shard, position });
            }
            for &MagnorBurst { shard, .. } in &bursts {
                if shard != actor {
                    project_fixed_one_creature_death(&mut runtime.borrow_combat_death(), shard)?;
                }
                self.phase.projected_death(runtime, shard)?;
            }
            self.magnor_bursts.extend(bursts);
            return Ok(true);
        }
        self.phase.projected_death(runtime, actor)
    }

    // Crystal 201189f grave_danger_king_zelos.lua:279–293.
    fn vampiric_blood_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        blood: ExactActorRef,
    ) -> Result<(), CarrierError> {
        use sha2::{Digest, Sha256};
        let (_, death) = runtime.borrow_combat_death().projected_death(blood)?;
        let red = self.phase.actors[RED].ok_or(CarrierError::PlanConflict)?;
        if !runtime.contains_live_creature(red) {
            return Ok(());
        }
        let at = runtime.read_actor_position(red)?.position();
        if death.floor != at.floor || death.x.abs_diff(at.x) > 1 || death.y.abs_diff(at.y) > 1 {
            return Ok(());
        }
        let identity = blood.placement_identity();
        let draw = Sha256::digest(identity);
        let damage = 20_000 + i64::from(u16::from_be_bytes([draw[0], draw[1]]) % 5001);
        let occurrence = format!("grave-danger-blood:{:032x}", u128::from_be_bytes(identity));
        let binding = format!("{occurrence}\0grave-danger-drowning");
        let before = runtime.encounter_parts().0.clone();
        let outcome = (|| {
            let (carrier, continuity) = runtime.encounter_parts_mut();
            let hit = carrier.commit_encounter_single_damage(
                continuity,
                red.0,
                OwnerDamageCommand {
                    target: ROLES[RED].creature.as_bytes(),
                    occurrence: occurrence.as_bytes(),
                    binding: binding.as_bytes(),
                    damage,
                },
                None,
                false,
                None,
                None,
            )?;
            if hit.health_after == 0 {
                project_fixed_one_creature_death(&mut runtime.borrow_combat_death(), red)?;
                self.phase.projected_death(runtime, red)?;
            }
            Ok(())
        })();
        if outcome.is_err() {
            *runtime.encounter_parts_mut().0 = before;
        }
        outcome
    }

    pub fn victors(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<&[(ExactActorRef, GameSessionId)]>, CarrierError> {
        self.phase.victors(runtime)
    }

    pub fn take_magnor_bursts(&mut self) -> Vec<MagnorBurst> {
        std::mem::take(&mut self.magnor_bursts)
    }

    pub fn next_due_us(&self) -> u64 {
        let mut due = self
            .start_us
            .saturating_add(LIMIT_US)
            .min(self.refill_due_us);
        if self.phase.progress.dead() & (1 << MASS) == 0
            && let Some(mass) = self.mass_due_us
        {
            due = due.min(mass);
        }
        if self.phase.progress.dead() & (1 << REWAR) == 0
            && let Some(fetters) = self.fetter_due_us
        {
            due = due.min(fetters);
        }
        if !self.magnor_bursts.is_empty() || self.phase.pending().next().is_some() {
            due = due.min(self.start_us);
        }
        if !self.king_gate_released && self.ritual_broken() {
            due = due.min(
                self.ritual_completed_us
                    .map_or(self.start_us, |completed| completed.saturating_add(300_000)),
            );
        }
        due
    }

    pub fn timeout_participants(&self, now: u64) -> Option<&[(ExactActorRef, GameSessionId)]> {
        (now.saturating_sub(self.start_us) >= LIMIT_US).then_some(&self.phase.participants)
    }

    pub fn retire(&self, runtime: &mut ChannelRuntimeV1) -> Result<(), CarrierError> {
        self.phase.current(runtime)?;
        for actor in self
            .phase
            .actors
            .iter()
            .copied()
            .flatten()
            .chain(self.fetters.iter().copied())
            .chain(self.soldiers.iter().copied())
            .chain(self.blood.iter().copied())
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
        self.phase.current(runtime)?;
        if self.timeout_participants(now).is_some() {
            self.retire(runtime)?;
            return Ok(false);
        }
        self.advance_rewar(runtime, now, &place)?;
        self.refill(runtime, now, &place)?;
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
            match role {
                FIRST_SHARD => self.spawn_shards(runtime, &place)?,
                MASS => {
                    if !runtime.native_summon_cell_free(self.nargol_at) {
                        return Err(CarrierError::PlanConflict);
                    }
                    self.phase
                        .spawn_role(runtime, role, self.nargol_at, None, None, &place)?;
                    self.mass_due_us = Some(now.saturating_add(REGENERATION_US));
                }
                _ => return Err(CarrierError::PlanConflict),
            }
        }
        if self.ritual_broken() && !self.king_gate_released {
            let completed = *self.ritual_completed_us.get_or_insert(now);
            if now >= completed.saturating_add(300_000) {
                let king = self.king()?;
                if completed.saturating_sub(self.start_us) > HALF_AFTER_US {
                    super::gates::halve(runtime, king)?;
                } else {
                    super::gates::unprotect(runtime, king)?;
                }
                self.king_gate_released = true;
            }
        }
        Ok(true)
    }

    fn spawn_shards(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        place: &impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
    ) -> Result<(), CarrierError> {
        let key = ROLES[FIRST_SHARD].creature;
        let health = runtime.companion_policy(key)?.maximum_health;
        let spawns = [(1, 0), (0, 1), (-1, 0), (0, -1)].map(|(dx, dy)| {
            (
                key,
                health,
                MovementLocalPosition {
                    x: self.magnor_at.x + dx,
                    y: self.magnor_at.y + dy,
                    ..self.magnor_at
                },
            )
        });
        if spawns
            .iter()
            .any(|&(_, _, at)| !place(runtime, at) || !runtime.native_summon_cell_free(at))
        {
            return Err(CarrierError::PlanConflict);
        }
        let before = runtime.encounter_parts().0.clone();
        let mut projected = self.phase.clone();
        let result = (|| {
            let spawned: [ExactActorRef; 4] = runtime
                .commit_quest_creature_spawn(&spawns)?
                .try_into()
                .map_err(|_| CarrierError::PlanConflict)?;
            for (i, actor) in spawned.into_iter().enumerate() {
                projected.attach(runtime, FIRST_SHARD + i, actor)?;
            }
            super::gates::link_health(runtime, spawned, key)
        })();
        if result.is_err() {
            *runtime.encounter_parts_mut().0 = before;
        } else {
            self.phase = projected;
        }
        result
    }

    pub fn north_gate_open(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        now: u64,
    ) -> bool {
        self.is_current(runtime)
            && self.timeout_participants(now).is_none()
            && self.king_gate_released
            && self.phase.participants.contains(&(actor, session))
            && self
                .king()
                .is_ok_and(|king| runtime.contains_live_creature(king))
    }

    fn ritual_broken(&self) -> bool {
        self.phase.progress.dead() & KNIGHTS == KNIGHTS
    }

    fn king(&self) -> Result<ExactActorRef, CarrierError> {
        self.phase.actors[KING].ok_or(CarrierError::PlanConflict)
    }

    // Crystal 201189f grave_danger_king_zelos.lua:74–96.
    fn refill(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now: u64,
        place: &impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
    ) -> Result<(), CarrierError> {
        if now < self.refill_due_us {
            return Ok(());
        }
        if self.phase.actors[RED].is_some_and(|actor| runtime.contains_live_creature(actor)) {
            refill_one(runtime, &mut self.blood, self.red_at, 7, BLOOD, place)?;
        }
        if runtime.contains_live_creature(self.king()?) {
            refill_one(runtime, &mut self.soldiers, self.king_at, 9, SOLDIER, place)?;
        }
        self.refill_due_us = now.saturating_add(REFILL_US);
        Ok(())
    }

    // Crystal 201189f grave_danger_king_zelos.lua:164–277.
    fn advance_rewar(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now: u64,
        place: &impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
    ) -> Result<(), CarrierError> {
        let Some(rewar) =
            self.phase.actors[REWAR].filter(|actor| runtime.contains_live_creature(*actor))
        else {
            return Ok(());
        };
        self.fetters
            .retain(|actor| runtime.contains_live_creature(*actor));
        if self.fetters.is_empty() && self.rewar_immune {
            self.change_rewar_form(runtime, rewar, false, place)?;
            self.rewar_immune = false;
        }

        let due = match self.fetter_due_us {
            Some(due) => now >= due,
            None => self.phase.participants.iter().any(|&(actor, session)| {
                runtime.player_control_facts(actor, session).is_ok()
                    && runtime.read_actor_position(actor).is_ok_and(|pos| {
                        let pos = pos.position();
                        pos.floor == self.rewar_at.floor
                            && pos.x.abs_diff(self.rewar_at.x) <= 8
                            && pos.y.abs_diff(self.rewar_at.y) <= 8
                    })
            }),
        };
        if !due {
            return Ok(());
        }
        let count = (FETTER_LIMIT - self.fetters.len()).min(2);
        let health = runtime.companion_policy(FETTER)?.maximum_health;
        let spawns = [(-2, 0), (2, 0), (0, -2), (0, 2)]
            .into_iter()
            .map(|(dx, dy)| MovementLocalPosition {
                x: self.rewar_at.x + dx,
                y: self.rewar_at.y + dy,
                ..self.rewar_at
            })
            .filter(|&at| place(runtime, at) && runtime.native_summon_cell_free(at))
            .take(count)
            .map(|at| (FETTER, health, at))
            .collect::<Vec<_>>();
        if !spawns.is_empty() {
            let before = runtime.encounter_parts().0.clone();
            let spawned = runtime.commit_quest_creature_spawn(&spawns)?;
            if !self.rewar_immune {
                if let Err(error) = self.change_rewar_form(runtime, rewar, true, place) {
                    *runtime.encounter_parts_mut().0 = before;
                    return Err(error);
                }
                self.rewar_immune = true;
            }
            self.fetters.extend(spawned);
        }
        self.fetter_due_us = Some(now.saturating_add(FETTER_WAVE_US));
        Ok(())
    }

    fn change_rewar_form(
        &self,
        runtime: &mut ChannelRuntimeV1,
        rewar: ExactActorRef,
        immune: bool,
        place: &impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
    ) -> Result<(), CarrierError> {
        let before = runtime.encounter_parts().0.clone();
        let result = (|| {
            let from = runtime.read_actor_position(rewar)?;
            if from.position() != self.rewar_at {
                if !place(runtime, self.rewar_at)
                    || runtime.position_occupied_by_other(rewar, self.rewar_at)?
                {
                    return Err(CarrierError::PlanConflict);
                }
                runtime
                    .borrow_movement_position()
                    .commit_creature_relocation(rewar, from, self.rewar_at)?;
            }
            let snapshot = runtime.companion_snapshot(rewar)?;
            let key = if immune {
                REWAR_IMMUNE
            } else {
                ROLES[REWAR].creature
            };
            let transform = runtime.prepare_creature_profile_transformation(&snapshot, key)?;
            runtime.commit_creature_profile_transformation(transform)?;
            if immune {
                super::gates::protect(runtime, &[rewar])
            } else {
                super::gates::unprotect(runtime, rewar)
            }
        })();
        if result.is_err() {
            *runtime.encounter_parts_mut().0 = before;
        }
        result
    }
}

fn refill_one(
    runtime: &mut ChannelRuntimeV1,
    owned: &mut Vec<ExactActorRef>,
    centre: MovementLocalPosition,
    radius: u32,
    key: &str,
    place: &impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
) -> Result<(), CarrierError> {
    owned.retain(|actor| runtime.contains_live_creature(*actor));
    let nearby = |actor| {
        runtime.read_actor_position(actor).is_ok_and(|snapshot| {
            let at = snapshot.position();
            at.floor == centre.floor
                && at.x.abs_diff(centre.x) <= radius
                && at.y.abs_diff(centre.y) <= radius
        })
    };
    if owned.len() >= 32 || owned.iter().copied().filter(|&actor| nearby(actor)).count() >= 3 {
        return Ok(());
    }
    let at = [
        (-1, 0),
        (1, 0),
        (0, -1),
        (0, 1),
        (-1, -1),
        (1, 1),
        (-1, 1),
        (1, -1),
    ]
    .into_iter()
    .map(|(dx, dy)| MovementLocalPosition {
        x: centre.x + dx,
        y: centre.y + dy,
        ..centre
    })
    .find(|&at| place(runtime, at) && runtime.native_summon_cell_free(at));
    if let Some(at) = at {
        let health = runtime.companion_policy(key)?.maximum_health;
        let summoned = runtime.commit_quest_creature_spawn(&[(key, health, at)])?;
        owned.extend(summoned);
    }
    Ok(())
}
