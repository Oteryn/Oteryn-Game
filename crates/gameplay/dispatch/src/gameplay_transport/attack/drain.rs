//! Ordered phases of the Channel owner's auto-attack pass.

use super::equipped_offence;
use super::player_block::PlayerBiteVitals;
use super::shared_experience::aggressive_monster;
use super::skill_training::{TrainingDue, swing_block, swing_family, train_hit};
use super::*;
use crate::ability::creature_bite::CreatureBiteVitals;
use crate::ability::exact_actor_resolution::{ExactActorProposal, resolve_exact_actor};
use crate::combat_attack::{SwingPoll, player_fist_formula, player_melee_formula};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::CharacterId;
use crate::foundation::owner_timer::SemanticTimeMicros as OwnerTime;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::gameplay_transport::kill_reward::KillSettlementQueue;
use crate::spell::cast::{CharacterCastFacts, SwingSkills};
use crate::spell::combat_execution::actor_atom;
use crate::spell::formula::FormulaInputs;
use crate::world_runtime::LocalObjectRuntime;
use oteryn_gameplay_combat_rules::attack::target::Swing;
use oteryn_gameplay_combat_rules::resolution::{
    creature_physical_response, creature_physically_immune, swing_mitigated,
};
use oteryn_gameplay_items::equipment_effects::apply_protection;
use oteryn_gameplay_progression::charm_effects::{
    CharmAttackerFacts, CharmHitSource, CharmHookEvent,
};

#[derive(Debug, Clone)]
pub(super) struct SelectedSwing {
    pub(super) target: ExactActorRef,
    pub(super) character_facts: CharacterCastFacts,
    pub(super) skills: SwingSkills,
    pub(super) weapon: WeaponFacts,
    pub(super) swing: Swing<ExactActorRef, CommandRef>,
    pub(super) ordinal: u16,
}

/// A PvP swing the attacker's pass drew, halved, for a player target; its defence is met after
/// the pass (PVP-DEFENCE-1).
pub(super) struct PvpSwing {
    pub(super) actor: ExactActorRef,
    pub(super) session: GameSessionId,
    pub(super) secure: bool,
    pub(super) target: ExactActorRef,
    pub(super) lineage: CommandRef,
    pub(super) ordinal: u16,
    pub(super) magnitude: i64,
    pub(super) black: bool,
    pub(super) revision: &'static str,
    pub(super) weapon: WeaponFacts,
    pub(super) attacker: CharacterId,
    pub(super) generation: u64,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// One owner pass over every held target: at most one swing per attacker (`DEADLINE_STATE`
    /// catch-up), never a backlog.
    pub(in crate::gameplay_transport) async fn drain_auto_attacks(&self) {
        let Some(room) = self.rooms.gate_auto_attack() else {
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
        let Some(mut pvp_permit) = self.spell_lane_permit().await else {
            return;
        };
        let now = self.owner_now();
        let mut runtime = self.runtime.lock().await;
        let digest = runtime.content_pin().server_artifact_digest();
        if runtime.owner_fence().is_err()
            || content.source_digest() != digest
            || room.compiled().server_digest() != digest
        {
            return;
        }
        let mut states = self.states.spell_states.lock().await;
        let objects = self.door.lock().await;
        let mut attack = self.states.attack.lock().await;
        attack.prune(&runtime);
        let ChannelAttackStates {
            entries,
            blocks,
            kills,
            ..
        } = &mut *attack;
        let mut activity_cells: Vec<_> = entries
            .iter()
            .filter_map(|e| e.shared_experience_activity)
            .collect();
        let root = GameplayDecisionRoot::from_bytes(digest);
        let mut trained = TrainingDue::new();
        let mut pvp_hits = Vec::new();
        for entry in entries.iter_mut() {
            let state_before = entry.state.clone();
            let ordinal_before = entry.next_ordinal;
            let Some(selected) =
                select_swing(entry, &runtime, &states, room, digest, now, constants)
            else {
                continue;
            };
            let pvp_black = if !runtime.contains_live_creature(selected.target) {
                let Some(black) = self
                    .pvp_target_legal(
                        &pvp_permit,
                        &runtime,
                        &states,
                        room,
                        &objects,
                        entry.actor,
                        selected.target,
                        selected.swing.lineage,
                        entry.modes.secure,
                    )
                    .await
                else {
                    entry.clear_target();
                    continue;
                };
                Some(black)
            } else {
                None
            };
            if selected.weapon.ranged.is_some()
                && !self
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
            {
                if pvp_permit.has_unresolved() {
                    return;
                }
                entry.state = state_before;
                entry.next_ordinal = ordinal_before;
                continue;
            }
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
                entry.clear_target();
                continue;
            };
            let character = *lease.character_id().as_bytes();
            let Some((damage, revision)) =
                draw_swing_damage(entry, &runtime, &selected, constants, &root)
            else {
                continue;
            };
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
            let offence =
                equipped_offence::attacker_offence(&states, &runtime, entry.actor, entry.session);
            let Some(damage) = equipped_offence::critical_swing(
                offence,
                &root,
                entry.actor,
                swing.lineage,
                ordinal,
                damage,
            ) else {
                continue;
            };
            if !runtime.contains_live_creature(target) {
                let Some(black) = pvp_black else {
                    continue;
                };
                let Ok(raw) = u32::try_from(damage) else {
                    entry.clear_target();
                    continue;
                };
                // `Combat::CombatHealthFunc` halves a PvP hit before the target's block meets it;
                // PVP-DEFENCE-1 resolves the defence after this pass, where the target's block
                // budget is reachable.
                pvp_hits.push(PvpSwing {
                    actor: entry.actor,
                    session: entry.session,
                    secure: entry.modes.secure,
                    target,
                    lineage: swing.lineage,
                    ordinal,
                    magnitude: i64::from(oteryn_gameplay_combat_rules::pvp::pvp_damage(raw, black)),
                    black,
                    revision,
                    weapon: selected.weapon.clone(),
                    attacker: lease.character_id(),
                    generation: lease.generation(),
                });
                continue;
            }
            let Ok(snapshot) = runtime.companion_snapshot(target) else {
                entry.clear_target();
                continue;
            };
            let race_key = snapshot.state.policy.definition_key.clone();
            // The creature's content defence and armor (ATTACK-0 §5, `monster.cpp:242-251`);
            // a creature without them takes no hit (fail closed).
            let (Some(defense), Some(armor)) =
                (snapshot.state.policy.defense, snapshot.state.policy.armor)
            else {
                continue;
            };
            // CREATURE-RESIST-MELEE-1: a creature immune to physical damage meets the swing
            // before its block (`Creature::blockHit` `isImmune`): no block, no training, no hit.
            if selected.weapon.ranged.is_none()
                && creature_physically_immune(&snapshot.state.policy)
            {
                continue;
            }
            if let Some(ranged) = &selected.weapon.ranged
                && ranged::immune(ranged, &snapshot.state.policy)
            {
                if matches!(ranged.damage, ranged::RangedDamage::Distance { .. }) {
                    ranged::train_distance(
                        &mut states,
                        &runtime,
                        content,
                        entry,
                        Some(oteryn_gameplay_progression::hit_training::HitBlock::Immunity),
                        now,
                        &mut trained,
                    );
                }
                continue;
            }
            let creature_max_health = u64::try_from(snapshot.maximum_health).unwrap_or(0);
            run_charm_hook(
                self.imported_charms,
                character,
                &race_key,
                &root,
                swing_occurrence(entry.actor, swing.lineage, ordinal, b"damage-calculation"),
                CharmHookEvent::AttackDamageCalculation {
                    source: CharmHitSource::CharacterAttack,
                },
            );
            let ranged_magnitude = if let Some(ranged) = &selected.weapon.ranged {
                ranged::creature_damage(
                    ranged,
                    &snapshot.state.policy,
                    damage,
                    (&root, constants, entry.actor, swing.lineage, ordinal),
                )
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
            let blocked = selected.weapon.ranged.is_none()
                && blocks[index].1.try_block(&constants.block, swing.at);
            let Some(magnitude) = (if selected.weapon.ranged.is_some() {
                ranged_magnitude
            } else {
                creature_mitigated(
                    &root,
                    constants,
                    entry.actor,
                    swing.lineage,
                    ordinal,
                    damage,
                    blocked,
                    defense,
                    armor,
                )
            }) else {
                continue;
            };
            // CREATURE-RESIST-MELEE-1: the creature's mitigation and physical response shape
            // what passed its defence and armor (`Creature::mitigateDamage`, `Monster::blockHit`).
            // Training already counted the swing by its block, so it trains by `magnitude`.
            let landed = if selected.weapon.ranged.is_some() {
                magnitude
            } else if magnitude > 0 {
                let Some(landed) = creature_physical_response(&snapshot.state.policy, magnitude)
                else {
                    continue;
                };
                landed
            } else {
                magnitude
            };
            if landed <= 0 {
                train_swing(
                    &mut states,
                    &runtime,
                    content,
                    entry,
                    &selected.weapon,
                    magnitude,
                    now,
                    &mut trained,
                );
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
                entry.clear_target();
                continue;
            };
            // The owner clock of this turn, in ms, stamps the hit's contributor and, for a lethal
            // hit, the death time the kill reward reads (§1.2).
            let committed = crate::ability::commit::commit_exact_owner_swing_damage(
                &mut runtime.borrow_exact_actor_commit_at(now.get() / 1_000),
                &resolved,
                &plan,
                entry.actor,
                swing.lineage,
                ordinal,
            );
            let Ok(result) = committed else {
                entry.clear_target();
                continue;
            };
            train_swing(
                &mut states,
                &runtime,
                content,
                entry,
                &selected.weapon,
                magnitude,
                now,
                &mut trained,
            );
            equipped_offence::leech_hit(
                &mut states,
                &runtime,
                entry.actor,
                entry.session,
                offence,
                &result,
            );
            if result.applied
                && result.health_after < result.health_before
                && snapshot.state.master.is_none()
                && !snapshot.state.policy.is_familiar
            {
                states.hunting_activity(&runtime, entry.actor, entry.session, now.get());
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
            run_charm_hook(
                self.imported_charms,
                character,
                &race_key,
                &root,
                swing_occurrence(entry.actor, swing.lineage, ordinal, b"committed-hit"),
                CharmHookEvent::committed_hit(
                    &result,
                    CharmHitSource::CharacterAttack,
                    CharmAttackerFacts {
                        level: character_facts.level,
                        max_health: u64::from(character_facts.max_health),
                        max_mana: u64::from(character_facts.max_mana),
                    },
                    creature_max_health,
                ),
            );
            if result.health_after <= 0 {
                let lethal = crate::combat::CapturedRewardPrincipal {
                    character: lease.character_id(),
                    lease_generation: lease.generation(),
                    session: entry.session,
                    actor: entry.actor,
                };
                settle_kill(
                    entry,
                    &mut runtime,
                    kills,
                    self.reward_table(),
                    &activity_cells,
                    target,
                    lethal,
                    now,
                );
            }
        }
        for hit in pvp_hits {
            self.land_pvp_swing(
                &mut pvp_permit,
                &mut runtime,
                &mut states,
                &mut attack,
                room,
                &objects,
                content,
                (&root, &digest, constants, now),
                hit,
                &mut trained,
            )
            .await;
        }
        drop(attack);
        drop(objects);
        drop(states);
        drop(runtime);
        drop(pvp_permit);
        self.save_trained_levels(trained).await;
    }
}

/// SKILL-TRAIN-1: the swing at a creature trains the swing's skill by how its block met it.
impl ComposedFreshAdmission<'_, '_, '_> {
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
        let Some(target_session) = states
            .actors
            .iter()
            .find(|(actor, _, _)| *actor == hit.target)
            .map(|(_, session, _)| *session)
        else {
            attack.clear_target_of(hit.actor, hit.session);
            return;
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
            states: &mut *states,
            attack: &mut *attack,
            formula: content.training_formula(),
            trained: &mut *trained,
        };
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
        } else {
            swing_mitigated(
                root,
                constants,
                hit.actor.placement_identity(),
                (
                    *hit.lineage.game_session_id().as_bytes(),
                    hit.lineage.command_id().get(),
                ),
                hit.ordinal,
                hit.magnitude,
                defence,
                armor,
            )
        }) else {
            return;
        };
        // EQUIP-PROTECTION-1 (`Player::blockHit`): what defence and armor left meets the
        // target's worn physical protection, summed and applied once (EQUIP-0 §3.3).
        let magnitude = match protection {
            Some(sum) if mitigated > 0 && hit.weapon.ranged.is_none() => {
                apply_protection(mitigated, sum).unwrap_or(mitigated)
            }
            _ => mitigated,
        };
        if magnitude <= 0 {
            train_pvp_swing(states, runtime, content, &hit, magnitude, now, trained);
            return;
        }
        let atom = actor_atom(hit.target);
        let Some(plan) = swing_plan(
            hit.actor,
            hit.lineage,
            hit.ordinal,
            magnitude,
            digest,
            &atom,
            hit.revision,
        ) else {
            return;
        };
        let batch = crate::spell::combat_batch::OwnerCombatBatch {
            caster: hit.actor,
            attacker: hit.attacker,
            current_lease_generation: hit.generation,
            command: hit.lineage,
            occurrence: plan.occurrence().clone().into(),
            binding: b"oteryn:pvp-melee:v1".to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1000,
            effects: vec![crate::spell::combat_batch::OwnerCombatEffect {
                target: hit.target,
                sub_ordinal: hit.ordinal,
                change: crate::spell::combat_batch::OwnerCombatChange::Damage {
                    target_atom: atom,
                    magnitude,
                },
            }],
            deferred: None,
        };
        if self
            .pvp_swing(
                permit,
                runtime,
                states,
                room,
                objects,
                batch,
                hit.ordinal,
                hit.secure,
                hit.black,
            )
            .await
        {
            train_pvp_swing(states, runtime, content, &hit, magnitude, now, trained);
        } else {
            attack.clear_target_of(hit.actor, hit.session);
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
    magnitude: i64,
    now: SemanticTimeMicros,
    trained: &mut TrainingDue,
) {
    if let Some(ranged) = &weapon.ranged {
        if matches!(ranged.damage, ranged::RangedDamage::Distance { .. }) {
            ranged::train_distance(
                states,
                runtime,
                content,
                entry,
                Some(swing_block(magnitude)),
                now,
                trained,
            );
        }
        return;
    }
    let block = swing_block(magnitude);
    train_hit(
        states,
        runtime,
        content.training_formula(),
        entry.actor,
        entry.session,
        swing_family(weapon.weapon),
        now.get(),
        |hits| hits.melee_hit(block),
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
    let character_facts = player.character_facts();
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
    let skills = player
        .swing_skills(attack_index, now.get())?
        .unwrap_or(SwingSkills {
            attack: constants.starting_skill,
            shielding: constants.starting_skill,
        });
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

fn draw_swing_damage(
    entry: &mut ActorAttack,
    runtime: &ChannelRuntimeV1,
    selected: &SelectedSwing,
    constants: &AttackConstants,
    root: &GameplayDecisionRoot,
) -> Option<(i64, &'static str)> {
    let SelectedSwing {
        target,
        character_facts,
        skills,
        swing,
        ordinal,
        ..
    } = *selected;
    if selected.weapon.ranged.is_some() && runtime.contains_live_creature(target) {
        entry
            .state
            .record_hit(swing.at, constants.in_fight_micros());
    }
    let mode = constants.fight_mode(fight_mode(entry.modes.fight_mode));
    let (formula, attack_value, revision) = if let Some(ranged) = &selected.weapon.ranged {
        let from = runtime.read_actor_position(entry.actor).ok()?.position();
        let to = runtime.read_actor_position(target).ok()?.position();
        let distance = from.x.abs_diff(to.x).max(from.y.abs_diff(to.y));
        let roll = swing_draw(
            root,
            entry.actor,
            swing.lineage,
            ordinal,
            b"hit-chance",
            "attack.hit-chance",
            0,
            99,
        )?;
        if !ranged.hit(distance, skills.attack, u64::try_from(roll).ok()?)? {
            return Some((-1, "formula:player-distance"));
        }
        ranged.formula()?
    } else {
        match selected.weapon.weapon {
            SwingWeapon::Fists => (
                player_fist_formula(constants),
                constants.fist.attack,
                "formula:player-fist",
            ),
            SwingWeapon::Melee { attack, .. } => (
                player_melee_formula(constants),
                attack,
                "formula:player-melee",
            ),
        }
    };
    let inputs = FormulaInputs {
        level: character_facts.level,
        magic_level: character_facts.magic_level,
        base_power: None,
        attack_skill: skills.attack,
        attack_value,
        attack_factor: mode.attack_factor,
        shielding_skill: skills.shielding,
        shield_defense: None,
    };
    let Ok((minimum, maximum)) = formula.bounds(&inputs) else {
        return None;
    };
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
    let creature = runtime.contains_live_creature(target);
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
    activity: &[SharedExperienceActivity],
    target: ExactActorRef,
    lethal: crate::combat::CapturedRewardPrincipal,
    now: SemanticTimeMicros,
) {
    crate::gameplay_transport::kill_reward::record_creature_kill(
        runtime,
        crate::gameplay_transport::kill_reward::KillSink {
            queue,
            rewards,
            activity,
        },
        target,
        lethal,
        now.get() / 1_000,
        Some(now),
    );
    entry.clear_target();
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
    ) -> Option<i64> {
        use crate::content::{ProjectV2RangedElement as Element, ReferenceResistanceKind as Kind};
        use crate::durability::character_equipment as equipment;
        use oteryn_gameplay_items::equipment_effects::apply_protection;
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
        let (physical, elemental, element) = ranged.split(hit.magnitude)?;
        let physical = if physical > 0 {
            let damage = oteryn_gameplay_combat_rules::resolution::swing_mitigated(
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
            )?;
            match effects.protections.get(&Kind::Physical) {
                Some(sum) => apply_protection(damage, *sum)?,
                None => damage,
            }
        } else {
            0
        };
        let kind = match element {
            Element::Physical => Kind::Physical,
            Element::Death => Kind::Death,
            Element::Earth => Kind::Earth,
            Element::Energy => Kind::Energy,
            Element::Fire => Kind::Fire,
            Element::Ice => Kind::Ice,
        };
        let elemental = match effects.protections.get(&kind) {
            Some(sum) => apply_protection(elemental, *sum)?,
            None => elemental,
        };
        runtime.player_control_facts(hit.target, session).ok()?;
        physical.checked_add(elemental)
    }
}

#[cfg(test)]
mod tests {
    use super::swing_lands;

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
