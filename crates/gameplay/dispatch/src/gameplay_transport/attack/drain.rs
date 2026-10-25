//! Ordered phases of the Channel owner's auto-attack pass.

use super::shared_experience::aggressive_monster;
use super::skill_training::{TrainingDue, swing_block, swing_family, train_hit};
use super::*;
use crate::ability::exact_actor_resolution::{ExactActorProposal, resolve_exact_actor};
use crate::combat_attack::{SwingPoll, player_fist_formula, player_melee_formula};
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::gameplay_transport::kill_reward::KillSettlementQueue;
use crate::spell::cast::{CharacterCastFacts, SwingSkills};
use crate::spell::combat_execution::actor_atom;
use crate::spell::formula::FormulaInputs;
use oteryn_gameplay_combat_rules::attack::target::Swing;
use oteryn_gameplay_combat_rules::resolution::{
    creature_physical_response, creature_physically_immune,
};
use oteryn_gameplay_progression::charm_effects::{
    CharmAttackerFacts, CharmHitSource, CharmHookEvent,
};

struct SelectedSwing {
    target: ExactActorRef,
    character_facts: CharacterCastFacts,
    skills: SwingSkills,
    weapon: WeaponFacts,
    swing: Swing<ExactActorRef, CommandRef>,
    ordinal: u16,
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
        for entry in entries.iter_mut() {
            let Some(selected) =
                select_swing(entry, &runtime, &states, room, digest, now, constants)
            else {
                continue;
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
                entry.clear_target();
                continue;
            };
            let character = *lease.character_id().as_bytes();
            let Some((damage, revision)) =
                draw_swing_damage(entry, &runtime, &selected, constants, &root)
            else {
                continue;
            };
            if !runtime.contains_live_creature(target) {
                let Some(black) = self
                    .pvp_target_legal(
                        &pvp_permit,
                        &runtime,
                        &states,
                        room,
                        &objects,
                        entry.actor,
                        target,
                        swing.lineage,
                        entry.modes.secure,
                    )
                    .await
                else {
                    entry.clear_target();
                    continue;
                };
                let Ok(raw) = u32::try_from(damage) else {
                    entry.clear_target();
                    continue;
                };
                let magnitude =
                    i64::from(oteryn_gameplay_combat_rules::pvp::pvp_damage(raw, black));
                let atom = actor_atom(target);
                let Some(plan) = swing_plan(
                    entry.actor,
                    swing.lineage,
                    ordinal,
                    magnitude,
                    &digest,
                    &atom,
                    revision,
                ) else {
                    continue;
                };
                let batch = crate::spell::combat_batch::OwnerCombatBatch {
                    caster: entry.actor,
                    attacker: lease.character_id(),
                    current_lease_generation: lease.generation(),
                    command: swing.lineage,
                    occurrence: plan.occurrence().clone().into(),
                    binding: b"oteryn:pvp-melee:v1".to_vec(),
                    anchor: None,
                    now_us: Some(now),
                    now_ms: now.get() / 1000,
                    effects: vec![crate::spell::combat_batch::OwnerCombatEffect {
                        target,
                        sub_ordinal: ordinal,
                        change: crate::spell::combat_batch::OwnerCombatChange::Damage {
                            target_atom: atom,
                            magnitude,
                        },
                    }],
                    deferred: None,
                };
                if !self
                    .pvp_swing(
                        &mut pvp_permit,
                        &mut runtime,
                        &mut states,
                        room,
                        &objects,
                        batch,
                        ordinal,
                        entry.modes.secure,
                        black,
                    )
                    .await
                {
                    entry.clear_target();
                }
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
            if creature_physically_immune(&snapshot.state.policy) {
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
            let blocked = blocks[index].1.try_block(&constants.block, swing.at);
            let Some(magnitude) = creature_mitigated(
                &root,
                constants,
                entry.actor,
                swing.lineage,
                ordinal,
                damage,
                blocked,
                defense,
                armor,
            ) else {
                continue;
            };
            // CREATURE-RESIST-MELEE-1: the creature's mitigation and physical response shape
            // what passed its defence and armor (`Creature::mitigateDamage`, `Monster::blockHit`).
            // Training already counted the swing by its block, so it trains by `magnitude`.
            let landed = if magnitude > 0 {
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
                    selected.weapon.weapon,
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
                selected.weapon.weapon,
                magnitude,
                now,
                &mut trained,
            );
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
        drop(attack);
        drop(objects);
        drop(states);
        drop(runtime);
        drop(pvp_permit);
        self.save_trained_levels(trained).await;
    }
}

/// SKILL-TRAIN-1: the swing at a creature trains the swing's skill by how its block met it.
#[allow(clippy::too_many_arguments)]
fn train_swing(
    states: &mut ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    content: &NativeGameplayState,
    entry: &ActorAttack,
    weapon: SwingWeapon,
    magnitude: i64,
    now: SemanticTimeMicros,
    trained: &mut TrainingDue,
) {
    let block = swing_block(magnitude);
    train_hit(
        states,
        runtime,
        content.training_formula(),
        entry.actor,
        entry.session,
        swing_family(weapon),
        now.get(),
        |hits| hits.melee_hit(block),
        trained,
    );
}

fn select_swing(
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
    // SPELL-LOCK-2 §1.3: while the attacker's own slot is reserved by a pending spell
    // batch, its swing defers to a later turn; the held target and swing clock stay.
    if states.has_pending_spell_commit(entry.actor, entry.session)
        || runtime.assert_actor_spell_unreserved(entry.actor).is_err()
    {
        return None;
    }
    // ATTACK-2: a swing waits for a swing weapon read under this pin within one attack
    // interval; the attacker's own connection refreshes it (`refresh_attack_weapon`).
    let weapon = entry.weapon.filter(|facts| {
        facts.content_digest == digest
            && now
                .elapsed_since(facts.read_at)
                .is_ok_and(|age| age <= constants.attack_interval_micros())
    })?;
    let character_facts = player.character_facts();
    // MELEE-SKILL-1: the swing reads the attacker's live skill of the swung weapon (fist for
    // fists) and live shielding, before the swing is taken, so an unresolved skill takes none.
    let attack_index = match weapon.weapon {
        SwingWeapon::Fists => 0,
        SwingWeapon::Melee { skill, .. } => skill.index(),
    };
    let skills = player
        .swing_skills(attack_index, now.get())?
        .unwrap_or(SwingSkills {
            attack: constants.starting_skill,
            shielding: constants.starting_skill,
        });
    let facts = target_facts(runtime, room, entry.actor, entry.session, target, now)?;
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
    let mode = constants.fight_mode(fight_mode(entry.modes.fight_mode));
    let (formula, attack_value, revision) = match selected.weapon.weapon {
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
    swing_lands(damage, creature).then_some((damage, revision))
}

/// `Game::combatBlockHit`: a 0 swing at a creature still meets its defence and armor (it spends
/// a block and trains as a blocked hit). A PvP swing does not meet the target's defence yet, so
/// a 0 swing at a player stays dropped.
const fn swing_lands(damage: i64, creature: bool) -> bool {
    damage > 0 || (damage == 0 && creature)
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

#[cfg(test)]
mod tests {
    use super::swing_lands;

    #[test]
    fn a_zero_swing_meets_a_creature_but_not_a_player() {
        assert!(swing_lands(0, true), "a 0 swing at a creature is blocked");
        assert!(
            !swing_lands(0, false),
            "a 0 swing at a player stays dropped"
        );
        assert!(swing_lands(5, true));
        assert!(swing_lands(5, false));
        assert!(!swing_lands(-1, true));
    }
}
