//! AI-4 (`GAME-AI-01-ACTION-INTEGRATION-FIRST-CREATURE-SLICE-V1` §4.6, §4.7): one creature bite
//! through Ability, issued by a typed AI issuer.
//!
//! The issuer is the creature's `ExactActorRef` and the occurrence is its think occurrence
//! `(issuer, think sequence)` (§4.2). The Channel owner revalidates everything at commit: the
//! issuer is a live creature generation, the target is the committed player of its GameSession,
//! the two stand one tile apart on one floor, the target is outside the PvE re-entry protection
//! window and the bite interval has elapsed. The damage goes through an Ability `EffectPlan` with
//! `ProposalSource::Ai` and lowers the player's runtime-actor-local health (SPELL-D2), never below
//! 1 (D54). The cooldown is written in the same owner step as the health.
//!
//! The first result of an occurrence is kept, accepted or rejected: a retry returns it and never
//! applies damage twice, and a rejected bite is never buffered for later. AI reads the result only
//! through its next snapshot.

use super::{intent::AiAbilityAdapter, RevisionSet};
use super::{AbilityIntent, AbilityOccurrence, CommitGroup, Effect, EffectPlan, ProposalSource};
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};

/// Retained bite state per Channel owner: one entry per live creature. The D57 envelope bounds
/// live creatures to 16 spawn sources x 4 creatures (§4.9 derived bound, 64).
pub(crate) const CREATURE_BITE_LEDGER_MAX: usize = 64;

/// The bite's content inputs (§4.8): the interval and the magnitude. Code never invents them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CreatureBiteDefinition {
    interval_micros: u64,
    magnitude: u32,
}

impl CreatureBiteDefinition {
    /// `None` for a zero interval or a zero magnitude; content validation rejects
    /// such a definition before it reaches the owner.
    pub(crate) fn new(interval_micros: u64, magnitude: u32) -> Option<Self> {
        (interval_micros > 0 && magnitude > 0).then_some(Self {
            interval_micros,
            magnitude,
        })
    }
}

/// A typed AI bite proposal (§4.6). Built only by [`AiAbilityAdapter::bite`]; its origin grants no
/// authority, and the owner resolves both actors again at commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AiBiteIntent {
    pub(super) issuer: ExactActorRef,
    pub(super) think_sequence: u64,
    pub(super) target: ExactActorRef,
    pub(super) target_session: GameSessionId,
}

impl AiAbilityAdapter {
    /// The bite a think occurrence `(issuer, think_sequence)` proposes against `target`.
    pub(crate) fn bite(
        issuer: ExactActorRef,
        think_sequence: u64,
        target: ExactActorRef,
        target_session: GameSessionId,
    ) -> AiBiteIntent {
        AiBiteIntent {
            issuer,
            think_sequence,
            target,
            target_session,
        }
    }
}

/// The PvE re-entry protection fact of the target (`DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_
/// DECISION.md`), which the owner reads from the protection owner in the same work item as the
/// commit: the end of the target's active window, or `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReentryProtection {
    pub(crate) protected_until: Option<SemanticTimeMicros>,
}

impl ReentryProtection {
    const fn protects_at(self, now: SemanticTimeMicros) -> bool {
        match self.protected_until {
            Some(until) => now.get() < until.get(),
            None => false,
        }
    }
}

/// D54: the health a creature hit leaves. Health never drops below 1; `applied` is the clamped
/// amount actually removed (0 when the target is already at 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FlooredDamage {
    pub(crate) applied: u32,
    pub(crate) health_after: u32,
}

pub(crate) const fn floor_creature_damage(health: u32, magnitude: u32) -> FlooredDamage {
    let health_after = if health <= 1 {
        health
    } else {
        let remaining = health.saturating_sub(magnitude);
        if remaining < 1 {
            1
        } else {
            remaining
        }
    };
    FlooredDamage {
        applied: health - health_after,
        health_after,
    }
}

/// A committed bite: the requested and applied (clamped) amounts and the target's resulting
/// vitals revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AppliedBite {
    pub(crate) requested: u32,
    pub(crate) damage: FlooredDamage,
    pub(crate) vitals_revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BiteRejection {
    /// The issuer is not a live creature generation of this owner.
    StaleIssuer,
    /// The target is not the committed player of its GameSession, or has no vitals.
    StaleTarget,
    /// The actors are not one tile apart on one floor under one position context.
    OutOfRange,
    /// The target is inside its PvE re-entry protection window.
    TargetProtected,
    CoolingDown,
    /// The Ability plan refused the bite.
    InvalidPlan,
    /// A retry named another target for the same think occurrence.
    OccurrenceConflict,
    /// An older think occurrence of the issuer after a newer one resolved.
    OccurrenceSuperseded,
    /// No retained entry could be made for a new issuer.
    LedgerFull,
}

/// The Channel owner's vitals of its present player actors (SPELL-D2). The vitals owner
/// implements it; a bite reaches the player's health only through this one method.
pub(crate) trait CreatureBiteVitals {
    /// Lower the committed player's health by one floored creature hit (D54) in one vitals
    /// write. `None` changes nothing.
    fn apply_creature_damage(
        &mut self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        target_session: GameSessionId,
        magnitude: u32,
        now: SemanticTimeMicros,
    ) -> Option<(FlooredDamage, u64)>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BiteEntry {
    issuer: ExactActorRef,
    ready_at: SemanticTimeMicros,
    /// The first result of the latest resolved think occurrence: (sequence, target, result).
    last: Option<(u64, ExactActorRef, Result<AppliedBite, BiteRejection>)>,
}

/// The Channel owner's bite cooldowns and occurrence results, beside `ChannelRuntimeV1` and used
/// only under its lock.
#[derive(Debug, Default)]
pub(crate) struct CreatureBiteLedger {
    entries: Vec<BiteEntry>,
}

impl CreatureBiteLedger {
    /// Drop the issuer's entry on its death or despawn. A no-op for an unknown issuer.
    pub(crate) fn retire(&mut self, issuer: ExactActorRef) {
        self.entries.retain(|entry| entry.issuer != issuer);
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    fn entry_index(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
    ) -> Result<usize, BiteRejection> {
        if let Some(index) = self.entries.iter().position(|entry| entry.issuer == issuer) {
            return Ok(index);
        }
        self.entries
            .retain(|entry| runtime.contains_live_creature(entry.issuer));
        if self.entries.len() >= CREATURE_BITE_LEDGER_MAX || self.entries.try_reserve(1).is_err() {
            return Err(BiteRejection::LedgerFull);
        }
        self.entries.push(BiteEntry {
            issuer,
            ready_at: SemanticTimeMicros::from_micros(0),
            last: None,
        });
        Ok(self.entries.len() - 1)
    }
}

/// One AI bite as one Channel-owner work item (§4.6, §4.7). A stale issuer is rejected before
/// anything is read or kept. Otherwise the first result of the think occurrence is kept and
/// returned for every retry of it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn commit_ai_bite(
    ledger: &mut CreatureBiteLedger,
    runtime: &ChannelRuntimeV1,
    vitals: &mut dyn CreatureBiteVitals,
    intent: AiBiteIntent,
    definition: CreatureBiteDefinition,
    revisions: RevisionSet,
    protection: ReentryProtection,
    now: SemanticTimeMicros,
) -> Result<AppliedBite, BiteRejection> {
    if !runtime.contains_live_creature(intent.issuer) {
        return Err(BiteRejection::StaleIssuer);
    }
    let index = ledger.entry_index(runtime, intent.issuer)?;
    let entry = ledger.entries[index];
    if let Some((sequence, target, result)) = entry.last {
        if intent.think_sequence < sequence {
            return Err(BiteRejection::OccurrenceSuperseded);
        }
        if intent.think_sequence == sequence {
            return if intent.target == target {
                result
            } else {
                Err(BiteRejection::OccurrenceConflict)
            };
        }
    }
    let result = resolve_and_apply(
        runtime, vitals, &entry, intent, definition, revisions, protection, now,
    );
    let entry = &mut ledger.entries[index];
    if result.is_ok() {
        entry.ready_at = now.saturating_add_micros(definition.interval_micros);
    }
    entry.last = Some((intent.think_sequence, intent.target, result));
    result
}

#[allow(clippy::too_many_arguments)]
fn resolve_and_apply(
    runtime: &ChannelRuntimeV1,
    vitals: &mut dyn CreatureBiteVitals,
    entry: &BiteEntry,
    intent: AiBiteIntent,
    definition: CreatureBiteDefinition,
    revisions: RevisionSet,
    protection: ReentryProtection,
    now: SemanticTimeMicros,
) -> Result<AppliedBite, BiteRejection> {
    runtime
        .player_control_facts(intent.target, intent.target_session)
        .map_err(|_| BiteRejection::StaleTarget)?;
    let issuer_at = runtime
        .read_actor_position(intent.issuer)
        .map_err(|_| BiteRejection::OutOfRange)?;
    let target_at = runtime
        .read_actor_position(intent.target)
        .map_err(|_| BiteRejection::OutOfRange)?;
    if issuer_at.context() != target_at.context()
        || !adjacent(issuer_at.position(), target_at.position())
    {
        return Err(BiteRejection::OutOfRange);
    }
    if protection.protects_at(now) {
        return Err(BiteRejection::TargetProtected);
    }
    if now < entry.ready_at {
        return Err(BiteRejection::CoolingDown);
    }
    let plan = bite_plan(intent, definition, revisions).ok_or(BiteRejection::InvalidPlan)?;
    let magnitude = match plan.effects() {
        [Effect::Damage { magnitude, .. }] => {
            u32::try_from(*magnitude).map_err(|_| BiteRejection::InvalidPlan)?
        }
        _ => return Err(BiteRejection::InvalidPlan),
    };
    let (damage, vitals_revision) = vitals
        .apply_creature_damage(
            runtime,
            intent.target,
            intent.target_session,
            magnitude,
            now,
        )
        .ok_or(BiteRejection::StaleTarget)?;
    Ok(AppliedBite {
        requested: magnitude,
        damage,
        vitals_revision,
    })
}

/// Range 1 (§4.6): one tile apart, diagonals included, on one floor.
fn adjacent(from: MovementLocalPosition, to: MovementLocalPosition) -> bool {
    let dx = (i64::from(to.x) - i64::from(from.x)).abs();
    let dy = (i64::from(to.y) - i64::from(from.y)).abs();
    from.floor == to.floor && dx.max(dy) == 1
}

/// The bite as one Ability plan: an AI proposal of one damage effect on one resolved target,
/// committed atomically by the Channel owner under the think occurrence's identity.
fn bite_plan(
    intent: AiBiteIntent,
    definition: CreatureBiteDefinition,
    revisions: RevisionSet,
) -> Option<EffectPlan> {
    let issuer = format!("actor:{}", hex(&intent.issuer.placement_identity()));
    let target = format!("actor:{}", hex(&intent.target.placement_identity()));
    let occurrence_id = format!(
        "ai-bite:{}:{}",
        hex(&intent.issuer.placement_identity()),
        intent.think_sequence
    );
    let occurrence = AbilityOccurrence::new(&occurrence_id, revisions).ok()?;
    let proposal =
        AbilityIntent::resolve(ProposalSource::Ai, &issuer, &[&target], &[&target]).ok()?;
    let effect = Effect::damage(&target, i64::from(definition.magnitude)).ok()?;
    let group = CommitGroup::atomic("channel-owner", &occurrence_id).ok()?;
    EffectPlan::immediate(occurrence, proposal, vec![effect], Vec::new(), group).ok()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creature_damage_never_takes_health_below_one() {
        let cases = [
            (185, 10, 10, 175),
            (11, 10, 10, 1),
            (10, 10, 9, 1),
            (5, u32::MAX, 4, 1),
            (1, 7, 0, 1),
        ];
        for (health, magnitude, applied, health_after) in cases {
            assert_eq!(
                floor_creature_damage(health, magnitude),
                FlooredDamage {
                    applied,
                    health_after
                }
            );
        }
    }

    #[test]
    fn bite_definition_rejects_zero_inputs() {
        assert!(CreatureBiteDefinition::new(0, 5).is_none());
        assert!(CreatureBiteDefinition::new(2_000_000, 0).is_none());
        assert!(CreatureBiteDefinition::new(2_000_000, 5).is_some());
    }
}
