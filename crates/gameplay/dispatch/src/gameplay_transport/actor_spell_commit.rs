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
    wheel_shielding: Vec<(ExactActorRef, u32)>,
    preprocessed_caster: Option<(ExactActorRef, GameSessionId, PlayerSpellState)>,
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

    /// Item spell leech consumes only actual staged damage, in the same owner install.
    pub(in crate::gameplay_transport) fn stage_item_spell_leech(
        &mut self,
        physical: &StagedSpellBatch,
        rates: (u32, u32),
        bounty: &[crate::gameplay_transport::task_board::bounty_talisman::SpellLeech],
    ) -> Result<(), Error> {
        if rates == (0, 0) && bounty.is_empty() {
            return Ok(());
        }
        let damage = self
            .batch
            .effects
            .iter()
            .filter(|effect| {
                crate::gameplay_transport::task_board::bounty_talisman::damage_effect(
                    &effect.change,
                )
            })
            .collect::<Vec<_>>();
        let affected = damage.len();
        if affected == 0 {
            return Ok(());
        }
        let mut gains = (0u32, 0u32);
        for effect in damage {
            let receipt = physical
                .preview_effect_receipts()
                .iter()
                .chain(self.health_receipts.iter())
                .find(|receipt| {
                    receipt.target == effect.target
                        && receipt.sub_ordinal == effect.sub_ordinal
                        && receipt.health.is_some()
                })
                .and_then(|receipt| receipt.health.as_ref())
                .ok_or(Error::InvalidBatch)?;
            if !receipt.applied {
                continue;
            }
            let real = receipt
                .health_before
                .saturating_sub(receipt.health_after.max(0))
                .max(0);
            let amount = |rate: u32| {
                (real as f64 * f64::from(rate.min(u32::from(u16::MAX))) / 10000.0
                    * (0.1 * affected as f64 + 0.9)
                    / affected as f64)
                    .round()
                    .clamp(0.0, real as f64) as u32
            };
            let bonus = bounty
                .iter()
                .find(|(target, ordinal, _)| {
                    *target == effect.target && *ordinal == effect.sub_ordinal
                })
                .map_or(0, |(_, _, rate)| u32::from(*rate));
            // Canary's combat callback adds its rate before the shared round/AoE cap.
            gains.0 = gains
                .0
                .saturating_add(amount(rates.0.saturating_add(bonus)));
            gains.1 = gains.1.saturating_add(amount(rates.1));
        }
        let caster = self
            .replacements
            .iter_mut()
            .find(|r| r.actor == self.batch.caster)
            .ok_or(Error::InvalidBatch)?;
        caster.next.apply_batch_leech(gains.0, gains.1);
        Ok(())
    }

    pub fn validate_current(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
    ) -> Result<(), SpellCastDisposition> {
        let caster = runtime
            .player_control_facts(self.batch.caster, self.batch.command.game_session_id())
            .map_err(|_| SpellCastDisposition::Rejected)?;
        if caster.control_loss.is_some() && !runtime.qualified_deferred_batch(&self.batch) {
            return Err(SpellCastDisposition::Rejected);
        }
        if let Some((actor, session, before)) = &self.preprocessed_caster
            && states.get(runtime, *actor, *session) != Some(before)
        {
            return Err(SpellCastDisposition::Rejected);
        }
        for (actor, shielding) in &self.wheel_shielding {
            let replacement = self
                .replacements
                .iter()
                .find(|r| r.actor == *actor)
                .ok_or(SpellCastDisposition::Rejected)?;
            let now = self
                .batch
                .now_us
                .map(|time| time.get())
                .or_else(|| self.batch.now_ms.checked_mul(1_000))
                .ok_or(SpellCastDisposition::Rejected)?;
            if replacement
                .before
                .wheel_battle_instinct(runtime, *actor, now)
                .map(|bonus| bonus.shielding)
                != Some(*shielding)
            {
                return Err(SpellCastDisposition::Rejected);
            }
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
    stage_player_batch_with_magnitude(runtime, states, batch, caster_successor, None)
}

/// The unchanged final player-vitals owner accepts a preprocessed magnitude only after
/// the same numerical owner proves every result against current physical/player state.
pub fn stage_player_batch_with_magnitude(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    batch: &OwnerCombatBatch,
    caster_successor: Option<PlayerSpellState>,
    magnitude: Option<&crate::spell::magnitude_owner::PreparedMagnitudeOwner>,
) -> Result<PlayerBatchPreflight, SpellCastDisposition> {
    if let Some(magnitude) = magnitude {
        magnitude
            .qualify_preprocessed_player_batch(runtime, batch, &|actor| {
                states
                    .actors
                    .iter()
                    .find(|(held, session, state)| {
                        *held == actor && states.get(runtime, *held, *session) == Some(state)
                    })
                    .map(|(_, _, state)| state)
            })
            .map_err(|_| SpellCastDisposition::Rejected)?;
    }
    stage_player_batch_inner(
        runtime,
        states,
        batch,
        caster_successor,
        magnitude.is_some(),
        None,
    )
}

pub fn stage_player_batch_with_attack(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    batch: &OwnerCombatBatch,
    attack: &crate::spell::cast::PreparedWheelAttackHit,
) -> Result<PlayerBatchPreflight, SpellCastDisposition> {
    if !attack.qualify_batch(runtime, batch, &|actor, session| {
        states.get(runtime, actor, session)
    }) {
        return Err(SpellCastDisposition::Rejected);
    }
    stage_player_batch_inner(runtime, states, batch, None, true, Some(attack))
}

/// A due Combat has no new resource payment, but consumes its current Focus window.
/// Its scheduler token and numerical predecessor remain independent required proofs.
pub(super) fn stage_due_player_batch(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    batch: &OwnerCombatBatch,
    magnitude: &crate::spell::magnitude_owner::PreparedMagnitudeOwner,
) -> Result<PlayerBatchPreflight, SpellCastDisposition> {
    if batch.anchor.is_some() || !runtime.qualified_deferred_batch(batch) {
        return Err(SpellCastDisposition::Rejected);
    }
    let mut preflight =
        stage_player_batch_with_magnitude(runtime, states, batch, None, Some(magnitude))?;
    let session = batch.command.game_session_id();
    let index = states
        .index(batch.caster, session)
        .ok_or(SpellCastDisposition::Rejected)?;
    if let Some(replacement) = preflight.replacements.iter_mut().find(|r| r.index == index) {
        magnitude
            .apply_wheel_cast_state(&mut replacement.next)
            .map_err(|_| SpellCastDisposition::Rejected)?;
    } else {
        let before = states
            .get(runtime, batch.caster, session)
            .ok_or(SpellCastDisposition::Rejected)?;
        let mut next = before.clone();
        magnitude
            .apply_wheel_cast_state(&mut next)
            .map_err(|_| SpellCastDisposition::Rejected)?;
        if next != *before {
            next.advance_batch_revision()?;
            preflight.replacements.push(Replacement {
                index,
                actor: batch.caster,
                session,
                before: before.clone(),
                next,
            });
        }
    }
    preflight.validate_current(runtime, states)?;
    Ok(preflight)
}

fn stage_player_batch_inner(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    batch: &OwnerCombatBatch,
    caster_successor: Option<PlayerSpellState>,
    preprocessed: bool,
    attack: Option<&crate::spell::cast::PreparedWheelAttackHit>,
) -> Result<PlayerBatchPreflight, SpellCastDisposition> {
    let preprocessed_caster = if preprocessed {
        let session = batch.command.game_session_id();
        Some((
            batch.caster,
            session,
            states
                .get(runtime, batch.caster, session)
                .ok_or(SpellCastDisposition::Rejected)?
                .clone(),
        ))
    } else {
        None
    };
    let mut deaths = Vec::new();
    let mut health_receipts = Vec::new();
    health_receipts
        .try_reserve(batch.effects.len())
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let mut replacements: Vec<Replacement> = Vec::new();
    let mut wheel_shielding = Vec::new();
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
                let now = batch
                    .now_us
                    .map(|time| time.get())
                    .or_else(|| batch.now_ms.checked_mul(1_000))
                    .ok_or(SpellCastDisposition::Rejected)?;
                let shielding = replacement
                    .next
                    .wheel_battle_instinct(runtime, *actor, now)
                    .ok_or(SpellCastDisposition::Rejected)?
                    .shielding;
                wheel_shielding.push((*actor, shielding));
                let (next, damage) = if let Some(attack) = attack {
                    attack.stage_target_hit(&replacement.next, now)
                } else if preprocessed {
                    crate::spell::actor_conditions::stage_preprocessed_player_hit(
                        &replacement.next,
                        magnitude,
                        now,
                    )
                } else {
                    crate::spell::actor_conditions::stage_creature_hit_in_world(
                        runtime,
                        *actor,
                        &replacement.next,
                        magnitude,
                        now,
                    )
                }?;
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
        wheel_shielding,
        preprocessed_caster,
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
