//! Crystal's four successive forms belong to one exact arena admission.
use super::*;
use oteryn_gameplay_encounters::bosses::phase_controller::{NativePhaseOwner, Plan, Role};

const PLAN: Plan = Plan {
    roles: &[
        Role {
            creature: "oteryn:creature.snake_god_essence",
            after: 0,
        },
        Role {
            creature: "oteryn:creature.snake_thing",
            after: 1,
        },
        Role {
            creature: "oteryn:creature.lizard_abomination",
            after: 2,
        },
        Role {
            creature: "oteryn:creature.mutated_zalamon",
            after: 4,
        },
    ],
    terminal: 8,
};

#[derive(Debug)]
pub(in crate::gameplay_transport) struct Payback {
    owner: NativePhaseOwner,
    actors: Vec<ExactActorRef>,
    current: ExactActorRef,
    next_at: Option<MovementLocalPosition>,
    deadline: u64,
    cells: crate::content::NativeEntryMovementCells,
}

impl Payback {
    pub(in crate::gameplay_transport) fn bind(
        runtime: &ChannelRuntimeV1,
        actors: &[ExactActorRef],
        party: &[(ExactActorRef, GameSessionId)],
        cells: crate::content::NativeEntryMovementCells,
        now_us: u64,
    ) -> Result<Self, CarrierError> {
        let current = actors
            .iter()
            .copied()
            .find(|actor| {
                runtime.matches_live_creature_identity(*actor, PLAN.roles[0].creature.as_bytes())
            })
            .ok_or(CarrierError::CreatureTargetMismatch)?;
        for role in PLAN.roles {
            runtime.companion_policy(role.creature)?;
        }
        let mut owner = NativePhaseOwner::bind(runtime, PLAN, party)?;
        owner.attach(runtime, 0, current)?;
        let deadline = now_us
            .checked_add(600_000_000)
            .ok_or(CarrierError::PlanConflict)?;
        Ok(Self {
            owner,
            actors: actors.to_vec(),
            current,
            next_at: None,
            deadline,
            cells,
        })
    }

    fn advance(&mut self, runtime: &mut ChannelRuntimeV1) {
        let Some(at) = self.next_at else { return };
        let Some((role, _)) = self.owner.pending().next() else {
            return;
        };
        if let Ok(actor) = self
            .owner
            .spawn_role(runtime, role, at, None, None, |runtime, at| {
                placement(&self.cells, runtime, at)
            })
        {
            self.actors.push(actor);
            self.current = actor;
            self.next_at = None;
        }
    }
}

impl Registry {
    pub(in crate::gameplay_transport) fn next_payback_due_us(&self) -> Option<u64> {
        Some(self.payback.lock().ok()?.as_ref()?.deadline)
    }

    pub(super) fn payback_death(&self, runtime: &mut ChannelRuntimeV1, actor: ExactActorRef) {
        let Ok(mut room) = self.payback.lock() else {
            return;
        };
        let Some(fight) = room.as_mut().filter(|fight| fight.current == actor) else {
            return;
        };
        if !matches!(fight.owner.projected_death(runtime, actor), Ok(true)) {
            return;
        }
        if fight
            .owner
            .victors(runtime)
            .is_ok_and(|victors| victors.is_some())
        {
            // Final remains keep their own item lifetime. Retire only the live trap actors.
            for actor in &fight.actors {
                if runtime.contains_live_creature(*actor) {
                    let _ = runtime.remove_encounter_actor(*actor);
                }
            }
            *room = None;
        } else {
            fight.next_at = runtime
                .borrow_combat_death()
                .projected_death(actor)
                .ok()
                .map(|(_, position)| position);
            fight.advance(runtime);
        }
    }

    pub(in crate::gameplay_transport) fn tick_payback(
        &self,
        runtime: &mut ChannelRuntimeV1,
        now_us: u64,
    ) {
        let Ok(mut room) = self.payback.lock() else {
            return;
        };
        let Some(fight) = room.as_mut() else { return };
        if fight.owner.victors(runtime).is_err() {
            *room = None;
            return;
        }
        if now_us >= fight.deadline {
            for actor in &fight.actors {
                if runtime.contains_live_creature(*actor) {
                    let _ = runtime.remove_encounter_actor(*actor);
                }
            }
            *room = None;
        } else {
            fight.advance(runtime);
        }
    }
}
