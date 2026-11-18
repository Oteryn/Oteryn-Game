//! Mortal Combat waves advance by deadline independently of earlier deaths.
use super::*;
use oteryn_gameplay_quests::quest::the_new_frontier::{
    MORTAL_REWARD_EXIT, MORTAL_ROOM, MORTAL_TIMEOUT_EXIT, MORTAL_TIMEOUT_US,
    MORTAL_WAVE_INTERVAL_US, MORTAL_WAVES, TIRECZ,
};

#[derive(Debug)]
pub(in crate::gameplay_transport) struct Run {
    scope: (
        crate::foundation::RuntimeScopeRefV1,
        crate::foundation::ScopeOwnershipGeneration,
    ),
    started: u64,
    next_wave: usize,
    tirecz: Option<ExactActorRef>,
    victors: Option<Vec<(ExactActorRef, GameSessionId)>>,
}

impl Run {
    pub(in crate::gameplay_transport) fn start(runtime: &ChannelRuntimeV1, now: u64) -> Self {
        Self {
            scope: runtime.encounter_owner_scope(),
            started: now,
            next_wave: 0,
            tirecz: None,
            victors: None,
        }
    }

    pub(super) fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.scope == runtime.encounter_owner_scope()
    }

    pub(super) fn due(&self) -> u64 {
        if self.victors.is_some() {
            return 0;
        }
        let delay = if self.next_wave < MORTAL_WAVES.len() {
            self.next_wave as u64 * MORTAL_WAVE_INTERVAL_US
        } else {
            MORTAL_TIMEOUT_US
        };
        self.started.saturating_add(delay)
    }

    pub(super) fn death(&mut self, runtime: &ChannelRuntimeV1, actor: ExactActorRef) {
        if self.tirecz == Some(actor)
            && self.victors.is_none()
            && self.is_current(runtime)
            && let Ok(census) = runtime.positioned_melee_census()
        {
            self.victors = Some(
                census
                    .into_iter()
                    .filter_map(|(actor, at, session)| {
                        in_room(at.position()).then_some((actor, session?))
                    })
                    .collect(),
            );
        }
    }

    pub(super) fn won(&self, actor: ExactActorRef) -> bool {
        self.tirecz == Some(actor) && self.victors.is_some()
    }

    fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        cells: &crate::content::NativeEntryMovementCells,
        map: &crate::map::boot::BundleMap,
        now: u64,
    ) {
        while self.next_wave < MORTAL_WAVES.len() && now >= self.due() {
            let sites = if self.next_wave == 0 {
                [(33065_u16, 31033_u16), (33066, 31037)]
            } else {
                [(33065_u16, 31035_u16), (33068, 31034)]
            };
            for (&creature, (x, y)) in MORTAL_WAVES[self.next_wave].iter().zip(sites) {
                let at = MovementLocalPosition {
                    x: i32::from(x),
                    y: i32::from(y),
                    floor: 3,
                };
                let Ok(policy) = runtime.companion_policy(creature) else {
                    continue;
                };
                let health = policy.maximum_health;
                if placement(cells, runtime, at)
                    && let Ok(actors) =
                        runtime.commit_quest_creature_spawn(&[(creature, health, at)])
                    && let Some(actor) = actors.into_iter().next()
                {
                    map.facts
                        .emit_movement_effect(crate::map::overlay::TilePos { x, y, floor: -3 }, 11);
                    if creature == TIRECZ {
                        self.tirecz = Some(actor);
                    }
                }
            }
            self.next_wave += 1;
        }
    }
}

fn in_room(at: MovementLocalPosition) -> bool {
    let [x1, y1, x2, y2, floor] = MORTAL_ROOM;
    i32::from(at.floor) == -floor && (x1..=x2).contains(&at.x) && (y1..=y2).contains(&at.y)
}

impl super::super::ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn drain_mortal_combat(&self) {
        use super::super::paced_movement::underwater_drown::underwater;
        let Some(map) = self.bundle_map() else { return };
        let now = self.owner_now().get();
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if runtime.owner_fence().is_err() {
            return;
        }
        let Ok(mut slot) = states.encounter_phases.mortal.lock() else {
            return;
        };
        let Some(mut run) = slot.take().filter(|run| run.is_current(&runtime)) else {
            return;
        };
        if run.due() > now {
            *slot = Some(run);
            return;
        }
        if run.victors.is_none() && now < run.started.saturating_add(MORTAL_TIMEOUT_US) {
            run.advance(&mut runtime, self.movement_cells(), map, now);
            *slot = Some(run);
            return;
        }
        let Ok(census) = runtime.positioned_melee_census() else {
            *slot = Some(run);
            return;
        };
        drop(slot);
        let doors = crate::map::floor::BundleDoors(&map.facts);
        let mut retry = false;
        for (actor, at, session) in census {
            if !in_room(at.position()) {
                continue;
            }
            let Some(session) = session else {
                let _ = runtime.remove_encounter_actor(actor);
                continue;
            };
            if states.is_dead(actor) {
                continue;
            }
            let won = run
                .victors
                .as_ref()
                .is_some_and(|victors| victors.contains(&(actor, session)));
            let exit = if won {
                MORTAL_REWARD_EXIT
            } else {
                MORTAL_TIMEOUT_EXIT
            };
            let destination = MovementLocalPosition {
                x: i32::from(exit.x),
                y: i32::from(exit.y),
                floor: -i16::from(exit.floor),
            };
            let Ok(proof) = crate::map::floor::prepare_quest_encounter_ejection(
                &runtime,
                self.movement_cells(),
                actor,
                session,
                at,
                destination,
                &doors,
            ) else {
                retry = true;
                continue;
            };
            let Ok(snapshot) = runtime.commit_source_step(proof) else {
                retry = true;
                continue;
            };
            states.stage_special_movement_step(
                &runtime,
                map,
                actor,
                session,
                now,
                Some(at.position()),
                snapshot.position(),
                None,
            );
            states.stage_underwater_step(
                &runtime,
                actor,
                session,
                now,
                underwater(map, at.position()),
                underwater(map, destination),
            );
            map.facts.emit_movement_effect(
                crate::map::overlay::TilePos {
                    x: exit.x,
                    y: exit.y,
                    floor: exit.floor,
                },
                11,
            );
            if won {
                self.states.chat.push_quest_feedback(session, "You have won! As new champion take the ancient armor as reward before you leave.");
            }
        }
        if retry && let Ok(mut slot) = states.encounter_phases.mortal.lock() {
            *slot = Some(run);
        }
    }
}
