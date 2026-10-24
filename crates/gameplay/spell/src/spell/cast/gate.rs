//! Cast admission gates and the self-effect cast composition.

use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellCastIntent, SpellTarget};
use oteryn_simulation_determinism::SemanticTimeMicros;

use super::PlayerSpellState;
use super::prepare::commit_effect;
use crate::ability::AbilityOccurrence;
use crate::spell::harmony::HarmonyMultiplier;
use crate::spell::plan::{CastPlanError, effect_plan};
use crate::spell::{
    CastRejection, CasterState, HarmonyRole, ResolvedEffect, SpellBook, SpellDefinition,
    resolve_cast,
};

/// Everything a cast reads besides the book, the actor's state and the intent. The owner derives
/// it from the command; nothing in it comes from the client.
pub struct CastContext<'a> {
    /// The caster's exact actor atom (the Ability target of its own heal).
    pub caster: &'a str,
    pub owner_scope: &'a str,
    pub occurrence: AbilityOccurrence,
    pub now: SemanticTimeMicros,
    pub draw: &'a mut dyn FnMut(i64, i64) -> i64,
}

/// The §3 mapping from a core rejection to the wire disposition. A rejection the table does not
/// name (weapon, shield, chain or party facts) cannot occur for the V1 book and fails closed.
pub const fn disposition(rejection: &CastRejection) -> SpellCastDisposition {
    match rejection {
        CastRejection::GroupCooling { .. } | CastRejection::SpellCooling { .. } => {
            SpellCastDisposition::CoolingDown
        }
        CastRejection::LevelTooLow { .. } => SpellCastDisposition::LevelTooLow,
        CastRejection::MagicLevelTooLow { .. } => SpellCastDisposition::MagicLevelTooLow,
        CastRejection::NotEnoughMana { .. } => SpellCastDisposition::NotEnoughMana,
        CastRejection::NotEnoughSoul { .. } => SpellCastDisposition::NotEnoughSoul,
        CastRejection::NotLearned
        | CastRejection::VocationCannotUse
        | CastRejection::PremiumRequired => SpellCastDisposition::NotAvailable,
        CastRejection::TargetRequired => SpellCastDisposition::TargetRequired,
        CastRejection::TargetNotAllowed => SpellCastDisposition::TargetIllegal,
        _ => SpellCastDisposition::Rejected,
    }
}

/// CONDITIONS-0 §3.5: the gate families answer first once the spell is identified, before
/// every other admission check (protection zone included), cost and cooldown. A rune takes
/// the rune gate, which `muted` never closes. The typed gate stays server-side (§7).
pub fn condition_gate(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    now: SemanticTimeMicros,
) -> Result<(), oteryn_gameplay_domain::domain::condition::ConditionGate> {
    store_condition_gate(&state.conditions, spell, now)
}

/// [`condition_gate`] over any condition store, such as the actor's runtime condition slot.
pub fn store_condition_gate<S: Clone>(
    store: &oteryn_gameplay_foundation::foundation::condition::ConditionStore<S>,
    spell: &SpellDefinition,
    now: SemanticTimeMicros,
) -> Result<(), oteryn_gameplay_domain::domain::condition::ConditionGate> {
    if matches!(spell.carrier, crate::spell::Carrier::Rune { .. }) {
        store.rune_gate(spell.aggressive, now.get())
    } else {
        store.spell_cast_gate(spell.aggressive, now.get())
    }
}
/// [`condition_gate`] on the wire: any gate refuses with `REJECTED`.
pub fn admit_condition_gates(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    now: SemanticTimeMicros,
) -> Result<(), SpellCastDisposition> {
    condition_gate(state, spell, now).map_err(|_| SpellCastDisposition::Rejected)
}

pub fn cast(
    book: &SpellBook,
    state: &PlayerSpellState,
    intent: &SpellCastIntent,
    context: CastContext<'_>,
) -> Result<PlayerSpellState, SpellCastDisposition> {
    let spell = book
        .indexed(intent.spell)
        .ok_or(SpellCastDisposition::Rejected)?;
    admit_condition_gates(state, spell, context.now)?;
    if state.pending_stance.is_some() || state.training_checkpoint.is_some() {
        return Err(SpellCastDisposition::Rejected);
    }
    cast_resolved(book, state, spell, intent.target, context, None).map(|(next, _)| next)
}
pub(super) fn cast_resolved(
    book: &SpellBook,
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    target: SpellTarget,
    context: CastContext<'_>,
    qualified: Option<(&CasterState, &crate::spell::OperationalCastFacts)>,
) -> Result<
    (
        PlayerSpellState,
        Option<crate::spell::executable_catalog::DefinitionRef>,
    ),
    SpellCastDisposition,
> {
    let has_target = match target {
        SpellTarget::None => false,
        // The attack target is owned by the Channel owner's
        // `gameplay_transport::attack::ChannelAttackStates` (ATTACK-0 §4). SPELL-TARGET-1 resolves
        // it in ordinary combat, which holds the world; this V1 self-effect path composes no
        // Target Resolver, so it resolves no target and `aim_at_target`, honoured only with
        // one (SPELL-D7), is ignored.
        SpellTarget::AttackTarget => false,
        // SPELL-D7: only a `cast_at_position` spell takes a position, and the core admits none.
        SpellTarget::Position(_) => return Err(SpellCastDisposition::Rejected),
    };
    // §8.2: a monk accepts no command before its initialization evaluation. A spender's damage
    // takes the multiplier of the current charges and Serene; no stance owner exists, so no
    // virtue (§4 interim rule).
    let harmony_multiplier = match &state.monk {
        Some(monk) => {
            monk.accept_command()
                .map_err(|_| SpellCastDisposition::Rejected)?;
            match spell.harmony_role {
                Some(HarmonyRole::Spender) => monk
                    .spender_multiplier(
                        state.facts.level,
                        state.stance == Some(oteryn_gameplay_spell_rules::native_actor_states::StandardStance::Harmony),
                    )
                    .map_err(|_| SpellCastDisposition::Rejected)?,
                Some(HarmonyRole::Builder) | None => HarmonyMultiplier::ONE,
            }
        }
        None => HarmonyMultiplier::ONE,
    };
    let mut resolution = match qualified {
        Some((caster, operational)) => {
            if caster.vocation != state.facts.vocation
                || caster.level != state.facts.level
                || caster.magic_level != state.owned_effective_magic_level(context.now.get())?
                || caster.mana != state.mana
                || caster.max_mana != state.facts.max_mana
                || caster.soul != state.soul
                || caster.harmony_multiplier != harmony_multiplier
            {
                return Err(SpellCastDisposition::Rejected);
            }
            crate::spell::resolve_cast_with_operational_facts(
                spell,
                caster,
                &state.cooldowns,
                context.now,
                operational,
                context.draw,
            )
        }
        None => {
            let mut caster = state.caster(harmony_multiplier);
            caster.magic_level = state.owned_effective_magic_level(context.now.get())?;
            resolve_cast(
                spell,
                &caster,
                &state.cooldowns,
                context.now,
                has_target,
                context.draw,
            )
        }
    }
    .map_err(|rejection| disposition(&rejection))?;
    let mut next = state.clone();
    let focus_profile = match &spell.execution {
        crate::spell::Execution::ActorFocus { profile } => Some(profile),
        crate::spell::Execution::NativeProfile(profile)
            if profile.spell()["execution"]["native_behavior"]["key"] == "monk_focus" =>
        {
            Some(&profile.spell()["execution"]["native_behavior"]["parameters"])
        }
        // Other native families must commit through their actual owner compositor.
        // An empty common ability plan never constitutes a successful native cast.
        crate::spell::Execution::NativeProfile(_) => return Err(SpellCastDisposition::Rejected),
        _ => None,
    };
    if let Some(profile) = focus_profile {
        if qualified.is_some() {
            let monk = state.monk.as_ref().ok_or(SpellCastDisposition::Rejected)?;
            let planned = oteryn_gameplay_spell_rules::native_actor_states::plan_focus(
                profile,
                oteryn_gameplay_spell_rules::native_actor_states::FocusFacts {
                    harmony: monk.harmony(),
                    serene: monk.serene(),
                    forced_until: monk.serene_forced_until(),
                    now: context.now,
                    level: state.facts.level,
                    sustain_active: state.stance
                        == Some(oteryn_gameplay_spell_rules::native_actor_states::StandardStance::Sustain),
                },
            )
            .map_err(|_| SpellCastDisposition::Rejected)?;
            if planned.healing.is_some() {
                return Err(SpellCastDisposition::TargetIllegal);
            }
        }
        let focus = crate::spell::actor_execution::apply_focus(
            &mut next,
            profile,
            context.now,
            context.draw,
        )?;
        resolution.cooldowns.reset_for_focus(book, &focus);
        if focus.rearm_cast_cooldowns {
            resolution
                .cooldowns
                .rearm(spell, context.now)
                .map_err(|rejection| disposition(&rejection))?;
        }
    }
    let plan = effect_plan(
        spell,
        &resolution,
        context.caster,
        None,
        context.occurrence,
        context.owner_scope,
    )
    .map_err(|error| match error {
        CastPlanError::MissingTarget => SpellCastDisposition::TargetRequired,
        _ => SpellCastDisposition::Rejected,
    })?;
    for effect in &plan.side_effects {
        match effect {
            ResolvedEffect::RemoveCondition { condition } => {
                crate::spell::actor_conditions::remove_condition_at(
                    &mut next,
                    condition,
                    context.now.get(),
                )?;
            }
            ResolvedEffect::ResolvedOther { profile, .. }
                if qualified.is_some()
                    && crate::spell::actor_conditions::is_presentation_only(profile) => {}
            ResolvedEffect::ResolvedOther { profile, .. } => {
                crate::spell::actor_conditions::apply_effect(
                    &mut next,
                    profile,
                    context.now.get() / 1_000,
                )?;
            }
            _ => return Err(SpellCastDisposition::Rejected),
        }
    }
    if let Some(plan) = &plan.effects {
        for effect in plan.effects() {
            commit_effect(&mut next, spell, effect, context.caster)?;
        }
    }
    // §8.2 PRIMARY COMMIT: Harmony changes in this same value, only because the cast succeeded
    // (§A.2 steps 3c and 4). A Harmony spell without a monk state is refused.
    if let Some(role) = spell.harmony_role {
        let monk = next.monk.as_mut().ok_or(SpellCastDisposition::Rejected)?;
        match role {
            HarmonyRole::Builder => monk.commit_builder().map(|_| ()),
            HarmonyRole::Spender => monk.commit_spender().map(|_| ()),
        }
        .map_err(|_| SpellCastDisposition::Rejected)?;
    }
    // §5 anchor: mana, soul and cooldowns are paid in this same value, only because it succeeded.
    next.mana = next
        .mana
        .checked_sub(resolution.mana_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.soul = next
        .soul
        .checked_sub(resolution.soul_spent)
        .ok_or(SpellCastDisposition::Rejected)?;
    next.cooldowns = resolution.cooldowns;
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or(SpellCastDisposition::Rejected)?;
    Ok((next, resolution.ability_variant))
}
