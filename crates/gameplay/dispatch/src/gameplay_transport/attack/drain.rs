//! Ordered phases of the Channel owner's auto-attack pass.

use super::player_block::PlayerBiteVitals;
use super::shared_experience::aggressive_monster;
use super::skill_training::{TrainingDue, swing_block, swing_family, train_hit};
use super::*;
use super::{charm_combat, equipped_offence};
use crate::ability::creature_bite::CreatureBiteVitals;
use crate::ability::exact_actor_resolution::{ExactActorProposal, resolve_exact_actor};
use crate::combat_attack::{SwingPoll, player_fist_formula, player_melee_formula};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::durability::{DurabilityError, character_progression::CharacterProgressionError};
use crate::foundation::CharacterId;
use crate::foundation::owner_timer::SemanticTimeMicros as OwnerTime;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::gameplay_transport::connection::AdmissionRefusal;
use crate::gameplay_transport::kill_reward::KillSettlementQueue;
use crate::gameplay_transport::task_board::bounty_talisman;
use crate::spell::cast::{CharacterCastFacts, SwingSkills};
use crate::spell::combat_execution::actor_atom;
use crate::spell::formula::FormulaInputs;
use crate::world_runtime::LocalObjectRuntime;
use oteryn_gameplay_combat_rules::attack::formulas::player_monk_formula;
use oteryn_gameplay_combat_rules::attack::target::Swing;
use oteryn_gameplay_combat_rules::resolution::{
    creature_melee_mitigation, creature_physical_resistance, creature_physically_immune,
    equipped_offence::{CRITICAL_ROLL_MAX, CRITICAL_ROLL_MIN, EquippedOffence},
    swing_mitigated,
};
use oteryn_gameplay_items::equipment_effects::apply_protection;
use oteryn_gameplay_progression::charm_effects::{
    CharmAttackerFacts, CharmEffectOutcome, CharmHitSource, CharmHookEvent,
};
use oteryn_gameplay_progression::hit_training::HitBlock;
use sha2::Digest;

#[derive(Debug, Clone)]
pub(super) struct SelectedSwing {
    pub(super) target: ExactActorRef,
    pub(super) character_facts: CharacterCastFacts,
    pub(super) skills: SwingSkills,
    pub(super) weapon: WeaponFacts,
    pub(super) swing: Swing<ExactActorRef, CommandRef>,
    pub(super) ordinal: u16,
}

/// A PvP swing the attacker's pass drew for a player target; its defence is met after
/// the pass (PVP-DEFENCE-1).
#[derive(Debug)]
pub(super) struct PvpSwing {
    pub(super) actor: ExactActorRef,
    pub(super) session: GameSessionId,
    pub(super) modes: oteryn_protocol_oteryn::pvp_expert::ExpertFightModes,
    pub(super) extension: bool,
    pub(super) train: bool,
    pub(super) target: ExactActorRef,
    pub(super) lineage: CommandRef,
    pub(super) ordinal: u16,
    // PvP halving and outgoing conditions have already shaped both components.
    pub(super) components: (i64, i64, crate::content::ProjectV2RangedElement),
    pub(super) target_protection: super::super::pvp_live::PvpSwingTarget,
    pub(super) revision: &'static str,
    pub(super) weapon: WeaponFacts,
    pub(super) attacker: CharacterId,
    pub(super) generation: u64,
}

// Inspect a clone: a missing source name read must not consume the real swing
// deadline/ordinal or draw any RNG. Cleave uses its existing exact target selector.
fn native_health_swing_targets(
    entry: &ActorAttack,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    digest: [u8; 32],
    now: SemanticTimeMicros,
    constants: &AttackConstants,
) -> Result<Vec<ExactActorRef>, ()> {
    let mut preview = entry.clone();
    let Some(selected) = select_swing(&mut preview, runtime, states, room, digest, now, constants)
    else {
        return Ok(Vec::new());
    };
    let mut targets = equipped_offence::cleave_targets(&preview, &selected, runtime, room, now);
    targets.extend(ranged::possible_area_targets(
        &preview, &selected, runtime, room, now,
    ));
    targets.push(selected.target);
    targets.retain(|target| runtime.contains_live_creature(*target));
    let mut required = Vec::new();
    for target in targets {
        if runtime.companion_snapshot(target).is_err() {
            let key = std::str::from_utf8(
                runtime
                    .current_live_creature_identity(target)
                    .map_err(|_| ())?,
            )
            .map_err(|_| ())?;
            if crate::content::native_monster_callbacks::requires_native_health_event(key) {
                return Err(());
            }
            continue;
        }
        if runtime.native_health_event(target).map_err(|_| ())?.is_some_and(|event| {
            event.kind() == crate::content::native_monster_callbacks::NativeHealthEventKind::HealthForgotten
        }) && !required.contains(&target) {
            required.push(target);
        }
    }
    Ok(required)
}

fn observe_native_attack_health(
    runtime: &ChannelRuntimeV1,
    names: Option<&crate::gameplay_transport::source_names::NativeHealthNamesData>,
    target: ExactActorRef,
) -> Result<Option<crate::foundation::NativeHealthChangeObservation>, ()> {
    match names {
        Some(names) => names.observe(runtime, target),
        None => match runtime.observe_native_health_change(
            target,
            &|_: crate::foundation::ExactActorRef,
              _: crate::foundation::MovementPositionSnapshot| None,
        ) {
            Ok(None) => Ok(None),
            Ok(Some(_)) | Err(_) => Err(()),
        },
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// One owner pass over every held target: at most one swing per attacker (`DEADLINE_STATE`
    /// catch-up), never a backlog.
    pub(in crate::gameplay_transport) async fn drain_auto_attacks(&self) {
        self.drain_auto_attacks_at_time(None).await;
    }

    #[cfg(test)]
    pub(in crate::gameplay_transport) async fn drain_auto_attacks_at(
        &self,
        now: SemanticTimeMicros,
    ) {
        self.drain_auto_attacks_at_time(Some(now)).await;
    }

    async fn drain_auto_attacks_at_time(&self, now: Option<SemanticTimeMicros>) {
        let Some(room) = self.rooms().gate_auto_attack() else {
            return;
        };
        let Some(content) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return;
        };
        let Some(constants) = AttackConstants::checked_in() else {
            return;
        };
        // A genuine Cloak hit has a synchronous source Item callback. Prepare
        // its existing map owner before retaining the physical lane for the hit.
        let cloak_target = {
            let runtime = self.runtime.lock().await;
            let attack = self.states.attack.lock().await;
            attack
                .entries
                .iter()
                .filter_map(|entry| entry.state.target())
                .any(|target| {
                    runtime
                        .matches_live_creature_identity(target, b"oteryn:creature.cloak_of_terror")
                })
        };
        if cloak_target && !self.ensure_source_map_initialized().await {
            return;
        }
        let Some(mut pvp_permit) = self.spell_lane_permit().await else {
            return;
        };
        // A failed native health callback retains its original occurrence.
        // Resolve it under this same physical lane before spending another
        // attacker's canonical clock or ordinal.
        if !self.drain_cloak_blood_with_permit(&pvp_permit).await {
            return;
        }
        let sessions = self
            .states
            .attack
            .lock()
            .await
            .entries
            .iter()
            .filter(|entry| entry.state.target().is_some())
            .map(|entry| entry.session)
            .collect::<std::collections::BTreeSet<_>>();
        let retry = crate::gameplay_transport::AdmissionRetryBudget::for_command(self.root);
        let mut talismans = std::collections::BTreeMap::new();
        for session in sessions {
            if let Ok(snapshot) = retry
                .retry_unavailable(async || {
                    self.bounty_talisman_for_session(session)
                        .await
                        .map_err(|error| match error {
                            CharacterProgressionError::Unavailable(
                                DurabilityError::RootUnavailable,
                            ) => AdmissionRefusal::Unavailable,
                            _ => AdmissionRefusal::Rejected,
                        })
                })
                .await
            {
                talismans.insert(session, snapshot);
            }
        }
        drop(retry);
        let now = now.unwrap_or_else(|| self.owner_now());
        let needs_health_names = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            let attack = self.states.attack.lock().await;
            let digest = runtime.content_pin().server_artifact_digest();
            let mut required = false;
            for entry in &attack.entries {
                // Missing metadata belongs to this entry; the real pass below
                // retries it without starving unrelated attackers.
                if let Ok(targets) = native_health_swing_targets(
                    entry, &runtime, &states, room, digest, now, constants,
                ) {
                    required |= !targets.is_empty();
                }
            }
            required
        };
        // The actual fixed-name SQL runs with no Runtime, spell-state or attack
        // guard held. Ordinary targets cause no extra name read.
        let health_names = if needs_health_names {
            self.read_native_health_names(&pvp_permit).await.ok()
        } else {
            None
        };
        let mut runtime = self.runtime.lock().await;
        let health_names_data =
            health_names.and_then(|read| self.seal_native_health_names(&runtime, read).ok());
        let digest = runtime.content_pin().server_artifact_digest();
        if runtime.owner_fence().is_err()
            || content.source_digest() != digest
            || room.movement_cells().scope().generation_digest != digest
        {
            return;
        }
        let mut states = self.states.spell_states.lock().await;
        let objects = self.door.lock().await;
        let mut attack = self.states.attack.lock().await;
        attack.prune(&runtime);
        let root = GameplayDecisionRoot::from_bytes(digest);
        let mut trained = TrainingDue::new();
        if !attack.pending_pvp.is_empty() {
            self.drain_pvp_impacts(
                &mut pvp_permit,
                &mut runtime,
                &mut states,
                &mut attack,
                room,
                &objects,
                content,
                (&root, &digest, constants, now),
                &mut trained,
            )
            .await;
            drop(attack);
            drop(objects);
            drop(states);
            drop(runtime);
            drop(pvp_permit);
            self.save_trained_levels(trained).await;
            return;
        }
        // Only lineages already held by the real attack owner are read. The
        // snapshot grants neither a swing nor payment; those are selected below.
        let source_players: Vec<_> = attack
            .entries
            .iter()
            .filter_map(|entry| {
                let (target, lineage) = entry.state.held_target_lineage()?;
                if entry
                    .state
                    .next_due(constants.attack_interval_micros())
                    .is_some_and(|due| due > now)
                {
                    return None;
                }
                let target = runtime.companion_snapshot(target).ok()?;
                content.policies().soul_war_health_policy(
                    &target.state.policy.definition_key,
                    &target.state.policy.definition_revision,
                )?;
                let control = runtime
                    .player_control_facts(entry.actor, entry.session)
                    .ok()?;
                if control.control_loss.is_some() {
                    return None;
                }
                Some((entry.actor, entry.session, lineage))
            })
            .collect();
        let mut unavailable_source_players = std::collections::BTreeSet::new();
        if !source_players.is_empty() {
            let Some(active) = self.active_generation else {
                return;
            };
            let read = if let Ok(pass) = self.root.try_issue_semantic_pass() {
                let mut context = (
                    self,
                    &mut *runtime,
                    &*states,
                    &pvp_permit,
                    active,
                    room,
                    &source_players,
                );
                pass.run_with_context(&mut context,move |holder,deadline,context| Box::pin(async move {
                use crate::durability::spell_item_transaction::{begin_spell_owner_transaction,assert_spell_item_scope_in_transaction,check_scope_transaction};
                let (owner,runtime,states,permit,active,room,sources)=context;
                let mut tx=begin_spell_owner_transaction(holder,deadline).await?;
                let authority=assert_spell_item_scope_in_transaction(&mut tx,permit,owner.root,owner.character,owner.holder,
                    runtime.binding().runtime_scope(),runtime.binding().scope_generation().get()).await
                    .map_err(|_|crate::durability::DurabilityError::Unavailable)?;
                crate::gameplay_transport::soul_war_callbacks::player_source::publish_auto_sources(
                    &mut tx,owner.root,&authority,active,room,runtime,states,sources,now.get()).await?;
                check_scope_transaction(&mut tx,&authority).await.map_err(|_|crate::durability::DurabilityError::Unavailable)?;
                tx.rollback().await?;
                Ok(())
            })).await
            } else {
                Err(crate::durability::DurabilityError::Unavailable)
            };
            if read.is_err() {
                // An unavailable source enrichment refuses its own affected
                // attackers. It cannot suppress unrelated ordinary combat.
                unavailable_source_players.extend(
                    source_players
                        .iter()
                        .map(|(actor, _, _)| actor.placement_identity()),
                );
            }
            if runtime.owner_fence().is_err() {
                return;
            }
        }
        let ChannelAttackStates {
            entries,
            pending_pvp,
            blocks,
            kills,
            ..
        } = &mut *attack;
        kills.bind_kill_notices(&self.states.server_messages);
        let mut activity_cells: Vec<_> = entries
            .iter()
            .filter_map(|e| e.shared_experience_activity)
            .collect();
        for (target, lethal, at) in std::mem::take(&mut states.equipment_reflection_kills) {
            crate::gameplay_transport::kill_reward::record_creature_kill(
                &mut runtime,
                crate::gameplay_transport::kill_reward::KillSink {
                    prey: Some(&states),
                    queue: kills,
                    rewards: self.reward_table(),
                    activity: &activity_cells,
                    charms: Some(self.imported_charms),
                },
                target,
                lethal,
                at / 1_000,
                Some(SemanticTimeMicros::from_micros(at)),
            );
        }
        for entry in entries.iter_mut() {
            if runtime
                .actor_staff_visibility(entry.actor)
                .is_ok_and(|flags| flags.0)
            {
                entry.clear_target();
                continue;
            }
            // An unavailable bounty read affects this attacker only. Defer
            // before consuming its swing clock or doing ranged payment.
            if !talismans.contains_key(&entry.session)
                || unavailable_source_players.contains(&entry.actor.placement_identity())
            {
                continue;
            }
            let Ok(required_health) =
                native_health_swing_targets(entry, &runtime, &states, room, digest, now, constants)
            else {
                continue;
            };
            if required_health.iter().any(|target| {
                observe_native_attack_health(&runtime, health_names_data.as_ref(), *target).is_err()
            }) {
                // A foreign identity change after the detached read retries
                // without consuming the actual held attack's timer or ordinal.
                continue;
            }
            let state_before = entry.state.clone();
            let ordinal_before = entry.next_ordinal;
            let Some(mut selected) =
                select_swing(entry, &runtime, &states, room, digest, now, constants)
            else {
                continue;
            };
            // The selected swing already owns the current weapon. Read its current
            // selections as for spell casts; no reward survives a weapon change.
            if let Some(catalogue) = &self.proficiency_catalogue
                && let Some(key) = selected.weapon.item_key.as_deref()
                && let Ok(sessions) = self.states.proficiencies.lock()
                && let Some(track) = sessions.get(&entry.session).and_then(|p| p.track(key))
                && let Some(perks) = catalogue.cast_perks(key, track, None)
            {
                let index = if selected.weapon.ranged.is_some() {
                    4
                } else {
                    match selected.weapon.weapon {
                        SwingWeapon::Fists => 0,
                        SwingWeapon::Melee { skill, .. } => skill.index(),
                    }
                };
                selected.skills.attack = selected.skills.attack.saturating_add(perks.skills[index]);
                selected.skills.shielding =
                    selected.skills.shielding.saturating_add(perks.skills[5]);
                selected.character_facts.magic_level = selected
                    .character_facts
                    .magic_level
                    .saturating_add(perks.skills[7]);
            }
            let proficiency = selected.weapon.item_key.as_deref().and_then(|key| {
                self.states
                    .proficiencies
                    .lock()
                    .ok()?
                    .get(&entry.session)?
                    .active_attack_effects(key)
            });
            let mut pure_elemental_with_attack_points = false;
            if let Some(points) =
                proficiency.and_then(|effects| u32::try_from(effects.flat_attack).ok())
                && points > 0
            {
                if let Some(ranged) = &mut selected.weapon.ranged {
                    if let ranged::RangedDamage::Distance { attack, .. } =
                        &mut Arc::make_mut(ranged).damage
                    {
                        *attack = attack.saturating_add(points);
                    }
                } else if let SwingWeapon::Melee {
                    attack, elemental, ..
                } = &mut selected.weapon.weapon
                {
                    pure_elemental_with_attack_points = elemental.is_some() && *attack == 0;
                    *attack = attack.saturating_add(points);
                }
            }
            if selected.weapon.ranged.is_none()
                && matches!(selected.weapon.weapon, SwingWeapon::Melee { skill, .. } if (1..=3).contains(&skill.index()))
                && states
                    .get(&runtime, entry.actor, entry.session)
                    .and_then(|state| state.wheel_effects_at(now.get()))
                    .is_some_and(|effects| effects.active_unique_perks.contains("battle_instinct"))
                && let Some(bonuses) = battle_instinct_for_actor(&runtime, entry.actor)
            {
                selected.skills.attack = selected.skills.attack.saturating_add(bonuses.melee);
                selected.skills.shielding =
                    selected.skills.shielding.saturating_add(bonuses.shielding);
            }
            let pvp_target = if !runtime.contains_live_creature(selected.target) {
                let legality = self
                    .pvp_target_legal(
                        &pvp_permit,
                        &runtime,
                        &states,
                        room,
                        &objects,
                        entry.actor,
                        selected.target,
                        selected.swing.lineage,
                        oteryn_protocol_oteryn::pvp_expert::ExpertFightModes {
                            modes: entry.modes,
                            expert: entry.expert_mode,
                        },
                    )
                    .await;
                match legality {
                    Ok(Some(target)) => Some(target),
                    Err(DurabilityError::RootUnavailable) => {
                        entry.state = state_before;
                        entry.next_ordinal = ordinal_before;
                        continue;
                    }
                    _ => {
                        entry.clear_target();
                        continue;
                    }
                }
            } else {
                None
            };
            let impact = if selected.weapon.ranged.is_some() {
                let Some(impact) = self
                    .pay_ranged_shot(
                        &mut pvp_permit,
                        &mut runtime,
                        &mut states,
                        &objects,
                        room,
                        content,
                        entry,
                        &selected,
                    )
                    .await
                else {
                    if pvp_permit.has_unresolved() {
                        break;
                    }
                    entry.state = state_before;
                    entry.next_ordinal = ordinal_before;
                    continue;
                };
                impact
            } else {
                ranged::ShotImpact::Ordinary
            };
            if selected.weapon.ranged.is_some()
                && content.training_formula().is_some_and(|formula| {
                    states
                        .get(&runtime, entry.actor, entry.session)
                        .is_some_and(|state| state.training_successor_due(formula, false))
                })
                && !trained.contains(&(entry.actor, entry.session))
            {
                trained.push((entry.actor, entry.session));
            }
            let mut offence =
                equipped_offence::attacker_offence(&states, &runtime, entry.actor, entry.session);
            // Conviction leech is part of the same primary-hit projection as equipment leech.
            // Recompute it for every due swing from the session's latest committed allocation;
            // the live spell-state Premium deadline prevents expired Premium from retaining it.
            if let Some(state) = states.get(&runtime, entry.actor, entry.session)
                && let Some(effects) = state.wheel_effects_at(now.get())
            {
                apply_wheel_leech(&mut offence, effects);
                let global_critical =
                    effects.gem_value_milli("critical_extra_damage", "Critical Extra Damage") / 10;
                let Ok(global_critical) = u16::try_from(global_critical) else {
                    continue;
                };
                offence.critical_damage = offence.critical_damage.saturating_add(global_critical);
                if effects.active_unique_perks.contains("ballistic_mastery")
                    && state.wheel_equipment().and_then(|hands| hands.ammunition())
                        == Some(crate::content::ReferenceAmmoType::Bolt)
                {
                    offence.critical_damage = offence.critical_damage.saturating_add(1_000);
                }
            }
            let primary = selected.target;
            let has_area = matches!(impact, ranged::ShotImpact::Area(_));
            let targets = match impact {
                ranged::ShotImpact::Refused => {
                    ranged::train_distance(
                        &mut states,
                        &runtime,
                        content,
                        entry,
                        None,
                        now,
                        &mut trained,
                    );
                    continue;
                }
                ranged::ShotImpact::Area(mut targets) => {
                    // A shot provides one distance-training event even when it hits several actors.
                    if let Some(index) = targets.iter().position(|target| *target == primary) {
                        targets.swap(0, index);
                    }
                    if targets.is_empty() {
                        ranged::train_distance(
                            &mut states,
                            &runtime,
                            content,
                            entry,
                            None,
                            now,
                            &mut trained,
                        );
                        continue;
                    }
                    targets
                }
                ranged::ShotImpact::Ordinary => {
                    let mut targets = if offence.cleave > 0 {
                        equipped_offence::cleave_targets(entry, &selected, &runtime, room, now)
                    } else {
                        Vec::new()
                    };
                    targets.push(primary);
                    targets
                }
            };
            let area_damage = if has_area {
                draw_swing_damage(
                    entry,
                    &runtime,
                    &selected,
                    constants,
                    SwingDamageContext {
                        root: &root,
                        offence: &offence,
                        pure_elemental_with_attack_points,
                        area: true,
                    },
                )
            } else {
                None
            };
            // D186: one uniform critical draw for the attack, compared with each
            // species' own augmented threshold. Damage/armor retain their target roots.
            let Some(critical_roll) = swing_draw(
                &root,
                entry.actor,
                selected.swing.lineage,
                selected.ordinal,
                b"critical-hit",
                "attack.critical-hit",
                CRITICAL_ROLL_MIN as i64,
                CRITICAL_ROLL_MAX as i64,
            )
            .and_then(|roll| u64::try_from(roll).ok()) else {
                continue;
            };
            for (index, target) in targets.into_iter().enumerate() {
                let extension = target != primary;
                let train = !has_area || index == 0;
                selected.target = target;
                // Independent tagged draws for each cleave target; the primary draw is unchanged.
                let root = if extension {
                    GameplayDecisionRoot::from_bytes(
                        sha2::Sha256::new()
                            .chain_update(digest)
                            .chain_update(selected.target.placement_identity())
                            .finalize()
                            .into(),
                    )
                } else {
                    GameplayDecisionRoot::from_bytes(digest)
                };
                let SelectedSwing {
                    target,
                    character_facts,
                    swing,
                    ordinal,
                    ..
                } = selected;
                let Ok(lease) = runtime
                    .borrow_exact_actor_commit()
                    .bound_attacker_lease(entry.actor, swing.lineage)
                else {
                    if !extension {
                        entry.clear_target();
                    }
                    continue;
                };
                let base_critical = (offence.critical_chance, offence.critical_damage);
                let mut offence = offence.clone();
                if let Ok(snapshot) = runtime.companion_snapshot(target) {
                    let source = if extension {
                        CharmHitSource::CharacterAutoAttackOffTarget
                    } else {
                        CharmHitSource::CharacterAttack
                    };
                    let outcomes = charm_combat::evaluate(
                        &states,
                        &runtime,
                        self.imported_charms,
                        entry.actor,
                        entry.session,
                        &snapshot.state.policy.definition_key,
                        &root,
                        swing_occurrence(
                            entry.actor,
                            swing.lineage,
                            ordinal,
                            b"damage-calculation",
                        ),
                        CharmHookEvent::AttackDamageCalculation { source },
                    );
                    charm_combat::bonuses(&mut offence, &outcomes);
                }
                let charm_critical =
                    base_critical != (offence.critical_chance, offence.critical_damage);
                let Some((mut damage, revision)) = (if has_area {
                    area_damage
                } else {
                    draw_swing_damage(
                        entry,
                        &runtime,
                        &selected,
                        constants,
                        SwingDamageContext {
                            root: &root,
                            offence: &offence,
                            pure_elemental_with_attack_points,
                            area: false,
                        },
                    )
                }) else {
                    continue;
                };
                // The drawn primary auto-attack receives one additive shrine multiplier
                // before any target defence or armor. Ranged attacks are not Monk melee.
                if selected.weapon.ranged.is_none()
                    && damage >= 0
                    && super::monk_shrines::is_monk(selected.character_facts.vocation)
                {
                    let serene = states
                        .get(&runtime, entry.actor, entry.session)
                        .and_then(|state| state.monk.as_ref())
                        .is_some_and(|monk| monk.serene());
                    let Some(boosted) =
                        oteryn_gameplay_combat_rules::monk_shrines::MonkShrineBonuses::checked_in()
                            .and_then(|tuning| {
                                tuning.attack(
                                    damage,
                                    super::monk_shrines::counts(
                                        &self.states.quest_sessions,
                                        entry.session,
                                    )
                                    .1,
                                    super::monk_shrines::is_monk(selected.character_facts.vocation),
                                    serene,
                                )
                            })
                    else {
                        continue;
                    };
                    damage = boosted;
                }
                if damage >= 0
                    && let Some(state) = states.get(&runtime, entry.actor, entry.session)
                {
                    damage = damage.saturating_mul(i64::from(
                        100 + state.virtue_party_bonuses(now.get()).auto_attack,
                    )) / 100;
                }
                if damage < 0 {
                    ranged::train_distance(
                        &mut states,
                        &runtime,
                        content,
                        entry,
                        None,
                        now,
                        &mut trained,
                    );
                    continue;
                }
                if extension && !has_area {
                    damage =
                        i64::try_from(i128::from(damage) * i128::from(offence.cleave) / 10_000)
                            .unwrap_or(i64::MAX);
                }
                let Some((physical, secondary, element)) =
                    (if let Some(ranged) = &selected.weapon.ranged {
                        ranged.split(damage)
                    } else {
                        selected.weapon.weapon.split_damage(damage)
                    })
                else {
                    continue;
                };
                let extend_component = |value| {
                    if extension {
                        // Cleave may use assigned critical charms, but cannot roll fatal.
                        if charm_critical && value > 0 {
                            u32::try_from(value)
                                .ok()
                                .map(|value| i64::from(offence.critical(value, critical_roll)))
                        } else {
                            Some(value)
                        }
                    } else {
                        equipped_offence::critical_swing(
                            &offence,
                            &root,
                            entry.actor,
                            swing.lineage,
                            ordinal,
                            value,
                            critical_roll,
                        )
                    }
                };
                let (Some(physical), Some(secondary)) =
                    (extend_component(physical), extend_component(secondary))
                else {
                    continue;
                };
                let Some((physical, secondary, element)) =
                    offence.convert_physical(physical, secondary, element)
                else {
                    continue;
                };
                let raw = if super::super::hazard::outgoing_damage(
                    &runtime,
                    entry.actor,
                    entry.session,
                    target,
                    physical.saturating_add(secondary),
                    (swing.lineage.command_id().get(), ordinal),
                ) == 0
                {
                    [0, 0]
                } else {
                    [physical, secondary]
                };
                // Apply outgoing conditions per component before target defence, after
                // PvP halving when the target is a player.
                let Ok(conditions) =
                    runtime.actor_conditions_at(entry.actor, Some(entry.session), now.get())
                else {
                    continue;
                };
                let Some(player) = states.get(&runtime, entry.actor, entry.session) else {
                    continue;
                };
                let Some(retained) = player.conditions.damage_dealt_percent_with_hex_at(
                    now.get(),
                    conditions.intense_hex_delta_at(now.get()),
                ) else {
                    continue;
                };
                if !runtime.contains_live_creature(target) {
                    let target_protection = if extension {
                        self.pvp_target_legal(
                            &pvp_permit,
                            &runtime,
                            &states,
                            room,
                            &objects,
                            entry.actor,
                            target,
                            swing.lineage,
                            oteryn_protocol_oteryn::pvp_expert::ExpertFightModes {
                                modes: entry.modes,
                                expert: entry.expert_mode,
                            },
                        )
                        .await
                        .ok()
                        .flatten()
                    } else {
                        pvp_target.clone()
                    };
                    let Some(target_protection) = target_protection else {
                        continue;
                    };
                    let Some(components) = pvp_swing_components(
                        (raw[0], raw[1], element),
                        target_protection.black,
                        retained,
                    ) else {
                        continue;
                    };
                    // `Combat::CombatHealthFunc` halves a PvP hit before the target's block meets it;
                    // PVP-DEFENCE-1 resolves the defence after this pass, where the target's block
                    // budget is reachable.
                    pending_pvp.push_back(PvpSwing {
                        actor: entry.actor,
                        session: entry.session,
                        modes: oteryn_protocol_oteryn::pvp_expert::ExpertFightModes {
                            modes: entry.modes,
                            expert: entry.expert_mode,
                        },
                        extension,
                        train,
                        target,
                        lineage: swing.lineage,
                        ordinal,
                        components,
                        target_protection,
                        revision,
                        weapon: selected.weapon.clone(),
                        attacker: lease.character_id(),
                        generation: lease.generation(),
                    });
                    continue;
                }
                let (Some(damage), Some(elemental)) = (
                    scale_outgoing(raw[0], retained),
                    scale_outgoing(raw[1], retained),
                ) else {
                    continue;
                };
                let Ok(snapshot) = runtime.companion_snapshot(target) else {
                    if !extension {
                        entry.clear_target();
                    }
                    continue;
                };
                let race_key = snapshot.state.policy.definition_key.clone();
                let Some(talisman) = talismans.get(&entry.session) else {
                    continue;
                };
                let Some([talisman_damage, talisman_life]) = bounty_talisman::hit_bonuses(
                    talisman,
                    &race_key,
                    &root,
                    swing_occurrence(entry.actor, swing.lineage, ordinal, b"bounty-talisman"),
                    0,
                ) else {
                    continue;
                };
                // The creature's content defence and armor (ATTACK-0 §5, `monster.cpp:242-251`);
                // a creature without them takes no hit (fail closed).
                let (Some(defense), Some(armor)) = (
                    runtime.effective_creature_defense(&snapshot).ok(),
                    snapshot.state.policy.armor,
                ) else {
                    continue;
                };
                // CREATURE-RESIST-MELEE-1: a creature immune to physical damage meets the swing
                // before its block (`Creature::blockHit` `isImmune`): no block, no training, no hit.
                let split_melee = selected.weapon.weapon.has_elemental_attack()
                    || offence.elemental_conversion.is_some();
                let physical_immune = creature_physically_immune(&snapshot.state.policy);
                let health_change = match observe_native_attack_health(
                    &runtime,
                    health_names_data.as_ref(),
                    target,
                ) {
                    Ok(observation) => observation,
                    Err(()) => continue,
                };
                let cruelty_drain = match runtime.soul_war_health_callback(target) {
                    Ok(Some(policy)) if policy.is_cruelty() => {
                        match runtime.soul_war_cruelty_drain(&snapshot) {
                            Ok(value) => Some(value.unwrap_or(2)),
                            Err(_) => continue,
                        }
                    }
                    Ok(_) => None,
                    Err(_) => continue,
                };
                let creature_max_health = u64::try_from(snapshot.maximum_health).unwrap_or(0);
                // The creature's healing map uses the components before outgoing conditions.
                let healing = |key, value| snapshot.state.policy.damage_healing(key, value).ok();
                let Some(healing) = healing("physical", raw[0])
                    .zip(healing(ranged::element_key(element), raw[1]))
                    .and_then(|(primary, secondary)| {
                        u64::try_from(primary.checked_add(secondary)?).ok()
                    })
                else {
                    continue;
                };
                let mut creature_health = u64::try_from(snapshot.health).unwrap_or(0);
                if healing > 0 {
                    let Ok(receipts) = runtime.commit_source_creature_heal_batch(
                        digest,
                        &[(target, race_key.clone(), creature_max_health, healing)],
                    ) else {
                        continue;
                    };
                    let [receipt] = receipts.as_slice() else {
                        continue;
                    };
                    creature_health = u64::try_from(receipt.health_after).unwrap_or(0);
                }
                let proficiency_damage = |value| {
                    proficiency.map_or(value, |effects| {
                        effects.apply_percent(value, creature_health, creature_max_health)
                    })
                };
                if selected.weapon.ranged.is_none() && !split_melee && physical_immune {
                    continue;
                }
                if let Some(ranged) = &selected.weapon.ranged
                    && ranged::immune(ranged, &snapshot.state.policy, &offence)
                {
                    if train && matches!(ranged.damage, ranged::RangedDamage::Distance { .. }) {
                        ranged::train_distance(
                            &mut states,
                            &runtime,
                            content,
                            entry,
                            Some(HitBlock::Immunity),
                            now,
                            &mut trained,
                        );
                    }
                    continue;
                }
                let mut resolved_element = crate::content::ProjectV2RangedElement::Physical;
                let mut training_block = (None, HitBlock::Unblocked);
                let mut ranged_magnitude = if let Some(ranged) = &selected.weapon.ranged {
                    ranged::creature_damage(
                        ranged,
                        &snapshot.state.policy,
                        (damage, elemental, element),
                        (&root, constants, entry.actor, swing.lineage, ordinal),
                        states
                            .get(&runtime, entry.actor, entry.session)
                            .map(|state| (state, now.get())),
                    )
                    .map(|shot| {
                        resolved_element = element;
                        training_block = shot.blocks;
                        shot.parts
                    })
                } else {
                    None
                };
                let index = match blocks.iter().position(|(creature, _)| *creature == target) {
                    Some(index) => index,
                    None => {
                        if blocks.len() >= runtime.actor_slot_capacity() {
                            continue;
                        }
                        // A creature's budget refills from spawn, not from the first hit on it.
                        blocks.push((target, BlockBudget::settled(&constants.block, swing.at)));
                        blocks.len() - 1
                    }
                };
                let melee_components = (selected.weapon.ranged.is_none() && split_melee)
                    .then_some((damage, elemental, element));
                let secondary_immune = melee_components.is_some_and(|(_, _, element)| {
                    snapshot
                        .state
                        .policy
                        .damage_immunities
                        .iter()
                        .any(|key| key == ranged::element_key(element))
                });
                if physical_immune && secondary_immune {
                    if train {
                        train_swing(
                            &mut states,
                            &runtime,
                            content,
                            entry,
                            &selected.weapon,
                            (None, HitBlock::Immunity),
                            now,
                            &mut trained,
                        );
                    }
                    continue;
                }
                // The physical primary meets defence even when its split was zero.
                let blocked = selected.weapon.ranged.is_none()
                    && !physical_immune
                    && blocks[index].1.try_block(&constants.block, swing.at);
                let melee_damage = |physical| {
                    creature_melee_damage(
                        &snapshot.state.policy,
                        physical,
                        blocked,
                        armor,
                        |damage, blocked, armor| {
                            creature_mitigated(
                                &root,
                                constants,
                                (entry.actor, swing.lineage, ordinal),
                                damage,
                                blocked.then_some((defense / 2, defense)),
                                armor,
                            )
                        },
                    )
                };
                if let Some((physical, elemental, element)) = melee_components {
                    ranged_magnitude = (|| {
                        let (physical, primary_block) = if physical_immune {
                            (0, HitBlock::Immunity)
                        } else {
                            let (block, landed) = melee_damage(physical)?;
                            (landed, block)
                        };
                        let elemental =
                            oteryn_gameplay_combat_rules::resolution::creature_damage_response(
                                &snapshot.state.policy,
                                elemental,
                                ranged::element_key(element),
                            )?;
                        resolved_element = element;
                        training_block = (
                            Some(primary_block),
                            if secondary_immune {
                                HitBlock::Immunity
                            } else {
                                HitBlock::Unblocked
                            },
                        );
                        Some([physical, elemental])
                    })();
                }
                let damage_parts = if selected.weapon.ranged.is_some() || split_melee {
                    let Some(parts) = ranged_magnitude else {
                        continue;
                    };
                    if parts[0].checked_add(parts[1]).is_none() {
                        continue;
                    }
                    parts
                } else {
                    let Some((block, landed)) = melee_damage(damage) else {
                        continue;
                    };
                    training_block = (None, block);
                    [landed, 0]
                };
                let prey = states.prey_percent(
                    &runtime,
                    entry.actor,
                    entry.session,
                    &race_key,
                    1,
                    now.get(),
                );
                let perfect =
                    if !extension
                        && selected.weapon.ranged.as_ref().is_some_and(|r| {
                            matches!(r.damage, ranged::RangedDamage::Distance { .. })
                        })
                    {
                        runtime
                            .read_actor_position(entry.actor)
                            .ok()
                            .zip(runtime.read_actor_position(target).ok())
                            .map_or(0, |(from, to)| {
                                let (from, to) = (from.position(), to.position());
                                offence.perfect_damage(from.x.abs_diff(to.x), from.y.abs_diff(to.y))
                            })
                    } else {
                        0
                    };
                let magic = selected
                    .weapon
                    .ranged
                    .as_ref()
                    .is_some_and(|r| matches!(r.damage, ranged::RangedDamage::Magic { .. }));
                let finish = |prey| {
                    let mut parts = boost_attack_components(damage_parts, prey, talisman_damage)
                        .map(proficiency_damage);
                    let ranged = selected.weapon.ranged.as_ref();
                    if let Some(ranged) = ranged {
                        parts = ranged::native_health_primary_components(
                            ranged.damage,
                            parts,
                            health_change.as_ref(),
                        )?;
                    }
                    if magic {
                        parts.swap(0, 1);
                    }
                    let mut parts = finish_melee_components(
                        parts,
                        |primary| match (ranged, health_change.as_ref()) {
                            (None, Some(event)) => primary
                                .checked_neg()
                                .and_then(|value| event.primary(value).ok())
                                .and_then(i64::checked_neg),
                            _ => Some(primary),
                        },
                        cruelty_drain,
                        |primary, secondary| {
                            if let Some(state) = states.get(&runtime, entry.actor, entry.session) {
                                state.finish_wheel_attack_components(
                                    primary,
                                    secondary,
                                    creature_health,
                                    creature_max_health,
                                    now.get(),
                                    crate::spell::cast::wheel_adjacent_target(
                                        &runtime,
                                        entry.actor,
                                        target,
                                    )?,
                                )
                            } else {
                                Some((primary, secondary))
                            }
                        },
                    )?;
                    if magic {
                        parts.swap(0, 1);
                    }
                    parts[0] = parts[0].saturating_add(perfect).max(0);
                    Some(elite_attack_components(parts, talisman.elite(&race_key)))
                };
                let Some(final_parts) = finish(prey) else {
                    continue;
                };
                let leech_parts = if prey == 0 {
                    final_parts
                } else {
                    finish(0).unwrap_or([0, 0])
                };
                let leech_damage =
                    u32::try_from(leech_parts[0].saturating_add(leech_parts[1]).max(0))
                        .unwrap_or(u32::MAX);
                let landed = final_parts[0].saturating_add(final_parts[1]);
                if landed <= 0 {
                    if train {
                        train_swing(
                            &mut states,
                            &runtime,
                            content,
                            entry,
                            &selected.weapon,
                            training_block,
                            now,
                            &mut trained,
                        );
                    }
                    continue;
                }
                let Some(plan) = swing_plan(
                    entry.actor,
                    swing.lineage,
                    ordinal,
                    landed,
                    &digest,
                    &race_key,
                    revision,
                ) else {
                    continue;
                };
                let Ok(resolved) = resolve_exact_actor(
                    &runtime.borrow_exact_actor_lookup(),
                    plan.occurrence(),
                    ExactActorProposal::client(target),
                ) else {
                    if !extension {
                        entry.clear_target();
                    }
                    continue;
                };
                // The owner clock of this turn, in ms, stamps the hit's contributor and, for a lethal
                // hit, the death time the kill reward reads (§1.2).
                let committed = {
                    let Ok(mut cache) = kills.1.lock() else {
                        continue;
                    };
                    let item = selected.weapon.ranged.as_ref().map_or_else(
                        || selected.weapon.item_key.as_deref().unwrap_or(""),
                        |loadout| loadout.weapon.definition.production_key.as_str(),
                    );
                    let creature = if extension { "" } else { race_key.as_str() };
                    let Ok(mut hits) = cache.hits.prepare(
                        &runtime,
                        entry.actor,
                        target,
                        swing.lineage,
                        ordinal,
                        creature,
                        item,
                        None,
                    ) else {
                        continue;
                    };
                    let committed = crate::ability::commit::commit_exact_owner_swing_damage(
                        &mut runtime.borrow_exact_actor_commit_at_us(now.get()),
                        &resolved,
                        &plan,
                        entry.actor,
                        swing.lineage,
                        ordinal,
                    );

                    if committed
                        .as_ref()
                        .is_ok_and(|result| result.committed_health_loss().is_some())
                        && let Ok(Some(witness)) = runtime
                            .borrow_exact_actor_commit_at(now.get() / 1_000)
                            .committed_hit_for_bound_attacker(
                                target,
                                entry.actor,
                                swing.lineage,
                                ordinal,
                            )
                    {
                        hits.capture(witness, creature, item, None);
                    }
                    committed
                };
                let Ok(result) = committed else {
                    if !extension {
                        entry.clear_target();
                    }
                    continue;
                };
                if result.applied
                    && result.health_after > 0
                    && selected
                        .weapon
                        .ranged
                        .as_ref()
                        .and_then(|r| r.scripted_arrow())
                        == Some(ranged::ScriptedArrow::Poison)
                {
                    ranged::apply_creature_poison(
                        &mut runtime,
                        target,
                        entry.actor,
                        &root,
                        swing_occurrence(entry.actor, swing.lineage, ordinal, b"poison-arrow"),
                        now,
                    );
                }
                if !extension && result.applied {
                    publish_hit_sound(
                        &mut states,
                        &runtime,
                        content,
                        entry.actor,
                        &selected.weapon,
                    );
                }
                if result.applied {
                    let value = u32::try_from(
                        result
                            .health_before
                            .saturating_sub(result.health_after)
                            .max(0),
                    )
                    .unwrap_or(0);
                    let primary =
                        super::super::item_spell_augments::leech_projection::owner_damage_amount(
                            &runtime,
                            target,
                            final_parts[0],
                            result.health_before,
                            0,
                        )
                        .unwrap_or(final_parts[0]);
                    let primary = reported_primary(primary, value);
                    let element = ranged::analyser_element(resolved_element);
                    states.analyser.impact(
                        entry.actor,
                        entry.session,
                        primary,
                        Some(oteryn_protocol_oteryn::damage_element::DamageElement::Physical),
                    );
                    states.analyser.impact(
                        entry.actor,
                        entry.session,
                        value.saturating_sub(primary),
                        Some(element),
                    );
                }
                if train {
                    train_swing(
                        &mut states,
                        &runtime,
                        content,
                        entry,
                        &selected.weapon,
                        training_block,
                        now,
                        &mut trained,
                    );
                }
                let outcomes = {
                    let source = if extension {
                        CharmHitSource::CharacterAutoAttackOffTarget
                    } else {
                        CharmHitSource::CharacterAttack
                    };
                    charm_combat::evaluate(
                        &states,
                        &runtime,
                        self.imported_charms,
                        entry.actor,
                        entry.session,
                        &race_key,
                        &root,
                        swing_occurrence(entry.actor, swing.lineage, ordinal, b"committed-hit"),
                        CharmHookEvent::committed_hit(
                            &result,
                            source,
                            CharmAttackerFacts {
                                level: character_facts.level,
                                max_health: u64::from(character_facts.max_health),
                                max_mana: u64::from(character_facts.max_mana),
                            },
                            creature_max_health,
                        ),
                    )
                };
                charm_combat::committed_hit_bonuses(&mut offence, &outcomes, extension);
                if !extension {
                    // Canary on_combat_taskboard.lua:24-27 adds the proc to the ordinary
                    // life skill. The owner uses the retained pre-Prey, pre-overkill basis.
                    offence.life_leech = offence.life_leech.saturating_add(talisman_life);
                }
                // Keep the owner's Enrage callback even though Leech excludes Prey
                // and HP clipping. A failed projection suppresses only regeneration,
                // never the already committed hit or its remaining kill/charm work.
                if let Ok(leech_damage) = crate::gameplay_transport::item_spell_augments::leech_projection::owner_damage_basis(
                    &runtime, target, leech_damage, result.health_before, 0,
                ) {
                    equipped_offence::leech_hit(
                        &mut states, &runtime, entry.actor, entry.session, &offence,
                        leech_damage, &result,
                    );
                }
                if result.committed_health_loss().is_some()
                    && snapshot.state.master.is_none()
                    && !snapshot.state.policy.is_familiar
                {
                    states.prey_activity(&runtime, entry.actor, entry.session, super::super::prey::PreyActivity {
                        creature: &race_key,
                        kind: oteryn_gameplay_progression::prey::difficulty::PreyActivityKind::DealtDamage,
                        now: now.get(),
                    });
                }
                record_hit_activity(
                    entry,
                    &runtime,
                    content,
                    target,
                    now,
                    &result,
                    &mut activity_cells,
                );
                if result.committed_health_loss().is_some() {
                    kills.boss_damage_dealt(&runtime, target, entry.actor, entry.session);
                }
                // Source-qualified leaf damage uses the existing independent replay stream.
                // It neither consumes another player command nor re-enters Charm evaluation.
                let mut lethal_result = result;
                if result.committed_health_loss().is_some() {
                    for outcome in &outcomes {
                        if let Some(CharmEffectOutcome::PreventCreatureFlee { duration_ms }) =
                            charm_combat::effect(outcome)
                            && runtime.companion_snapshot(target).is_ok_and(|s| {
                                s.health > 0
                                    && s.state.master.is_none()
                                    && !s.state.policy.is_familiar
                            })
                            && let Some(until) =
                                now.get().checked_add(u64::from(duration_ms) * 1_000)
                        {
                            let creature = crate::content::ProjectV2DefinitionRef {
                                family: crate::content::ProjectV2Family::Creature,
                                key: snapshot.state.policy.definition_key.clone(),
                                revision: snapshot.state.policy.definition_revision.clone(),
                            };
                            if states.monster_melee.creature_ai.state(target).is_none() {
                                let _ = states.monster_melee.creature_ai.admit_native_creature(
                                    &runtime,
                                    content,
                                    target,
                                    &creature,
                                    now.get(),
                                );
                            }
                            states
                                .monster_melee
                                .creature_ai
                                .apply_fatal_hold(target, until);
                        }
                        if let Some(CharmEffectOutcome::ParalyseCreature { duration_ms }) =
                            charm_combat::effect(outcome)
                        {
                            charm_combat::paralyse_creature(
                                &mut runtime,
                                entry.actor,
                                entry.session,
                                target,
                                duration_ms,
                                &root,
                                swing_occurrence(
                                    entry.actor,
                                    swing.lineage,
                                    ordinal,
                                    b"committed-hit",
                                ),
                                now.get(),
                            );
                        }
                    }
                }
                if result.applied
                    && result.health_after > 0
                    && result.health_before > result.health_after
                {
                    for outcome in &outcomes {
                        let Some(CharmEffectOutcome::ProcDamage { damage, amount }) =
                            charm_combat::effect(outcome)
                        else {
                            continue;
                        };
                        let Some(draw) = swing_draw(
                            &root,
                            entry.actor,
                            swing.lineage,
                            ordinal,
                            b"charm-armor",
                            "attack.charm-armor",
                            0,
                            i64::MAX,
                        ) else {
                            continue;
                        };
                        let Some(amount) = charm_combat::damage_amount(
                            &runtime,
                            target,
                            damage,
                            amount,
                            draw as u64,
                        )
                        .filter(|n| *n > 0) else {
                            continue;
                        };
                        let amount = if talisman.elite(&race_key) {
                            amount / 2
                        } else {
                            amount
                        };
                        if amount == 0 {
                            continue;
                        }
                        let stream = format!(
                            "charm-attack:{}:{}",
                            swing.lineage.command_id().get(),
                            outcome.charm_key
                        );
                        if let Ok(child) = runtime.commit_source_damage_at_us(
                            target,
                            entry.actor,
                            entry.session,
                            stream.as_bytes(),
                            u64::from(ordinal),
                            amount,
                            now.get(),
                        ) {
                            if child.applied {
                                states.analyser.impact_amount(
                                    entry.actor,
                                    entry.session,
                                    u64::try_from(
                                        child.health_before.saturating_sub(child.health_after),
                                    )
                                    .unwrap_or(0),
                                    Some(super::super::analyser::charm_element(damage.element)),
                                );
                            }
                            record_hit_activity(
                                entry,
                                &runtime,
                                content,
                                target,
                                now,
                                &child,
                                &mut activity_cells,
                            );
                            lethal_result = child;
                        }
                    }
                }
                if lethal_result.health_after <= 0 {
                    let lethal = crate::combat::CapturedRewardPrincipal {
                        character: lease.character_id(),
                        lease_generation: lease.generation(),
                        session: entry.session,
                        actor: entry.actor,
                    };
                    // Canary 47dfd51f iobestiary.cpp:51-97: Carnage affects only the four
                    // cardinal neighbouring monsters. Source damage never re-enters charms.
                    for outcome in &outcomes {
                        let Some(CharmEffectOutcome::AreaDamageAroundKill { damage, amount }) =
                            charm_combat::effect(outcome)
                        else {
                            continue;
                        };
                        let origin = snapshot.position;
                        let neighbours: Vec<_> = runtime
                            .visible_entities()
                            .creatures
                            .into_iter()
                            .filter_map(|creature| {
                                let candidate = runtime.companion_snapshot(creature.actor).ok()?;
                                let at = candidate.position;
                                (creature.actor != target
                                    && candidate.health > 0
                                    && candidate.state.master.is_none()
                                    && !candidate.state.policy.is_familiar
                                    && candidate.state.policy.flags.attackable
                                    && at.floor == origin.floor
                                    && ((at.x == origin.x && at.y.abs_diff(origin.y) == 1)
                                        || (at.y == origin.y && at.x.abs_diff(origin.x) == 1)))
                                    .then_some(creature.actor)
                            })
                            .collect();
                        for neighbour in neighbours {
                            let occurrence =
                                swing_occurrence(neighbour, swing.lineage, ordinal, b"carnage");
                            let Ok(draw) =
                                oteryn_simulation_determinism::deterministic_decision_u64(
                                    &root,
                                    occurrence,
                                    "charm.carnage-armor",
                                    0,
                                )
                            else {
                                continue;
                            };
                            let Some(amount) = charm_combat::damage_amount(
                                &runtime, neighbour, damage, amount, draw,
                            )
                            .filter(|amount| *amount > 0) else {
                                continue;
                            };
                            let amount =
                                if runtime
                                    .companion_snapshot(neighbour)
                                    .is_ok_and(|candidate| {
                                        talisman.elite(&candidate.state.policy.definition_key)
                                    })
                                {
                                    amount / 2
                                } else {
                                    amount
                                };
                            if amount == 0 {
                                continue;
                            }
                            let stream = format!(
                                "charm-carnage:{}:{}:{}",
                                swing.lineage.command_id().get(),
                                outcome.charm_key,
                                hex(&target.placement_identity())
                            );
                            let Ok(child) = runtime.commit_source_damage(
                                neighbour,
                                entry.actor,
                                entry.session,
                                stream.as_bytes(),
                                u64::from(ordinal),
                                amount,
                                now.get() / 1_000,
                            ) else {
                                continue;
                            };
                            if child.applied {
                                kills.boss_damage_dealt(
                                    &runtime,
                                    neighbour,
                                    entry.actor,
                                    entry.session,
                                );
                                states.analyser.impact_amount(
                                    entry.actor,
                                    entry.session,
                                    u64::try_from(
                                        child.health_before.saturating_sub(child.health_after),
                                    )
                                    .unwrap_or(0),
                                    Some(super::super::analyser::charm_element(damage.element)),
                                );
                                record_hit_activity(
                                    entry,
                                    &runtime,
                                    content,
                                    neighbour,
                                    now,
                                    &child,
                                    &mut activity_cells,
                                );
                                if child.health_after == 0 {
                                    settle_kill(
                                        entry,
                                        &mut runtime,
                                        kills,
                                        self.reward_table(),
                                        self.imported_charms,
                                        &activity_cells,
                                        &states,
                                        neighbour,
                                        lethal,
                                        now,
                                        false,
                                    );
                                }
                            }
                        }
                    }
                    settle_kill(
                        entry,
                        &mut runtime,
                        kills,
                        self.reward_table(),
                        self.imported_charms,
                        &activity_cells,
                        &states,
                        target,
                        lethal,
                        now,
                        !extension,
                    );
                }
            }
            // Complete this swing (including cleave) before its health callback;
            // leave later attackers' untouched clocks for the next coalesced turn.
            if runtime
                .pending_cloak_blood_events(1)
                .is_ok_and(|events| !events.is_empty())
            {
                states.owner_wake.notify_one();
                break;
            }
        }
        self.drain_pvp_impacts(
            &mut pvp_permit,
            &mut runtime,
            &mut states,
            &mut attack,
            room,
            &objects,
            content,
            (&root, &digest, constants, now),
            &mut trained,
        )
        .await;
        drop(attack);
        drop(objects);
        drop(states);
        drop(runtime);
        // No later Item command can overtake the normal hit's source callback.
        let _ = self.drain_cloak_blood_with_permit(&pvp_permit).await;
        drop(pvp_permit);
        self.save_trained_levels(trained).await;
        let _ = self.drain_native_quest_hits().await;
    }
}

fn apply_wheel_leech(
    offence: &mut oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence,
    wheel: &oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects,
) {
    offence.life_leech = offence
        .life_leech
        .saturating_add(u16::try_from(wheel.life_leech_hundredths_percent).unwrap_or(u16::MAX));
    offence.mana_leech = offence
        .mana_leech
        .saturating_add(u16::try_from(wheel.mana_leech_hundredths_percent).unwrap_or(u16::MAX));
}

#[cfg(test)]
mod wheel_leech_tests {
    use super::*;

    #[test]
    fn conviction_leech_joins_equipment_without_wrapping() {
        let mut offence =
            oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence {
                life_leech: 1_000,
                mana_leech: u16::MAX - 5,
                ..Default::default()
            };
        let wheel = oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects {
            life_leech_hundredths_percent: 250,
            mana_leech_hundredths_percent: 10,
            ..Default::default()
        };

        apply_wheel_leech(&mut offence, &wheel);

        assert_eq!(offence.life_leech, 1_250);
        assert_eq!(offence.mana_leech, u16::MAX);
    }
}

fn battle_instinct_for_actor(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
) -> Option<oteryn_gameplay_progression::wheel_effects::BattleInstinctBonuses> {
    let adjacent = crate::spell::cast::wheel_adjacent_monsters(runtime, actor)?;
    let data = oteryn_gameplay_progression::wheel_gem_data::WheelGemData::embedded().ok()?;
    oteryn_gameplay_progression::wheel_effects::battle_instinct_bonuses(data.wheel(), adjacent)
}

/// SKILL-TRAIN-1: the swing at a creature trains the swing's skill by how its block met it.
impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(clippy::too_many_arguments)]
    async fn drain_pvp_impacts(
        &self,
        permit: &mut SpellLanePermit,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        attack: &mut ChannelAttackStates,
        room: &QualifiedNativeEntryRoom,
        objects: &LocalObjectRuntime,
        content: &NativeGameplayState,
        facts: (
            &GameplayDecisionRoot,
            &[u8; 32],
            &AttackConstants,
            SemanticTimeMicros,
        ),
        trained: &mut TrainingDue,
    ) {
        while !permit.has_unresolved() {
            let Some(hit) = attack.pending_pvp.pop_front() else {
                break;
            };
            self.land_pvp_swing(
                permit, runtime, states, attack, room, objects, content, facts, hit, trained,
            )
            .await;
        }
        if !attack.pending_pvp.is_empty() {
            states.owner_wake.notify_one();
        }
    }

    /// PVP-DEFENCE-1 (`Creature::blockHit`, `Player::getDefense`): a PvP swing spends one of the
    /// target's blocks and meets the defence of what the target's hands hold, then the target's
    /// worn armor. A block trains the target's shielding (SKILL-TRAIN-1), and the attacker trains
    /// by what is left: a swing they stop commits nothing and trains as blocked.
    #[allow(clippy::too_many_arguments)]
    async fn land_pvp_swing(
        &self,
        permit: &mut SpellLanePermit,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        attack: &mut ChannelAttackStates,
        room: &QualifiedNativeEntryRoom,
        objects: &LocalObjectRuntime,
        content: &NativeGameplayState,
        (root, digest, constants, now): (
            &GameplayDecisionRoot,
            &[u8; 32],
            &AttackConstants,
            SemanticTimeMicros,
        ),
        hit: PvpSwing,
        trained: &mut TrainingDue,
    ) {
        if hit.weapon.content_digest != *digest {
            return;
        }
        let Some(target_session) = states
            .actors
            .iter()
            .find(|(actor, _, _)| *actor == hit.target)
            .map(|(_, session, _)| *session)
        else {
            if !hit.extension {
                attack.clear_target_of(hit.actor, hit.session);
            }
            return;
        };
        // An earlier queued impact can spend the victim's last charge before this swing lands.
        let target_protection = if hit.target_protection.equipment.is_some() {
            let Ok(Some(current)) = self
                .pvp_target_legal(
                    permit,
                    runtime,
                    states,
                    room,
                    objects,
                    hit.actor,
                    hit.target,
                    hit.lineage,
                    hit.modes,
                )
                .await
            else {
                return;
            };
            if current.black != hit.target_protection.black {
                return;
            }
            current
        } else {
            hit.target_protection.clone()
        };
        let ranged_damage = if let Some(ranged) = &hit.weapon.ranged {
            self.ranged_player_damage(
                permit,
                runtime,
                content,
                target_session,
                ranged,
                &hit,
                constants,
                root,
                states.target_armor(runtime, hit.target, target_session),
            )
            .await
        } else {
            None
        };
        let mut vitals = PlayerBiteVitals {
            quest_sessions: Some(&self.states.quest_sessions),
            charm: None,
            conditions: None,
            states: &mut *states,
            attack: &mut *attack,
            formula: content.training_formula(),
            trained: &mut *trained,
        };
        let (physical_raw, secondary_raw, converted_element) = hit.components;
        // The physical primary spends an available block even on a pure elemental swing.
        let defence = if hit.weapon.ranged.is_some() {
            None
        } else {
            vitals.target_defence(
                runtime,
                hit.target,
                target_session,
                OwnerTime::from_micros(now.get()),
            )
        };
        let armor = vitals.target_armor(runtime, hit.target, target_session);
        let protection = vitals.target_physical_protection(runtime, hit.target, target_session);
        let Some(mitigated) = (if hit.weapon.ranged.is_some() {
            ranged_damage
                .as_ref()
                .and_then(|read| {
                    read.components
                        .primary
                        .0
                        .checked_add(read.components.secondary.0)
                })
                .map(i64::from)
        } else {
            let reduce = |damage, defence, armor| {
                swing_mitigated(
                    root,
                    constants,
                    hit.actor.placement_identity(),
                    (
                        *hit.lineage.game_session_id().as_bytes(),
                        hit.lineage.command_id().get(),
                    ),
                    hit.ordinal,
                    damage,
                    defence,
                    armor,
                )
            };
            reduce(physical_raw, defence, 0).and_then(|damage| {
                player_melee_after_defence(damage, protection, |protected| {
                    reduce(protected, None, armor)
                })
            })
        }) else {
            return;
        };
        let perfect_damage = if !hit.extension
            && hit
                .weapon
                .ranged
                .as_ref()
                .is_some_and(|r| matches!(r.damage, ranged::RangedDamage::Distance { .. }))
        {
            let offence =
                equipped_offence::attacker_offence(states, runtime, hit.actor, hit.session);
            let Some((from, to)) = runtime
                .read_actor_position(hit.actor)
                .ok()
                .zip(runtime.read_actor_position(hit.target).ok())
            else {
                return;
            };
            let (from, to) = (from.position(), to.position());
            offence.perfect_damage(from.x.abs_diff(to.x), from.y.abs_diff(to.y))
        } else {
            0
        };
        let parts = if hit.weapon.ranged.is_some() {
            let Some(read) = &ranged_damage else {
                return;
            };
            read.components
        } else {
            let Ok(primary) = u32::try_from(mitigated) else {
                return;
            };
            crate::spell::cast::WheelAttackComponents {
                primary: (primary, crate::player_lethal::PlayerHitKind::Physical),
                secondary: (
                    u32::try_from(secondary_raw).unwrap_or(0),
                    match converted_element {
                        crate::content::ProjectV2RangedElement::Fire => {
                            crate::player_lethal::PlayerHitKind::Fire
                        }
                        crate::content::ProjectV2RangedElement::Earth => {
                            crate::player_lethal::PlayerHitKind::Earth
                        }
                        crate::content::ProjectV2RangedElement::Ice => {
                            crate::player_lethal::PlayerHitKind::Ice
                        }
                        crate::content::ProjectV2RangedElement::Energy => {
                            crate::player_lethal::PlayerHitKind::Energy
                        }
                        crate::content::ProjectV2RangedElement::Death => {
                            crate::player_lethal::PlayerHitKind::Death
                        }
                        crate::content::ProjectV2RangedElement::Physical => {
                            crate::player_lethal::PlayerHitKind::Physical
                        }
                    },
                ),
            }
        };
        let Some(attacker) = states.get(runtime, hit.actor, hit.session) else {
            return;
        };
        let Some(target) = states.get(runtime, hit.target, target_session) else {
            return;
        };
        let mut spends = std::collections::BTreeSet::new();
        let charges = if let Some(equipment) = &target_protection.equipment {
            let Some(charges) = super::super::equipment_move::equipment_effects_owner::combat_charges::CombatCharges::qualify(
                equipment, content,
            ) else { return; };
            Some(charges)
        } else {
            None
        };
        let mut after_equipment = |magnitude: u32, kind: crate::player_lethal::PlayerHitKind| {
            use crate::content::ReferenceResistanceKind as Kind;
            use crate::player_lethal::PlayerHitKind;
            let magnitude = if let Some(charges) = &charges {
                charges.mitigate(&mut spends, magnitude, kind)?
            } else {
                magnitude
            };
            let kind = match kind {
                PlayerHitKind::Physical => Kind::Physical,
                PlayerHitKind::Death => Kind::Death,
                PlayerHitKind::Earth => Kind::Earth,
                PlayerHitKind::Energy => Kind::Energy,
                PlayerHitKind::Fire => Kind::Fire,
                PlayerHitKind::Ice => Kind::Ice,
                PlayerHitKind::Holy => Kind::Holy,
                _ => return None,
            };
            let sum = ranged_damage
                .as_ref()
                .and_then(|read| read.protections.get(&kind).copied())
                .or_else(|| {
                    if hit.weapon.ranged.is_none() && kind != Kind::Physical {
                        target.equipped_element_protection(kind)
                    } else {
                        None
                    }
                });
            let magnitude = match sum {
                Some(sum) => apply_protection(i64::from(magnitude), sum)?,
                None => i64::from(magnitude),
            };
            let magnitude = if hit.weapon.ranged.is_none() {
                super::monk_shrines::taken(
                    &self.states.quest_sessions,
                    target_session,
                    target.character_facts().vocation,
                    magnitude,
                )?
            } else {
                magnitude
            };
            u32::try_from(magnitude).ok()
        };
        let Some(damage) = crate::spell::cast::PreparedWheelAttackHit::prepare(
            runtime,
            crate::spell::cast::WheelAttackActor {
                actor: hit.actor,
                session: hit.session,
                state: attacker,
            },
            crate::spell::cast::WheelAttackActor {
                actor: hit.target,
                session: target_session,
                state: target,
            },
            parts,
            perfect_damage,
            now.get(),
            &mut after_equipment,
        ) else {
            return;
        };
        let magnitude = i64::from(damage.magnitude());
        if magnitude == 0 && spends.is_empty() {
            if !hit.extension {
                publish_hit_sound(states, runtime, content, hit.actor, &hit.weapon);
            }
            train_pvp_swing(states, runtime, content, &hit, magnitude, now, trained);
            return;
        }
        let atom = actor_atom(hit.target);
        let Some(occurrence) =
            swing_ability_occurrence(hit.actor, hit.lineage, hit.ordinal, digest, hit.revision)
        else {
            return;
        };
        let poison = hit
            .weapon
            .ranged
            .as_ref()
            .filter(|_| magnitude > 0)
            .and_then(|r| r.scripted_arrow())
            .filter(|arrow| *arrow == ranged::ScriptedArrow::Poison)
            .and_then(|_| ranged::poison_definition())
            .map(|definition| {
                (
                    definition,
                    swing_occurrence(hit.actor, hit.lineage, hit.ordinal, b"poison-arrow"),
                )
            });
        let batch = crate::spell::combat_batch::OwnerCombatBatch {
            caster: hit.actor,
            attacker: hit.attacker,
            current_lease_generation: hit.generation,
            command: hit.lineage,
            occurrence: occurrence.into(),
            binding: b"oteryn:pvp-melee:v1".to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1000,
            effects: vec![crate::spell::combat_batch::OwnerCombatEffect {
                target: hit.target,
                sub_ordinal: 0,
                change: crate::spell::combat_batch::OwnerCombatChange::Damage {
                    target_atom: atom,
                    magnitude,
                },
            }],
            deferred: None,
        };
        match self
            .pvp_swing(
                permit,
                runtime,
                states,
                room,
                objects,
                batch,
                hit.ordinal,
                hit.modes,
                super::super::pvp_live::PvpSwingProtection {
                    target: target_protection,
                    spends,
                },
                damage,
                poison,
            )
            .await
        {
            Ok(true) => {
                if !hit.extension {
                    publish_hit_sound(states, runtime, content, hit.actor, &hit.weapon);
                }
                train_pvp_swing(states, runtime, content, &hit, magnitude, now, trained);
            }
            Err(DurabilityError::RootUnavailable) => {}
            _ if !hit.extension => {
                attack.clear_target_of(hit.actor, hit.session);
            }
            _ => {}
        }
    }
}

/// The attacker of `hit` trains its weapon skill by what the target's defence and armor left.
fn train_pvp_swing(
    states: &mut ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    content: &NativeGameplayState,
    hit: &PvpSwing,
    magnitude: i64,
    now: SemanticTimeMicros,
    trained: &mut TrainingDue,
) {
    let block = swing_block(magnitude);
    if !hit.train {
        return;
    }
    if let Some(ranged) = &hit.weapon.ranged {
        if matches!(ranged.damage, ranged::RangedDamage::Distance { .. }) {
            train_hit(
                states,
                runtime,
                content.training_formula(),
                hit.actor,
                hit.session,
                oteryn_gameplay_progression::hit_training::SkillFamily::Distance,
                now.get(),
                |hits| hits.distance_shot(Some(block)),
                trained,
            );
        }
        return;
    }
    train_hit(
        states,
        runtime,
        content.training_formula(),
        hit.actor,
        hit.session,
        swing_family(hit.weapon.weapon),
        now.get(),
        |hits| hits.melee_hit(block),
        trained,
    );
}

#[allow(clippy::too_many_arguments)]
fn train_swing(
    states: &mut ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    content: &NativeGameplayState,
    entry: &ActorAttack,
    weapon: &WeaponFacts,
    (previous, block): (Option<HitBlock>, HitBlock),
    now: SemanticTimeMicros,
    trained: &mut TrainingDue,
) {
    let distance = match weapon.ranged.as_ref().map(|ranged| ranged.damage) {
        Some(ranged::RangedDamage::Magic { .. }) => return,
        Some(ranged::RangedDamage::Distance { .. }) => true,
        None => false,
    };
    train_hit(
        states,
        runtime,
        content.training_formula(),
        entry.actor,
        entry.session,
        if distance {
            oteryn_gameplay_progression::hit_training::SkillFamily::Distance
        } else {
            swing_family(weapon.weapon)
        },
        now.get(),
        |hits| {
            if distance {
                if let Some(previous) = previous {
                    hits.distance_shot(Some(previous));
                }
                hits.distance_shot(Some(block))
            } else {
                if let Some(previous) = previous {
                    hits.melee_hit(previous);
                }
                hits.melee_hit(block)
            }
        },
        trained,
    );
}

pub(super) fn select_swing(
    entry: &mut ActorAttack,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    digest: [u8; 32],
    now: SemanticTimeMicros,
    constants: &AttackConstants,
) -> Option<SelectedSwing> {
    let target = entry.state.target()?;
    let player = states.get(runtime, entry.actor, entry.session)?;
    if states.player_hidden_from(runtime, entry.actor, target, now) {
        entry.clear_target();
        return None;
    }
    if states.is_dead(entry.actor) {
        entry.clear_target();
        return None;
    }
    // CONDITIONS-0 §3.5: applying `pacified` clears a running target.
    if pacified_applied_since(
        runtime,
        states,
        entry.actor,
        entry.session,
        entry.target_set_at,
    ) {
        entry.clear_target();
        return None;
    }
    // Canary Player::doAttacking pauses swings without dropping the held target or timer.
    if player
        .cooldowns
        .group_ready_at(oteryn_gameplay_spell_rules::cast::STAIRHOP_COOLDOWN_KEY)
        .is_some_and(|until| now < until)
    {
        return None;
    }
    // SPELL-LOCK-2 §1.3: while the attacker's own slot is reserved by a pending spell
    // batch, its swing defers to a later turn; the held target and swing clock stay.
    if states.has_pending_spell_commit(entry.actor, entry.session)
        || runtime.assert_actor_spell_unreserved(entry.actor).is_err()
    {
        return None;
    }
    // ATTACK-2: a swing waits for a swing weapon read under this pin within one attack
    // interval; the attacker's own connection refreshes it (`refresh_attack_weapon`).
    let weapon = entry
        .weapon
        .as_ref()
        .filter(|facts| {
            facts.content_digest == digest
                && now
                    .elapsed_since(facts.read_at)
                    .is_ok_and(|age| age <= constants.attack_interval_micros())
        })
        .cloned()?;
    let mut character_facts = player.character_facts();
    character_facts.magic_level = player.owned_effective_magic_level(now.get()).ok()?;
    // MELEE-SKILL-1: the swing reads the attacker's live skill of the swung weapon (fist for
    // fists) and live shielding, before the swing is taken, so an unresolved skill takes none.
    let attack_index = if weapon.ranged.is_some() {
        4
    } else {
        match weapon.weapon {
            SwingWeapon::Fists => 0,
            SwingWeapon::Melee { skill, .. } => skill.index(),
        }
    };
    let mut skills = player
        .swing_skills(attack_index, now.get())?
        .unwrap_or(SwingSkills {
            attack: constants.starting_skill,
            shielding: constants.starting_skill,
        });
    if attack_index == 4
        && player
            .wheel_effects_at(now.get())
            .is_some_and(|effects| effects.active_unique_perks.contains("positional_tactics"))
        && crate::spell::cast::wheel_adjacent_monsters(runtime, entry.actor)? == 0
    {
        // Crystal player_wheel.cpp:3009-3017: the no-adjacent-monster branch adds 3 distance.
        skills.attack = skills.attack.checked_add(3)?;
    }
    let mut facts = target_facts(runtime, room, entry.actor, entry.session, target, now)?;
    // Vanished/dead/nonattackable targets still reach the original clearing rules, even
    // when ammunition is missing. Cost/range gates may only stall a valid held target.
    if let Some(ranged) = &weapon.ranged
        && facts.present_and_visible
        && facts.attackable_kind
        && facts.alive
    {
        ranged
            .ready(
                facts.distance,
                facts.present_and_visible,
                player.vitals().mana,
            )
            .ok()?;
        // Range was checked with the qualified launcher range. All other target gates and
        // the timer remain the existing attack owner's rules.
        facts.distance = facts.distance.min(1);
    }
    let swing = match entry
        .state
        .poll_swing(now, constants.attack_interval_micros(), &facts)
    {
        SwingPoll::Swing(swing) => swing,
        SwingPoll::Cleared(_) => {
            entry.target_ref = None;
            return None;
        }
        SwingPoll::NoTarget | SwingPoll::Waiting(_) | SwingPoll::NotDue { .. } => {
            return None;
        }
    };
    // The ordinal advances on every swing, a zero draw included, so every swing has its
    // own RNG occurrence; the lineage ends at the last ordinal.
    let ordinal = entry.next_ordinal;
    match ordinal.checked_add(1) {
        Some(next) => entry.next_ordinal = next,
        None => entry.clear_target(),
    }
    Some(SelectedSwing {
        target,
        character_facts,
        skills,
        weapon,
        swing,
        ordinal,
    })
}

fn player_melee_after_defence(
    damage: i64,
    protection: impl IntoIterator<Item = crate::content::ReferenceRationalPercent>,
    armor: impl FnOnce(i64) -> Option<i64>,
) -> Option<i64> {
    let protected = protection.into_iter().try_fold(damage, |damage, percent| {
        percent.validate().ok()?;
        let denominator = 100 * i128::from(percent.denominator);
        let reduction = i128::from(percent.numerator).clamp(-denominator, denominator);
        let remaining = i128::from(damage).checked_mul(denominator - reduction)? / denominator;
        i64::try_from(remaining).ok()
    })?;
    armor(protected)
}

/// Training observes the defence or armor block before the creature's mitigation.
pub(super) fn creature_melee_damage(
    policy: &oteryn_gameplay_content::content::creature_policy::CompiledCreaturePolicy,
    damage: i64,
    blocked: bool,
    armor: u32,
    reduce: impl Fn(i64, bool, u32) -> Option<(i64, HitBlock)>,
) -> Option<(HitBlock, i64)> {
    let (defended, defence_block) = reduce(damage, blocked, 0)?;
    if defended == 0 {
        return Some((defence_block, 0));
    }
    let resisted = creature_physical_resistance(policy, defended)?;
    let (armored, armor_block) = reduce(resisted, false, armor)?;
    let landed = creature_melee_mitigation(policy, armored)?;
    Some((armor_block, landed))
}

fn melee_swing_formula(
    constants: &AttackConstants,
    vocation: crate::spell::Vocation,
    weapon: SwingWeapon,
) -> (crate::spell::formula::Formula, u32, &'static str) {
    let (mut formula, revision) = if matches!(
        vocation,
        crate::spell::Vocation::Monk | crate::spell::Vocation::ExaltedMonk
    ) {
        (player_monk_formula(constants), "formula:player-monk")
    } else {
        match weapon {
            SwingWeapon::Fists => (player_fist_formula(constants), "formula:player-fist"),
            SwingWeapon::Melee { .. } => (player_melee_formula(constants), "formula:player-melee"),
        }
    };
    if matches!(
        weapon,
        SwingWeapon::Melee {
            attack: 0,
            elemental: Some(_),
            ..
        }
    ) {
        formula.minimum = crate::spell::formula::Expression::Const(0.0);
    }
    (formula, weapon.attack(constants.fist.attack), revision)
}

pub(super) struct SwingDamageContext<'a> {
    pub(super) root: &'a GameplayDecisionRoot,
    pub(super) offence: &'a EquippedOffence,
    pub(super) pure_elemental_with_attack_points: bool,
    pub(super) area: bool,
}

pub(super) fn draw_swing_damage(
    entry: &mut ActorAttack,
    runtime: &ChannelRuntimeV1,
    selected: &SelectedSwing,
    constants: &AttackConstants,
    context: SwingDamageContext<'_>,
) -> Option<(i64, &'static str)> {
    let SwingDamageContext {
        root,
        offence,
        pure_elemental_with_attack_points,
        area,
    } = context;
    let SelectedSwing {
        target,
        character_facts,
        skills,
        swing,
        ordinal,
        ..
    } = *selected;
    let creature = runtime.contains_live_creature(target);
    if selected.weapon.ranged.is_some() && creature {
        entry
            .state
            .record_hit(swing.at, constants.in_fight_micros());
    }
    let mut scripted = None;
    let (formula, attack_value, revision) = if let Some(ranged) = &selected.weapon.ranged {
        if !area {
            let from = runtime.read_actor_position(entry.actor).ok()?.position();
            let to = runtime.read_actor_position(target).ok()?.position();
            if !ranged.hits_target(entry, selected, from, to, root, offence)? {
                return Some((-1, "formula:player-distance"));
            }
        }
        scripted = ranged.scripted_arrow();
        ranged.formula()?
    } else {
        melee_swing_formula(constants, character_facts.vocation, selected.weapon.weapon)
    };
    let inputs = FormulaInputs {
        level: character_facts.level,
        magic_level: character_facts.magic_level,
        base_power: None,
        attack_skill: skills.attack,
        attack_value,
        attack_factor: AttackConstants::PLAYER_ATTACK_FACTOR,
        shielding_skill: skills.shielding,
        shield_defense: None,
    };
    if let Some(script) = scripted {
        return script.damage(
            &inputs,
            root,
            swing_occurrence(
                entry.actor,
                swing.lineage,
                ordinal,
                b"scripted-arrow-damage",
            ),
        );
    }
    let (mut minimum, mut maximum) = if let Some(ranged) = &selected.weapon.ranged {
        ranged.roll_bounds(&formula, &inputs, creature)?
    } else {
        formula.bounds(&inputs).ok()?
    };
    if pure_elemental_with_attack_points {
        minimum = i64::from(character_facts.level / 5);
        maximum = maximum.max(minimum);
    }
    let damage = swing_draw(
        root,
        entry.actor,
        swing.lineage,
        ordinal,
        b"damage-draw",
        "attack.damage",
        minimum,
        maximum,
    )?;
    if creature {
        entry
            .state
            .record_hit(swing.at, constants.in_fight_micros());
    }
    swing_lands(damage).then_some((damage, revision))
}

/// `Game::combatBlockHit`: a 0 swing still meets the target's defence and armor (it spends a
/// block and trains as a blocked hit), at a creature and, since PVP-DEFENCE-1, at a player.
const fn swing_lands(damage: i64) -> bool {
    damage >= 0
}

fn record_hit_activity(
    entry: &mut ActorAttack,
    runtime: &ChannelRuntimeV1,
    content: &NativeGameplayState,
    target: ExactActorRef,
    now: SemanticTimeMicros,
    result: &crate::foundation::OwnerDamageResult,
    activity_cells: &mut Vec<SharedExperienceActivity>,
) {
    if result.applied
        && result.health_after < result.health_before
        && aggressive_monster(runtime, content, target)
    {
        entry.record_shared_experience_activity(runtime, now);
        if let Some(cell) = entry.shared_experience_activity {
            if let Some(old) = activity_cells
                .iter_mut()
                .find(|old| old.character == cell.character)
            {
                *old = cell;
            } else {
                activity_cells.push(cell);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn settle_kill(
    entry: &mut ActorAttack,
    runtime: &mut ChannelRuntimeV1,
    queue: &mut KillSettlementQueue,
    rewards: &crate::content::creature_reward::CreatureRewardTable,
    charms: &crate::content::charm_source::CanonicalCharmCatalogue,
    activity: &[SharedExperienceActivity],
    states: &super::super::actor_spell::ChannelSpellStates,
    target: ExactActorRef,
    lethal: crate::combat::CapturedRewardPrincipal,
    now: SemanticTimeMicros,
    clear: bool,
) {
    crate::gameplay_transport::kill_reward::record_creature_kill(
        runtime,
        crate::gameplay_transport::kill_reward::KillSink {
            prey: Some(states),
            queue,
            rewards,
            activity,
            charms: Some(charms),
        },
        target,
        lethal,
        now.get() / 1_000,
        Some(now),
    );
    if clear {
        entry.clear_target();
    }
}

fn scale_outgoing(value: i64, retained: i32) -> Option<i64> {
    value
        .checked_mul(i64::from(retained))
        .map(|value| value / 100)
        .filter(|value| *value >= 0)
}

fn pvp_swing_components(
    (physical, secondary, element): (i64, i64, crate::content::ProjectV2RangedElement),
    black: bool,
    retained: i32,
) -> Option<(i64, i64, crate::content::ProjectV2RangedElement)> {
    let scale = |value| {
        let half = oteryn_gameplay_combat_rules::pvp::pvp_damage(u32::try_from(value).ok()?, black);
        scale_outgoing(i64::from(half), retained)
    };
    Some((scale(physical)?, scale(secondary)?, element))
}

struct RangedPlayerDamage {
    components: crate::spell::cast::WheelAttackComponents,
    protections: std::collections::BTreeMap<
        crate::content::ReferenceResistanceKind,
        crate::content::ReferenceRationalPercent,
    >,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(clippy::too_many_arguments)]
    async fn ranged_player_damage(
        &self,
        permit: &SpellLanePermit,
        runtime: &ChannelRuntimeV1,
        content: &NativeGameplayState,
        session: GameSessionId,
        ranged: &ranged::RangedLoadout,
        hit: &PvpSwing,
        constants: &AttackConstants,
        root: &GameplayDecisionRoot,
        armor: u32,
    ) -> Option<RangedPlayerDamage> {
        use crate::content::ProjectV2RangedElement as Element;
        use crate::durability::character_equipment as equipment;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(self.root.clone())
                .current_session_at(session)
                .await
                .ok()?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current)?;
        let pass = self.root.try_issue_semantic_pass().ok()?;
        let mut context = (self, permit, content);
        let command = CommandRef::new(session, hit.lineage.command_id());
        let effects = pass.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let (owner, _permit, content) = context;
            let mut tx = crate::durability::spell_item_transaction::begin_spell_owner_transaction(holder, deadline).await?;
            let authority = equipment::assert_equipment_authority_in_transaction(&mut tx, owner.root, owner.character, owner.holder, &fence, command, content.source_digest()).await.map_err(|_| crate::durability::DurabilityError::Unavailable)?;
            let equipment = equipment::read_equipment_in_transaction(&mut tx, &authority).await.map_err(|_| crate::durability::DurabilityError::Unavailable)?;
            crate::gameplay_transport::equipment_move::equipment_effects_owner::equipment_effects(&equipment, content).ok_or(crate::durability::DurabilityError::Unavailable)
        })).await.ok()?;
        let (physical, elemental, element) = hit.components;
        let physical = if physical > 0 {
            oteryn_gameplay_combat_rules::resolution::swing_mitigated(
                root,
                constants,
                hit.actor.placement_identity(),
                (
                    *hit.lineage.game_session_id().as_bytes(),
                    hit.lineage.command_id().get(),
                ),
                hit.ordinal,
                physical,
                None,
                armor,
            )?
        } else {
            0
        };
        runtime.player_control_facts(hit.target, session).ok()?;
        let element = match element {
            Element::Physical => crate::player_lethal::PlayerHitKind::Physical,
            Element::Death => crate::player_lethal::PlayerHitKind::Death,
            Element::Earth => crate::player_lethal::PlayerHitKind::Earth,
            Element::Energy => crate::player_lethal::PlayerHitKind::Energy,
            Element::Fire => crate::player_lethal::PlayerHitKind::Fire,
            Element::Ice => crate::player_lethal::PlayerHitKind::Ice,
        };
        let physical = u32::try_from(physical).ok()?;
        let elemental = u32::try_from(elemental).ok()?;
        let components = if matches!(ranged.damage, ranged::RangedDamage::Magic { .. }) {
            crate::spell::cast::WheelAttackComponents {
                primary: (elemental, element),
                secondary: (0, element),
            }
        } else {
            crate::spell::cast::WheelAttackComponents {
                primary: (physical, crate::player_lethal::PlayerHitKind::Physical),
                secondary: (elemental, element),
            }
        };
        Some(RangedPlayerDamage {
            components,
            protections: effects.protections,
        })
    }
}

/// Canary weapons.cpp:23-33, player.cpp:11959-12053 (99902524): weapon
/// sounds originate at the wielder; the hit cue precedes the attack cue.
fn publish_hit_sound(
    states: &mut ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    content: &NativeGameplayState,
    actor: ExactActorRef,
    weapon: &WeaponFacts,
) {
    let Ok(position) = runtime.read_actor_position(actor) else {
        return;
    };
    let (cue, secondary) = if let Some(ranged) = &weapon.ranged {
        use crate::content::{ProjectV2RangedWeapon, ReferenceAmmoType};
        let Some(profile) = content
            .item_policy(
                &ranged.weapon.definition.production_key,
                &ranged.weapon.definition.revision_ref,
            )
            .and_then(|policy| policy.record().authoring.ranged.as_ref())
        else {
            return;
        };
        match profile {
            ProjectV2RangedWeapon::Distance {
                ammunition: ReferenceAmmoType::Arrow,
                ..
            } => (13, 6),
            ProjectV2RangedWeapon::Distance {
                ammunition: ReferenceAmmoType::Bolt,
                ..
            } => (14, 7),
            ProjectV2RangedWeapon::Wand(_) | ProjectV2RangedWeapon::Rod(_) => (17, 9),
            _ => (15, 8),
        }
    } else {
        let skill = match weapon.weapon {
            SwingWeapon::Fists => 0,
            SwingWeapon::Melee { skill, .. } => skill.index(),
        };
        (
            match skill {
                1 => 4,
                2 => 3,
                3 => 5,
                _ => 1,
            },
            0,
        )
    };
    if let Some(owner) = states.presentations.as_mut() {
        let _ = states.sounds.capture(owner, runtime);
    }
    states
        .sounds
        .record(runtime, actor, position.position(), cue, secondary);
}

// Native health callbacks change only primary; Cruelty scales each component
// before the shared Wheel owner. Keep both results through elemental conversion.
fn finish_melee_components(
    parts: [i64; 2],
    health_primary: impl FnOnce(i64) -> Option<i64>,
    cruelty_drain: Option<u64>,
    finish: impl FnOnce(i64, i64) -> Option<(i64, i64)>,
) -> Option<[i64; 2]> {
    let primary = super::cruelty_health_component(health_primary(parts[0])?, cruelty_drain)?;
    let secondary = super::cruelty_health_component(parts[1], cruelty_drain)?;
    // A zero Cruelty result exits Game's HP path before Wheel can add damage.
    if cruelty_drain.is_some() && primary == 0 && secondary == 0 {
        return None;
    }
    let (primary, secondary) = finish(primary, secondary)?;
    Some([primary, secondary])
}

fn boost_attack_components(parts: [i64; 2], prey: u8, talisman: u16) -> [i64; 2] {
    parts.map(|part| crate::spell::magnitude_owner::boost_creature_damage(part, prey, talisman))
}

fn elite_attack_components(parts: [i64; 2], elite: bool) -> [i64; 2] {
    if elite {
        parts.map(|part| part / 2)
    } else {
        parts
    }
}

// Creature health is consumed in component order on a lethal hit.
fn reported_primary(primary: i64, committed: u32) -> u32 {
    u32::try_from(primary.clamp(0, i64::from(committed))).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{melee_swing_formula, swing_lands};

    #[test]
    fn global_parity_wiki_200_damage_with_zaoan_helmet_protection_amulet_and_armor_9()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::content::ReferenceRationalPercent;
        use oteryn_gameplay_combat_rules::resolution::player_armor_mitigated;

        let constants = super::AttackConstants::checked_in().ok_or("attack constants")?;
        let helmet = ReferenceRationalPercent::new(5, 1)?;
        let amulet = ReferenceRationalPercent::new(6, 1)?;
        let equipped_total = ReferenceRationalPercent::new(11, 1)?;
        let helmet_only = super::player_melee_after_defence(200, [helmet], Some);
        assert_eq!(helmet_only, Some(190));
        for (draw, expected) in (0..4).zip([174, 173, 172, 171]) {
            let armor = |protected| {
                assert_eq!(protected, 178);
                let protected = u32::try_from(protected).ok()?;
                Some(i64::from(player_armor_mitigated(
                    constants, protected, 9, draw,
                )))
            };
            assert_eq!(
                super::player_melee_after_defence(200, [helmet, amulet], armor),
                Some(expected)
            );
            // The current equipment snapshot supplies one summed physical protection.
            assert_eq!(
                super::player_melee_after_defence(200, Some(equipped_total), armor),
                Some(expected)
            );
        }
        Ok(())
    }

    #[test]
    fn t174_converted_melee_wheel_preserves_both_mitigated_components()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_gameplay_progression::{
            wheel::{WHEEL_SLOTS, WheelAllocation, WheelSlots},
            wheel_effects::derive_runtime_effects,
            wheel_gem_data::WheelGemData,
        };
        let data = WheelGemData::embedded()?;
        let mut points = [0; WHEEL_SLOTS];
        let mut remaining = 250u16;
        for (index, slot) in data.wheel()["topology"]
            .as_array()
            .ok_or("topology")?
            .iter()
            .enumerate()
        {
            if slot["domain"] == "green" {
                let capacity = u16::try_from(slot["capacity"].as_u64().ok_or("capacity")?)?;
                points[index] = capacity.min(remaining);
                remaining -= points[index];
            }
        }
        assert_eq!(remaining, 0);
        let allocation = WheelAllocation {
            wheel_ruleset_revision: Some(data.revision().to_owned()),
            wheel_revision: 1,
            slots: WheelSlots::new(points).map_err(|_| "wheel slots")?,
            current: true,
        };
        let effects = derive_runtime_effects(data.wheel(), &allocation, "knight", 1000, true, true)
            .ok_or("effects")?;
        assert_eq!(effects.flat_damage_bonus(), Some(4));
        let parts = super::finish_melee_components([50, 50], Some, None, |primary, secondary| {
            effects.finish_damage_pair(primary, secondary, 1000, 1000, false)
        });
        assert_eq!(parts, Some([54, 54]));
        assert_eq!(parts.map(|parts| parts[0] + parts[1]), Some(108));
        Ok(())
    }

    #[test]
    fn t174_converted_melee_native_health_and_cruelty_precede_wheel() {
        let parts = super::finish_melee_components(
            [50, 50],
            |primary| {
                assert_eq!(primary, 50);
                Some(primary + 10)
            },
            Some(50),
            |primary, secondary| {
                assert_eq!((primary, secondary), (30, 25));
                Some((primary + 4, secondary + 4))
            },
        );
        assert_eq!(parts, Some([34, 29]));
        let mut wheel_called = false;
        assert_eq!(
            super::finish_melee_components([50, 50], Some, Some(100), |_, _| {
                wheel_called = true;
                None
            }),
            None
        );
        assert!(!wheel_called, "fully drained health must not reach Wheel");
    }

    #[test]
    fn t174_converted_melee_analyser_uses_final_scaled_components() {
        let boosted = super::boost_attack_components([50, 50], 20, 2500);
        assert_eq!(boosted, [75, 75]);
        assert_eq!(super::boost_attack_components([50, 50], 25, 0), [63, 63]);
        assert_eq!(super::reported_primary(boosted[0], 150), 75);
        // Lethal clipping uses all of the available primary before spending secondary.
        assert_eq!(super::reported_primary(boosted[0], 120), 75);
        assert_eq!(super::reported_primary(boosted[0], 0), 0);
    }

    #[test]
    fn crystal_equipment_zero_talisman_contribution_preserves_prey_rounding() {
        assert_eq!(
            super::boost_attack_components([1, 1], 25, 300),
            super::boost_attack_components([1, 1], 25, 0),
        );
    }

    #[test]
    fn t174_elite_bounty_converted_melee_reports_halved_components() -> Result<(), &'static str> {
        use crate::content::ProjectV2RangedElement as E;
        use crate::durability::task_board::BountyTalismanSnapshot;
        use oteryn_gameplay_quests::quest::task_board::full::{
            BountyStatus, BountyTask, TaskBoardData,
        };
        let mut board = TaskBoardData::default();
        board.bounty.state = BountyStatus::Active;
        board.bounty.active = Some(BountyTask {
            elite: true,
            dust: 0,
            race: "oteryn:creature.rat".into(),
            required: 25,
            current: 0,
            experience: 1850,
            points: 25,
            grade: 0,
            difficulty: 0,
            slot: 0,
        });
        let talisman = BountyTalismanSnapshot::from_board(&board, &Default::default(), true);
        let offence = oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence {
            elemental_conversion: Some((E::Fire, 50)),
            ..Default::default()
        };
        let (physical, elemental, element) = offence
            .convert_physical(100, 0, E::Physical)
            .ok_or("conversion")?;
        let boosted = super::boost_attack_components([physical, elemental], 20, 2500);
        let parts = super::elite_attack_components(boosted, talisman.elite("oteryn:creature.rat"));
        // 50 -> 60 after Prey -> 75 after Talisman -> 37 per component.
        // Halving the combined 150 would incorrectly land 75 instead of 74.
        assert_eq!(parts, [37, 37]);
        let landed = u32::try_from(parts[0] + parts[1]).map_err(|_| "landed")?;
        let reported_physical = super::reported_primary(parts[0], landed);
        let reported_elemental = landed - reported_physical;
        assert_eq!(
            (reported_physical, reported_elemental, landed),
            (37, 37, 74)
        );
        assert_eq!(
            super::ranged::analyser_element(element),
            oteryn_protocol_oteryn::damage_element::DamageElement::Fire
        );
        assert_eq!(
            super::elite_attack_components(boosted, talisman.elite("oteryn:creature.cave_rat")),
            [75, 75]
        );
        Ok(())
    }

    #[test]
    fn crystal_equipment_pvp_swing_halves_loaded_battleaxe_components_individually()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::content::ProjectV2RangedElement;
        use crate::content::native_gameplay::{
            NativeGameplayInput, compile_native_gameplay, decode,
        };
        use oteryn_gameplay_combat_rules::resolution::swing_weapon;

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let mut input =
            NativeGameplayInput::from_manifest(&root.join("content/spells.manifest.json"))?;
        input.source_world = None;
        let room = crate::content::qualify_native_entry_room(
            crate::content::native_gameplay_test_world_id()?,
        )?;
        let state =
            decode(&compile_native_gameplay(room.compiled(), &input)?.server_artifact)?.state;
        let weapon = swing_weapon(&state, |slot| {
            (slot == 5).then_some(("oteryn:item.tibia.i49523", "definition-r1"))
        })
        .ok_or("Inferniarch Battleaxe")?;
        let (physical, elemental, element) = weapon.split_damage(107).ok_or("weapon components")?;
        assert_eq!(element, ProjectV2RangedElement::Fire);
        assert_eq!(
            super::pvp_swing_components((physical, elemental, element), false, 100),
            Some((physical / 2, elemental / 2, element)),
        );
        Ok(())
    }

    #[test]
    fn crystal_equipment_amber_slayer_uses_intrinsic_attack_for_swing()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::combat_attack::AttackConstants;
        use crate::content::native_gameplay::{
            NativeGameplayInput, compile_native_gameplay, decode,
        };
        use oteryn_gameplay_combat_rules::resolution::swing_weapon;

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let mut input =
            NativeGameplayInput::from_manifest(&root.join("content/spells.manifest.json"))?;
        input.source_world = None;
        let room = crate::content::qualify_native_entry_room(
            crate::content::native_gameplay_test_world_id()?,
        )?;
        let artifact = compile_native_gameplay(room.compiled(), &input)?;
        let state = decode(&artifact.server_artifact)?.state;
        let weapon = swing_weapon(&state, |slot| {
            (slot == 5).then_some(("oteryn:item.tibia.i47368", "definition-r1"))
        })
        .ok_or("Amber slayer")?;
        let constants = AttackConstants::checked_in().ok_or("attack constants")?;
        let (_, attack, _) = melee_swing_formula(constants, crate::spell::Vocation::Knight, weapon);
        assert_eq!(attack, 50);
        assert_eq!(
            weapon.split_damage(100),
            Some((0, 100, crate::content::ProjectV2RangedElement::Death))
        );
        Ok(())
    }

    #[test]
    fn crystal_equipment_pure_elemental_swing_formula_can_roll_zero()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::combat_attack::AttackConstants;
        use crate::content::native_gameplay::{
            NativeGameplayInput, compile_native_gameplay, decode,
        };
        use crate::spell::formula::FormulaInputs;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let mut input =
            NativeGameplayInput::from_manifest(&root.join("content/spells.manifest.json"))?;
        input.source_world = None;
        let room = crate::content::qualify_native_entry_room(
            crate::content::native_gameplay_test_world_id()?,
        )?;
        let state =
            decode(&compile_native_gameplay(room.compiled(), &input)?.server_artifact)?.state;
        let constants = AttackConstants::checked_in().ok_or("attack constants")?;
        for id in [47368, 47369, 47370, 47374, 47375, 47376] {
            let key = format!("oteryn:item.tibia.i{id}");
            let weapon = oteryn_gameplay_combat_rules::resolution::swing_weapon(&state, |slot| {
                (slot == 5).then_some((key.as_str(), "definition-r1"))
            })
            .ok_or("pure elemental weapon")?;
            let (formula, attack_value, _) =
                melee_swing_formula(constants, crate::spell::Vocation::Knight, weapon);
            let (minimum, maximum) = formula.bounds(&FormulaInputs {
                level: 330,
                magic_level: 0,
                base_power: None,
                attack_skill: 100,
                attack_value,
                attack_factor: 1.0,
                shielding_skill: 0,
                shield_defense: None,
            })?;
            assert_eq!(minimum, 0, "{id}");
            assert!(maximum > 0, "{id}");
        }
        Ok(())
    }

    #[test]
    fn autoatk_monk_and_knight_select_their_own_formula() -> Result<(), Box<dyn std::error::Error>>
    {
        use crate::combat_attack::AttackConstants;
        use crate::gameplay_transport::attack::SwingWeapon;
        use crate::spell::Vocation;
        use crate::spell::formula::FormulaInputs;

        let constants = AttackConstants::checked_in().ok_or("checked-in constants parse")?;
        for (vocation, expected, unarmed, revision) in [
            (
                Vocation::Monk,
                (293, 872),
                (129, 218),
                "formula:player-monk",
            ),
            (
                Vocation::ExaltedMonk,
                (293, 872),
                (129, 218),
                "formula:player-monk",
            ),
            (
                Vocation::Knight,
                (196, 486),
                (114, 159),
                "formula:player-melee",
            ),
            (
                Vocation::EliteKnight,
                (196, 486),
                (114, 159),
                "formula:player-melee",
            ),
        ] {
            let (formula, attack_value, actual_revision) = melee_swing_formula(
                constants,
                vocation,
                SwingWeapon::Melee {
                    attack: 44,
                    elemental: None,
                    skill: oteryn_gameplay_combat_rules::resolution::MeleeSkill::Fist,
                },
            );
            let inputs = FormulaInputs {
                level: 500,
                magic_level: 0,
                base_power: None,
                attack_skill: 100,
                attack_value,
                attack_factor: 1.0,
                shielding_skill: 0,
                shield_defense: None,
            };
            assert_eq!(formula.bounds(&inputs), Ok(expected), "{vocation:?}");
            assert_eq!(actual_revision, revision);
            let (formula, attack_value, actual_revision) =
                melee_swing_formula(constants, vocation, SwingWeapon::Fists);
            assert_eq!(
                formula.bounds(&FormulaInputs {
                    attack_value,
                    ..inputs
                }),
                Ok(unarmed)
            );
            assert_eq!(
                actual_revision,
                if revision == "formula:player-monk" {
                    revision
                } else {
                    "formula:player-fist"
                }
            );
        }
        Ok(())
    }

    #[test]
    fn a_zero_swing_meets_the_targets_defence() {
        assert!(
            swing_lands(0),
            "a 0 swing at a creature or a player is blocked"
        );
        assert!(swing_lands(5));
        assert!(!swing_lands(-1));
    }
}
