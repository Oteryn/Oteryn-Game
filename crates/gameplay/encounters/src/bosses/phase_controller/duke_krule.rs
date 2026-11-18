//! Crystal 201189f creaturescript_duke_krule_tranform.lua:54–134.
use super::{NativePhaseOwner, Plan, Role};
use oteryn_gameplay_entities::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use sha2::{Digest, Sha256};

const DUKE: &str = "oteryn:creature.duke_krule";
const SCOURGE: &str = "oteryn:creature.soul_scourge";
const ROLES: [Role; 1] = [Role {
    creature: DUKE,
    after: 0,
}];
const ACTIVE_US: u64 = 6_000_000;
const COOLDOWN_US: u64 = 10_000_000;
const PULSE_US: u64 = 2_000_000;
const LIMIT_US: u64 = 20 * 60 * 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElementalForm {
    Fire,
    Water,
}

#[derive(Clone, Copy, Debug)]
pub struct ElementalPlayer {
    pub actor: ExactActorRef,
    pub session: GameSessionId,
    pub form: ElementalForm,
}

#[derive(Debug)]
enum Stage {
    WaitingForHit {
        health_at_admission: i64,
    },
    Rest {
        due: u64,
    },
    Active {
        starts: u64,
        pulse: u8,
        forms: Vec<ElementalPlayer>,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum DukeEffect {
    Transform {
        player: ElementalPlayer,
        finish_us: u64,
    },
    Damage {
        target: ElementalPlayer,
        attacker: ElementalPlayer,
        amount: u32,
        starts: u64,
        pulse: u8,
    },
}

#[derive(Debug)]
pub struct DukeKruleEncounter {
    owner: NativePhaseOwner,
    boss: ExactActorRef,
    scourges: Vec<ExactActorRef>,
    expires: u64,
    stage: Stage,
}

impl DukeKruleEncounter {
    pub fn bind(
        runtime: &ChannelRuntimeV1,
        players: &[(ExactActorRef, GameSessionId)],
        actors: &[ExactActorRef],
        now: u64,
    ) -> Result<Self, CarrierError> {
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
        let boss = actors
            .iter()
            .copied()
            .find(|&actor| runtime.matches_live_creature_identity(actor, DUKE.as_bytes()))
            .ok_or(CarrierError::CreatureTargetMismatch)?;
        owner.attach(runtime, 0, boss)?;
        let mut scourges = Vec::new();
        for &actor in actors {
            if actor != boss {
                if !runtime.matches_live_creature_identity(actor, SCOURGE.as_bytes()) {
                    return Err(CarrierError::CreatureTargetMismatch);
                }
                scourges.push(actor);
            }
        }
        Ok(Self {
            owner,
            boss,
            scourges,
            expires: now.saturating_add(LIMIT_US),
            stage: Stage::WaitingForHit {
                health_at_admission: runtime.companion_snapshot(boss)?.health,
            },
        })
    }

    pub fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.owner.current(runtime).is_ok()
    }

    pub fn contains(&self, actor: ExactActorRef) -> bool {
        self.boss == actor
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
        self.expires.min(match &self.stage {
            Stage::WaitingForHit { .. } => self.expires,
            Stage::Rest { due } => *due,
            Stage::Active { starts, pulse, .. } => {
                starts.saturating_add(u64::from(*pulse) * PULSE_US)
            }
        })
    }

    pub fn timeout_participants(&self, now: u64) -> Option<&[(ExactActorRef, GameSessionId)]> {
        (now >= self.expires).then_some(&self.owner.participants)
    }

    pub fn retire(&self, runtime: &mut ChannelRuntimeV1) -> Result<(), CarrierError> {
        self.owner.current(runtime)?;
        for actor in std::iter::once(self.boss).chain(self.scourges.iter().copied()) {
            if runtime.contains_live_creature(actor) {
                runtime.remove_encounter_actor(actor)?;
            }
        }
        Ok(())
    }

    pub fn due_effects(
        &mut self,
        runtime: &ChannelRuntimeV1,
        now: u64,
        in_room: impl Fn(MovementLocalPosition) -> bool,
    ) -> Result<(ExactActorRef, Vec<DukeEffect>), CarrierError> {
        self.owner.current(runtime)?;
        let mut effects = Vec::new();
        if now >= self.expires || !runtime.contains_live_creature(self.boss) {
            return Ok((self.boss, effects));
        }
        let position = |actor, session| {
            runtime.player_control_facts(actor, session).ok()?;
            let at = runtime.read_actor_position(actor).ok()?.position();
            in_room(at).then_some(at)
        };
        match &mut self.stage {
            Stage::WaitingForHit {
                health_at_admission,
            } => {
                if runtime.companion_snapshot(self.boss)?.health < *health_at_admission {
                    self.stage = Stage::Rest {
                        due: now.saturating_add(3_000_000),
                    };
                }
            }
            Stage::Rest { due } if now >= *due => {
                let finish_us = now.saturating_add(ACTIVE_US);
                let forms = self
                    .owner
                    .participants
                    .iter()
                    .filter_map(|&(actor, session)| {
                        position(actor, session)?;
                        let draw = draw(self.boss, actor, actor, now, 0);
                        let player = ElementalPlayer {
                            actor,
                            session,
                            form: if draw & 1 == 0 {
                                ElementalForm::Fire
                            } else {
                                ElementalForm::Water
                            },
                        };
                        effects.push(DukeEffect::Transform { player, finish_us });
                        Some(player)
                    })
                    .collect();
                self.stage = Stage::Active {
                    starts: now,
                    pulse: 0,
                    forms,
                };
            }
            _ => {}
        }
        if let Stage::Active {
            starts,
            pulse,
            forms,
        } = &mut self.stage
        {
            let elapsed = now.saturating_sub(*starts);
            if elapsed >= ACTIVE_US {
                self.stage = Stage::Rest {
                    due: starts.saturating_add(ACTIVE_US + COOLDOWN_US),
                };
            } else if elapsed >= u64::from(*pulse) * PULSE_US {
                for &attacker in forms.iter() {
                    let Some(from) = position(attacker.actor, attacker.session) else {
                        continue;
                    };
                    for &target in forms.iter() {
                        if attacker.form == target.form {
                            continue;
                        }
                        let Some(to) = position(target.actor, target.session) else {
                            continue;
                        };
                        let dx = from.x.abs_diff(to.x);
                        let dy = from.y.abs_diff(to.y);
                        if dx <= 3 && dy <= 3 && dx * dx + dy * dy <= 10 {
                            effects.push(DukeEffect::Damage {
                                attacker,
                                target,
                                amount: 1100
                                    + draw(
                                        self.boss,
                                        attacker.actor,
                                        target.actor,
                                        *starts,
                                        *pulse,
                                    ) % 901,
                                starts: *starts,
                                pulse: *pulse,
                            });
                        }
                    }
                }
                *pulse += 1;
            }
        }
        Ok((self.boss, effects))
    }
}

fn draw(
    boss: ExactActorRef,
    from: ExactActorRef,
    to: ExactActorRef,
    starts: u64,
    pulse: u8,
) -> u32 {
    let hash = Sha256::new()
        .chain_update(b"grave-danger-duke-krule")
        .chain_update(boss.placement_identity())
        .chain_update(from.placement_identity())
        .chain_update(to.placement_identity())
        .chain_update(starts.to_le_bytes())
        .chain_update([pulse])
        .finalize();
    u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]])
}
