//! Qualified pre-Prey, pre-HP-cap direct-spell leech inputs. No client fields,
//! random re-rolls, or new persistence surface; the cast retains these on retry.
use crate::foundation::{ChannelRuntimeV1, ExactActorRef};
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::gameplay_transport::attack::charm_combat;
use crate::spell::combat_batch::{Error, OwnerCombatBatch, OwnerCombatChange};
use crate::spell::magnitude_owner::PreparedMagnitudeOwner;
use oteryn_gameplay_progression::charm_effects::{
    CharmCatalogueRead, CharmEffectOutcome, CharmHitSource, CharmHookEvent,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

#[derive(Debug, Clone)]
pub(in crate::gameplay_transport) struct Target {
    pub target: ExactActorRef,
    pub ordinal: u16,
    /// None preserves the existing actual-health-loss PvP policy. Monster
    /// damage is retained before Prey and before a lethal hit is clipped.
    pub damage: Option<u32>,
    pub rates: (u32, u32),
    pub affected: usize,
}

#[allow(
    clippy::too_many_arguments,
    reason = "cast, actor, catalogue and deterministic provenance remain explicit"
)]
pub(in crate::gameplay_transport) fn capture(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    catalogue: &dyn CharmCatalogueRead,
    batch: &OwnerCombatBatch,
    rates: (u32, u32),
    magnitude: Option<&PreparedMagnitudeOwner>,
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
) -> Result<Vec<Target>, Error> {
    if batch.effects.len() > crate::foundation::runtime_actor_spell_types::MAX_EFFECTS {
        return Err(Error::TooManyEffects);
    }
    let mut targets = Vec::new();
    targets
        .try_reserve(batch.effects.len())
        .map_err(|_| Error::AllocationFailed)?;
    for effect in &batch.effects {
        let damage = match effect.change {
            OwnerCombatChange::Damage { magnitude, .. } => magnitude,
            OwnerCombatChange::DamageWithHealing { damage, .. } => damage,
            _ => continue,
        };
        let mut target_rates = rates;
        let creature = runtime.companion_snapshot(effect.target).ok();
        if let Some(creature) = &creature {
            let outcomes = charm_combat::evaluate(
                states,
                runtime,
                catalogue,
                batch.caster,
                batch.command.game_session_id(),
                &creature.state.policy.definition_key,
                root,
                occurrence,
                CharmHookEvent::LeechCalculation {
                    source: CharmHitSource::CharacterAttack,
                },
            );
            for outcome in &outcomes {
                let (rate, percent) = match charm_combat::effect(outcome) {
                    Some(CharmEffectOutcome::LifeLeechBonus { percent }) => {
                        (&mut target_rates.0, percent)
                    }
                    Some(CharmEffectOutcome::ManaLeechBonus { percent }) => {
                        (&mut target_rates.1, percent)
                    }
                    _ => continue,
                };
                // Match the existing auto-attack contract: a charm augments,
                // rather than creates, the corresponding ordinary leech skill.
                if *rate > 0 {
                    *rate = rate.saturating_add(percent.hundredths());
                }
            }
        }
        targets.push(Target {
            target: effect.target,
            ordinal: effect.sub_ordinal,
            damage: runtime
                .contains_live_creature(effect.target)
                .then(|| u32::try_from(damage.max(0)).unwrap_or(u32::MAX)),
            rates: target_rates,
            affected: magnitude
                .and_then(|owner| owner.leech_target_count(effect.target))
                .unwrap_or(0),
        });
    }
    let fallback = targets.len();
    for target in &mut targets {
        if target.affected == 0 {
            target.affected = fallback;
        }
    }
    Ok(targets)
}
