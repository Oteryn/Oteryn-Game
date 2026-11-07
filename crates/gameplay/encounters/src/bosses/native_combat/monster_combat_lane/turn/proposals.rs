//! Scheduled summon, defense and attack phases.

use super::*;

pub(super) fn dispatch_summons<S: MonsterCombatStates, R: MonsterCombatReader>(
    owners: &mut ThinkOwners<'_>,
    input: &mut ThinkInput<'_, S, R>,
    a: &mut NativeActor,
    plan: &SchedulePlan,
    result: &mut ThinkCombatResult,
) {
    let stamp = input.stamp;
    let due = input.due;
    let runtime = &mut *input.runtime;
    let fence = input.fence;
    let reader = &mut *input.reader;
    let summons = &mut *owners.summons;
    let child_closure = owners.child_closure;
    let new_children = &mut *owners.new_children;
    if let Some(catalog) = &a.summons {
        for proposal in &plan.summon_proposals {
            if child_closure.as_ref().is_none_or(|c| {
                c.content != a.content || !c.children.iter().any(|(r, _)| *r == proposal.creature)
            }) {
                result.summons.push((
                    proposal.creature.clone(),
                    Err(crate::monster_summon::SummonError::InvalidSource),
                ));
                continue;
            }
            let outcome = summons.execute(
                runtime,
                fence,
                stamp,
                catalog,
                proposal,
                &mut SummonMapReader(reader),
            );
            if let Ok(child) = &outcome
                && child.newly_created
            {
                new_children.push((child.child, proposal.creature.clone(), due, stamp));
            }
            result.summons.push((proposal.creature.clone(), outcome));
        }
    } else {
        result.deferred_summons = plan.summon_proposals.len();
    }
}

pub(super) fn dispatch_proposal<S: MonsterCombatStates, R: MonsterCombatReader>(
    owners: &mut ThinkOwners<'_>,
    input: &mut ThinkInput<'_, S, R>,
    a: &mut NativeActor,
    proposal: ProfileAbilityProposal,
    root: &GameplayDecisionRoot,
    result: &mut ThinkCombatResult,
) {
    let stamp = input.stamp;
    let occurrence = input.occurrence;
    let due = input.due;
    let runtime = &mut *input.runtime;
    let states = &mut *input.states;
    let fence = input.fence;
    let reader = &mut *input.reader;
    let melee = &mut *owners.melee;
    let spells = &mut *owners.spells;
    let icicle_owner = &mut *owners.icicle_owner;
    let defense_summon_owner = &mut *owners.defense_summon_owner;
    let time_guardian_owner = &mut *owners.time_guardian_owner;
    let callback_spawn_owner = &mut *owners.callback_spawn_owner;
    let child_closure = owners.child_closure;
    let native_appearance_content = owners.native_appearance_content;
    let new_children = &mut *owners.new_children;
    if let Some(source) = a
        .seven_sources
        .iter()
        .find(|s| s.ability() == proposal.ability)
        .cloned()
    {
        if matches!(source.kind(),oteryn_gameplay_encounters::bosses::seven_encounters::source_encounter_seven::SevenKind::TimeGuardian|oteryn_gameplay_encounters::bosses::seven_encounters::source_encounter_seven::SevenKind::TimeGuardiann) {
            let hit=a.dispatch_time_phase(runtime,fence,stamp,time_guardian_owner,&source,&proposal,due);
            result.time_phases.push((proposal.ability,hit));
        } else {
            // Descriptive pending work ONLY: no quest-coordinate relocation grant exists.
            result.seven_map_pending.push((proposal.ability.clone(),source.prepare_map_work(runtime,fence,stamp,&proposal)));
        }
        return;
    }

    if let Some(source) = a
        .callback_spawn_sources
        .iter()
        .find(|s| s.ability() == &proposal.ability)
    {
        let admitted = callback_spawn_owner.schedule(runtime, fence, stamp, source, &proposal, due);
        result.callback_spawns.push((proposal.ability, admitted));
        return;
    }
    if let Some(native) = native_appearance_content.as_deref().filter(|n| {
        n.source_item_ability(&a.creature, &proposal.ability)
            .is_some()
    }) {
        let item = crate::creature_source_items::ScheduledCreatureSourceItems::prepare(
            runtime,
            fence,
            stamp,
            native,
            &a.creature,
            &proposal,
        );
        result.source_items.push((proposal.ability.clone(), item));
        return;
    }
    if proposal.list != ScheduleList::Attack {
        if let Some(source) = a
            .icicle
            .as_ref()
            .filter(|s| s.ability == proposal.ability && s.index == proposal.entry_index)
        {
            let hit = icicle_owner.execute(
                runtime,
                fence,
                stamp,
                source,
                &proposal,
                melee,
                &mut IcicleMapReader {
                    inner: reader,
                    ability: &source.ability,
                },
                due,
            );
            if hit.as_ref().is_ok_and(|h| h.cleared_target) {
                a.selected_target = None;
            }
            result.defenses.push((
                proposal.ability,
                hit.map(DefenseOutcome::Icicle)
                    .map_err(DefenseError::Icicle),
            ));
            return;
        }
        if let Some(source) = a
            .defense_variants
            .iter()
            .find(|s| s.ability == proposal.ability)
        {
            let hit = spells.execute_with_native(
                runtime,
                fence,
                stamp,
                states,
                source,
                &proposal,
                reader,
                due,
                native_appearance_content.as_deref(),
            );
            result.defense_variants.push((proposal.ability, hit));
            return;
        }
        if let Some(source) = a
            .defense_summons
            .iter()
            .find(|s| s.index == proposal.entry_index)
        {
            let hit = if child_closure.as_ref().is_some_and(|c| {
                c.content == a.content && c.children.iter().any(|(r, _)| r == source.child())
            }) {
                defense_summon_owner.execute(
                    runtime,
                    fence,
                    stamp,
                    source,
                    occurrence,
                    &proposal,
                    &mut SummonMapReader(reader),
                )
            } else {
                Err(crate::monster_summon::SummonError::InvalidSource)
            };
            if let Ok(batch) = &hit {
                for child in &batch.children {
                    if child.newly_created {
                        new_children.push((child.child, source.child().clone(), due, stamp));
                    }
                }
            }
            result.defense_summons.push((proposal.ability, hit));
            return;
        }
        dispatch_self_defense(owners, input, a, proposal, root, result);
        return;
    }
    dispatch_attack(owners, input, a, proposal, result);
}

fn dispatch_self_defense<S: MonsterCombatStates, R: MonsterCombatReader>(
    owners: &mut ThinkOwners<'_>,
    input: &mut ThinkInput<'_, S, R>,
    a: &mut NativeActor,
    proposal: ProfileAbilityProposal,
    root: &GameplayDecisionRoot,
    result: &mut ThinkCombatResult,
) {
    let stamp = input.stamp;
    let occurrence = input.occurrence;
    let due = input.due;
    let runtime = &mut *input.runtime;
    let states = &mut *input.states;
    let fence = input.fence;
    let reader = &mut *input.reader;
    let spells = &mut *owners.spells;
    let heals = &mut *owners.heals;
    let area_owner = &mut *owners.area_owner;
    let threshold_heal_owner = &mut *owners.threshold_heal_owner;
    let native_appearance_content = owners.native_appearance_content;
    let area_catalog = owners.area_catalog;
    let hash = Sha256::new()
        .chain_update(b"oteryn:native-self-defense:v1")
        .chain_update(proposal.occurrence.id().as_str().as_bytes())
        .finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&hash[..16]);
    let id = DecisionOccurrenceId::from_bytes(id);
    if let Some(source) = a
        .threshold_heals
        .iter()
        .find(|s| s.ability() == &proposal.ability)
    {
        let hit = threshold_heal_owner
            .cast(runtime, fence, stamp, source, &proposal, due.get())
            .map(DefenseOutcome::ThresholdHeal)
            .map_err(DefenseError::ThresholdHeal);
        result.defenses.push((proposal.ability, hit));
    } else if let Some(source) = a
        .heals
        .iter()
        .find(|s| s.index == proposal.entry_index && s.ability == proposal.ability)
    {
        let (minimum, maximum) = source.source.magnitude_range();
        let hit = deterministic_decision_u64(
            root,
            id,
            "self_heal_magnitude",
            proposal.entry_index as u64,
        )
        .map_err(|_| DefenseError::InvalidDraw)
        .and_then(|draw| {
            let draw = crate::spell::uniform_draw(draw, minimum as i64, maximum as i64);
            (if source.source.removes_paralysis() {
                states.commit_self_heal_and_paralysis_removal(
                    runtime,
                    heals,
                    &source.registration,
                    a.actor,
                    occurrence.sequence,
                    draw,
                    due.get(),
                    fence,
                    stamp,
                )
            } else {
                runtime.commit_creature_self_heal(
                    heals,
                    &source.registration,
                    a.actor,
                    occurrence.sequence,
                    draw,
                )
            })
            .map(DefenseOutcome::SelfHeal)
            .map_err(DefenseError::Carrier)
        });
        result.defenses.push((proposal.ability, hit));
    } else if let Some((_, source)) = a
        .area_heals
        .iter()
        .find(|(i, _)| *i == proposal.entry_index)
    {
        let hit = match area_catalog.as_ref() {
            Some(catalog) => {
                let mut policy = AreaHealMapReader {
                    inner: reader,
                    ability: &proposal.ability,
                };
                area_owner
                    .execute(
                        runtime,
                        fence,
                        stamp,
                        catalog,
                        source,
                        &proposal,
                        &mut policy,
                        states,
                        due.get(),
                    )
                    .map(DefenseOutcome::AreaHeal)
                    .map_err(DefenseError::AreaHeal)
            }
            None => Err(DefenseError::AreaHeal(AreaHealError::InvalidSource)),
        };
        result.defenses.push((proposal.ability, hit));
    } else if let Some((_, source)) = a
        .appearance_defenses
        .iter()
        .find(|(i, _)| *i == proposal.entry_index)
    {
        let hit = spells.execute_with_native(
            runtime,
            fence,
            stamp,
            states,
            source,
            &proposal,
            reader,
            due,
            native_appearance_content.as_deref(),
        );
        result.defense_variants.push((proposal.ability, hit));
    } else if let Some((_, source)) = a.invisible.iter().find(|(i, _)| *i == proposal.entry_index) {
        let config = reader.self_condition_facts(runtime, a.actor, stamp);
        let facts = config.map(|c| ApplicationFacts {
            now: due.get(),
            base_speed: c.base_speed,
            mana_shield_capacity: c.mana_shield_capacity,
            target_reentry_protected: c.target_reentry_protected,
            source_reentry_protected: c.source_reentry_protected,
            target_is_player: false,
            decision_root: root,
            occurrence: id,
        });
        let hit = match facts {
            Some(facts) => commit_native_defense_condition(
                runtime,
                states,
                fence,
                stamp,
                a.actor,
                proposal.entry_index,
                occurrence.sequence,
                NativeSelfConditionSource::Invisible(source),
                facts,
                &mut a.condition_casts,
                reader,
            )
            .map(DefenseOutcome::Invisible),
            None => Err(DefenseError::Invisible(
                InvisibilityError::MissingApplicationFacts,
            )),
        };
        result.defenses.push((proposal.ability, hit));
    } else if let Some((_, source)) = a.speed.iter().find(|(i, _)| *i == proposal.entry_index) {
        let config = reader.self_condition_facts(runtime, a.actor, stamp);
        let facts = config.map(|c| ApplicationFacts {
            now: due.get(),
            base_speed: source.base_speed(),
            mana_shield_capacity: c.mana_shield_capacity,
            target_reentry_protected: c.target_reentry_protected,
            source_reentry_protected: c.source_reentry_protected,
            target_is_player: false,
            decision_root: root,
            occurrence: id,
        });
        let hit = match facts {
            Some(facts) => commit_native_defense_condition(
                runtime,
                states,
                fence,
                stamp,
                a.actor,
                proposal.entry_index,
                occurrence.sequence,
                NativeSelfConditionSource::Speed(source),
                facts,
                &mut a.condition_casts,
                reader,
            )
            .map(DefenseOutcome::Speed),
            None => Err(DefenseError::Speed(
                InvisibilityError::MissingApplicationFacts,
            )),
        };
        result.defenses.push((proposal.ability, hit));
    } else if let Some((_, source)) = a
        .presentations
        .iter()
        .find(|(i, _)| *i == proposal.entry_index)
    {
        let event = source
            .produce(
                runtime,
                fence,
                stamp,
                &proposal,
                &mut crate::creature_damage_spell::SpellPresentationReader(reader),
            )
            .map(DefenseOutcome::Presentation)
            .map_err(DefenseError::Presentation);
        result.defenses.push((proposal.ability, event));
    } else {
        result.deferred_defence.push(proposal.ability)
    }
}

fn dispatch_attack<S: MonsterCombatStates, R: MonsterCombatReader>(
    owners: &mut ThinkOwners<'_>,
    input: &mut ThinkInput<'_, S, R>,
    a: &mut NativeActor,
    proposal: ProfileAbilityProposal,
    result: &mut ThinkCombatResult,
) {
    let stamp = input.stamp;
    let due = input.due;
    let runtime = &mut *input.runtime;
    let states = &mut *input.states;
    let fence = input.fence;
    let reader = &mut *input.reader;
    let spells = &mut *owners.spells;
    let composed = &mut *owners.composed;
    let chains = &mut *owners.chains;
    let callback_casts = &mut *owners.callback_casts;
    let smelly = &mut *owners.smelly;
    let native_appearance_content = owners.native_appearance_content;
    let area_catalog = owners.area_catalog;
    if let Some(source) = a
        .callback_casts
        .iter()
        .find(|s| s.parent() == &proposal.ability)
    {
        let hit = callback_casts.schedule(runtime, fence, stamp, source, &proposal, due);
        result.callback_casts.push((proposal.ability, hit));
        return;
    }
    if let Some(source) = a
        .smelly_source
        .as_ref()
        .filter(|s| s.ability() == &proposal.ability)
    {
        let hit = smelly.schedule(runtime, fence, stamp, source, &proposal, due);
        result.smelly_scheduled.push((proposal.ability, hit));
        return;
    }
    if let Some((_, source)) = a.composed.iter().find(|(i, _)| *i == proposal.entry_index) {
        let hit = match area_catalog.as_ref() {
            Some(catalog) => composed.execute(
                runtime,
                fence,
                stamp,
                states,
                catalog,
                source,
                &proposal,
                &mut ComposedReader(reader),
                due,
            ),
            None => Err(AttackError::InvalidSource),
        };
        result.composed.push((proposal.ability, hit));
        return;
    }
    if let Some((_, source)) = a
        .spells
        .iter()
        .find(|(i, source)| *i == proposal.entry_index && source.chain_details().is_some())
    {
        let shared = std::cell::RefCell::new(&mut *reader);
        let mut world = ChainSpellReader(&shared);
        let mut map = ChainMapReader(&shared);
        let hit = chains.schedule(
            runtime, fence, stamp, source, &proposal, &mut world, &mut map, due,
        );
        result.chains.push((proposal.ability, hit));
        return;
    }
    let hit = match a.spells.iter().find(|(i, _)| *i == proposal.entry_index) {
        Some((_, source)) => spells.execute_with_native(
            runtime,
            fence,
            stamp,
            states,
            source,
            &proposal,
            reader,
            due,
            native_appearance_content.as_deref(),
        ),
        None => Err(AttackError::UnsupportedShape),
    };
    result.spells.push((proposal.ability, hit));
}
