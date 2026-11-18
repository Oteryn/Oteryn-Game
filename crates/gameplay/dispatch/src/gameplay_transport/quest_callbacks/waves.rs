//! Timed waves use the same Channel source-owner turn as other quest encounters.
use super::*;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::quest_state::wave_egress::WaveEgress;
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_gameplay_quests::quest::world_callbacks::{
    Encounter, EncounterMonster,
    waves::{Wave, WaveSpawn},
};
use std::collections::BTreeSet;
use std::time::Duration;
use tokio::time::Instant;

pub(super) struct ActiveWaveRun {
    pub(super) key: u64,
    pub(super) room: Encounter,
    callback: String,
    pub(super) participants: Vec<(ExactActorRef, GameSessionId)>,
    scope: (RuntimeScopeRefV1, ScopeOwnershipGeneration),
    started: Instant,
    next: usize,
    creatures: Vec<ExactActorRef>,
    terminal: Option<ExactActorRef>,
    exit: Option<(Position, bool)>,
    lever: Option<(u64, crate::map::view::EntryFacts)>,
}

impl ActiveWaveRun {
    pub(super) fn new(
        runtime: &ChannelRuntimeV1,
        key: u64,
        room: Encounter,
        callback: String,
        participants: Vec<(ExactActorRef, GameSessionId)>,
        lever: Option<(u64, crate::map::view::EntryFacts)>,
    ) -> Self {
        Self {
            key,
            room,
            callback,
            participants,
            scope: runtime.encounter_owner_scope(),
            started: Instant::now(),
            next: 0,
            creatures: Vec::new(),
            terminal: None,
            exit: None,
            lever,
        }
    }

    pub(super) fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.scope == runtime.encounter_owner_scope()
    }

    pub(super) fn overlaps(&self, room: &Encounter) -> bool {
        let a = &self.room;
        a.room_min.floor == room.room_min.floor
            && a.room_min.x <= room.room_max.x
            && room.room_min.x <= a.room_max.x
            && a.room_min.y <= room.room_max.y
            && room.room_min.y <= a.room_max.y
    }

    fn process(
        &mut self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
        now: Instant,
    ) -> bool {
        let Some(plan) = &self.room.wave_run else {
            return false;
        };
        let elapsed = now.saturating_duration_since(self.started);
        self.creatures
            .retain(|actor| runtime.contains_live_creature(*actor));
        let timed_out = elapsed >= Duration::from_millis(plan.timeout_ms.into());
        if self.exit.is_none() {
            self.exit = if let Some(victory) = &plan.victory
                && self
                    .terminal
                    .is_some_and(|actor| runtime.retains_corpse_projection(actor))
            {
                Some((victory.exit, true))
            } else {
                timed_out.then_some((plan.timeout_exit, false))
            };
        }
        if let Some((destination, victory)) = self.exit {
            if !eject(
                owner,
                runtime,
                states,
                map,
                &self.room,
                destination,
                &self.participants,
                plan.credit_transition(victory)
                    .map(|key| (self.callback.as_str(), key, victory)),
            ) {
                return true;
            }
            super::personal_trial::cleanup(runtime, &mut self.creatures);
            if let Some((revision, restore)) = self.lever
                && map.facts.object_revision(self.key) == revision
            {
                return !map.facts.apply_quest_callback(
                    self.key,
                    revision,
                    restore,
                    &[],
                    &[],
                    true,
                    None,
                );
            }
            return false;
        }
        if let Some(wave) = plan.waves.get(self.next)
            && elapsed >= Duration::from_millis(wave.at_ms.into())
        {
            let terminal = if self.next + 1 == plan.waves.len() {
                plan.victory
                    .as_ref()
                    .map(|victory| victory.creature.as_str())
            } else {
                None
            };
            self.terminal = spawn_wave(owner, runtime, map, wave, &mut self.creatures, terminal);
            self.next += 1;
        }
        true
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn drain_wave_credit(&self) {
        let queue = &self.states.quest_callbacks.wave_credits;
        let Some(credit) = queue.lock().ok().and_then(|q| q.front().cloned()) else {
            return;
        };
        let Some(catalogue) = &self.quest_catalogue else {
            return;
        };
        let command = credit.command();
        let session = command.game_session_id();
        let result = match self.current_quest_fence(session).await {
            Ok(Some(fence)) => {
                let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
                slot.commit_quest_wave_egress(
                    self.root,
                    self.character,
                    self.holder,
                    fence,
                    credit,
                    catalogue.clone(),
                )
                .await
            }
            Ok(None) => Err(CharacterProgressionError::AuthorityRejected),
            Err(_) => return,
        };
        let retry = matches!(
            result,
            Err(CharacterProgressionError::Unavailable(_)
                | CharacterProgressionError::CharacterRevisionMismatch)
        );
        if let Ok(mut queue) = queue.lock()
            && queue
                .front()
                .is_some_and(|front| front.command() == command)
            && let Some(credit) = queue.pop_front()
            && retry
        {
            queue.push_back(credit);
        }
        match result {
            Ok(
                QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_),
            ) => {
                self.schedule_quest_refresh(session);
            }
            Err(error) => super::super::operator_event(&format!(
                "quest_wave_credit_deferred={retry} reason={error}"
            )),
            Ok(_) => return,
        }
    }

    pub(super) fn tick_wave_runs(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
    ) {
        let Ok(mut active) = self.states.quest_callbacks.waves.lock() else {
            return;
        };
        let now = Instant::now();
        active
            .retain_mut(|run| run.current(runtime) && run.process(self, runtime, states, map, now));
    }
}

fn spawn_wave(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    map: &BundleMap,
    wave: &Wave,
    actors: &mut Vec<ExactActorRef>,
    terminal: Option<&str>,
) -> Option<ExactActorRef> {
    let census = runtime.positioned_melee_census().ok()?;
    let mut occupied: BTreeSet<_> = census
        .into_iter()
        .filter_map(|(_, at, _)| {
            let at = native(at.position())?;
            Some((at.x, at.y, at.floor))
        })
        .collect();
    let mut places = Vec::new();
    let mut place = |creature: &str, at: Position| {
        if !occupied.contains(&(at.x, at.y, at.floor))
            && super::periodic_bosses::spawn_cell(owner, map, at)
            && let Ok(policy) = runtime.companion_policy(creature)
        {
            occupied.insert((at.x, at.y, at.floor));
            places.push((creature.to_owned(), policy.maximum_health, local(at)));
            return true;
        }
        false
    };
    match &wave.spawn {
        WaveSpawn::Fixed { monsters } => {
            for EncounterMonster { creature, position } in monsters {
                place(creature, *position);
            }
        }
        WaveSpawn::Scatter {
            creature,
            count,
            attempts_per_creature,
            min,
            max,
        } => {
            let mut spawned = 0;
            for _ in 0..u16::from(*count) * u16::from(*attempts_per_creature) {
                if spawned == *count {
                    break;
                }
                let Some(x) = super::periodic_bosses::random_below(u32::from(max.x - min.x) + 1)
                else {
                    break;
                };
                let Some(y) = super::periodic_bosses::random_below(u32::from(max.y - min.y) + 1)
                else {
                    break;
                };
                if place(
                    creature,
                    Position {
                        x: min.x + x as u16,
                        y: min.y + y as u16,
                        floor: min.floor,
                    },
                ) {
                    spawned += 1;
                }
            }
        }
    }
    actors.try_reserve(places.len()).ok()?;
    let mut terminal_actor = None;
    for chunk in places.chunks(8) {
        let batch: Vec<_> = chunk
            .iter()
            .map(|(key, health, at)| (key.as_str(), *health, *at))
            .collect();
        if let Ok(spawned) = runtime.commit_quest_creature_spawn(&batch) {
            for (actor, (creature, _, _)) in spawned.iter().zip(&batch) {
                if terminal == Some(*creature) {
                    terminal_actor = Some(*actor);
                }
            }
            actors.extend(spawned);
            for (_, _, at) in &batch {
                if let Some(pos) = native(*at) {
                    map.facts.emit_movement_effect(pos, 11);
                }
            }
        }
    }
    terminal_actor
}

#[allow(
    clippy::too_many_arguments,
    reason = "one bounded egress binds its room and exact survivor credit"
)]
fn eject(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    map: &BundleMap,
    room: &Encounter,
    destination: Position,
    participants: &[(ExactActorRef, GameSessionId)],
    credit: Option<(&str, &str, bool)>,
) -> bool {
    let Ok(census) = runtime.positioned_melee_census() else {
        return false;
    };
    let closed = crate::map::floor::BundleDoors(&map.facts);
    let mut players = Vec::new();
    let mut proofs = Vec::new();
    let mut more = false;
    for (actor, position, session) in census {
        let Some(session) = session else { continue };
        if states.is_dead(actor)
            || !super::room_contains(room, position.position())
            || (!room.wave_run.as_ref().is_some_and(|run| run.eject_room)
                && !participants.contains(&(actor, session)))
        {
            continue;
        }
        if proofs.len() == 16 {
            more = true;
            break;
        }
        let Ok(proof) = crate::map::floor::prepare_quest_encounter_ejection(
            runtime,
            owner.movement_cells(),
            actor,
            session,
            position,
            local(destination),
            &closed,
        ) else {
            return false;
        };
        let lease = if credit.is_some() {
            let Ok(lease) = runtime.bound_attacker_lease(actor, session) else {
                return false;
            };
            Some(lease)
        } else {
            None
        };
        players.push((actor, session, position.position(), lease));
        proofs.push(proof);
    }
    if proofs.is_empty() {
        return true;
    }
    let mut credits = if credit.is_some() {
        let Ok(mut queue) = owner.states.quest_callbacks.wave_credits.lock() else {
            return false;
        };
        if queue.len() + proofs.len() > 128 || queue.try_reserve(proofs.len()).is_err() {
            return false;
        }
        Some(queue)
    } else {
        None
    };
    let Ok(snapshots) = runtime.commit_quest_relocation(proofs, || true) else {
        return false;
    };
    let now = owner.owner_now().get();
    for ((actor, session, from, lease), after) in players.into_iter().zip(snapshots) {
        if let (Some(queue), Some((callback, transition, victory)), Some(lease)) =
            (&mut credits, credit, lease)
            && let Some(command) =
                super::step_command(actor, session, after.revision(), owner.reset_epoch)
        {
            queue.push_back(WaveEgress::from_committed_movement(
                callback.to_owned(),
                transition.to_owned(),
                actor,
                command,
                victory,
                lease,
                runtime.encounter_owner_scope().1,
                after.revision(),
                owner.reset_epoch,
            ));
        }
        states.stage_special_movement_step(
            runtime,
            map,
            actor,
            session,
            now,
            Some(from),
            after.position(),
            None,
        );
        states.stage_underwater_step(
            runtime,
            actor,
            session,
            now,
            underwater(map, from),
            underwater(map, after.position()),
        );
        map.facts.emit_movement_effect(tile(destination), 11);
    }
    !more
}
