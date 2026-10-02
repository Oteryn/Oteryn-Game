//! One immediate spell Damage enters the existing Ability -> physical Channel-owner commit.
//!
//! This boundary owns no health, target-selection or payment store. The caller must already
//! have checked spell legality, range and mitigation and must hold the runtime and spell-state
//! locks throughout preparation and commit. It must preflight the caster replacement before
//! this mutation and replace it infallibly afterwards. This helper alone is not a whole-cast
//! atomic commit, and must not be used as admission for area, chain, heal or side effects.

use crate::ability::commit::{OwnerCommitError, commit_exact_owner_damage};
use crate::ability::exact_actor_resolution::{
    ExactActorProposal, ExactActorResolutionError, ResolvedExactActor, resolve_exact_actor,
};
use crate::ability::{CommitGroupMode, Effect, EffectPlan, ProposalSource};
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, CharacterId, CommandRef, ExactActorRef, OwnerDamageResult,
};

use super::plan::CastPlan;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Error {
    UnsupportedEffects,
    CasterSubstitution,
    CasterUncontrolled,
    PositionContextMismatch,
    InvalidLeaseGeneration,
    Resolution(ExactActorResolutionError),
    Owner(CarrierError),
    Commit(OwnerCommitError),
}

/// Immutable prepared effect, not an authority capability. Retain this exact value on a
/// command retry: re-resolving a killed target would discard its valid lethal replay receipt.
#[derive(Debug, Clone)]
pub(crate) struct PreparedOwnerDamage {
    caster: ExactActorRef,
    command: CommandRef,
    resolved: ResolvedExactActor,
    plan: EffectPlan,
}

impl PreparedOwnerDamage {
    pub(crate) fn plan(&self) -> &EffectPlan {
        &self.plan
    }

    pub(crate) fn target(&self) -> ExactActorRef {
        self.resolved.target()
    }
}

/// Same exact-actor spelling as the existing live spell cast composition.
pub(crate) fn actor_atom(actor: ExactActorRef) -> String {
    let placement = actor.placement_identity();
    let mut atom = String::with_capacity(38);
    atom.push_str("actor:");
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in placement {
        atom.push(char::from(HEX[usize::from(byte >> 4)]));
        atom.push(char::from(HEX[usize::from(byte & 15)]));
    }
    atom
}

fn check_current_caster(
    runtime: &ChannelRuntimeV1,
    caster: ExactActorRef,
    command: CommandRef,
) -> Result<(), Error> {
    let facts = runtime
        .player_control_facts(caster, command.game_session_id())
        .map_err(Error::Owner)?;
    if facts.control_loss.is_some() {
        return Err(Error::CasterUncontrolled);
    }
    Ok(())
}

/// Prepare from a real owner-issued target; this does not invent an AttackTarget selection.
/// The effect's target atom remains bound by the carrier to its stored creature identity.
pub(crate) fn prepare(
    runtime: &ChannelRuntimeV1,
    caster: ExactActorRef,
    command: CommandRef,
    target: ExactActorRef,
    cast_plan: &CastPlan,
) -> Result<PreparedOwnerDamage, Error> {
    let plan = cast_plan
        .effects
        .as_ref()
        .ok_or(Error::UnsupportedEffects)?;
    if !cast_plan.side_effects.is_empty()
        || plan.effects().len() != 1
        || plan.intent().candidate_count() != 1
        || plan.intent().resolved_targets().len() != 1
        || plan.intent().proposal_source() != ProposalSource::Client
        || plan.commit_group().mode() != CommitGroupMode::Atomic
    {
        return Err(Error::UnsupportedEffects);
    }
    let Effect::Damage {
        target: effect_target,
        magnitude,
    } = &plan.effects()[0]
    else {
        return Err(Error::UnsupportedEffects);
    };
    if *magnitude <= 0 || effect_target != &plan.intent().resolved_targets()[0] {
        return Err(Error::UnsupportedEffects);
    }
    if plan.intent().actor() != actor_atom(caster) {
        return Err(Error::CasterSubstitution);
    }
    check_current_caster(runtime, caster, command)?;
    let caster_position = runtime.read_actor_position(caster).map_err(Error::Owner)?;
    let target_position = runtime.read_actor_position(target).map_err(Error::Owner)?;
    if caster_position.context() != target_position.context() {
        return Err(Error::PositionContextMismatch);
    }
    let resolved = resolve_exact_actor(
        &runtime.borrow_exact_actor_lookup(),
        plan.occurrence(),
        ExactActorProposal::client(target),
    )
    .map_err(Error::Resolution)?;
    Ok(PreparedOwnerDamage {
        caster,
        command,
        resolved,
        plan: plan.clone(),
    })
}

/// The caller supplies the independently current Character/lease from its existing durable
/// gameplay fence, never from the prepared plan. This returns the carrier's own receipt:
/// `applied=false` is a reconciled replay and must not cause a second caster payment.
pub(crate) fn commit(
    runtime: &mut ChannelRuntimeV1,
    prepared: &PreparedOwnerDamage,
    attacker: CharacterId,
    lease_generation: u64,
) -> Result<OwnerDamageResult, Error> {
    if lease_generation == 0 {
        return Err(Error::InvalidLeaseGeneration);
    }
    check_current_caster(runtime, prepared.caster, prepared.command)?;
    commit_exact_owner_damage(
        &mut runtime.borrow_exact_actor_commit(),
        &prepared.resolved,
        &prepared.plan,
        attacker,
        lease_generation,
        prepared.command,
    )
    .map_err(Error::Commit)
}

#[cfg(test)]
#[path = "combat_execution_tests.rs"]
mod tests;
