//! Private phases of one native combat turn, under the caller's existing guards.

mod proposals;

use super::*;
use crate::ai_think::profile_schedule::SchedulePlan;
use crate::creature_auto_attack::ProfileAbilityProposal;
use proposals::{dispatch_proposal, dispatch_summons};

pub(super) struct ThinkShared<'a> {
    pub child_closure: &'a Option<std::sync::Arc<NativeChildClosure>>,
    pub native_appearance_content:
        &'a Option<std::sync::Arc<crate::content::native_gameplay::NativeGameplayState>>,
    pub new_children: &'a mut Vec<(ExactActorRef, Ref, SemanticTimeMicros, RuntimeWorkStamp)>,
    pub revisions: &'a RevisionSet,
}

struct ThinkOwners<'a> {
    melee: &'a mut AutoAttackOwner,
    spells: &'a mut DamageSpellOwner,
    composed: &'a mut ComposedOwner,
    chains: &'a mut ChainOwner<SpellSource>,
    heals: &'a mut CreatureSelfHealLedger,
    area_owner: &'a mut AreaHealOwner,
    threshold_heal_owner: &'a mut ThresholdHealOwner,
    icicle_owner: &'a mut oteryn_gameplay_encounters::bosses::icicle::creature_icicle::IcicleOwner,
    defense_summon_owner: &'a mut crate::monster_summon::DefenseSummonOwner,
    summons: &'a mut crate::monster_summon::NativeSummonOwner,
    time_guardian_owner: &'a mut oteryn_gameplay_encounters::bosses::seven_encounters::source_encounter_seven::TimeGuardianOwner,
    callback_casts: &'a mut oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast::CallbackCastOwner,
    callback_spawn_owner: &'a mut oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_spawn::CallbackSpawnOwner,
    smelly: &'a mut oteryn_gameplay_encounters::bosses::smelly_cheese::smelly_encounter_dispatch::SmellyEncounterOwner,
    child_closure: &'a Option<std::sync::Arc<NativeChildClosure>>,
    native_appearance_content: &'a Option<std::sync::Arc<crate::content::native_gameplay::NativeGameplayState>>,
    area_catalog: &'a Option<CreatureHealCatalog>,
    new_children: &'a mut Vec<(ExactActorRef, Ref, SemanticTimeMicros, RuntimeWorkStamp)>,
}

struct ThinkInput<'a, S, R> {
    runtime: &'a mut ChannelRuntimeV1,
    states: &'a mut S,
    fence: &'a ScopeRuntimeFence,
    reader: &'a mut R,
    stamp: RuntimeWorkStamp,
    occurrence: ThinkOccurrence,
    due: SemanticTimeMicros,
    revisions: &'a RevisionSet,
}

fn think_result(occurrence: ThinkOccurrence) -> ThinkCombatResult {
    ThinkCombatResult {
        seven_map_pending: Vec::new(),
        time_phases: Vec::new(),
        source_items: Vec::new(),
        callback_casts: Vec::new(),
        callback_spawns: Vec::new(),
        smelly_scheduled: Vec::new(),
        occurrence,
        selection: Ok(()),
        schedule: Ok(()),
        spells: Vec::new(),
        defense_variants: Vec::new(),
        composed: Vec::new(),
        chains: Vec::new(),
        unqualified: Vec::new(),
        deferred_defence: Vec::new(),
        defenses: Vec::new(),
        defense_summons: Vec::new(),
        deferred_summons: 0,
        follow: None,
        summons: Vec::new(),
    }
}

fn run_think<S: MonsterCombatStates, R: MonsterCombatReader>(
    actors: &mut [NativeActor],
    owners: &mut ThinkOwners<'_>,
    input: &mut ThinkInput<'_, S, R>,
) -> ThinkCombatResult {
    let stamp = input.stamp;
    let occurrence = input.occurrence;
    let due = input.due;
    let revisions = input.revisions;
    let runtime = &mut *input.runtime;
    let states = &mut *input.states;
    let fence = input.fence;
    let reader = &mut *input.reader;
    let melee = &mut *owners.melee;
    let threshold_heal_owner = &mut *owners.threshold_heal_owner;
    let mut result = think_result(occurrence);
    let role = match crate::monster_summon::current_source_creature_role(runtime, occurrence.actor)
    {
        Ok(
            role @ (crate::monster_summon::CurrentSourceCreatureRole::Ordinary
            | crate::monster_summon::CurrentSourceCreatureRole::IntrinsicMonsterSummon(_)),
        ) => role,
        _ => {
            result.selection = Err(AttackError::InvalidSource);
            return result;
        }
    };
    let master = role.intrinsic_master();
    let inherited = master.and_then(|m| {
        actors
            .iter()
            .find(|a| a.actor == m && runtime.contains_live_creature(m))
            .and_then(|a| a.selected_target)
    });
    let Some(a) = actors.iter_mut().find(|a| a.actor == occurrence.actor) else {
        result.selection = Err(AttackError::InvalidSource);
        return result;
    };
    result.unqualified = a.unqualified.clone();
    if a.content != runtime.content_pin().server_artifact_digest() {
        melee.clear_target(a.actor);
        result.selection = Err(AttackError::ContentChanged);
        return result;
    }
    let selected = if master.is_some() {
        Some(inherited.filter(|(target, session)| {
            reader.summon_inherited_target_eligible(
                runtime,
                a.actor,
                *target,
                *session,
                stamp,
                &a.behavior,
            ) == Some(true)
        }))
    } else {
        reader.selected_player(runtime, stamp, occurrence, &a.behavior)
    };
    a.selected_target = None;
    let target = match selected {
        Some(Some((target, session))) => {
            if !a.behavior.targeting.hostile || !a.behavior.targeting.can_target {
                melee.clear_target(a.actor);
                None
            } else if runtime.player_control_facts(target, session).is_err() {
                melee.clear_target(a.actor);
                result.selection = Err(AttackError::StaleTarget);
                return result;
            } else {
                if !a.melee.is_empty()
                    && let Err(e) = melee.accept_ai_target(
                        runtime, fence, stamp, occurrence, target, session, &a.melee, due,
                    )
                {
                    result.selection = Err(e);
                    return result;
                }
                let distance = runtime
                    .read_actor_position(a.actor)
                    .ok()
                    .zip(runtime.read_actor_position(target).ok())
                    .and_then(|(i, t)| {
                        if i.position().floor == t.position().floor {
                            u16::try_from(
                                (i64::from(i.position().x) - i64::from(t.position().x))
                                    .abs()
                                    .max(
                                        (i64::from(i.position().y) - i64::from(t.position().y))
                                            .abs(),
                                    ),
                            )
                            .ok()
                        } else {
                            None
                        }
                    });
                Some(AttackTarget {
                    actor: target,
                    same_floor_distance: distance,
                })
            }
        }
        Some(None) => {
            melee.clear_target(a.actor);
            None
        }
        None => {
            melee.clear_target(a.actor);
            result.selection = Err(AttackError::MissingCombatFacts);
            None
        }
    };
    a.selected_target = if target.is_some() {
        selected.flatten()
    } else {
        None
    };
    if let Some(parent) = master
        && target.is_none()
        && a.behavior.movement.can_walk
    {
        result.follow = Some(
            match reader.summon_follow_movement(runtime, a.actor, parent, &a.creature, stamp) {
                Some(facts) => commit_paced_summon_follow(
                    runtime,
                    fence,
                    stamp,
                    a.actor,
                    parent,
                    facts,
                    a.native_speed.as_ref(),
                    &mut a.move_pacer,
                    due,
                ),
                None => Err(SummonFollowError::MissingMap),
            },
        );
    }
    // Source timers are delivered even without a fresh chance/admitted cast.
    for source in &a.threshold_heals {
        match threshold_heal_owner.tick(runtime, fence, stamp, source, a.actor, due.get()) {
            Ok(Some(receipt)) => result.defenses.push((
                source.ability().clone(),
                Ok(DefenseOutcome::ThresholdHeal(receipt)),
            )),
            Ok(None) => {}
            Err(e) => result.defenses.push((
                source.ability().clone(),
                Err(DefenseError::ThresholdHeal(e)),
            )),
        }
    }
    let root = GameplayDecisionRoot::from_bytes(a.content);
    let facts = if let Some(catalog) = &a.summons {
        let path_target = target
            .as_ref()
            .map(|t| t.actor)
            .filter(|t| reader.summon_target_reachable(runtime, a.actor, *t, stamp) == Some(true));
        Some(catalog.facts(runtime, occurrence, path_target))
    } else if master.is_some() {
        a.behavior
            .summons
            .as_ref()
            .map(|entries| MonsterSummonFacts {
                occurrence,
                is_summon: true,
                target_with_path: None,
                total_count: runtime.native_summon_count(a.actor, None),
                entry_counts: entries
                    .entries
                    .iter()
                    .map(
                        |entry| crate::ai_think::profile_schedule::ObservedSummonCount {
                            creature: entry.creature.clone(),
                            count: runtime.native_summon_count(a.actor, Some(&entry.creature.key)),
                        },
                    )
                    .collect(),
            })
    } else {
        reader.summon_facts(runtime, stamp, occurrence, &a.behavior)
    };
    let plan = a.schedule.prepare_with_summons(
        ProfileScheduleInput {
            occurrence,
            behavior: &a.behavior,
            abilities: &a.abilities,
            target,
            revisions,
            root: &root,
        },
        facts.as_ref(),
    );
    let plan = match plan {
        Ok(p) => p,
        Err(e) => {
            result.schedule = Err(e);
            return result;
        }
    };
    let mut input = ThinkInput {
        runtime,
        states,
        fence,
        reader,
        stamp,
        occurrence,
        due,
        revisions,
    };
    dispatch_summons(owners, &mut input, a, &plan, &mut result);
    for proposal in plan.proposals {
        dispatch_proposal(owners, &mut input, a, proposal, &root, &mut result);
    }
    result
}

impl MonsterCombatLane {
    pub(super) fn run_due_thinks<R: MonsterCombatReader>(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut impl MonsterCombatStates,
        current: &mut ScopeRuntimeFence,
        clock: &impl OwnerClock,
        reader: &mut R,
        shared: ThinkShared<'_>,
    ) -> Result<Vec<ThinkCombatResult>, CombatLaneError> {
        let time_guardian_owner = &mut self.time_guardian_owner;
        let actors = &mut self.actors;
        let melee = &mut self.melee;
        let spells = &mut self.spells;
        let composed = &mut self.composed;
        let chains = &mut self.chains;
        let heals = &mut self.heals;
        let area_catalog = &self.area_catalog;
        let area_owner = &mut self.area_owner;
        let threshold_heal_owner = &mut self.threshold_heal_owner;
        let icicle_owner = &mut self.icicle_owner;
        let defense_summon_owner = &mut self.defense_summon_owner;
        let summons = &mut self.summons;
        if self.callback_casts.is_none() {
            self.callback_casts = Some(
                oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast::CallbackCastOwner::new(runtime, current)
                    .map_err(CombatLaneError::Attack)?,
            );
        }
        let callback_casts = self
            .callback_casts
            .as_mut()
            .ok_or(CombatLaneError::Attack(AttackError::InvalidPlan))?;
        if self.callback_spawn_owner.is_none() {
            self.callback_spawn_owner = Some(
                oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_spawn::CallbackSpawnOwner::new(runtime, current)
                    .map_err(CombatLaneError::Attack)?,
            );
        }
        let callback_spawn_owner = self
            .callback_spawn_owner
            .as_mut()
            .ok_or(CombatLaneError::Attack(AttackError::InvalidPlan))?;
        let smelly = &mut self.smelly;
        let mut owners = ThinkOwners {
            melee,
            spells,
            composed,
            chains,
            heals,
            area_owner,
            threshold_heal_owner,
            icicle_owner,
            defense_summon_owner,
            summons,
            time_guardian_owner,
            callback_casts,
            callback_spawn_owner,
            smelly,
            area_catalog,
            child_closure: shared.child_closure,
            native_appearance_content: shared.native_appearance_content,
            new_children: shared.new_children,
        };
        self.timer
            .run_due(
                runtime,
                states,
                current,
                clock,
                |runtime, states, fence, stamp, occurrence, due| {
                    run_think(
                        actors,
                        &mut owners,
                        &mut ThinkInput {
                            runtime,
                            states,
                            fence,
                            reader,
                            stamp,
                            occurrence,
                            due,
                            revisions: shared.revisions,
                        },
                    )
                },
            )
            .map_err(CombatLaneError::Cycle)
    }
}

impl MonsterCombatLane {
    pub(super) fn run_time_returns(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        current: &mut ScopeRuntimeFence,
        clock: &impl OwnerClock,
    ) -> Result<Vec<Result<oteryn_gameplay_encounters::bosses::seven_encounters::source_encounter_seven::TimePhaseOutcome, AttackError>>, CombatLaneError>{
        let return_ordinal = current
            .accept_input(runtime.binding().scope_generation())
            .map_err(|e| CombatLaneError::Cycle(CycleError::Ordinal(e)))?;
        let return_stamp = current.stamp(return_ordinal);
        let due_returns = self
            .time_guardian_owner
            .due_returns(runtime, current, clock)
            .map_err(CombatLaneError::Attack)?;
        let mut time_returns = Vec::new();
        time_returns
            .try_reserve(due_returns.len())
            .map_err(|_| CombatLaneError::Attack(AttackError::LedgerFull))?;
        for due in due_returns {
            let result = (|| {
                let prepared = due?;
                let actor =
                    oteryn_gameplay_encounters::bosses::seven_encounters::source_encounter_seven::TimeGuardianOwner::returned_actor(&prepared);
                let native = self
                    .actors
                    .iter_mut()
                    .find(|a| a.actor == actor)
                    .ok_or(AttackError::StaleIssuer)?;
                native.dispatch_time_return(
                    runtime,
                    current,
                    return_stamp,
                    &mut self.time_guardian_owner,
                    prepared,
                )
            })();
            time_returns.push(result);
        }
        Ok(time_returns)
    }

    pub(super) fn tick_player_conditions<R: MonsterCombatReader>(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut impl MonsterCombatStates,
        reader: &mut R,
        stamp: RuntimeWorkStamp,
        now: SemanticTimeMicros,
    ) -> Result<Vec<(ExactActorRef, Option<NativeOwnedPlayerTickReceipt>)>, AttackError> {
        // One target-owned pass per current player, independent of any creature/think roster.
        // Retained caster provenance may be dead; actual condition owner handles that distinction.
        let players = reader.current_players(runtime, stamp);
        match players {
            Some(players) if players.len() <= 256 => {
                let mut keys = std::collections::BTreeSet::new();
                if players
                    .iter()
                    .any(|(a, _)| !keys.insert(a.placement_identity()))
                {
                    Err(AttackError::InvalidPlan)
                } else {
                    Ok(players
                        .into_iter()
                        .map(|(actor, session)| {
                            let facts = reader
                                .source_player_condition_tick_facts(runtime, actor, session, stamp);
                            let result = facts.and_then(|facts| {
                                let hash = Sha256::new()
                                    .chain_update(b"oteryn:aggregate-native-condition-tick:v1")
                                    .chain_update(
                                        format!("{:?}:{:?}:{}", actor, session, now.get())
                                            .as_bytes(),
                                    )
                                    .finalize();
                                let mut id = [0; 16];
                                id.copy_from_slice(&hash[..16]);
                                states.tick_source_player_conditions(
                                    runtime,
                                    actor,
                                    session,
                                    DecisionOccurrenceId::from_bytes(id),
                                    facts,
                                    now,
                                )
                            });
                            (actor, result)
                        })
                        .collect())
                }
            }
            _ => Err(AttackError::MissingCombatFacts),
        }
    }
}
