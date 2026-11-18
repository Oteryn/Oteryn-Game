//! Phantom Army source survival on the Channel's shared encounter deadline owner.
//! Crystal 201189f8 movements_click.lua:1-125.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use crate::map::facts::QuestCallbackProjection;
use oteryn_gameplay_encounter_rules::raid::{RaidArea, RaidPosition, RaidRandom};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::phantom_army::{QUESTLINE, SURVIVED};
use std::collections::BTreeSet;

const PLATES: [(u16, u16); 4] = [
    (33258, 31080),
    (33266, 31084),
    (33259, 31089),
    (33263, 31093),
];
const STAIR: TilePos = TilePos {
    x: 33265,
    y: 31116,
    floor: -8,
};
const RETURN: MovementLocalPosition = MovementLocalPosition {
    x: 33353,
    y: 31410,
    floor: 8,
};
const INTERVAL_US: u64 = 30_000_000;
const DURATION_US: u64 = 600_000_000;
const GUARDIAN: &str = "oteryn:creature.eternal_guardian";
const CHOSEN: &str = "oteryn:creature.lizard_chosen";

pub(super) struct SurvivalRun {
    scope: (RuntimeScopeRefV1, ScopeOwnershipGeneration),
    map: [u8; 32],
    content: [u8; 32],
    began: u64,
    wave: u8,
    credits: Option<Vec<(ExactActorRef, CommandRef)>>,
    retry_at: u64,
}

impl SurvivalRun {
    pub(super) fn next_due(&self) -> u64 {
        if self.credits.is_some() {
            return self.retry_at;
        }
        self.began
            .saturating_add((u64::from(self.wave) * INTERVAL_US).min(DURATION_US))
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn step_phantom_army(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
    ) {
        let Some(at) = native(entered.position()) else {
            return;
        };
        if at.floor != -8 || !PLATES.contains(&(at.x, at.y)) {
            return;
        }
        let Some(map) = self.bundle_map() else { return };
        let mut runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        if permit.runtime_scope() != runtime.binding().runtime_scope()
            || runtime.read_actor_position(actor) != Ok(entered)
            || states.is_dead(actor)
            || !self.states.quest_sessions.lock().is_ok_and(|sessions| {
                sessions
                    .get(&session)
                    .and_then(|quest| quest.copy.as_ref())
                    .is_some_and(|quest| quest.tracks().get(QUESTLINE) == Some(&19))
            })
        {
            return;
        }
        let Ok(mut active) = self.states.quest_callbacks.survival.lock() else {
            return;
        };
        if active.is_some() {
            return;
        }
        let Ok(census) = runtime.positioned_melee_census() else {
            return;
        };
        let mut occupants = 0;
        for (x, y) in PLATES {
            if let Some((who, _, Some(other))) = census.iter().find(|(who, position, _)| {
                let at = position.position();
                at.x == i32::from(x)
                    && at.y == i32::from(y)
                    && at.floor == 8
                    && runtime.is_current_top_actor(*who) == Ok(true)
            }) && !states.is_dead(*who)
            {
                self.states
                    .chat
                    .push_quest_feedback(*other, "A clicking sound tatters the silence.");
                occupants += 1;
            }
        }
        if occupants != PLATES.len() {
            return;
        }
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return;
        };
        if runtime.owner_fence().is_err()
            || native.install_companion_policies(&mut runtime).is_err()
            || [GUARDIAN, CHOSEN]
                .into_iter()
                .any(|species| runtime.companion_policy(species).is_err())
        {
            return;
        }
        let Some(base) = map.overlay.base().tile(STAIR.x, STAIR.y, STAIR.floor) else {
            return;
        };
        let Some((key, restore)) = base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, depth)| **depth == 0)
            .enumerate()
            .find_map(|(ordinal, (id, _))| {
                let ordinal = u8::try_from(ordinal).ok()?;
                let key = placement_key(STAIR, ordinal)?;
                let facts = map.facts.base_entry(STAIR, ordinal, *id)?;
                (facts.appearance_id == 1897 && !map.facts.base_hidden(key)).then_some((key, facts))
            })
        else {
            return;
        };
        let Some(open) = map.facts.quest_callback_replacement(key, 1977) else {
            return;
        };
        let began = self.owner_now().get();
        let Some(due) = began.checked_add(INTERVAL_US) else {
            return;
        };
        if !map.facts.apply_quest_callback(
            key,
            map.facts.object_revision(key),
            open,
            &[],
            &[],
            false,
            Some(&QuestCallbackProjection::TimedPassableTransform { due, restore }),
        ) {
            return;
        }
        *active = Some(SurvivalRun {
            scope: runtime.encounter_owner_scope(),
            map: map.overlay.base().digest(),
            content: runtime.content_pin().server_artifact_digest(),
            began,
            wave: 1,
            credits: None,
            retry_at: due,
        });
        self.states.chat.push_quest_feedback(session,
            "The army is complete again. You hear a hatch opening elsewhere, followed by a grinding sound.");
        states.owner_wake.notify_one();
    }

    pub(super) async fn drain_survival_encounter(&self, permit: &SpellLanePermit) {
        let Some(map) = self.bundle_map() else { return };
        let now = self.owner_now().get();
        let credits = {
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let Ok(mut active) = self.states.quest_callbacks.survival.lock() else {
                return;
            };
            let Some(run) = active.as_mut() else { return };
            if permit.runtime_scope() != runtime.binding().runtime_scope()
                || runtime.owner_fence().is_err()
                || runtime.encounter_owner_scope() != run.scope
                || map.overlay.base().digest() != run.map
                || runtime.content_pin().server_artifact_digest() != run.content
            {
                *active = None;
                return;
            }
            if now < run.next_due() {
                return;
            }
            if now < run.began.saturating_add(DURATION_US) {
                while run.wave < 20 && now >= run.next_due() {
                    summon_wave(
                        &mut runtime,
                        map,
                        self.movement_cells(),
                        run.began,
                        run.wave,
                    );
                    run.wave += 1;
                }
                return;
            }
            if run.credits.is_none() {
                let Some(credit) = self.eject_survivors(&mut runtime, &mut states, map, now) else {
                    return;
                };
                run.credits = Some(credit);
            }
            run.credits.take().unwrap_or_default()
        };

        let mut retry = Vec::new();
        for (actor, command) in credits {
            if !self.settle_survivor(actor, command).await {
                retry.push((actor, command));
            }
        }
        if let Ok(mut active) = self.states.quest_callbacks.survival.lock() {
            if retry.is_empty() {
                *active = None;
            } else if let Some(run) = active.as_mut() {
                run.credits = Some(retry);
                run.retry_at = now.saturating_add(1_000_000);
            }
        }
    }

    fn eject_survivors(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
        now: u64,
    ) -> Option<Vec<(ExactActorRef, CommandRef)>> {
        let room = |at: MovementLocalPosition| {
            at.floor == 7 && (33242..=33294).contains(&at.x) && (31099..=31139).contains(&at.y)
        };
        let census = runtime.positioned_melee_census().ok()?;
        let players = census
            .iter()
            .filter_map(|(actor, position, session)| {
                let session = (*session)?;
                (room(position.position()) && !states.is_dead(*actor))
                    .then_some((*actor, *position, session))
            })
            .collect::<Vec<_>>();
        let closed = crate::map::floor::BundleDoors(map.facts.doors());
        let proofs = players
            .iter()
            .map(|(actor, position, session)| {
                crate::map::floor::prepare_quest_encounter_ejection(
                    runtime,
                    self.movement_cells(),
                    *actor,
                    *session,
                    *position,
                    RETURN,
                    &closed,
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        let snapshots = runtime.commit_quest_relocation(proofs, || true).ok()?;
        for (actor, position, session) in census {
            if session.is_none()
                && room(position.position())
                && runtime.contains_live_creature(actor)
            {
                let _ = runtime.remove_encounter_actor(actor);
            }
        }
        let mut credits = Vec::new();
        for ((actor, before, session), after) in players.into_iter().zip(snapshots) {
            states.stage_special_movement_step(
                runtime,
                map,
                actor,
                session,
                now,
                Some(before.position()),
                after.position(),
                None,
            );
            states.stage_underwater_step(
                runtime,
                actor,
                session,
                now,
                underwater(map, before.position()),
                underwater(map, after.position()),
            );
            map.facts.emit_movement_effect(
                TilePos {
                    x: 33353,
                    y: 31410,
                    floor: -8,
                },
                11,
            );
            if let Some(command) = step_command(actor, session, after.revision(), self.reset_epoch)
            {
                credits.push((actor, command));
            }
        }
        Some(credits)
    }

    async fn settle_survivor(&self, actor: ExactActorRef, command: CommandRef) -> bool {
        let session = command.game_session_id();
        if self
            .runtime
            .lock()
            .await
            .player_control_facts(actor, session)
            .is_err()
        {
            return true;
        }
        let Some(catalogue) = self.quest_catalogue.as_ref() else {
            return true;
        };
        let fence = match self.current_quest_fence(session).await {
            Ok(Some(fence)) => fence,
            Ok(None) => return true,
            Err(()) => return false,
        };
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_quest_transition(
                self.root,
                self.character,
                self.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: SURVIVED.into(),
                    cause: QuestCause::Step(command),
                },
                std::sync::Arc::clone(catalogue),
            )
            .await;
        drop(slot);
        match outcome {
            Ok(
                QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_),
            ) => {
                self.refresh_quest_session(session, false).await;
                true
            }
            Ok(QuestTransitionOutcome::Refused(_)) => true,
            _ => false,
        }
    }
}

fn summon_wave(
    runtime: &mut ChannelRuntimeV1,
    map: &BundleMap,
    cells: &crate::content::NativeEntryMovementCells,
    started: u64,
    wave: u8,
) {
    let species = if wave.is_multiple_of(4) {
        CHOSEN
    } else {
        GUARDIAN
    };
    let Ok(policy) = runtime.companion_policy(species) else {
        return;
    };
    let health = policy.maximum_health;
    let Ok(census) = runtime.positioned_melee_census() else {
        return;
    };
    let mut occupied = census
        .into_iter()
        .map(|(_, snapshot, _)| {
            let at = snapshot.position();
            (at.x, at.y, at.floor)
        })
        .collect::<BTreeSet<_>>();
    let mut firing = [0_u8; 16];
    firing[..8].copy_from_slice(&started.to_be_bytes());
    let random = RaidRandom::new(runtime.content_pin().server_artifact_digest(), firing);
    let area = RaidArea {
        from: RaidPosition {
            x: 33252,
            y: 31105,
            floor: 7,
        },
        to: RaidPosition {
            x: 33288,
            y: 31134,
            floor: 7,
        },
    };
    let mut spawned = 0;
    for attempt in 0..200 {
        let Ok(candidate) = area.candidate_shared(&random, u16::from(wave), 0, 0, attempt) else {
            break;
        };
        let (Ok(x), Ok(y)) = (u16::try_from(candidate.x), u16::try_from(candidate.y)) else {
            continue;
        };
        let at = TilePos { x, y, floor: -7 };
        let cell = LogicalCell {
            x: candidate.x,
            y: candidate.y,
            z: 7,
        };
        if occupied.contains(&(cell.x, cell.y, 7))
            || cells.index().lookup(cells.scope(), cell) != Ok(CollisionClass::Walkable)
            || !cells
                .spell_tiles()
                .lookup(cells.scope(), cell)
                .is_ok_and(|tile| !tile.flags().protection_zone && !tile.flags().floor_change)
            || map.facts.house_tile(at)
            || map
                .facts
                .quest_callback_blocked(map.overlay.base(), at, false)
        {
            continue;
        }
        if runtime
            .commit_quest_creature_spawn(&[(
                species,
                health,
                MovementLocalPosition {
                    x: cell.x,
                    y: cell.y,
                    floor: 7,
                },
            )])
            .is_ok()
        {
            occupied.insert((cell.x, cell.y, 7));
            map.facts.emit_movement_effect(at, 11);
            spawned += 1;
            if spawned == 20 {
                break;
            }
        }
    }
}
