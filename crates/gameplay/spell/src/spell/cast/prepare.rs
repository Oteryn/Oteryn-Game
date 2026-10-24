//! Common payment and effect preparation for the actual cast owners.

use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use oteryn_simulation_determinism::SemanticTimeMicros;

use super::PlayerSpellState;
use super::gate::{CastContext, admit_condition_gates, cast_resolved, disposition};
use crate::ability::Effect;
use crate::spell::harmony::HarmonyMultiplier;
use crate::spell::{CasterState, HarmonyRole, SpellBook, SpellDefinition};

#[derive(Debug)]
pub struct PaidSourceSelfCast {
    pub ability_variant: Option<crate::spell::executable_catalog::DefinitionRef>,
    pub next: PlayerSpellState,
    pub anchor: crate::spell::combat_batch::SpellAnchor,
}
#[derive(Debug)]
pub struct PaidFocusOwnerCast {
    pub next: PlayerSpellState,
    pub anchor: crate::spell::combat_batch::SpellAnchor,
    pub plan: oteryn_gameplay_spell_rules::native_actor_states::FocusPlan,
}
/// Fill/Serenity/cooldowns on the genuine caster successor. Healing remains
/// a typed source roll for the real party compositor to target and draw once.
/// This helper performs no healing draw and introduces no solo-party fact.
pub fn prepare_focus_owner_cast_with_caster(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    book: &SpellBook,
    operational: &crate::spell::OperationalCastFacts,
    qualified_caster: &CasterState,
    context: CastContext<'_>,
) -> Result<PaidFocusOwnerCast, SpellCastDisposition> {
    admit_condition_gates(state, spell, context.now)?;
    let crate::spell::Execution::NativeProfile(profile) = &spell.execution else {
        return Err(SpellCastDisposition::Rejected);
    };
    if profile.spell()["execution"]["native_behavior"]["key"] != "monk_focus"
        || !book.spells.iter().any(|active| active == spell)
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let monk = state.monk.as_ref().ok_or(SpellCastDisposition::Rejected)?;
    monk.accept_command()
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let facts = oteryn_gameplay_spell_rules::native_actor_states::FocusFacts {
        harmony: monk.harmony(),
        serene: monk.serene(),
        forced_until: monk.serene_forced_until(),
        now: context.now,
        level: state.facts.level,
        sustain_active: state.stance
            == Some(oteryn_gameplay_spell_rules::native_actor_states::StandardStance::Sustain),
    };
    let mut paid = prepare_native_owner_cast_with_caster(
        state,
        spell,
        operational,
        crate::spell::native::Facts::Focus(facts),
        context.now,
        context.draw,
        qualified_caster,
    )?;
    let crate::spell::native::Plan::Focus(plan) = paid.plan else {
        return Err(SpellCastDisposition::Rejected);
    };
    let mut next_monk = paid
        .next
        .monk
        .as_ref()
        .ok_or(SpellCastDisposition::Rejected)?
        .clone();
    let gained = next_monk
        .commit_fill()
        .map_err(|_| SpellCastDisposition::Rejected)?;
    if gained != plan.gained_charges || next_monk.harmony() != plan.harmony_after {
        return Err(SpellCastDisposition::Rejected);
    }
    if plan.arm_forced_serene {
        next_monk
            .commit_focus_serenity(context.now)
            .map_err(|_| SpellCastDisposition::Rejected)?;
    }
    if next_monk.serene() != plan.serene_after
        || next_monk.serene_forced_until() != plan.forced_until
    {
        return Err(SpellCastDisposition::Rejected);
    }
    paid.next.monk = Some(next_monk);
    paid.next.cooldowns.reset_for_focus(book, &plan);
    if plan.rearm_cast_cooldowns {
        paid.next
            .cooldowns
            .rearm(spell, context.now)
            .map_err(|reason| disposition(&reason))?;
    }
    paid.anchor.cooldown_deadlines = paid.next.cooldowns.canonical_deadlines();
    Ok(PaidFocusOwnerCast {
        next: paid.next,
        anchor: paid.anchor,
        plan: *plan,
    })
}
/// Source self effects and their real anchor are prepared on one exact actor successor.
pub fn prepare_source_self_cast_with_caster(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    book: &SpellBook,
    operational: &crate::spell::OperationalCastFacts,
    qualified_caster: &CasterState,
    context: CastContext<'_>,
) -> Result<PaidSourceSelfCast, SpellCastDisposition> {
    admit_condition_gates(state, spell, context.now)?;
    if let crate::spell::Execution::NativeProfile(profile) = &spell.execution {
        if profile.spell()["execution"]["native_behavior"]["key"] != "monk_focus"
            || !book.spells.iter().any(|active| active == spell)
        {
            return Err(SpellCastDisposition::Rejected);
        }
        let monk = state.monk.as_ref().ok_or(SpellCastDisposition::Rejected)?;
        monk.accept_command()
            .map_err(|_| SpellCastDisposition::Rejected)?;
        let facts = oteryn_gameplay_spell_rules::native_actor_states::FocusFacts {
            harmony: monk.harmony(),
            serene: monk.serene(),
            forced_until: monk.serene_forced_until(),
            now: context.now,
            level: state.facts.level,
            sustain_active: state.stance
                == Some(oteryn_gameplay_spell_rules::native_actor_states::StandardStance::Sustain),
        };
        let mut paid = prepare_native_owner_cast_with_caster(
            state,
            spell,
            operational,
            crate::spell::native::Facts::Focus(facts),
            context.now,
            context.draw,
            qualified_caster,
        )?;
        // Source chooses the lowest absolute HP among self and visible party.
        // This route has no genuine membership/no-party owner proof, so refuse
        // a gained-charge heal before the first draw or any owner mutation.
        let crate::spell::native::Plan::Focus(planned) = &paid.plan else {
            return Err(SpellCastDisposition::Rejected);
        };
        if planned.healing.is_some() {
            return Err(SpellCastDisposition::TargetIllegal);
        }
        let focus = crate::spell::actor_execution::apply_focus(
            &mut paid.next,
            &profile.spell()["execution"]["native_behavior"]["parameters"],
            context.now,
            context.draw,
        )?;
        paid.next.cooldowns.reset_for_focus(book, &focus);
        if focus.rearm_cast_cooldowns {
            paid.next
                .cooldowns
                .rearm(spell, context.now)
                .map_err(|r| disposition(&r))?;
        }
        paid.anchor.cooldown_deadlines = paid.next.cooldowns.canonical_deadlines();
        return Ok(PaidSourceSelfCast {
            next: paid.next,
            anchor: paid.anchor,
            ability_variant: None,
        });
    }
    if state.pending_stance.is_some()
        || state.training_checkpoint.is_some()
        || spell.needs_target
        || spell.target_or_direction
        || spell.chain.is_some()
        || operational.target.is_some()
        || operational.target_position.is_some()
        || !book.spells.iter().any(|active| active == spell)
        || !matches!(
            spell.execution,
            crate::spell::Execution::Effects(_)
                | crate::spell::Execution::AbilityVariants(_)
                | crate::spell::Execution::ActorFocus { .. }
        )
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let (next, ability_variant) = cast_resolved(
        book,
        state,
        spell,
        SpellTarget::None,
        context,
        Some((qualified_caster, operational)),
    )?;
    let anchor = crate::spell::combat_batch::SpellAnchor {
        expected_revision: state.revision,
        next_revision: next.revision,
        paid_mana: state
            .mana
            .checked_sub(next.mana)
            .ok_or(SpellCastDisposition::Rejected)?,
        paid_soul: state
            .soul
            .checked_sub(next.soul)
            .ok_or(SpellCastDisposition::Rejected)?,
        cooldown_deadlines: next.cooldowns.canonical_deadlines(),
    };
    Ok(PaidSourceSelfCast {
        next,
        anchor,
        ability_variant,
    })
}

/// An ordinary source cast pays only its common anchor here. Every resolved
/// effect is retained for the actual target/condition/presentation owner batch.
#[derive(Debug)]
pub struct PaidOrdinaryCast {
    pub next: PlayerSpellState,
    pub anchor: crate::spell::combat_batch::SpellAnchor,
    pub resolution: crate::spell::CastResolution,
    pub harmony_gained: u8,
    pub harmony_gain_healing:
        Option<oteryn_gameplay_spell_rules::native_actor_states::HealingRollPlan>,
}
#[allow(clippy::too_many_arguments)]
pub fn prepare_ordinary_owner_cast_with_caster(
    book: &SpellBook,
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &crate::spell::OperationalCastFacts,
    now: SemanticTimeMicros,
    caster: &CasterState,
    party: Option<&dyn crate::spell::party::PartyWorld>,
    chain: Option<(
        &dyn crate::spell::chain::ChainWorld,
        crate::spell::chain::ChainStart,
    )>,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<PaidOrdinaryCast, SpellCastDisposition> {
    admit_condition_gates(state, spell, now)?;
    if state.pending_stance.is_some()
        || state.training_checkpoint.is_some()
        || !book.spells.contains(spell)
        || !matches!(
            spell.execution,
            crate::spell::Execution::Effects(_)
                | crate::spell::Execution::AbilityVariants(_)
                | crate::spell::Execution::PartyBuff(_)
        )
        || caster.vocation != state.facts.vocation
        || caster.level != state.facts.level
        || caster.magic_level != state.owned_effective_magic_level(now.get())?
        || caster.mana != state.mana
        || caster.max_mana != state.facts.max_mana
        || caster.soul != state.soul
        || caster.harmony_multiplier != state.owned_harmony_multiplier(spell)?
    {
        return Err(SpellCastDisposition::Rejected);
    }
    crate::spell::check_operational_rules(spell, operational, false)
        .map_err(|e| disposition(&e))?;
    if spell.authored.is_some() && spell.key == "candidate:spell/cancel_magic_shield" {
        crate::spell::actor_conditions::require_active_mana_shield(&state.conditions, now.get())?;
    }
    let resolution = if matches!(spell.execution, crate::spell::Execution::PartyBuff(_)) {
        crate::spell::resolve_party_cast(
            spell,
            caster,
            &state.cooldowns,
            now,
            party.ok_or(SpellCastDisposition::TargetRequired)?,
            draw,
        )
    } else if spell.chains(operational.target.is_some()) {
        let (world, start) = chain.ok_or(SpellCastDisposition::TargetRequired)?;
        crate::spell::resolve_chain_cast(
            spell,
            caster,
            &state.cooldowns,
            now,
            world,
            start,
            &mut |minimum, _| minimum,
        )
    } else {
        crate::spell::resolve_cast_with_operational_facts(
            spell,
            caster,
            &state.cooldowns,
            now,
            operational,
            draw,
        )
    }
    .map_err(|e| disposition(&e))?;
    let mut next = state.clone();
    next.mana = next
        .mana
        .checked_sub(resolution.mana_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.soul = next
        .soul
        .checked_sub(resolution.soul_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.cooldowns = resolution.cooldowns.clone();
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or(SpellCastDisposition::Rejected)?;
    let mut harmony_gained = 0;
    let mut harmony_gain_healing = None;
    if let Some(role) = spell.harmony_role {
        let monk = next.monk.as_mut().ok_or(SpellCastDisposition::Rejected)?;
        match role {
            HarmonyRole::Builder => {
                harmony_gained = monk
                    .commit_builder()
                    .map_err(|_| SpellCastDisposition::Rejected)?;
                if harmony_gained > 0 {
                    harmony_gain_healing = oteryn_gameplay_spell_rules::native_actor_states::plan_harmony_gain_healing(
                        crate::spell::actor_execution::harmony_gain_profile()?,
                        harmony_gained,
                        next.facts.level,
                        monk.serene(),
                        next.stance == Some(oteryn_gameplay_spell_rules::native_actor_states::StandardStance::Sustain),
                    )
                    .map_err(|_| SpellCastDisposition::Rejected)?;
                }
            }
            HarmonyRole::Spender => {
                monk.commit_spender()
                    .map_err(|_| SpellCastDisposition::Rejected)?;
            }
        }
    }
    let anchor = crate::spell::combat_batch::SpellAnchor {
        expected_revision: state.revision,
        next_revision: next.revision,
        paid_mana: resolution.mana_spent,
        paid_soul: resolution.soul_spent,
        cooldown_deadlines: next.cooldowns.canonical_deadlines(),
    };
    Ok(PaidOrdinaryCast {
        next,
        anchor,
        resolution,
        harmony_gained,
        harmony_gain_healing,
    })
}
/// Source chain callbacks obtain a fresh numerical cast from actual due owner
/// facts. This does not recheck or pay cooldowns, mana, soul or Harmony receipts.
pub fn resolve_ordinary_due_effects(
    spell: &SpellDefinition,
    caster: &CasterState,
    step: u32,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<Vec<crate::spell::ResolvedEffect>, SpellCastDisposition> {
    let crate::spell::Execution::Effects(effects) = &spell.execution else {
        return Err(SpellCastDisposition::Rejected);
    };
    let chain = spell.chain.as_ref().ok_or(SpellCastDisposition::Rejected)?;
    if spell.authored.is_none() || step > chain.max_targets {
        return Err(SpellCastDisposition::Rejected);
    }
    let inputs = crate::spell::formula::FormulaInputs {
        level: caster.level,
        magic_level: caster.magic_level,
        base_power: spell.base_power,
        attack_skill: caster.attack_skill,
        attack_value: caster.attack_value,
        attack_factor: caster.attack_factor,
        shielding_skill: caster.shielding_skill,
        shield_defense: caster.shield_defense,
    };
    effects
        .iter()
        .map(|effect| {
            crate::spell::resolve_effect(effect, &inputs, caster.harmony_multiplier, draw)
                .map(|effect| crate::spell::scale(effect, step, chain.damage_step_percent))
                .map_err(|_| SpellCastDisposition::Rejected)
        })
        .collect()
}

/// A prepared native cast has no authority to mutate either owner. The transport compositor
/// joins this exact anchor to a physical/world/durable owner transaction and compares the
/// actual player predecessor again before installing `next`.
#[derive(Debug)]
pub struct PaidNativeCast {
    pub next: PlayerSpellState,
    pub anchor: crate::spell::combat_batch::SpellAnchor,
    pub plan: crate::spell::native::Plan,
}

pub fn prepare_native_owner_cast(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &crate::spell::OperationalCastFacts,
    facts: crate::spell::native::Facts<'_>,
    now: SemanticTimeMicros,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<PaidNativeCast, SpellCastDisposition> {
    prepare_native_owner_cast_impl(state, spell, operational, facts, now, draw, None)
}

pub fn prepare_native_owner_cast_with_caster(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &crate::spell::OperationalCastFacts,
    facts: crate::spell::native::Facts<'_>,
    now: SemanticTimeMicros,
    draw: &mut dyn FnMut(i64, i64) -> i64,
    caster: &CasterState,
) -> Result<PaidNativeCast, SpellCastDisposition> {
    prepare_native_owner_cast_impl(state, spell, operational, facts, now, draw, Some(caster))
}
fn prepare_native_owner_cast_impl(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &crate::spell::OperationalCastFacts,
    facts: crate::spell::native::Facts<'_>,
    now: SemanticTimeMicros,
    draw: &mut dyn FnMut(i64, i64) -> i64,
    qualified_caster: Option<&CasterState>,
) -> Result<PaidNativeCast, SpellCastDisposition> {
    admit_condition_gates(state, spell, now)?;
    if state.pending_stance.is_some() || state.training_checkpoint.is_some() {
        return Err(SpellCastDisposition::Rejected);
    }
    let multiplier = match &state.monk {
        Some(monk) => {
            monk.accept_command()
                .map_err(|_| SpellCastDisposition::Rejected)?;
            if spell.harmony_role == Some(HarmonyRole::Spender) {
                monk.spender_multiplier(
                    state.facts.level,
                    state.stance == Some(oteryn_gameplay_spell_rules::native_actor_states::StandardStance::Harmony),
                )
                .map_err(|_| SpellCastDisposition::Rejected)?
            } else {
                HarmonyMultiplier::ONE
            }
        }
        None => HarmonyMultiplier::ONE,
    };
    let mut baseline = state.caster(multiplier);
    baseline.magic_level = state.owned_effective_magic_level(now.get())?;
    let caster = qualified_caster.unwrap_or(&baseline);
    if caster.vocation != state.facts.vocation
        || caster.level != state.facts.level
        || caster.magic_level != state.owned_effective_magic_level(now.get())?
        || caster.mana != state.mana
        || caster.max_mana != state.facts.max_mana
        || caster.soul != state.soul
        || caster.harmony_multiplier != multiplier
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let resolved = crate::spell::resolve_native_cast(
        spell,
        caster,
        &state.cooldowns,
        now,
        operational,
        facts,
        draw,
    )
    .map_err(|error| disposition(&error))?;
    let mut next = state.clone();
    next.mana = next
        .mana
        .checked_sub(resolved.common.mana_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.soul = next
        .soul
        .checked_sub(resolved.common.soul_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.cooldowns = resolved.common.cooldowns;
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or(SpellCastDisposition::Rejected)?;
    if let Some(role) = spell.harmony_role {
        let monk = next.monk.as_mut().ok_or(SpellCastDisposition::Rejected)?;
        match role {
            HarmonyRole::Builder => monk.commit_builder().map(|_| ()),
            HarmonyRole::Spender => monk.commit_spender().map(|_| ()),
        }
        .map_err(|_| SpellCastDisposition::Rejected)?;
    }
    let anchor = crate::spell::combat_batch::SpellAnchor {
        expected_revision: state.revision,
        next_revision: next.revision,
        paid_mana: resolved.common.mana_spent,
        paid_soul: resolved.common.soul_spent,
        cooldown_deadlines: next.cooldowns.canonical_deadlines(),
    };
    Ok(PaidNativeCast {
        next,
        anchor,
        plan: resolved.plan,
    })
}

#[derive(Debug)]
pub struct PaidNativeCallbackCast {
    pub next: PlayerSpellState,
    pub anchor: crate::spell::combat_batch::SpellAnchor,
}

pub fn prepare_native_callback_owner_cast_with_caster(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &crate::spell::OperationalCastFacts,
    now: SemanticTimeMicros,
    qualified_caster: &CasterState,
) -> Result<PaidNativeCallbackCast, SpellCastDisposition> {
    prepare_native_callback_owner_cast_core(state, spell, operational, now, qualified_caster, None)
}
/// The source acquisition owns its dynamic Creature mana. Its qualified header
/// is still resolved through all ordinary common checks exactly once.
pub fn prepare_direct_acquisition_owner_cast_with_caster(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &crate::spell::OperationalCastFacts,
    now: SemanticTimeMicros,
    qualified_caster: &CasterState,
    reservation: &crate::spell::companion_lifecycle::DirectCompanionReservation,
) -> Result<PaidNativeCallbackCast, SpellCastDisposition> {
    admit_condition_gates(state, spell, now)?;
    let crate::spell::Execution::NativeProfile(profile) = &spell.execution else {
        return Err(SpellCastDisposition::Rejected);
    };
    if !reservation.qualifies_payment(profile, state.mana)
        || crate::spell::mana_cost(spell, qualified_caster) != 0
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let mut paid = prepare_native_callback_owner_cast_with_caster(
        state,
        spell,
        operational,
        now,
        qualified_caster,
    )?;
    if paid.anchor.paid_mana != 0 {
        return Err(SpellCastDisposition::Rejected);
    }
    let cost = reservation.mana_to_deduct();
    paid.next.mana = paid
        .next
        .mana
        .checked_sub(cost)
        .ok_or(SpellCastDisposition::NotEnoughMana)?;
    paid.anchor.paid_mana = cost;
    Ok(paid)
}
pub fn prepare_native_callback_owner_cast_with_carried_item(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &crate::spell::OperationalCastFacts,
    now: SemanticTimeMicros,
    qualified_caster: &CasterState,
    target: &crate::durability::spell_item_transaction::CarriedSpellTarget,
) -> Result<PaidNativeCallbackCast, SpellCastDisposition> {
    prepare_native_callback_owner_cast_core(
        state,
        spell,
        operational,
        now,
        qualified_caster,
        Some(target),
    )
}
fn prepare_native_callback_owner_cast_core(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &crate::spell::OperationalCastFacts,
    now: SemanticTimeMicros,
    qualified_caster: &CasterState,
    carried: Option<&crate::durability::spell_item_transaction::CarriedSpellTarget>,
) -> Result<PaidNativeCallbackCast, SpellCastDisposition> {
    admit_condition_gates(state, spell, now)?;
    if state.pending_stance.is_some() || state.training_checkpoint.is_some() {
        return Err(SpellCastDisposition::Rejected);
    }
    let multiplier = match &state.monk {
        Some(monk) => {
            monk.accept_command()
                .map_err(|_| SpellCastDisposition::Rejected)?;
            if spell.harmony_role == Some(HarmonyRole::Spender) {
                monk.spender_multiplier(
                    state.facts.level,
                    state.stance == Some(oteryn_gameplay_spell_rules::native_actor_states::StandardStance::Harmony),
                )
                .map_err(|_| SpellCastDisposition::Rejected)?
            } else {
                HarmonyMultiplier::ONE
            }
        }
        None => HarmonyMultiplier::ONE,
    };
    let caster = qualified_caster;
    if caster.vocation != state.facts.vocation
        || caster.level != state.facts.level
        || caster.magic_level != state.owned_effective_magic_level(now.get())?
        || caster.mana != state.mana
        || caster.max_mana != state.facts.max_mana
        || caster.soul != state.soul
        || caster.harmony_multiplier != multiplier
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let common = match carried {
        Some(target) => crate::spell::resolve_native_callback_common_for_carried_item(
            spell,
            caster,
            &state.cooldowns,
            now,
            operational,
            target,
        ),
        None => crate::spell::resolve_native_callback_common(
            spell,
            caster,
            &state.cooldowns,
            now,
            operational,
        ),
    }
    .map_err(|error| disposition(&error))?;
    let mut next = state.clone();
    next.mana = next
        .mana
        .checked_sub(common.mana_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.soul = next
        .soul
        .checked_sub(common.soul_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.cooldowns = common.cooldowns;
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or(SpellCastDisposition::Rejected)?;
    if let Some(role) = spell.harmony_role {
        let monk = next.monk.as_mut().ok_or(SpellCastDisposition::Rejected)?;
        match role {
            HarmonyRole::Builder => monk.commit_builder().map(|_| ()),
            HarmonyRole::Spender => monk.commit_spender().map(|_| ()),
        }
        .map_err(|_| SpellCastDisposition::Rejected)?;
    }
    let anchor = crate::spell::combat_batch::SpellAnchor {
        expected_revision: state.revision,
        next_revision: next.revision,
        paid_mana: common.mana_spent,
        paid_soul: common.soul_spent,
        cooldown_deadlines: next.cooldowns.canonical_deadlines(),
    };
    Ok(PaidNativeCallbackCast { next, anchor })
}

/// Pure common payment for the exact ordinary Conjure source profile. Items
/// and the real reagent custody must be staged by their durable owner; this
/// helper performs no grant, charge mutation or random selection.
pub fn prepare_conjure_owner_cast_with_caster(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &crate::spell::OperationalCastFacts,
    now: SemanticTimeMicros,
    qualified_caster: &CasterState,
) -> Result<PaidSourceSelfCast, SpellCastDisposition> {
    admit_condition_gates(state, spell, now)?;
    let crate::spell::Execution::Conjure {
        reagent,
        result,
        count,
    } = &spell.execution
    else {
        return Err(SpellCastDisposition::Rejected);
    };
    if *result == 0
        || *count == 0
        || *count > 50000
        || spell.needs_target
        || spell.target_or_direction
        || operational.target.is_some()
        || operational.target_position.is_some()
    {
        return Err(SpellCastDisposition::Rejected);
    }
    if state.pending_stance.is_some() || state.training_checkpoint.is_some() {
        return Err(SpellCastDisposition::Rejected);
    }
    let multiplier = match &state.monk {
        Some(monk) => {
            monk.accept_command()
                .map_err(|_| SpellCastDisposition::Rejected)?;
            if spell.harmony_role == Some(HarmonyRole::Spender) {
                monk.spender_multiplier(
                    state.facts.level,
                    state.stance == Some(oteryn_gameplay_spell_rules::native_actor_states::StandardStance::Harmony),
                )
                .map_err(|_| SpellCastDisposition::Rejected)?
            } else {
                HarmonyMultiplier::ONE
            }
        }
        None => HarmonyMultiplier::ONE,
    };
    let caster = qualified_caster;
    if caster.vocation != state.facts.vocation
        || caster.level != state.facts.level
        || caster.magic_level != state.owned_effective_magic_level(now.get())?
        || caster.mana != state.mana
        || caster.max_mana != state.facts.max_mana
        || caster.soul != state.soul
        || caster.harmony_multiplier != multiplier
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let mut unexpected_draw = false;
    let common = crate::spell::resolve_cast_with_operational_facts(
        spell,
        caster,
        &state.cooldowns,
        now,
        operational,
        &mut |minimum, _| {
            unexpected_draw = true;
            minimum
        },
    )
    .map_err(|error| disposition(&error))?;
    if unexpected_draw
        || common.effects
            != vec![crate::spell::ResolvedEffect::Conjure {
                reagent: *reagent,
                result: *result,
                count: *count,
            }]
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let mut next = state.clone();
    next.mana = next
        .mana
        .checked_sub(common.mana_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.soul = next
        .soul
        .checked_sub(common.soul_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.cooldowns = common.cooldowns;
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or(SpellCastDisposition::Rejected)?;
    if let Some(role) = spell.harmony_role {
        let monk = next.monk.as_mut().ok_or(SpellCastDisposition::Rejected)?;
        match role {
            HarmonyRole::Builder => monk.commit_builder().map(|_| ()),
            HarmonyRole::Spender => monk.commit_spender().map(|_| ()),
        }
        .map_err(|_| SpellCastDisposition::Rejected)?;
    }
    let anchor = crate::spell::combat_batch::SpellAnchor {
        expected_revision: state.revision,
        next_revision: next.revision,
        paid_mana: common.mana_spent,
        paid_soul: common.soul_spent,
        cooldown_deadlines: next.cooldowns.canonical_deadlines(),
    };
    Ok(PaidSourceSelfCast {
        next,
        anchor,
        ability_variant: None,
    })
}

/// The source name resolver's rejected Find Person callback starts its engine
/// cooldowns but debits no resources and produces no successful Harmony change.
pub fn prepare_locate_name_failure_owner_cast_with_caster(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &crate::spell::OperationalCastFacts,
    now: SemanticTimeMicros,
    caster: &CasterState,
) -> Result<PaidNativeCallbackCast, SpellCastDisposition> {
    admit_condition_gates(state, spell, now)?;
    let crate::spell::Execution::NativeProfile(profile) = &spell.execution else {
        return Err(SpellCastDisposition::Rejected);
    };
    let behavior = &profile.spell()["execution"]["native_behavior"];
    if behavior["key"] != "locate_message"
        || behavior["parameters"]["name_resolution_failure_starts_cooldown"] != true
        || behavior["parameters"]["name_resolution_failure_spends_mana"] != false
        || spell.harmony_role.is_some()
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let common =
        prepare_native_callback_owner_cast_with_caster(state, spell, operational, now, caster)?;
    let mut next = state.clone();
    next.cooldowns = common.next.cooldowns;
    next.revision = common.next.revision;
    let anchor = crate::spell::combat_batch::SpellAnchor {
        expected_revision: state.revision,
        next_revision: next.revision,
        paid_mana: 0,
        paid_soul: 0,
        cooldown_deadlines: next.cooldowns.canonical_deadlines(),
    };
    Ok(PaidNativeCallbackCast { next, anchor })
}

/// Source engine name-resolution failure: access checks precede name lookup,
/// then only spell/group cooldowns change. No healing draw or Harmony success.
pub fn prepare_named_player_failure_owner_cast_with_caster(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    now: SemanticTimeMicros,
    caster: &CasterState,
) -> Result<PaidNativeCallbackCast, SpellCastDisposition> {
    admit_condition_gates(state, spell, now)?;
    let profile = spell
        .authored
        .as_ref()
        .ok_or(SpellCastDisposition::Rejected)?;
    if !matches!(
        spell.name.as_str(),
        "Heal Friend" | "Nature's Embrace" | "Restore Balance"
    ) || profile.header.targeting.parameter != "player_name"
        || !spell.needs_target
        || spell.aggressive
        || state.pending_stance.is_some()
        || state.training_checkpoint.is_some()
        || caster.vocation != state.facts.vocation
        || caster.level != state.facts.level
        || caster.magic_level != state.owned_effective_magic_level(now.get())?
        || caster.mana != state.mana
        || caster.max_mana != state.facts.max_mana
        || caster.soul != state.soul
        || caster.harmony_multiplier != state.owned_harmony_multiplier(spell)?
    {
        return Err(SpellCastDisposition::Rejected);
    }
    if let Some(group) = spell.groups.first() {
        crate::spell::check_group(&state.cooldowns, &group.key, now)
            .map_err(|e| disposition(&e))?;
    }
    if state
        .cooldowns
        .spell_ready_at(&spell.key)
        .is_some_and(|at| at > now)
    {
        return Err(SpellCastDisposition::CoolingDown);
    }
    for group in spell.groups.iter().skip(1) {
        crate::spell::check_group(&state.cooldowns, &group.key, now)
            .map_err(|e| disposition(&e))?;
    }
    if caster.level < spell.level {
        return Err(SpellCastDisposition::LevelTooLow);
    }
    if caster.mana < crate::spell::mana_cost(spell, caster) {
        return Err(SpellCastDisposition::NotEnoughMana);
    }
    if caster.soul < spell.soul {
        return Err(SpellCastDisposition::NotEnoughSoul);
    }
    if (spell.learning_required && !caster.learned.contains(&spell.key))
        || (!spell.learning_required && !spell.vocations.contains(&caster.vocation))
        || (spell.needs_weapon && !caster.melee_weapon)
        || !caster.premium.admits(spell)
        || (spell.needs_shield && caster.shield_defense.is_none())
    {
        return Err(SpellCastDisposition::Rejected);
    }
    let mut next = state.clone();
    next.cooldowns
        .rearm(spell, now)
        .map_err(|e| disposition(&e))?;
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or(SpellCastDisposition::Rejected)?;
    let anchor = crate::spell::combat_batch::SpellAnchor {
        expected_revision: state.revision,
        next_revision: next.revision,
        paid_mana: 0,
        paid_soul: 0,
        cooldown_deadlines: next.cooldowns.canonical_deadlines(),
    };
    Ok(PaidNativeCallbackCast { next, anchor })
}

/// The owner commit of one planned effect onto the caster's own vitals. Only a heal of the caster
/// itself is admitted in V1; health never rises above its maximum.
pub(super) fn commit_effect(
    next: &mut PlayerSpellState,
    spell: &SpellDefinition,
    effect: &Effect,
    caster: &str,
) -> Result<(), SpellCastDisposition> {
    match effect {
        Effect::Heal { target, magnitude } if target.as_str() == caster && *magnitude > 0 => {
            let healed = u64::from(next.health)
                .saturating_add(u64::try_from(*magnitude).unwrap_or(u64::MAX))
                .min(u64::from(next.facts.max_health));
            next.health = u32::try_from(healed).map_err(|_| SpellCastDisposition::Rejected)?;
            Ok(())
        }
        // A damage or foreign-target effect needs the Target Resolver, which V1 does not compose.
        _ if spell.aggressive => Err(SpellCastDisposition::TargetIllegal),
        _ => Err(SpellCastDisposition::Rejected),
    }
}
