//! One physical Channel transaction and the existing player-vitals owner.
//! The guards are held through each phase: the check phase and the infallible install phase. Every fallible check,
//! clone, allocation and effect application occurs before either owner mutates. A physical
//! receipt replay never charges resources or installs another player successor.

use super::{ChannelSpellStates, SpellCastDisposition};
use crate::foundation::StagedSpellBatch;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use crate::spell::cast::PlayerSpellState;
use crate::spell::combat_batch::{CombatBatchReceipt, Error, OwnerCombatBatch, OwnerCombatChange};

#[derive(Debug)]
struct Replacement {
    index: usize,
    actor: ExactActorRef,
    session: GameSessionId,
    before: PlayerSpellState,
    next: PlayerSpellState,
}

/// Construction is private. Its retained values describe a prepared transaction; each
/// installation must independently compare the actual current actors and player state.
#[derive(Debug)]
pub struct PlayerBatchPreflight {
    batch: OwnerCombatBatch,
    replacements: Vec<Replacement>,
    health_receipts: Vec<crate::foundation::runtime_actor_spell_types::EffectReceipt>,
    pub(in crate::gameplay_transport) deaths:
        Vec<(ExactActorRef, GameSessionId, super::PlayerDeath)>,
    pub(in crate::gameplay_transport) pvp_locks: Vec<(ExactActorRef, GameSessionId, bool)>,
}

impl crate::foundation::runtime_actor_spell_types::player_proof_seal::Sealed
    for PlayerBatchPreflight
{
}

impl crate::foundation::runtime_actor_spell_types::PlayerBatchProof for PlayerBatchPreflight {
    fn matches_batch(&self, batch: &OwnerCombatBatch) -> bool {
        self.matches_batch(batch)
    }
    fn health_receipt(
        &self,
        target: ExactActorRef,
        sub_ordinal: u16,
    ) -> Option<crate::foundation::OwnerDamageResult> {
        self.health_receipts
            .iter()
            .find(|effect| effect.target == target && effect.sub_ordinal == sub_ordinal)
            .and_then(|effect| effect.health)
    }
}

impl PlayerBatchPreflight {
    /// Stage teleport pacification beside the existing paid/victim successors. Installation
    /// remains atomic with the sealed relocation, including unknown-COMMIT reconciliation.
    pub(super) fn stage_stairhop(
        &mut self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        actor: ExactActorRef,
        now: oteryn_simulation_determinism::SemanticTimeMicros,
    ) -> Result<(), SpellCastDisposition> {
        use oteryn_gameplay_spell_rules::cast::{STAIRHOP_COOLDOWN_KEY, STAIRHOP_MICROS};
        let Some(index) = states.actors.iter().position(|(held, session, _)| {
            *held == actor && states.get(runtime, *held, *session).is_some()
        }) else {
            // No cast state means no spell or auto-attack can execute for this actor.
            return Ok(());
        };
        let until = now
            .checked_add(STAIRHOP_MICROS)
            .map_err(|_| SpellCastDisposition::Rejected)?;
        if !self.replacements.iter().any(|r| r.index == index) {
            let (_, session, before) = &states.actors[index];
            self.replacements.push(Replacement {
                index,
                actor,
                session: *session,
                before: before.clone(),
                next: before.clone(),
            });
        }
        if let Some(replacement) = self.replacements.iter_mut().find(|r| r.index == index) {
            replacement
                .next
                .cooldowns
                .groups
                .insert(STAIRHOP_COOLDOWN_KEY.into(), until);
        }
        Ok(())
    }

    pub fn matches_batch(&self, batch: &OwnerCombatBatch) -> bool {
        &self.batch == batch
    }

    /// The borrowing check that completes an already allocated caster successor with the
    /// genuine training receipt-qualified payment, without rerunning or reallocating combat
    /// effects (ARCH-SPELL-LOCK-2 §1.6).
    pub fn check_rebind_training(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        qualified_paid: &PlayerSpellState,
    ) -> Result<(), SpellCastDisposition> {
        self.validate_current(runtime, states)?;
        let anchor = self
            .batch
            .anchor
            .as_ref()
            .ok_or(SpellCastDisposition::Rejected)?;
        let replacement = self
            .caster_replacement()
            .ok_or(SpellCastDisposition::Rejected)?;
        replacement
            .next
            .check_rebind_staged_training(&replacement.before, anchor, qualified_paid)
            .map_err(|_| SpellCastDisposition::Rejected)
    }

    /// The infallible move, only after [`Self::check_rebind_training`] passed under the same
    /// guards.
    #[allow(
        clippy::expect_used,
        reason = "post-validation commit invariant; phase 1 proved the caster replacement"
    )]
    pub fn install_rebind_training(&mut self, qualified_paid: PlayerSpellState) {
        let caster = self.batch.caster;
        let session = self.batch.command.game_session_id();
        self.replacements
            .iter_mut()
            .find(|value| value.actor == caster && value.session == session)
            .expect("checked caster replacement")
            .next
            .install_rebind_staged_training(qualified_paid);
    }

    fn caster_replacement(&self) -> Option<&Replacement> {
        self.replacements.iter().find(|value| {
            value.actor == self.batch.caster
                && value.session == self.batch.command.game_session_id()
        })
    }

    pub fn validate_current(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
    ) -> Result<(), SpellCastDisposition> {
        let caster = runtime
            .player_control_facts(self.batch.caster, self.batch.command.game_session_id())
            .map_err(|_| SpellCastDisposition::Rejected)?;
        if caster.control_loss.is_some() {
            return Err(SpellCastDisposition::Rejected);
        }
        for replacement in &self.replacements {
            let Some((actor, session, actual)) = states.actors.get(replacement.index) else {
                return Err(SpellCastDisposition::Rejected);
            };
            if *actor != replacement.actor
                || *session != replacement.session
                || actual != &replacement.before
                || states.get(runtime, *actor, *session) != Some(actual)
            {
                return Err(SpellCastDisposition::Rejected);
            }
        }
        Ok(())
    }

    // Only the compositor below calls this, after the unchanged exclusive owner borrow
    // has validated every index. No allocation, lookup, callback or fallible step follows.
    fn install(self, states: &mut ChannelSpellStates) {
        states.deaths.extend(self.deaths);
        states.pvp_locks.retain(|(actor, session, _)| {
            states
                .actors
                .iter()
                .any(|(a, s, _)| a == actor && s == session)
        });
        if let Some(now) = self.batch.now_us {
            for (actor, session, aggression) in self.pvp_locks {
                let lock = super::super::pvp_live::lock_mut(states, actor, session);
                lock.on_pvp_hit(now);
                if aggression {
                    lock.on_aggression(now);
                }
            }
        }
        for replacement in self.replacements {
            states.actors[replacement.index].2 = replacement.next;
        }
    }
}

pub fn stage_player_batch(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    batch: &OwnerCombatBatch,
    caster_successor: Option<PlayerSpellState>,
) -> Result<PlayerBatchPreflight, SpellCastDisposition> {
    let mut deaths = Vec::new();
    let mut health_receipts = Vec::new();
    health_receipts
        .try_reserve(batch.effects.len())
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let mut replacements: Vec<Replacement> = Vec::new();
    replacements
        .try_reserve(batch.effects.len().saturating_add(1))
        .map_err(|_| SpellCastDisposition::Rejected)?;
    if let Some(anchor) = &batch.anchor {
        let session = batch.command.game_session_id();
        let before = states
            .get(runtime, batch.caster, session)
            .ok_or(SpellCastDisposition::Rejected)?;
        let next = caster_successor.ok_or(SpellCastDisposition::Rejected)?;
        if !next.paid_successor_of(before, anchor) {
            return Err(SpellCastDisposition::Rejected);
        }
        replacements.push(Replacement {
            index: states
                .index(batch.caster, session)
                .ok_or(SpellCastDisposition::Rejected)?,
            actor: batch.caster,
            session,
            before: before.clone(),
            next,
        });
    } else if caster_successor.is_some() {
        return Err(SpellCastDisposition::Rejected);
    }
    for effect in &batch.effects {
        if runtime.contains_live_creature(effect.target) {
            continue;
        }
        let index = states
            .actors
            .iter()
            .position(|(actor, _, _)| *actor == effect.target)
            .ok_or(SpellCastDisposition::Rejected)?;
        let (actor, session, _) = &states.actors[index];
        let before = states
            .get(runtime, *actor, *session)
            .ok_or(SpellCastDisposition::Rejected)?;
        if !replacements.iter().any(|r| r.index == index) {
            let mut next = before.clone();
            next.advance_batch_revision()?;
            replacements.push(Replacement {
                index,
                actor: *actor,
                session: *session,
                before: before.clone(),
                next,
            });
        }
        let replacement = replacements
            .iter_mut()
            .find(|r| r.index == index)
            .ok_or(SpellCastDisposition::Rejected)?;
        match &effect.change {
            OwnerCombatChange::Damage {
                target_atom,
                magnitude,
            } => {
                if *target_atom != crate::spell::combat_execution::actor_atom(*actor)
                    || states.is_dead(*actor)
                    || batch.caster == *actor
                {
                    return Err(SpellCastDisposition::TargetIllegal);
                }
                let magnitude =
                    u32::try_from(*magnitude).map_err(|_| SpellCastDisposition::Rejected)?;
                let health_before = replacement.next.vitals().health;
                let (next, damage) = crate::spell::actor_conditions::stage_creature_hit(
                    &replacement.next,
                    magnitude,
                    batch
                        .now_ms
                        .checked_mul(1000)
                        .ok_or(SpellCastDisposition::Rejected)?,
                )?;
                replacement.next = next;
                health_receipts.push(
                    crate::foundation::runtime_actor_spell_types::EffectReceipt {
                        target: *actor,
                        sub_ordinal: effect.sub_ordinal,
                        health: Some(crate::foundation::OwnerDamageResult {
                            applied: true,
                            health_before: i64::from(health_before),
                            health_after: i64::from(damage.health_after),
                        }),
                    },
                );
                if damage.health_after == 0 && !deaths.iter().any(|(a, _, _)| a == actor) {
                    deaths.push((
                        *actor,
                        *session,
                        super::PlayerDeath {
                            occurrence: (states.mint_death)()
                                .ok_or(SpellCastDisposition::Rejected)?,
                            cell: runtime
                                .read_actor_position(*actor)
                                .map_err(|_| SpellCastDisposition::Rejected)?
                                .position(),
                        },
                    ));
                }
            }
            OwnerCombatChange::Heal {
                target_atom,
                magnitude,
            } => {
                if *target_atom != crate::spell::combat_execution::actor_atom(*actor) {
                    return Err(SpellCastDisposition::Rejected);
                }
                let health_before = replacement.next.vitals().health;
                replacement.next.apply_batch_heal(*magnitude)?;
                health_receipts.push(
                    crate::foundation::runtime_actor_spell_types::EffectReceipt {
                        target: *actor,
                        sub_ordinal: effect.sub_ordinal,
                        health: Some(crate::foundation::OwnerDamageResult {
                            applied: true,
                            health_before: i64::from(health_before),
                            health_after: i64::from(replacement.next.vitals().health),
                        }),
                    },
                );
            }
            OwnerCombatChange::PlayerConditions { expected, next } => {
                replacement.next.apply_batch_conditions(expected, next)?;
            }
            change if crate::spell::actor_conditions::supports_condition_change(change) => {
                crate::spell::actor_conditions::apply_combat_change(
                    &mut replacement.next,
                    change,
                    batch.now_ms,
                )?;
            }
            // PvP has its own accepted owner and legality. This player-spell path cannot
            // silently acquire that authority or turn a creature AI override into player state.
            _ => return Err(SpellCastDisposition::TargetIllegal),
        }
    }
    let preflight = PlayerBatchPreflight {
        batch: batch.clone(),
        replacements,
        health_receipts,
        deaths,
        pvp_locks: Vec::new(),
    };
    preflight.validate_current(runtime, states)?;
    Ok(preflight)
}

/// Proof that [`check_owner_batch`] passed for one staged batch and preflight under the held
/// guards. Only that check builds it, and only [`install_owner_batch`] consumes it.
#[derive(Debug)]
pub struct CheckedOwnerBatch {
    _sealed: (),
}

pub fn commit_owner_batch(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    staged: StagedSpellBatch,
    preflight: Option<PlayerBatchPreflight>,
) -> Result<CombatBatchReceipt, Error> {
    let checked = check_owner_batch(runtime, states, &staged, preflight.as_ref())?;
    Ok(install_owner_batch(
        runtime, states, staged, preflight, checked,
    ))
}

/// The borrowing check of [`commit_owner_batch`] (ARCH-SPELL-LOCK-2 §1.6): it changes no slot,
/// no player state, no staged batch and no preflight.
pub fn check_owner_batch(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    staged: &StagedSpellBatch,
    preflight: Option<&PlayerBatchPreflight>,
) -> Result<CheckedOwnerBatch, Error> {
    if !staged.will_apply() {
        runtime.check_spell_batch_commit(staged)?;
        return Ok(CheckedOwnerBatch { _sealed: () });
    }
    let preflight = preflight.ok_or(Error::PlayerVitalsOwnerRequired)?;
    preflight
        .validate_current(runtime, states)
        .map_err(|_| Error::SnapshotChanged)?;
    if staged.batch().effects.iter().any(|effect| {
        !runtime.contains_live_creature(effect.target)
            && matches!(effect.change, OwnerCombatChange::Damage { .. })
            && !preflight
                .pvp_locks
                .iter()
                .any(|(target, _, _)| *target == effect.target)
    }) {
        return Err(Error::PlayerVitalsOwnerRequired);
    }
    if !preflight.matches_batch(staged.batch()) {
        return Err(Error::InvalidBatch);
    }
    // With the preflight proven to match, the binding below cannot fail, and the bound batch
    // passes `check_spell_batch_commit` exactly when it passes the staged comparison.
    runtime.validate_staged_spell_batch(staged)?;
    Ok(CheckedOwnerBatch { _sealed: () })
}

/// The infallible install, only after [`check_owner_batch`] passed under the same held guards.
#[allow(
    clippy::expect_used,
    reason = "post-validation commit invariant; phase 1 proved the preflight and its binding"
)]
pub fn install_owner_batch(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    mut staged: StagedSpellBatch,
    preflight: Option<PlayerBatchPreflight>,
    _checked: CheckedOwnerBatch,
) -> CombatBatchReceipt {
    if !staged.will_apply() {
        return runtime.install_spell_batch(staged);
    }
    let preflight = preflight.expect("checked player preflight");
    staged
        .bind_player_preflight(&preflight)
        .expect("checked matching player preflight");
    let receipt = runtime.install_spell_batch(staged);
    if receipt.applied {
        preflight.install(states);
        states.owner_wake.notify_one();
    }
    receipt
}

#[cfg(test)]
#[path = "actor_spell_commit_tests.rs"]
mod tests;
