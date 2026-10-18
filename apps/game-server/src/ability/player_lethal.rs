//! Source combat trait bridge into the existing native DEATH-2 player vitals owner.
//! Only ChannelSpellStates commits these receipts; attack consumers retain their own
//! bounded occurrence ordering ledger. A receipt is provenance, never current authority.
use crate::foundation::{
    ChannelId, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
    ScopeOwnershipGeneration, WorldId,
};

/// Historical source data, never current actor/owner authority. Attribution callers must
/// resolve this from the actual committing source, not from a fallback classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LethalSource {
    /// D1255: explicit missing historical carrier, never an Environment fallback.
    Unattributed,
    Character(crate::domain::CharacterId),
    SummonOf {
        owner: crate::domain::CharacterId,
        creature: ExactActorRef,
    },
    Creature(ExactActorRef),
    #[allow(dead_code)]
    // D1244 admits environment sources; condition caller is not activated here.
    Environment(crate::foundation::ConditionType),
}
impl LethalSource {
    /// Current owner lookup of an admitted source. Missing player lease refuses; a
    /// valid summon with unknown durable owner stays Creature as D1244 specifies.
    pub(crate) fn current_actor(runtime: &ChannelRuntimeV1, actor: ExactActorRef) -> Option<Self> {
        let census = runtime.positioned_actor_census().ok()?;
        let (_, _, session) = census.iter().find(|(current, _, _)| *current == actor)?;
        if let Some(session) = session {
            return Some(Self::Character(
                crate::domain::CharacterId::from_bytes(
                    *runtime
                        .bound_attacker_lease(actor, *session)
                        .ok()?
                        .character_id()
                        .as_bytes(),
                )
                .ok()?,
            ));
        }
        // Player summons/familiars use the existing companion owner, not the monster
        // summon book (whose parent admission is creature-only).
        if let Ok(companion) = runtime.companion_snapshot(actor)
            && let Some(master) = companion.state.master
        {
            let owner = runtime
                .bound_attacker_lease(master.actor, master.session)
                .ok()
                .and_then(|lease| {
                    crate::domain::CharacterId::from_bytes(*lease.character_id().as_bytes()).ok()
                });
            return Some(owner.map_or(Self::Creature(actor), |owner| Self::SummonOf {
                owner,
                creature: actor,
            }));
        }
        let master = runtime.native_summon_role(actor).ok()?;
        if let Some(master) = master {
            // D1244: a valid live summon with no known durable owner remains Creature.
            // Missing parent/lease never invents an owner; source validity was checked above.
            let owner = census
                .iter()
                .find(|(current, _, _)| *current == master)
                .and_then(|(_, _, session)| *session)
                .and_then(|session| runtime.bound_attacker_lease(master, session).ok())
                .and_then(|lease| {
                    crate::domain::CharacterId::from_bytes(*lease.character_id().as_bytes()).ok()
                });
            if let Some(owner) = owner {
                return Some(Self::SummonOf {
                    owner,
                    creature: actor,
                });
            }
        }
        Some(Self::Creature(actor))
    }
    /// Validate historical source data at the native HP boundary. Character membership
    /// does not replace the caller's original actual-source admission. The owning caller
    /// supplies environment data explicitly; it is never a lookup fallback.
    pub(crate) fn current(self, runtime: &ChannelRuntimeV1) -> Option<Self> {
        match self {
            Self::Creature(actor) => Self::current_actor(runtime, actor),
            Self::SummonOf { creature, .. } => {
                (Self::current_actor(runtime, creature)? == self).then_some(self)
            }
            Self::Character(character) => runtime
                .positioned_actor_census()
                .ok()?
                .iter()
                .filter_map(|(actor, _, session)| session.map(|session| (*actor, session)))
                .any(|(actor, session)| {
                    runtime
                        .bound_attacker_lease(actor, session)
                        .is_ok_and(|lease| lease.character_id().as_bytes() == character.as_bytes())
                })
                .then_some(self),
            Self::Environment(_) => Some(self),
            Self::Unattributed => None,
        }
    }
}
fn compare_lethal_actors(a: ExactActorRef, b: ExactActorRef) -> std::cmp::Ordering {
    (
        *a.world_id().as_bytes(),
        *a.channel_id().as_bytes(),
        a.scope_generation().get(),
        a.actor_local_id(),
        a.actor_local_generation(),
    )
        .cmp(&(
            *b.world_id().as_bytes(),
            *b.channel_id().as_bytes(),
            b.scope_generation().get(),
            b.actor_local_id(),
            b.actor_local_generation(),
        ))
}
impl Ord for LethalSource {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use LethalSource::*;
        match (*self, *other) {
            (Unattributed, Unattributed) => std::cmp::Ordering::Equal,
            (Unattributed, _) => std::cmp::Ordering::Less,
            (_, Unattributed) => std::cmp::Ordering::Greater,
            (Character(a), Character(b)) => a.cmp(&b),
            (
                SummonOf {
                    owner: a,
                    creature: x,
                },
                SummonOf {
                    owner: b,
                    creature: y,
                },
            ) => a.cmp(&b).then_with(|| compare_lethal_actors(x, y)),
            (Creature(a), Creature(b)) => compare_lethal_actors(a, b),
            (Environment(a), Environment(b)) => a.cmp(&b),
            (Character(_), _)
            | (SummonOf { .. }, Creature(_) | Environment(_))
            | (Creature(_), Environment(_)) => std::cmp::Ordering::Less,
            _ => std::cmp::Ordering::Greater,
        }
    }
}
impl PartialOrd for LethalSource {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LedgerEntry {
    /// Sum of the retained `contributions`; it never includes an expired hit.
    pub(crate) hp_total: u64,
    pub(crate) last_at_us: crate::foundation::owner_timer::SemanticTimeMicros,
    /// Per-hit window: oldest-first one-second buckets of (latest hit in bucket, HP).
    /// A bucket expires with its latest hit, so retention exceeds the exact per-hit
    /// window by under one second and never drops a contribution early.
    contributions:
        std::collections::VecDeque<(crate::foundation::owner_timer::SemanticTimeMicros, u64)>,
}
#[allow(dead_code)] // Historical fields are consumed by E after #2193; no E writer edit here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LethalAttribution {
    pub(crate) killer: LethalSource,
    /// The killer's retained five-minute HP total, including the final blow, so the
    /// death consumer can form the all-source denominator (`killer_hp` + assists).
    pub(crate) killer_hp: u64,
    pub(crate) assists: Vec<(LethalSource, u64)>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PlayerDamageLedger {
    entries: std::collections::BTreeMap<LethalSource, LedgerEntry>,
    last_observed: Option<crate::foundation::owner_timer::SemanticTimeMicros>,
    last_applied: Option<(
        LethalSource,
        crate::foundation::owner_timer::SemanticTimeMicros,
    )>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DamageLedgerError {
    NonMonotonicTime,
    Overflow,
    MissingFinalHit,
}
impl PlayerDamageLedger {
    const WINDOW_US: u64 = 300_000_000;
    const MAX_SOURCES: usize = 64;
    /// Bucket width for per-hit expiry; bounds a source to 301 buckets in the window.
    const BUCKET_US: u64 = 1_000_000;

    /// Called exactly once AFTER a fresh successful HP commit. Replay detection belongs
    /// before that commit; this ledger is not an occurrence memo and must not replace it.
    /// Zero applied HP cannot introduce a source. Refusals leave the entire ledger intact.
    pub(crate) fn observe(
        &mut self,
        source: LethalSource,
        hp_applied: u32,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Result<(), DamageLedgerError> {
        if self.last_observed.is_some_and(|last| now < last) {
            return Err(DamageLedgerError::NonMonotonicTime);
        }
        let in_window = |at: crate::foundation::owner_timer::SemanticTimeMicros| {
            now.get().saturating_sub(at.get()) <= Self::WINDOW_US
        };
        // Each hit expires on its own timestamp, not on the source's latest hit.
        let expired_hp = |entry: &LedgerEntry| {
            entry
                .contributions
                .iter()
                .take_while(|(at, _)| !in_window(*at))
                .fold(0_u64, |sum, (_, hp)| sum.saturating_add(*hp))
        };
        let total = self
            .entries
            .get(&source)
            .map_or(0, |entry| entry.hp_total.saturating_sub(expired_hp(entry)))
            .checked_add(u64::from(hp_applied))
            .ok_or(DamageLedgerError::Overflow)?;
        self.entries.retain(|_, entry| {
            let expired = expired_hp(entry);
            while entry
                .contributions
                .front()
                .is_some_and(|(at, _)| !in_window(*at))
            {
                entry.contributions.pop_front();
            }
            entry.hp_total = entry.hp_total.saturating_sub(expired);
            !entry.contributions.is_empty()
        });
        self.last_observed = Some(now);
        self.last_applied = (hp_applied > 0).then_some((source, now));
        if hp_applied == 0 {
            return Ok(());
        }
        if !self.entries.contains_key(&source) && self.entries.len() == Self::MAX_SOURCES {
            // BTreeMap order resolves equal-time ties, retaining the new actual final hit.
            if let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(key, entry)| (entry.last_at_us, **key))
                .map(|(key, _)| *key)
            {
                self.entries.remove(&oldest);
            }
        }
        let entry = self.entries.entry(source).or_insert_with(|| LedgerEntry {
            hp_total: 0,
            last_at_us: now,
            contributions: std::collections::VecDeque::new(),
        });
        match entry.contributions.back_mut() {
            Some((at, hp)) if at.get() / Self::BUCKET_US == now.get() / Self::BUCKET_US => {
                *at = now;
                *hp = hp.saturating_add(u64::from(hp_applied));
            }
            _ => entry.contributions.push_back((now, u64::from(hp_applied))),
        }
        entry.hp_total = total;
        entry.last_at_us = now;
        Ok(())
    }
    /// Snapshot immediately after observing the actual final hit. No current-authority
    /// claim is carried. The final source is separate from the HP-sorted other sources.
    pub(crate) fn snapshot(
        &self,
        killer: LethalSource,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Result<LethalAttribution, DamageLedgerError> {
        if self.last_applied != Some((killer, now)) {
            return Err(DamageLedgerError::MissingFinalHit);
        }
        let killer_hp = match self.entries.get(&killer) {
            Some(entry) if entry.last_at_us == now => entry.hp_total,
            _ => return Err(DamageLedgerError::MissingFinalHit),
        };
        let mut assists: Vec<_> = self
            .entries
            .iter()
            .filter(|(source, _)| **source != killer)
            .map(|(source, entry)| (*source, entry.hp_total))
            .collect();
        assists.sort_by(|(a, hp_a), (b, hp_b)| hp_b.cmp(hp_a).then_with(|| a.cmp(b)));
        Ok(LethalAttribution {
            killer,
            killer_hp,
            assists,
        })
    }
}

#[cfg(test)]
mod damage_ledger_tests {
    use super::*;
    use crate::foundation::owner_timer::SemanticTimeMicros;
    fn at(value: u64) -> SemanticTimeMicros {
        SemanticTimeMicros::from_micros(value)
    }
    fn source(value: u8) -> LethalSource {
        LethalSource::Environment(crate::foundation::ConditionType::SpellSkills(value))
    }
    #[test]
    fn exact_window_retains_boundary_and_prunes_one_microsecond_older()
    -> Result<(), DamageLedgerError> {
        let mut ledger = PlayerDamageLedger::default();
        ledger.observe(source(1), 10, at(0))?;
        ledger.observe(source(2), 20, at(1))?;
        ledger.observe(source(3), 1, at(300_000_001))?;
        assert!(!ledger.entries.contains_key(&source(1)));
        assert!(ledger.entries.contains_key(&source(2)));
        Ok(())
    }
    #[test]
    fn each_hit_expires_on_its_own_timestamp_not_the_source_latest_hit()
    -> Result<(), DamageLedgerError> {
        let mut ledger = PlayerDamageLedger::default();
        ledger.observe(source(1), 10, at(0))?;
        ledger.observe(source(1), 20, at(299_000_000))?;
        ledger.observe(source(2), 5, at(301_000_000))?;
        let snapshot = ledger.snapshot(source(2), at(301_000_000))?;
        assert_eq!(snapshot.killer_hp, 5);
        assert_eq!(snapshot.assists, vec![(source(1), 20)]);
        // A repeated attacker keeps only its in-window hits, including the final blow.
        ledger.observe(source(1), 7, at(302_000_000))?;
        assert_eq!(
            ledger.entries.get(&source(1)).map(|entry| entry.hp_total),
            Some(27)
        );
        ledger.observe(source(1), 1, at(600_000_000))?;
        assert_eq!(
            ledger.entries.get(&source(1)).map(|entry| entry.hp_total),
            Some(8)
        );
        assert!(ledger.entries.contains_key(&source(2)));
        Ok(())
    }
    #[test]
    fn continuous_hits_stay_bounded_and_within_one_bucket_of_the_window()
    -> Result<(), DamageLedgerError> {
        let mut ledger = PlayerDamageLedger::default();
        for i in 0..2_000_u64 {
            ledger.observe(source(1), 1, at(i * 250_000))?;
        }
        let entry = ledger
            .entries
            .get(&source(1))
            .ok_or(DamageLedgerError::MissingFinalHit)?;
        assert!(entry.contributions.len() <= 301);
        // Last hit at 499.75 s. Exact per-hit retention keeps 1201 hits (>= 199.75 s);
        // bucket 199 s (latest hit 199.75 s) adds its three earlier sub-second hits.
        assert_eq!(entry.contributions.len(), 301);
        assert_eq!(entry.hp_total, 1_204);
        assert_eq!(
            entry.contributions.iter().map(|(_, hp)| *hp).sum::<u64>(),
            entry.hp_total
        );
        Ok(())
    }
    #[test]
    fn bounded_oldest_eviction_retains_new_final_hit_and_hp_sorted_assists()
    -> Result<(), DamageLedgerError> {
        let mut ledger = PlayerDamageLedger::default();
        for i in 0..64 {
            ledger.observe(source(i), u32::from(i) + 1, at(u64::from(i)))?;
        }
        ledger.observe(source(64), 1, at(64))?;
        assert_eq!(ledger.entries.len(), 64);
        assert!(!ledger.entries.contains_key(&source(0)));
        let snapshot = ledger.snapshot(source(64), at(64))?;
        assert_eq!(snapshot.killer, source(64));
        assert_eq!(snapshot.killer_hp, 1);
        assert_eq!(snapshot.assists.first(), Some(&(source(63), 64)));
        assert_eq!(snapshot.assists.len(), 63);
        Ok(())
    }
    #[test]
    fn snapshot_retains_killer_prior_damage_for_the_all_source_total()
    -> Result<(), DamageLedgerError> {
        let mut ledger = PlayerDamageLedger::default();
        ledger.observe(source(1), 96, at(1))?;
        ledger.observe(source(2), 4, at(2))?;
        ledger.observe(source(1), 3, at(3))?;
        let snapshot = ledger.snapshot(source(1), at(3))?;
        assert_eq!(snapshot.killer, source(1));
        assert_eq!(snapshot.killer_hp, 99);
        assert_eq!(snapshot.assists, vec![(source(2), 4)]);
        let all: u64 = snapshot.killer_hp + snapshot.assists.iter().map(|(_, hp)| hp).sum::<u64>();
        assert_eq!(all, 103);
        Ok(())
    }
    #[test]
    fn clock_regression_and_overflow_refuse_without_pruning_or_partial_update()
    -> Result<(), DamageLedgerError> {
        let mut ledger = PlayerDamageLedger::default();
        ledger.observe(source(1), 1, at(10))?;
        let before = ledger.clone();
        assert_eq!(
            ledger.observe(source(2), 1, at(9)),
            Err(DamageLedgerError::NonMonotonicTime)
        );
        assert_eq!(ledger, before);
        ledger
            .entries
            .get_mut(&source(1))
            .ok_or(DamageLedgerError::MissingFinalHit)?
            .hp_total = u64::MAX;
        let before = ledger.clone();
        assert_eq!(
            ledger.observe(source(1), 1, at(11)),
            Err(DamageLedgerError::Overflow)
        );
        assert_eq!(ledger, before);
        Ok(())
    }
    #[test]
    fn zero_hp_cannot_create_killer_and_missing_or_delayed_final_hit_refuses()
    -> Result<(), DamageLedgerError> {
        let mut ledger = PlayerDamageLedger::default();
        ledger.observe(source(1), 0, at(1))?;
        assert_eq!(
            ledger.snapshot(source(1), at(1)),
            Err(DamageLedgerError::MissingFinalHit)
        );
        ledger.observe(source(2), 7, at(2))?;
        assert_eq!(
            ledger.snapshot(source(1), at(2)),
            Err(DamageLedgerError::MissingFinalHit)
        );
        ledger.observe(source(1), 0, at(2))?;
        assert_eq!(
            ledger.snapshot(source(2), at(2)),
            Err(DamageLedgerError::MissingFinalHit)
        );
        assert_eq!(
            ledger.snapshot(source(2), at(3)),
            Err(DamageLedgerError::MissingFinalHit)
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerDamageSourceContext {
    SourceAttack,
    CreatureMelee {
        creature: ExactActorRef,
    },
    Condition {
        kind: crate::foundation::ConditionType,
        source: LethalSource,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlayerLethalReceipt {
    actor: ExactActorRef,
    session: GameSessionId,
    world: WorldId,
    channel: ChannelId,
    generation: ScopeOwnershipGeneration,
    occurrence: [u8; 16],
    revision: u64,
    position: MovementLocalPosition,
    attribution: LethalAttribution,
    source_context: PlayerDamageSourceContext,
}
impl PlayerLethalReceipt {
    /// Metadata from the SAME committed native DEATH-2 hit. No alternate mint/queue.
    /// Caller is the sole ChannelSpellStates bridge; this receipt is never authority.
    #[allow(
        clippy::too_many_arguments,
        reason = "native commit identity, occurrence, position and source provenance remain explicit"
    )]
    pub(crate) fn from_native_commit(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        revision: u64,
        occurrence: [u8; 16],
        position: MovementLocalPosition,
        attribution: LethalAttribution,
        source_context: PlayerDamageSourceContext,
    ) -> Self {
        let binding = runtime.binding();
        Self {
            actor,
            session,
            world: binding.world_id(),
            channel: binding.channel_id(),
            generation: binding.scope_generation(),
            occurrence,
            revision,
            position,
            attribution,
            source_context,
        }
    }
    pub(crate) const fn actor(&self) -> ExactActorRef {
        self.actor
    }
    pub(crate) const fn session(&self) -> GameSessionId {
        self.session
    }
    pub(crate) const fn world(&self) -> WorldId {
        self.world
    }
    pub(crate) const fn channel(&self) -> ChannelId {
        self.channel
    }
    pub(crate) const fn generation(&self) -> ScopeOwnershipGeneration {
        self.generation
    }
    pub(crate) const fn occurrence_bytes(&self) -> [u8; 16] {
        self.occurrence
    }
    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }
    pub(crate) const fn position(&self) -> MovementLocalPosition {
        self.position
    }
    #[allow(dead_code)] // E death writer consumes this historical data after #2193 merge.
    #[allow(dead_code)] // E death writer consumes this historical data after #2193 merge.
    pub(crate) const fn attribution(&self) -> &LethalAttribution {
        &self.attribution
    }
    #[allow(dead_code)] // Historical context for E's post-#2193 consumer.
    pub(crate) const fn source_context(&self) -> PlayerDamageSourceContext {
        self.source_context
    }
    /// Independently current carrier facts; never reconstruct current authority from receipt.
    pub(crate) fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
        let binding = runtime.binding();
        binding.world_id() == self.world
            && binding.channel_id() == self.channel
            && binding.scope_generation() == self.generation
            && runtime
                .player_control_facts(self.actor, self.session)
                .is_ok_and(|facts| facts.control_loss.is_none())
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlayerDamageReceipt {
    pub(crate) applied: u32,
    pub(crate) health_after: u32,
    pub(crate) vitals_revision: u64,
    pub(crate) death: Option<PlayerLethalReceipt>,
}
/// An MP-only owner commit; carries no HP mutation or player-death authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerManaDrainReceipt {
    pub(crate) applied: u32,
    pub(crate) mana_before: u32,
    pub(crate) mana_after: u32,
    pub(crate) vitals_revision: u64,
}
/// Native transient source heal, no mana/death/cooldown/condition mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerHealReceipt {
    pub(crate) applied: u32,
    pub(crate) health_before: u32,
    pub(crate) health_after: u32,
    pub(crate) max_health: u32,
    pub(crate) vitals_revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceAppearanceReceipt {
    pub(crate) actor: ExactActorRef,
    pub(crate) applied: bool,
}
pub(crate) struct SourceAppearanceTarget<'a> {
    pub(crate) actor: ExactActorRef,
    pub(crate) session: Option<GameSessionId>,
    pub(crate) facts: crate::foundation::ApplicationFacts<'a>,
    pub(crate) immunities: Vec<crate::foundation::ConditionType>,
}
pub(crate) trait PlayerLethalVitals {
    /// Source cast owns occurrence/replay and independently current tile/Combat admission.
    /// The actual Player and physical Creature stores are staged together or all refuse.
    fn apply_source_appearance_batch(
        &mut self,
        _runtime: &mut ChannelRuntimeV1,
        _fence: &crate::foundation::ScopeRuntimeFence,
        _stamp: crate::foundation::RuntimeWorkStamp,
        _source: ExactActorRef,
        _definitions: &[crate::foundation::ConditionDefinition],
        _targets: &[SourceAppearanceTarget<'_>],
    ) -> Option<Vec<SourceAppearanceReceipt>> {
        None
    }

    /// Independently current actor/session HP view for composed source preflight.
    /// No caller-supplied maximum or HP field becomes current authority.
    fn source_player_health(
        &self,
        _runtime: &ChannelRuntimeV1,
        _target: ExactActorRef,
        _session: GameSessionId,
    ) -> Option<(u32, u32, u64)> {
        None
    }

    /// Private source-qualified caller seam. Unsupported owners refuse, never synthesize HP.
    fn apply_source_player_heal(
        &mut self,
        _runtime: &mut ChannelRuntimeV1,
        _target: ExactActorRef,
        _session: GameSessionId,
        _magnitude: u32,
        _occurrence: &str,
        _now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<PlayerHealReceipt> {
        None
    }

    /// Secondary-only source-qualified area dispel. Caller owns cast replay/Combat admission.
    /// Unsupported owners refuse; this creates no HP/MP receipt.
    // Keep remove_attack_invisibility ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    fn remove_attack_invisibility(
        &mut self,
        _runtime: &mut ChannelRuntimeV1,
        _target: ExactActorRef,
        _session: GameSessionId,
        _source: ExactActorRef,
        _now: u64,
        _fence: &crate::foundation::ScopeRuntimeFence,
        _stamp: crate::foundation::RuntimeWorkStamp,
    ) -> bool {
        false
    }

    fn apply_attack_mana_drain(
        &mut self,
        _runtime: &mut ChannelRuntimeV1,
        _target: ExactActorRef,
        _session: GameSessionId,
        _magnitude: u32,
        _occurrence: &str,
        _now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<PlayerManaDrainReceipt> {
        None
    }

    /// An admitted secondary-only effect, with no primary HP receipt or synthetic hit.
    /// Owners without a condition store explicitly refuse.
    // Keep apply_attack_conditions ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    fn apply_attack_conditions(
        &mut self,
        _runtime: &mut ChannelRuntimeV1,
        _target: ExactActorRef,
        _session: GameSessionId,
        _occurrence: &str,
        _source: ExactActorRef,
        _definitions: &[crate::foundation::ConditionDefinition],
        _facts: &crate::foundation::ApplicationFacts<'_>,
        _immunities: &[crate::foundation::ConditionType],
        _fence: &crate::foundation::ScopeRuntimeFence,
        _stamp: crate::foundation::RuntimeWorkStamp,
    ) -> bool {
        false
    }

    /// Secondary definitions require independently current admission facts. Owners which do
    /// not implement atomic conditions must refuse before committing primary damage.
    // Keep apply_composite_attack_damage ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    fn apply_composite_attack_damage(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        occurrence: &str,
        _source: ExactActorRef,
        definitions: &[crate::foundation::ConditionDefinition],
        _facts: &crate::foundation::ApplicationFacts<'_>,
        _immunities: &[crate::foundation::ConditionType],
        fence: &crate::foundation::ScopeRuntimeFence,
        stamp: crate::foundation::RuntimeWorkStamp,
    ) -> Option<PlayerDamageReceipt> {
        let binding = runtime.binding();
        if !fence.is_current_for_scope(
            crate::foundation::RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
            binding.scope_generation(),
        ) || !fence.accepts_stamp(stamp)
        {
            return None;
        }

        if definitions.is_empty() {
            let source = LethalSource::current_actor(runtime, _source)?;
            self.apply_attack_damage(
                runtime,
                target,
                session,
                magnitude,
                occurrence,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(_facts.now),
                source,
            )
        } else {
            None
        }
    }

    /// Source-aware melee bridge. Unsupported owners refuse; the old source-free HP
    /// primitive cannot silently emit an unattributed production lethal result.
    #[allow(clippy::too_many_arguments)]
    fn apply_melee_attack_damage(
        &mut self,
        _runtime: &mut ChannelRuntimeV1,
        _target: ExactActorRef,
        _session: GameSessionId,
        _magnitude: u32,
        _occurrence: &str,
        _now: crate::foundation::owner_timer::SemanticTimeMicros,
        _creature: ExactActorRef,
    ) -> Option<PlayerDamageReceipt> {
        None
    }

    #[allow(clippy::too_many_arguments)] // D1244 keeps actual source separate from target/session/HP/occurrence/time.
    fn apply_attack_damage(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        occurrence: &str,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
        source: LethalSource,
    ) -> Option<PlayerDamageReceipt>;
}
