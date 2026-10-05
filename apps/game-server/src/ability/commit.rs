use super::effects::Effect;
use super::plan::{EffectPlan, SubOccurrenceRef};
use super::{AbilityError, AbilityOccurrenceId};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitReceipt {
    applied: bool,
    suboccurrences: Vec<SubOccurrenceRef>,
}

impl CommitReceipt {
    #[must_use]
    pub const fn applied(&self) -> bool {
        self.applied
    }

    #[must_use]
    pub const fn applied_suboccurrences(&self) -> usize {
        self.suboccurrences.len()
    }

    #[must_use]
    pub fn suboccurrences(&self) -> &[SubOccurrenceRef] {
        &self.suboccurrences
    }
}

#[derive(Debug)]
enum CommitRecord {
    Complete(EffectPlan),
    Sequential {
        plan: EffectPlan,
        next_effect: usize,
    },
}

impl CommitRecord {
    fn plan(&self) -> &EffectPlan {
        match self {
            Self::Complete(plan) | Self::Sequential { plan, .. } => plan,
        }
    }
}

#[derive(Debug, Default)]
pub struct AbilityEngine {
    committed: BTreeMap<AbilityOccurrenceId, CommitRecord>,
    fixture_health: BTreeMap<String, i64>,
}

impl AbilityEngine {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn commit(&mut self, plan: EffectPlan) -> Result<CommitReceipt, AbilityError> {
        let occurrence_id = plan.occurrence().id().clone();
        let next_effect = if let Some(existing) = self.committed.get(&occurrence_id) {
            let existing_plan = existing.plan();
            if existing_plan.occurrence().revisions() != plan.occurrence().revisions() {
                return Err(AbilityError::OccurrenceRevisionConflict);
            }
            if existing_plan != &plan {
                return Err(AbilityError::OccurrencePlanConflict);
            }
            match existing {
                CommitRecord::Complete(_) => {
                    return Ok(CommitReceipt {
                        applied: false,
                        suboccurrences: Vec::new(),
                    });
                }
                CommitRecord::Sequential { next_effect, .. } => *next_effect,
            }
        } else {
            0
        };

        match plan.commit_group().mode() {
            super::CommitGroupMode::Atomic => self.commit_atomic(occurrence_id, plan),
            super::CommitGroupMode::OrderedSequential => {
                self.commit_sequential(occurrence_id, plan, next_effect)
            }
        }
    }

    #[must_use]
    pub fn fixture_health(&self, target: &str) -> Option<i64> {
        self.fixture_health.get(target).copied()
    }
}

impl AbilityEngine {
    fn commit_atomic(
        &mut self,
        occurrence_id: AbilityOccurrenceId,
        plan: EffectPlan,
    ) -> Result<CommitReceipt, AbilityError> {
        let mut next_health = self.fixture_health.clone();
        for effect in plan.effects() {
            apply_fixture_effect(&mut next_health, effect)?;
        }
        self.fixture_health = next_health;
        self.committed
            .insert(occurrence_id, CommitRecord::Complete(plan));
        Ok(CommitReceipt {
            applied: true,
            suboccurrences: Vec::new(),
        })
    }

    fn commit_sequential(
        &mut self,
        occurrence_id: AbilityOccurrenceId,
        plan: EffectPlan,
        next_effect: usize,
    ) -> Result<CommitReceipt, AbilityError> {
        for index in next_effect..plan.effects().len() {
            let effect = plan.effects()[index].clone();
            if let Err(error) = apply_fixture_effect(&mut self.fixture_health, &effect) {
                self.committed.insert(
                    occurrence_id,
                    CommitRecord::Sequential {
                        plan,
                        next_effect: index,
                    },
                );
                return Err(error);
            }
        }
        let suboccurrences = (next_effect..plan.effects().len())
            .map(|ordinal| plan.sub_occurrence(ordinal))
            .collect::<Option<Vec<_>>>()
            .ok_or(AbilityError::MissingSubOccurrence)?;
        self.committed
            .insert(occurrence_id, CommitRecord::Complete(plan));
        Ok(CommitReceipt {
            applied: !suboccurrences.is_empty(),
            suboccurrences,
        })
    }
}

fn apply_fixture_effect(
    fixture_health: &mut BTreeMap<String, i64>,
    effect: &Effect,
) -> Result<(), AbilityError> {
    effect.validate_magnitude()?;
    let target = effect.target().as_str().to_owned();
    let health = fixture_health.entry(target).or_insert(0);
    *health = match effect {
        Effect::Damage { magnitude, .. } => health
            .checked_sub(*magnitude)
            .ok_or(AbilityError::NumericOverflow)?,
        Effect::Heal { magnitude, .. } => health
            .checked_add(*magnitude)
            .ok_or(AbilityError::NumericOverflow)?,
    };
    Ok(())
}

/// Byte-exact bounded encoding of every semantic plan field. The carrier
/// retains this entire value, rather than a digest with collision risk. Every
/// atom is validated at plan construction and cannot contain zero bytes.
#[allow(dead_code)]
fn encode_owner_damage_plan(plan: &EffectPlan) -> Result<Vec<u8>, AbilityError> {
    let mut encoded = Vec::new();
    encoded
        .try_reserve_exact(super::MAX_EFFECT_PLAN_BYTES)
        .map_err(|_| AbilityError::RetainedByteOverflow)?;
    fn append(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), AbilityError> {
        if out
            .len()
            .checked_add(bytes.len())
            .is_none_or(|size| size > super::MAX_EFFECT_PLAN_BYTES)
        {
            return Err(AbilityError::EffectPlanTooLarge);
        }
        out.extend_from_slice(bytes);
        Ok(())
    }
    fn atom(out: &mut Vec<u8>, value: &str) -> Result<(), AbilityError> {
        append(out, value.as_bytes())?;
        append(out, &[0])
    }
    atom(&mut encoded, plan.occurrence().id().as_str())?;
    let revisions = plan.occurrence().revisions();
    for revision in [
        revisions.ruleset(),
        revisions.content(),
        revisions.world_policy(),
        revisions.formula(),
        revisions.simulation(),
    ] {
        atom(&mut encoded, revision)?;
    }
    let intent = plan.intent();
    append(
        &mut encoded,
        &[match intent.proposal_source() {
            super::ProposalSource::Client => 1,
            super::ProposalSource::Ai => 2,
            super::ProposalSource::Script => 3,
        }],
    )?;
    atom(&mut encoded, intent.actor())?;
    append(
        &mut encoded,
        &(intent.candidate_count() as u64).to_be_bytes(),
    )?;
    append(
        &mut encoded,
        &(intent.resolved_targets().len() as u64).to_be_bytes(),
    )?;
    for target in intent.resolved_targets() {
        atom(&mut encoded, target.as_str())?;
    }
    append(&mut encoded, &(plan.effects().len() as u64).to_be_bytes())?;
    for effect in plan.effects() {
        append(
            &mut encoded,
            &[match effect {
                Effect::Damage { .. } => 1,
                Effect::Heal { .. } => 2,
            }],
        )?;
        atom(&mut encoded, effect.target().as_str())?;
        append(&mut encoded, &effect.magnitude().to_be_bytes())?;
    }
    append(
        &mut encoded,
        &(plan.calculation_stages().len() as u64).to_be_bytes(),
    )?;
    for stage in plan.calculation_stages() {
        atom(&mut encoded, stage.as_str())?;
    }
    atom(&mut encoded, plan.commit_group().owner_scope())?;
    atom(&mut encoded, plan.commit_group().group_id())?;
    append(
        &mut encoded,
        &[match plan.commit_group().mode() {
            super::CommitGroupMode::Atomic => 1,
            super::CommitGroupMode::OrderedSequential => 2,
        }],
    )?;
    Ok(encoded)
}

/// A real typed Ability→Foundation bridge. The fixture BTreeMap engine above is intentionally not
/// on this path.
///
/// A2 (D295 item 4): the attacker authority is the bound player slot `attacker`, read by the
/// owner in the same runtime-lock critical section as the write. The slot must hold
/// `command`'s session, carry no write fence and have its Character lease bound
/// (CHARM-DESC-FENCE-LEASE §3 item 2); a caller never supplies the Character or the lease.
///
/// D4 (D141): the owner's replay identity for this commit is derived by the carrier from
/// `(character, lease_generation, command.game_session_id(), command.command_id(), sub_ordinal)`
/// of the bound lease, and `sub_ordinal` is the committed effect's own index in the plan. The
/// plan's opaque `AbilityOccurrenceId` is never passed as identity. `command` must be the actual
/// FND-02 [`CommandRef`] of the attacker's command.
#[allow(dead_code, reason = "no live gameplay caller is composed yet")]
pub(crate) fn commit_exact_owner_damage(
    owner: &mut crate::foundation::CurrentOwnerExactActorCommit<'_>,
    resolved: &super::exact_actor_resolution::ResolvedExactActor,
    plan: &EffectPlan,
    attacker: crate::foundation::ExactActorRef,
    command: crate::foundation::CommandRef,
) -> Result<crate::foundation::OwnerDamageResult, OwnerCommitError> {
    use super::exact_actor_resolution::ExactActorSource;
    if plan.occurrence() != resolved.occurrence()
        || plan.intent().candidate_count() != 1
        || plan.intent().resolved_targets().len() != 1
        || plan.effects().len() != 1
        || plan.commit_group().mode() != super::CommitGroupMode::Atomic
        || !matches!(
            (resolved.source(), plan.intent().proposal_source()),
            (ExactActorSource::Client, super::ProposalSource::Client)
                | (ExactActorSource::Ai, super::ProposalSource::Ai)
        )
    {
        return Err(OwnerCommitError::InvalidPlan);
    }
    let Effect::Damage { target, magnitude } = &plan.effects()[0] else {
        return Err(OwnerCommitError::InvalidPlan);
    };
    if target != &plan.intent().resolved_targets()[0] || *magnitude <= 0 {
        return Err(OwnerCommitError::InvalidPlan);
    }
    let sub_ordinal = plan
        .sub_occurrence(0)
        .and_then(|sub| u16::try_from(sub.ordinal()).ok())
        .ok_or(OwnerCommitError::InvalidPlan)?;
    let binding = encode_owner_damage_plan(plan).map_err(OwnerCommitError::Plan)?;
    owner
        .commit_damage_for_bound_attacker(
            resolved.target(),
            attacker,
            command,
            sub_ordinal,
            crate::foundation::OwnerDamageCommand {
                target: target.as_str().as_bytes(),
                // Not read for an attributed commit: the carrier derives the identity.
                occurrence: &[],
                binding: &binding,
                damage: *magnitude,
            },
        )
        .map_err(OwnerCommitError::Owner)
}

/// ATTACK-1b `AutoAttack` bridge: the same exact one-target Damage plan checks as
/// [`commit_exact_owner_damage`], committed under the swing identity `(lineage, swing_ordinal)`
/// of [`crate::foundation::CurrentOwnerExactActorCommit::commit_swing_damage_for_bound_attacker`].
/// The plan's own sub-occurrence must be 0: a swing plan has exactly one effect.
#[allow(
    dead_code,
    reason = "tests/ability_engine.rs path-loads Ability without the Channel owner attack drain"
)]
pub(crate) fn commit_exact_owner_swing_damage(
    owner: &mut crate::foundation::CurrentOwnerExactActorCommit<'_>,
    resolved: &super::exact_actor_resolution::ResolvedExactActor,
    plan: &EffectPlan,
    attacker: crate::foundation::ExactActorRef,
    lineage: crate::foundation::CommandRef,
    swing_ordinal: u16,
) -> Result<crate::foundation::OwnerDamageResult, OwnerCommitError> {
    use super::exact_actor_resolution::ExactActorSource;
    if plan.occurrence() != resolved.occurrence()
        || plan.intent().candidate_count() != 1
        || plan.intent().resolved_targets().len() != 1
        || plan.effects().len() != 1
        || plan.commit_group().mode() != super::CommitGroupMode::Atomic
        || !matches!(
            (resolved.source(), plan.intent().proposal_source()),
            (ExactActorSource::Client, super::ProposalSource::Client)
        )
    {
        return Err(OwnerCommitError::InvalidPlan);
    }
    let Effect::Damage { target, magnitude } = &plan.effects()[0] else {
        return Err(OwnerCommitError::InvalidPlan);
    };
    if target != &plan.intent().resolved_targets()[0]
        || *magnitude <= 0
        || plan.sub_occurrence(0).map(|sub| sub.ordinal()) != Some(0)
    {
        return Err(OwnerCommitError::InvalidPlan);
    }
    let binding = encode_owner_damage_plan(plan).map_err(OwnerCommitError::Plan)?;
    owner
        .commit_swing_damage_for_bound_attacker(
            resolved.target(),
            attacker,
            lineage,
            swing_ordinal,
            crate::foundation::OwnerDamageCommand {
                target: target.as_str().as_bytes(),
                occurrence: &[],
                binding: &binding,
                damage: *magnitude,
            },
        )
        .map_err(OwnerCommitError::Owner)
}

/// Immutable provenance minted only after the existing atomic owner commit. This is not current
/// authority: the descendant still requires an independently supplied owner and command fence.
#[derive(Debug)]
#[cfg_attr(
    test,
    allow(
        dead_code,
        reason = "the standalone Ability suite has no native owner fixture"
    )
)]
pub(crate) struct OwnerCommittedPrimaryDamage {
    plan: EffectPlan,
    target: crate::foundation::ExactActorRef,
    attacker: crate::foundation::CharacterId,
    lease_generation: u64,
    command: crate::foundation::CommandRef,
    result: crate::foundation::OwnerDamageResult,
}

impl OwnerCommittedPrimaryDamage {
    #[cfg_attr(
        test,
        allow(
            dead_code,
            reason = "actual receipt evaluation runs in the native owner suite"
        )
    )]
    pub(crate) const fn result(&self) -> &crate::foundation::OwnerDamageResult {
        &self.result
    }
}

/// Mint the sealed parent from real owner-issued HP facts, never a predicted or supplied result.
/// The provenance records the bound lease the write was admitted under.
#[allow(dead_code, reason = "no live gameplay caller is composed yet")]
pub(crate) fn commit_exact_owner_primary_damage(
    owner: &mut crate::foundation::CurrentOwnerExactActorCommit<'_>,
    resolved: &super::exact_actor_resolution::ResolvedExactActor,
    plan: &EffectPlan,
    attacker: crate::foundation::ExactActorRef,
    command: crate::foundation::CommandRef,
) -> Result<OwnerCommittedPrimaryDamage, OwnerCommitError> {
    let lease = owner
        .bound_attacker_lease(attacker, command)
        .map_err(OwnerCommitError::Owner)?;
    let result = commit_exact_owner_damage(owner, resolved, plan, attacker, command)?;
    Ok(OwnerCommittedPrimaryDamage {
        plan: plan.clone(),
        target: resolved.target(),
        attacker: lease.character_id(),
        lease_generation: lease.generation(),
        command,
        result,
    })
}

/// A leaf source. This receipt cannot be used as a sealed primary; consumers map it explicitly
/// to CharmHitSource::CharmDamage and never re-enter Charm hooks from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnerCharmDamageSource {
    CharmDamage,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OwnerCharmDamageResult {
    pub(crate) result: crate::foundation::OwnerDamageResult,
    pub(crate) source: OwnerCharmDamageSource,
}

/// Frozen post-commit root entries: the exact committed primary prefix at index 0 and the new
/// generated damage at index 1. Only index 1 is committed; the original atomic binding is never
/// replaced by this expanded plan. Retain this value for retries, including parent-receipt eviction.
#[derive(Debug)]
pub(crate) struct OwnerCharmDamagePlan {
    plan: EffectPlan,
    binding: Box<[u8]>,
    target: crate::foundation::ExactActorRef,
    attacker: crate::foundation::CharacterId,
    lease_generation: u64,
    command: crate::foundation::CommandRef,
}

impl OwnerCharmDamagePlan {
    /// Run reaction evaluation from `parent.result()` first, then freeze its mitigated outcome.
    /// A lethal/non-reducing parent is terminal for these attack procs; it stays committed.
    #[cfg_attr(
        test,
        allow(
            dead_code,
            reason = "native descendant preparation runs in the owner suite"
        )
    )]
    pub(crate) fn prepare(
        parent: &OwnerCommittedPrimaryDamage,
        plan: EffectPlan,
    ) -> Result<Option<Self>, OwnerCommitError> {
        if parent.result.health_after <= 0
            || parent.result.health_before <= parent.result.health_after
        {
            return Ok(None);
        }
        if plan.effects().len() != 2
            || plan.commit_group().mode() != super::CommitGroupMode::OrderedSequential
            || plan.occurrence() != parent.plan.occurrence()
            || plan.intent() != parent.plan.intent()
            || plan.commit_group().owner_scope() != parent.plan.commit_group().owner_scope()
            || plan.commit_group().group_id() != parent.plan.commit_group().group_id()
            || plan.effects()[0] != parent.plan.effects()[0]
        {
            return Err(OwnerCommitError::InvalidPlan);
        }
        let Effect::Damage { target, magnitude } = &plan.effects()[1] else {
            return Err(OwnerCommitError::InvalidPlan);
        };
        if *magnitude <= 0 || target != parent.plan.effects()[0].target() {
            return Err(OwnerCommitError::InvalidPlan);
        }
        let prefix = encode_owner_damage_plan(&parent.plan).map_err(OwnerCommitError::Plan)?;
        let child = encode_owner_damage_plan(&plan).map_err(OwnerCommitError::Plan)?;
        let tag = b"oteryn.charm.damage.descendant.v1\0";
        let bytes = tag
            .len()
            .checked_add(8 + 16)
            .and_then(|size| size.checked_add(prefix.len()))
            .and_then(|size| size.checked_add(child.len()))
            .filter(|size| *size <= super::MAX_EFFECT_PLAN_BYTES)
            .ok_or(OwnerCommitError::Plan(AbilityError::EffectPlanTooLarge))?;
        let mut binding = Vec::new();
        binding
            .try_reserve_exact(bytes)
            .map_err(|_| OwnerCommitError::Plan(AbilityError::RetainedByteOverflow))?;
        binding.extend_from_slice(tag);
        for encoded in [&prefix, &child] {
            let length = u32::try_from(encoded.len())
                .map_err(|_| OwnerCommitError::Plan(AbilityError::RetainedByteOverflow))?;
            binding.extend_from_slice(&length.to_be_bytes());
            binding.extend_from_slice(encoded);
        }
        binding.extend_from_slice(&parent.result.health_before.to_be_bytes());
        binding.extend_from_slice(&parent.result.health_after.to_be_bytes());
        Ok(Some(Self {
            plan,
            binding: binding.into_boxed_slice(),
            target: parent.target,
            attacker: parent.attacker,
            lease_generation: parent.lease_generation,
            command: parent.command,
        }))
    }
}

/// Commit only the actual descendant entry 1 through the same physical owner boundary. Immutable
/// parent provenance never supplies current authority: the attacker's current lease is read from
/// its bound player slot `attacker` at this write (A2, D295 item 4), never from `frozen`. A
/// fenced, rebound or unbound slot refuses as `SupersededAttackerSession`. A lease generation
/// other than the frozen primary's means the attacker's session was superseded after the primary
/// commit; the creature's high-water mark alone cannot see that, so it is refused here before the
/// owner write.
#[allow(dead_code, reason = "no live gameplay caller is composed yet")]
pub(crate) fn commit_exact_owner_charm_damage(
    owner: &mut crate::foundation::CurrentOwnerExactActorCommit<'_>,
    frozen: &OwnerCharmDamagePlan,
    attacker: crate::foundation::ExactActorRef,
    command: crate::foundation::CommandRef,
) -> Result<OwnerCharmDamageResult, OwnerCommitError> {
    if command != frozen.command {
        return Err(OwnerCommitError::InvalidPlan);
    }
    let current_lease = owner
        .bound_attacker_lease(attacker, command)
        .map_err(OwnerCommitError::Owner)?;
    if current_lease.character_id() != frozen.attacker {
        return Err(OwnerCommitError::InvalidPlan);
    }
    if !current_lease.accepts_generation(frozen.lease_generation) {
        return Err(OwnerCommitError::Owner(
            crate::foundation::CarrierError::SupersededAttackerSession,
        ));
    }
    let Effect::Damage { target, magnitude } = &frozen.plan.effects()[1] else {
        return Err(OwnerCommitError::InvalidPlan);
    };
    let sub_ordinal = frozen
        .plan
        .sub_occurrence(1)
        .and_then(|sub| u16::try_from(sub.ordinal()).ok())
        .ok_or(OwnerCommitError::InvalidPlan)?;
    let result = owner
        .commit_damage_for_bound_attacker(
            frozen.target,
            attacker,
            command,
            sub_ordinal,
            crate::foundation::OwnerDamageCommand {
                target: target.as_str().as_bytes(),
                occurrence: &[],
                binding: &frozen.binding,
                damage: *magnitude,
            },
        )
        .map_err(OwnerCommitError::Owner)?;
    Ok(OwnerCharmDamageResult {
        result,
        source: OwnerCharmDamageSource::CharmDamage,
    })
}

#[derive(Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum OwnerCommitError {
    InvalidPlan,
    Plan(AbilityError),
    Owner(crate::foundation::CarrierError),
}

#[cfg(test)]
mod tests {
    use super::super::TargetId;
    use super::*;

    #[test]
    fn invalid_effects_fail_before_fixture_mutation() -> Result<(), AbilityError> {
        let target = TargetId::new("target:fixture")?;
        for effect in [
            Effect::Damage {
                target: target.clone(),
                magnitude: 0,
            },
            Effect::Damage {
                target: target.clone(),
                magnitude: -7,
            },
            Effect::Heal {
                target: target.clone(),
                magnitude: 0,
            },
            Effect::Heal {
                target: target.clone(),
                magnitude: -7,
            },
        ] {
            let mut fixture_health = BTreeMap::from([(target.as_str().to_owned(), 23)]);
            assert_eq!(
                apply_fixture_effect(&mut fixture_health, &effect),
                Err(AbilityError::InvalidMagnitude)
            );
            assert_eq!(fixture_health.get(target.as_str()), Some(&23));
            assert_eq!(fixture_health.len(), 1);
        }
        Ok(())
    }
}
