use super::*;

/// `COMBAT01-DAMAGE-RECEIPTS-PER-CREATURE-GENERATION` (`docs/contracts/RESOURCE_LIMITS_REGISTRY.json`;
/// D140, `docs/architecture/reviews/OTERYN_GAME_D4_MULTI_HIT_DAMAGE_RECEIPT_DECISION_2026-09-29.md`
/// section 4.1): at most this many committed damage receipts are retained at once per live
/// creature actor generation. Retention, not admission: an evictable oldest receipt is evicted to
/// admit a new distinct occurrence (D141); only 16 simultaneously non-evictable receipts refuse
/// one with [`CarrierError::DamageReceiptCapacityExceeded`].
pub const COMBAT01_DAMAGE_RECEIPTS_PER_CREATURE_GENERATION_MAX: usize = 16;

/// Mirror of the already-registered `ABILITY01-EFFECT-PLAN-ENTRIES` hard maximum
/// (`ability/mod.rs` `MAX_EFFECT_PLAN_ENTRIES`, no new row): the number of distinct
/// `sub_ordinal` values one command can produce. Foundation must not import Ability, so the
/// value is mirrored here and pinned to it by a test in `channel_owner_ability_commit_tests.rs`.
pub const ABILITY01_EFFECT_PLAN_ENTRIES_MAX: u16 = 2;

/// `(CharacterId, character_lease_generation, GameSessionId, CommandId sequence, sub_ordinal)`:
/// the carrier-derived replay identity of one attributed damage occurrence (D141 point 3).
pub type DamageOrigin = (CharacterId, u64, GameSessionId, u64, u16);

/// `(character_lease_generation, GameSessionId, sequence, sub_ordinal)`: one attacker's
/// high-water mark (D141 point 4). The lease generation is the per-character monotonic
/// generation of the durable `CharacterLease` (`CurrentCharacterGameplayFence::
/// character_lease_generation`), which is what proves at this mutation boundary which session is
/// current: a lower generation is a superseded session.
pub(super) type HighWater = (u64, GameSessionId, u64, u16);

/// D140/D142: the bounded, ephemeral, per-creature-generation receipt list and the owner
/// damage-application ordinal counter. Fresh on every admission, dropped with the slot.
///
/// Only the carrier appends receipts or advances the ordinal; no other crate can:
///
/// ```
/// use oteryn_gameplay_entities::carrier::DamageReceipts;
/// let _empty = DamageReceipts::default();
/// ```
///
/// ```compile_fail
/// use oteryn_gameplay_entities::carrier::DamageReceipts;
/// fn forge(receipts: &mut DamageReceipts) { receipts.next_ordinal = 0; }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DamageReceipts {
    pub(super) entries: Vec<OwnerCommitRecord>,
    pub(super) next_ordinal: u64,
    /// The owner clock (ms) of the turn that committed the lethal receipt, when that turn
    /// supplied one. Set once by the lethal commit; a replay never changes it.
    lethal_at_ms: Option<u64>,
    /// EQMOD2: source-owned streams have their own clock, never a player command number.
    pub(super) reflection_marks: std::collections::BTreeMap<[u8; 32], u64>,
}

impl DamageReceipts {
    /// Test builds only: the retained receipts, oldest first.
    #[cfg(any(test, feature = "test-support"))]
    pub fn entries(&self) -> &[OwnerCommitRecord] {
        &self.entries
    }

    /// Test builds only: the retained receipts, mutable so a test can stage a carrier
    /// whose receipt list was lost.
    #[cfg(any(test, feature = "test-support"))]
    pub fn entries_mut(&mut self) -> &mut Vec<OwnerCommitRecord> {
        &mut self.entries
    }

    /// Test builds only: the next owner damage-application ordinal.
    #[cfg(any(test, feature = "test-support"))]
    pub const fn next_ordinal(&self) -> u64 {
        self.next_ordinal
    }

    test_support_pub! {
        /// D144: the one retained receipt whose commit drove health to zero, if any.
        fn lethal(&self) -> Option<&OwnerCommitRecord> {
            self.entries.iter().find(|record| {
                record.result.applied
                    && record.result.health_before > 0
                    && record.result.health_after == 0
            })
        }
    }

    /// D141 point 4: the oldest evictable receipt. A receipt is evictable iff it is sequenced,
    /// its attacker is currently tracked with a high-water mark, and either its origin lease
    /// generation was superseded (fenced, and refused by [`DamageContributors::admission`], so
    /// it can never be re-applied) or the current session's mark covers its
    /// `(sequence, sub_ordinal)`. EQMOD2 reflections instead use the source stream's
    /// retained clock. Other unsequenced receipts are never evictable.
    fn oldest_evictable(&self, contributors: &DamageContributors) -> Option<usize> {
        self.entries.iter().position(|record| {
            if let Some((key, sequence)) = record.reflection {
                return self
                    .reflection_marks
                    .get(&key)
                    .is_some_and(|mark| *mark >= sequence);
            }
            let Some((character, lease, session, sequence, sub_ordinal)) = record.origin else {
                return false;
            };
            let Some((mark_lease, mark_session, mark_sequence, mark_sub)) =
                contributors.high_water(character)
            else {
                return false;
            };
            mark_lease > lease
                || (mark_lease == lease
                    && mark_session == session
                    && (mark_sequence, mark_sub) >= (sequence, sub_ordinal))
        })
    }
}

/// `COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE` (`docs/contracts/RESOURCE_LIMITS_REGISTRY.json`;
/// D132, `docs/architecture/reviews/OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md`
/// §4.3): at most this many distinct `CharacterId` contributors are tracked per live creature
/// actor. Past this bound, a creature's own damage/HP math stays completely unaffected -- only a
/// *new* (17th+) distinct attacker's damage stops being added to the map; every already-tracked
/// contributor keeps accumulating normally (fail-open for combat, fail-closed only for
/// attribution).
pub const COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX: usize = 16;

/// D132: one tracked attacker's running damage total against a live creature actor, and the
/// ordinal (this creature's own damage-application sequence) at which that total was last
/// reached. A contributor's total only ever increases, so "last reached" already is "first
/// reached this exact value" -- no separate history is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageContributor {
    pub(super) character: CharacterId,
    pub total: u64,
    pub(super) last_update_ordinal: u64,
    /// D141: this attacker's `(session, sequence, sub_ordinal)` high-water mark. A new current
    /// session replaces it outright; within one session it only ever rises.
    pub(super) high_water: Option<HighWater>,
    /// The latest owner clock (ms) at which this attacker's damage was recorded. A hit whose
    /// turn supplied no clock leaves it unchanged, so it never moves later than a known hit.
    pub(super) last_damage_at_ms: Option<u64>,
}

/// Original contributor data captured with a death; a CharacterId alone cannot rebind
/// a previous session's direct damage award to a successor session.
#[derive(Debug, Clone, Copy)]
pub struct RewardDamageShare {
    pub character: CharacterId,
    pub total: u64,
    pub lease_generation: Option<u64>,
    pub session: Option<GameSessionId>,
}

/// D132/D3-3: bounded, ephemeral, per-creature-generation damage-contributor accumulation.
/// Lives only inside its owning `Slot::CreatureOccupied` -- a fresh, empty map on every creature
/// admission (`admit_inner`), dropped with the slot on `remove` (administrative removal or
/// respawn). Never grows past `COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX` and never
/// initialized from, or retained across, a prior generation: a scope move drops whatever
/// in-progress accumulation was pending, exactly the accepted D52 loss ("Przepadają, bez
/// duplikatów" -- nothing duplicated, nothing silently completed by a later generation).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DamageContributors {
    pub entries: Vec<DamageContributor>,
}

impl DamageContributors {
    /// Attribute one already-applied, positive damage amount to `character`. A *new* distinct
    /// contributor past `COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX` is silently not tracked:
    /// the caller's own HP mutation already committed independently of this call and is never
    /// affected by it. An already-tracked contributor keeps accumulating regardless of map size.
    ///
    /// `ordinal` is the owner damage-application ordinal (D142) the carrier assigned to this
    /// commit at the same mutation boundary; this accumulator keeps no counter of its own.
    /// `high_water`, when supplied, replaces the attacker's mark (D141). `at_ms` is the
    /// committing owner turn's clock.
    pub(super) fn record(
        &mut self,
        character: CharacterId,
        damage: u64,
        ordinal: u64,
        high_water: Option<HighWater>,
        at_ms: Option<u64>,
    ) {
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|contributor| contributor.character == character)
        {
            existing.total = existing.total.saturating_add(damage);
            existing.last_update_ordinal = ordinal;
            if high_water.is_some() {
                existing.high_water = high_water;
            }
            existing.last_damage_at_ms = existing.last_damage_at_ms.max(at_ms);
            return;
        }
        if self.entries.len() >= COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX {
            return;
        }
        self.entries.push(DamageContributor {
            character,
            total: damage,
            last_update_ordinal: ordinal,
            high_water,
            last_damage_at_ms: at_ms,
        });
    }

    /// D141: `character`'s current high-water mark, if it is tracked and has one.
    pub(super) fn high_water(&self, character: CharacterId) -> Option<HighWater> {
        self.entries
            .iter()
            .find(|contributor| contributor.character == character)
            .and_then(|contributor| contributor.high_water)
    }

    /// D141 admission of a new attributed occurrence against its attacker's mark, ordered by
    /// the per-character `character_lease_generation` (which proves which session is current):
    /// a lower generation, or an equal one with a differing `GameSessionId`, is a superseded
    /// session (`SupersededAttackerSession`); an equal generation and session at or below the
    /// mark's `(sequence, sub_ordinal)` is `StaleAttackerSequence`; a higher generation is a
    /// genuinely newer session (FND-04B sections 16 and 21) and replaces the mark.
    fn admission(&self, attacker: AttackerCommand) -> Result<(), CarrierError> {
        let Some((lease, session, sequence, sub_ordinal)) = self.high_water(attacker.character)
        else {
            return Ok(());
        };
        match attacker.lease_generation.cmp(&lease) {
            std::cmp::Ordering::Less => Err(CarrierError::SupersededAttackerSession),
            std::cmp::Ordering::Greater => Ok(()),
            std::cmp::Ordering::Equal if attacker.session != session => {
                Err(CarrierError::SupersededAttackerSession)
            }
            std::cmp::Ordering::Equal
                if (attacker.sequence, attacker.sub_ordinal) <= (sequence, sub_ordinal) =>
            {
                Err(CarrierError::StaleAttackerSequence)
            }
            std::cmp::Ordering::Equal => Ok(()),
        }
    }

    /// D132's deterministic two-rule tie-break: the highest running total wins; among equal top
    /// totals, the contributor whose total *first reached* that value (the smaller
    /// `last_update_ordinal`) wins; a remaining same-ordinal tie resolves to the
    /// lexicographically lowest `CharacterId` byte sequence (`CharacterId`'s derived `Ord` is
    /// exactly that byte order, `crates/protocol-oteryn/src/lib.rs`'s
    /// `foundation_uuid_v7_id!`). A pure function of already-recorded state, independent of
    /// insertion or iteration order.
    pub(super) fn top_damage_character(&self) -> Option<CharacterId> {
        self.top_contributor()
            .map(|contributor| contributor.character)
    }

    /// The whole winning entry under the same tie-break as [`Self::top_damage_character`].
    fn top_contributor(&self) -> Option<DamageContributor> {
        self.entries.iter().copied().reduce(|best, candidate| {
            if is_more_preferred_contributor(&candidate, &best) {
                candidate
            } else {
                best
            }
        })
    }
}

/// `true` when `candidate` outranks `current_best` under D132's tie-break order (see
/// [`DamageContributors::top_damage_character`]).
fn is_more_preferred_contributor(
    candidate: &DamageContributor,
    current_best: &DamageContributor,
) -> bool {
    match candidate.total.cmp(&current_best.total) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => {
            match candidate
                .last_update_ordinal
                .cmp(&current_best.last_update_ordinal)
            {
                std::cmp::Ordering::Less => true,
                std::cmp::Ordering::Greater => false,
                std::cmp::Ordering::Equal => candidate.character < current_best.character,
            }
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
impl CommittedLethalReceipt {
    /// Test builds only: the carried projection, mutable so a test can present Combat with
    /// a receipt that no longer matches the owner commit.
    pub fn projection_mut(&mut self) -> &mut RuntimeCorpseProjection {
        &mut self.projection
    }
}

/// D2b: mints a fresh v7-shaped `ExperienceRewardOccurrence` byte pattern
/// (RFC 9562 version/variant nibbles; the remaining bits are a SHA-256
/// digest of the death's exact actor reference, the reward principal and the
/// current wall-clock millisecond, so two calls never collide). Called at
/// most once per (death, character): `reward_occurrence_inner` memoizes the
/// first result and every later call returns that same value.
fn mint_reward_occurrence_bytes(actor: ActorRef, character: [u8; 16]) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0);
    let digest = Sha256::new()
        .chain_update(b"oteryn:combat-death-xp-occurrence:v1")
        .chain_update(actor.world_id.as_bytes())
        .chain_update(actor.channel_id.as_bytes())
        .chain_update(ExactActorRef(actor).scope_generation().get().to_be_bytes())
        .chain_update(actor.actor_local_id.0.to_be_bytes())
        .chain_update(actor.actor_local_generation.0.to_be_bytes())
        .chain_update(character)
        .chain_update(millis.to_be_bytes())
        .finalize();
    let mut bytes = [0_u8; 16];
    bytes[0..6].copy_from_slice(&millis.to_be_bytes()[2..8]);
    bytes[6] = 0x70 | (digest[0] & 0x0f);
    bytes[7] = digest[1];
    bytes[8] = 0x80 | (digest[2] & 0x3f);
    bytes[9..16].copy_from_slice(&digest[3..10]);
    bytes
}

impl ChannelActorCarrier {
    /// D140-D142: up to [`COMBAT01_DAMAGE_RECEIPTS_PER_CREATURE_GENERATION_MAX`] receipts are
    /// retained in the creature's own slot. An identical replay of a retained occurrence returns
    /// the recorded transition without mutation. A new distinct occurrence against a live
    /// creature either applies (evicting the oldest evictable receipt when the list is full) or
    /// is refused as stale / over capacity. All validation, checked arithmetic and fallible
    /// allocation precede the first mutation. Administrative removal drops the receipts with the
    /// slot and emits no death or corpse event.
    ///
    /// An attributed commit (`attacker` is `Some`) is identified solely by the carrier-derived
    /// `(CharacterId, GameSessionId, sequence, sub_ordinal)`; `command.occurrence` is not read.
    /// An unattributed commit keeps the caller's opaque `occurrence` identity and is never
    /// evictable.
    // Structural sibling supplies no current content authority; registered encounter actors refuse.
    pub fn commit_creature_damage_inner(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor: ActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail_before_write: bool,
    ) -> Result<OwnerDamageResult, CarrierError> {
        self.commit_creature_damage_with_pin_inner(
            continuity,
            actor,
            command,
            attacker,
            fail_before_write,
            None,
            Some(ABILITY01_EFFECT_PLAN_ENTRIES_MAX),
            None,
        )
    }
    /// `max_sub_ordinal` and `now_ms` as in [`Self::commit_creature_damage_inner_limited`];
    /// `current_pin` revalidates registered shared-HP encounter actors.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn commit_creature_damage_with_pin_inner(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail_before_write: bool,
        current_pin: Option<&ChannelContentPin>,
        max_sub_ordinal: Option<u16>,
        now_ms: Option<u64>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        if let Some(hooks) = self.encounter_hooks.take() {
            let result = hooks.damage(
                self,
                continuity,
                actor_ref,
                command,
                attacker,
                fail_before_write,
                current_pin,
                max_sub_ordinal,
                now_ms,
            );
            self.encounter_hooks = Some(hooks);
            if let Some(result) = result {
                return result;
            }
        }
        let (index, staged, result) = self.prepare_creature_damage_inner_bounded(
            continuity,
            actor_ref,
            command,
            attacker,
            fail_before_write,
            max_sub_ordinal,
            false,
            now_ms,
        )?;
        if let Some(staged) = staged {
            self.slots[index] = staged;
        }
        Ok(result)
    }
    pub(super) fn prepare_creature_damage_inner(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail_before_write: bool,
    ) -> Result<(usize, Option<Slot>, OwnerDamageResult), CarrierError> {
        self.prepare_creature_damage_inner_bounded(
            continuity,
            actor_ref,
            command,
            attacker,
            fail_before_write,
            Some(ABILITY01_EFFECT_PLAN_ENTRIES_MAX),
            false,
            None,
        )
    }
    /// ATTACK-1b: refuses a swing ordinal at or below the attacker's mark for the same lineage
    /// (`lease`, session and sequence) unless its exact receipt is retained, which the commit
    /// then replays idempotently.
    pub(super) fn swing_lineage_admission(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        attacker: AttackerCommand,
    ) -> Result<(), CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        let Slot::CreatureOccupied {
            committed,
            damage_contributors,
            ..
        } = &self.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if committed
            .entries
            .iter()
            .any(|record| record.origin == Some(attacker.origin()))
        {
            return Ok(());
        }
        match damage_contributors.high_water(attacker.character) {
            Some((lease, session, sequence, sub_ordinal))
                if lease == attacker.lease_generation
                    && session == attacker.session
                    && sequence == attacker.sequence
                    && attacker.sub_ordinal <= sub_ordinal =>
            {
                Err(CarrierError::StaleAttackerSequence)
            }
            _ => Ok(()),
        }
    }
    // Preserve main95 source batch limits/deferred high-water semantics. This entry
    // has no current content pin, so registered shared-HP encounter actors refuse.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn commit_creature_damage_inner_bounded(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail_before_write: bool,
        max_sub_ordinal: u16,
        deferred: bool,
        now_ms: Option<u64>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        self.commit_creature_damage_inner_limited(
            continuity,
            actor_ref,
            command,
            attacker,
            fail_before_write,
            Some(max_sub_ordinal),
            deferred,
            now_ms,
        )
    }
    /// `max_sub_ordinal` is the exclusive bound on an attributed `sub_ordinal`; `None` admits
    /// the whole `u16` range (ATTACK-1b swing ordinals). `now_ms` is the committing owner
    /// turn's clock, recorded on the receipt.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn commit_creature_damage_inner_limited(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail_before_write: bool,
        max_sub_ordinal: Option<u16>,
        deferred: bool,
        now_ms: Option<u64>,
    ) -> Result<OwnerDamageResult, CarrierError> {
        if self
            .encounter_hooks
            .as_ref()
            .is_some_and(|hooks| hooks.contains(ExactActorRef(actor_ref)))
        {
            return Err(CarrierError::PositionContextMismatch);
        }
        let (index, staged, result) = self.prepare_creature_damage_inner_bounded(
            continuity,
            actor_ref,
            command,
            attacker,
            fail_before_write,
            max_sub_ordinal,
            deferred,
            now_ms,
        )?;
        if let Some(staged) = staged {
            self.slots[index] = staged;
        }
        Ok(result)
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_creature_damage_inner_bounded(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        command: OwnerDamageCommand<'_>,
        attacker: Option<AttackerCommand>,
        fail_before_write: bool,
        max_sub_ordinal: Option<u16>,
        deferred: bool,
        now_ms: Option<u64>,
    ) -> Result<(usize, Option<Slot>, OwnerDamageResult), CarrierError> {
        let OwnerDamageCommand {
            target,
            occurrence,
            binding,
            damage,
        } = command;
        if let (Some(attacker), Some(max)) = (attacker, max_sub_ordinal)
            && attacker.sub_ordinal >= max
        {
            return Err(CarrierError::SubOrdinalOutOfRange);
        }
        let index = self.validate_ref(continuity, actor_ref)?;
        self.assert_slot_spell_unreserved(index)?;
        if binding.is_empty()
            || attacker.is_some_and(|attacker| attacker.lease_generation == 0)
            || (attacker.is_none() && (occurrence.is_empty() || occurrence.contains(&0)))
        {
            // An unsequenced occurrence containing the NUL delimiter would make its identity
            // ambiguous against `binding.split(0)`, so it is rejected here.
            return Err(CarrierError::InvalidCommitBinding);
        }
        if binding.len() > MAX_OWNER_COMMIT_BINDING_BYTES
            || (attacker.is_none() && occurrence.len() > MAX_OWNER_COMMIT_BINDING_BYTES)
        {
            return Err(CarrierError::CommitBindingTooLarge);
        }
        if attacker.is_none()
            && (!binding.starts_with(occurrence) || binding.get(occurrence.len()) != Some(&0))
        {
            return Err(CarrierError::InvalidCommitBinding);
        }
        if damage <= 0 {
            return Err(CarrierError::InvalidDamage);
        }
        let Slot::CreatureOccupied {
            generation,
            target_identity,
            health,
            committed,
            damage_contributors,
            ..
        } = &self.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if *generation != actor_ref.actor_local_generation.0 {
            return Err(CarrierError::StaleActorGeneration);
        }
        if target_identity.as_ref() != target {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let prior = committed
            .entries
            .iter()
            .find(|record| match (attacker, record.origin) {
                (Some(attacker), Some(origin)) => attacker.origin() == origin,
                (None, None) => record.binding.split(|byte| *byte == 0).next() == Some(occurrence),
                _ => false,
            });
        if let Some(prior) = prior {
            if prior.binding.as_ref() != binding || prior.damage != damage {
                return Err(CarrierError::PlanConflict);
            }
            return Ok((
                index,
                None,
                OwnerDamageResult {
                    applied: false,
                    ..prior.result
                },
            ));
        }
        if *health == 0 {
            return Err(CarrierError::CreatureNotActionable);
        }
        if let Some(attacker) = attacker {
            if deferred {
                if let Some((lease, session, _, _)) =
                    damage_contributors.high_water(attacker.character)
                    && (attacker.lease_generation < lease
                        || (attacker.lease_generation == lease && attacker.session != session))
                {
                    return Err(CarrierError::SupersededAttackerSession);
                }
            } else {
                damage_contributors.admission(attacker)?;
            }
        }
        let read_len = committed.entries.len();
        let evict = if read_len >= COMBAT01_DAMAGE_RECEIPTS_PER_CREATURE_GENERATION_MAX {
            Some(
                committed
                    .oldest_evictable(damage_contributors)
                    .ok_or(CarrierError::DamageReceiptCapacityExceeded)?,
            )
        } else {
            None
        };
        // D142: the owner damage-application ordinal, assigned to distinct occurrences only.
        let ordinal = committed.next_ordinal;
        let next_ordinal = ordinal
            .checked_add(1)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        let next = health
            .checked_sub(damage)
            .ok_or(CarrierError::DamageOverflow)?
            .max(0);
        let result = OwnerDamageResult {
            applied: true,
            health_before: *health,
            health_after: next,
        };
        let binding = copy_bounded_binding(binding)?;
        let receipt = OwnerCommitRecord {
            binding,
            damage,
            result,
            ordinal,
            origin: attacker.map(AttackerCommand::origin),
            reflection: None,
        };
        if fail_before_write {
            return Err(CarrierError::InjectedCommitFailure);
        }
        // Recheck current authority and local generation at the mutation
        // boundary, independent of Ability's earlier resolution snapshot.
        let write_index = self.validate_ref(continuity, actor_ref)?;
        if write_index != index {
            return Err(CarrierError::StaleActorGeneration);
        }
        let mut staged = self.slots[index].clone();
        let Slot::CreatureOccupied {
            generation,
            target_identity,
            health,
            committed,
            damage_contributors,
            companion,
            ..
        } = &mut staged
        else {
            return Err(CarrierError::StaleActorGeneration);
        };
        if *generation != actor_ref.actor_local_generation.0
            || target_identity.as_ref() != target
            || *health != result.health_before
            || committed.next_ordinal != ordinal
            || committed.entries.len() != read_len
        {
            return Err(CarrierError::StaleActorGeneration);
        }
        // Every fallible allocation happens before the first mutation below.
        if evict.is_none() {
            committed
                .entries
                .try_reserve(1)
                .map_err(|_| CarrierError::CommitAllocationFailed)?;
        }
        if attacker.is_some()
            && damage_contributors.entries.len() < COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX
        {
            damage_contributors
                .entries
                .try_reserve(1)
                .map_err(|_| CarrierError::CommitAllocationFailed)?;
        }
        if let Some(evicted) = evict {
            committed.entries.remove(evicted);
        }
        committed.entries.push(receipt);
        committed.next_ordinal = next_ordinal;
        if next == 0 && *health > 0 {
            committed.lethal_at_ms = now_ms;
        }
        *health = next;
        // Frozen source Monster::drainHealth removes invisibility on an applied
        // health drain. This is the target's actual condition owner; players
        // retain their invisibility and historical replay returns above.
        if let Some(companion) = companion {
            companion
                .conditions
                .remove_type(oteryn_gameplay_domain::domain::condition::ConditionType::Invisible);
        }
        if next == 0 {
            committed.conditions.die();
        }
        // D132/D3-3: attribute this applied hit to its attacker only now, at the sole mutation
        // boundary, after every replay/staleness check above -- an idempotent replay of a
        // retained occurrence returns earlier and never reaches here, so it can never
        // double-attribute. D141 sets the attacker's high-water mark from the same commit.
        if let Some(attacker) = attacker {
            // Codex P1: credit the HP actually removed, not the requested/planned damage -- an
            // overkill hit (e.g. 100 damage on 1 remaining HP) must only ever credit the 1 HP
            // that could actually be removed (`next <= health` always, since `damage > 0`).
            let removed = result.health_before.saturating_sub(result.health_after);
            damage_contributors.record(
                attacker.character,
                u64::try_from(removed).unwrap_or(u64::MAX),
                ordinal,
                Some(match damage_contributors.high_water(attacker.character) {
                    Some(mark)
                        if deferred
                            && mark.0 == attacker.lease_generation
                            && mark.1 == attacker.session
                            && (mark.2, mark.3) > (attacker.sequence, attacker.sub_ordinal) =>
                    {
                        mark
                    }
                    _ => attacker.high_water(),
                }),
                now_ms,
            );
        }
        Ok((index, Some(staged), result))
    }
    /// D132/D3-3: read-only, non-mutating lookup of `actor`'s current
    /// `DamageContributors::top_damage_character()` in this generation's still-live slot. See
    /// [`CurrentOwnerCombatDeath::top_damage_character`].
    pub(super) fn top_damage_character_inner(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<Option<CharacterId>, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        let Slot::CreatureOccupied {
            generation,
            damage_contributors,
            ..
        } = &self.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if *generation != actor_ref.actor_local_generation.0 {
            return Err(CarrierError::StaleActorGeneration);
        }
        Ok(damage_contributors.top_damage_character())
    }
    /// The winner's identity is the lease generation and session of its D141 mark, and its
    /// slot is the committed player slot bound to exactly that lease and session. A winner
    /// without a mark, or whose slot is gone or rebound to a successor, is `Gone`: nothing
    /// looks it up by `CharacterId` alone.
    pub(super) fn top_damage_contributor_inner(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<TopDamagePrincipal, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        let Slot::CreatureOccupied {
            generation,
            damage_contributors,
            ..
        } = &self.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if *generation != actor_ref.actor_local_generation.0 {
            return Err(CarrierError::StaleActorGeneration);
        }
        let Some(winner) = damage_contributors.top_contributor() else {
            return Ok(TopDamagePrincipal::Untracked);
        };
        let Some((lease_generation, session, _, _)) = winner.high_water else {
            return Ok(TopDamagePrincipal::Gone);
        };
        let slot = self.attackers.iter().find_map(|entry| {
            let lease = entry.authority.lease?;
            if lease.character_id() != winner.character || lease.generation() != lease_generation {
                return None;
            }
            match self.slots.get(entry.index)? {
                Slot::Occupied {
                    generation,
                    game_session_id: Some(bound),
                    committed: true,
                    ..
                } if *generation == entry.generation && *bound == session => {
                    Some((entry.index, *generation))
                }
                _ => None,
            }
        });
        let Some((slot_index, slot_generation)) = slot else {
            return Ok(TopDamagePrincipal::Gone);
        };
        Ok(TopDamagePrincipal::Present(TopDamageContributor {
            character: winner.character,
            lease_generation,
            session,
            actor: ExactActorRef(ActorRef::from_owner_parts(
                self.world_id,
                self.channel_id,
                self.scope_generation,
                ActorLocalId(
                    u32::try_from(slot_index + 1)
                        .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
                ),
                ActorLocalGeneration(slot_generation),
            )),
            last_damage_at_ms: winner.last_damage_at_ms,
        }))
    }
    pub(super) fn committed_lethal_receipt_inner(
        &self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
    ) -> Result<CommittedLethalReceipt, CarrierError> {
        let index = self.validate_ref(continuity, actor_ref)?;
        if let Some(existing) = self
            .corpse_projections
            .iter()
            .find(|projection| projection.occurrence.actor == ExactActorRef(actor_ref))
        {
            return Ok(CommittedLethalReceipt {
                projection: RuntimeCorpseProjection {
                    occurrence: CreatureDeathOccurrenceRef {
                        actor: existing.occurrence.actor,
                        commit_binding: copy_bounded_binding(&existing.occurrence.commit_binding)?,
                        damage: existing.occurrence.damage,
                        health_before: existing.occurrence.health_before,
                    },
                    position: existing.position,
                    death_at_ms: existing.death_at_ms,
                },
            });
        }
        let Slot::CreatureOccupied {
            generation,
            position,
            health,
            committed,
            ..
        } = &self.slots[index]
        else {
            return Err(CarrierError::CommittedLethalUnavailable);
        };
        if *generation != actor_ref.actor_local_generation.0 {
            return Err(CarrierError::StaleActorGeneration);
        }
        let death_at_ms = committed.lethal_at_ms;
        // D144: the unique retained receipt whose commit drove health to zero.
        let Some(committed) = committed.lethal().filter(|_| *health == 0) else {
            return Err(CarrierError::CommittedLethalUnavailable);
        };
        let position = position.ok_or(CarrierError::PositionUnavailable)?;
        Ok(CommittedLethalReceipt {
            projection: RuntimeCorpseProjection {
                occurrence: CreatureDeathOccurrenceRef {
                    actor: ExactActorRef(actor_ref),
                    commit_binding: copy_bounded_binding(&committed.binding)?,
                    damage: committed.damage,
                    health_before: committed.result.health_before,
                },
                position,
                death_at_ms,
            },
        })
    }
    pub(super) fn validate_lethal_receipt(
        &self,
        continuity: &NamespaceContinuityGuard,
        receipt: &CommittedLethalReceipt,
    ) -> Result<(), CarrierError> {
        let projection = &receipt.projection;
        let actor_ref = projection.occurrence.actor.0;
        let index = self.validate_ref(continuity, actor_ref)?;
        let Slot::CreatureOccupied {
            generation,
            position: Some(position),
            health,
            committed,
            ..
        } = &self.slots[index]
        else {
            return Err(CarrierError::StaleActorGeneration);
        };
        if *generation != actor_ref.actor_local_generation.0 || committed.entries.is_empty() {
            return Err(CarrierError::StaleActorGeneration);
        }
        // D144: the unique retained receipt whose commit drove health to zero.
        let Some(committed) = committed.lethal() else {
            return Err(CarrierError::CorpseReceiptMismatch);
        };
        if *health != 0
            || committed.binding.as_ref() != projection.occurrence.commit_binding.as_ref()
            || committed.damage != projection.occurrence.damage
            || committed.result.health_before != projection.occurrence.health_before
            || *position != projection.position
        {
            return Err(CarrierError::CorpseReceiptMismatch);
        }
        Ok(())
    }
    /// AI-2: per actor now, not a channel-wide singleton (§4.1 envelope). A different actor's
    /// already-projected death never blocks or is disturbed by this call.
    pub(super) fn project_committed_lethal_inner(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        receipt: CommittedLethalReceipt,
        fail_before_write: bool,
        fail_after_write: bool,
    ) -> Result<&RuntimeCorpseProjection, CarrierError> {
        let actor = receipt.projection.occurrence.actor;
        self.validate_ref(continuity, actor.0)?;
        if self.is_native_summon(actor) {
            return Err(CarrierError::SummonHasNoRewards);
        }
        if self
            .corpse_projections
            .iter()
            .any(|projection| projection.occurrence.actor == actor)
        {
            return self.existing_corpse_projection(&receipt);
        }
        self.validate_summon_actor(continuity, actor)?;
        self.validate_lethal_receipt(continuity, &receipt)?;
        if fail_before_write {
            return Err(CarrierError::InjectedCorpseProjectionFailure);
        }
        self.cascade_native_summon_deaths(continuity, actor)?;
        let projected = receipt.projection;
        let captured_sealed_key = projected.occurrence.death_key();
        // Bounded FIFO retention (`MAX_RETAINED_CORPSE_PROJECTIONS`): a retained projection
        // deliberately outlives its actor's slot (lost-response replay, above), so this Vec is
        // not otherwise pruned; evict the oldest unpinned entry rather than grow without bound
        // (a pinned one is an accepted kill settlement's, Codex 4248192466). The
        // evicted death's memoized reward occurrence goes with it: a reward occurrence never
        // outlives, and is never evicted independently of, its projection, so a retained
        // projection can never re-mint a second occurrence for the same death.
        self.retain_corpse_projection(projected);
        if let Some(hooks) = self.encounter_hooks.as_mut() {
            hooks.projected_death(actor, captured_sealed_key);
        }
        if fail_after_write {
            return Err(CarrierError::InjectedCorpseResponseFailure);
        }
        self.corpse_projections
            .last()
            .ok_or(CarrierError::CorpseProjectionConflict)
    }
    fn existing_corpse_projection(
        &self,
        receipt: &CommittedLethalReceipt,
    ) -> Result<&RuntimeCorpseProjection, CarrierError> {
        let actor = receipt.projection.occurrence.actor;
        let existing = self
            .corpse_projections
            .iter()
            .find(|projection| projection.occurrence.actor == actor)
            .ok_or(CarrierError::CorpseProjectionConflict)?;
        if existing == &receipt.projection {
            Ok(existing)
        } else {
            Err(CarrierError::CorpseProjectionConflict)
        }
    }
    /// Codex 4248192466: the eviction takes the oldest unpinned projection, so a projection that
    /// an accepted kill settlement still binds to survives any number of newer deaths (a full
    /// kill queue's, a familiar's, an encounter's). At most `MAX_RETAINED_CORPSE_PROJECTIONS`
    /// unpinned and `MAX_PINNED_CORPSE_PROJECTIONS` pinned projections are retained.
    pub(super) fn retain_corpse_projection(&mut self, projected: RuntimeCorpseProjection) {
        while self.unpinned_corpse_projections() >= MAX_RETAINED_CORPSE_PROJECTIONS {
            let pins = &self.corpse_projection_pins;
            let Some(index) = self
                .corpse_projections
                .iter()
                .position(|projection| !pins.contains(&projection.occurrence.actor))
            else {
                break;
            };
            let evicted = self.corpse_projections.remove(index).occurrence.actor.0;
            self.death_reward_occurrences
                .retain(|reward| reward.actor != evicted);
        }
        self.corpse_projections.push(projected);
    }
    fn unpinned_corpse_projections(&self) -> usize {
        self.corpse_projections
            .iter()
            .filter(|projection| {
                !self
                    .corpse_projection_pins
                    .contains(&projection.occurrence.actor)
            })
            .count()
    }
    /// Whether `actor`'s committed death is projected and retained in this world, Channel and
    /// scope generation.
    pub(super) fn retains_corpse_projection(&self, actor: ExactActorRef) -> bool {
        actor.world_id() == self.world_id
            && actor.channel_id() == self.channel_id
            && actor.scope_generation() == self.scope_generation
            && self
                .corpse_projections
                .iter()
                .any(|projection| projection.occurrence.actor == actor)
    }
    pub(super) fn pin_corpse_projection_inner(
        &mut self,
        actor: ExactActorRef,
    ) -> Result<(), CarrierError> {
        if !self.retains_corpse_projection(actor) {
            return Err(CarrierError::CommittedLethalUnavailable);
        }
        if self.corpse_projection_pins.contains(&actor) {
            return Ok(());
        }
        if self.corpse_projection_pins.len() >= MAX_PINNED_CORPSE_PROJECTIONS {
            return Err(CarrierError::CapacityExceeded);
        }
        self.corpse_projection_pins
            .try_reserve(1)
            .map_err(|_| CarrierError::AllocationFailed)?;
        self.corpse_projection_pins.push(actor);
        Ok(())
    }
    pub(super) fn unpin_corpse_projection_inner(&mut self, actor: ExactActorRef) {
        self.corpse_projection_pins
            .retain(|pinned| *pinned != actor);
    }
    /// D2b: the memoized XP reward occurrence of this generation's committed
    /// death (DUR-03 decision §4.2). The projected corpse must already name
    /// `actor_ref`. A first call mints and stores the occurrence; every later
    /// call for the same `(actor, character)` returns the identical stored
    /// value with no new mint; a different `character` for the same actor
    /// conflicts (`RewardPrincipalConflict`) rather than silently reassigning
    /// the reward.
    /// Returns the occurrence bytes and whether this exact call minted them
    /// (`true`) or reused an already-memoized value (`false`). The caller
    /// uses that distinction to decide whether progression initialization is
    /// needed again: on a fresh mint (a first attempt this generation) it is;
    /// on reuse (a retry) the R7 P03 XP writer's own occurrence-keyed replay
    /// resolves without it.
    pub(super) fn reward_occurrence_inner(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        character: [u8; 16],
    ) -> Result<([u8; 16], bool), CarrierError> {
        self.validate_ref(continuity, actor_ref)?;
        if self.is_native_summon(ExactActorRef(actor_ref)) {
            return Err(CarrierError::SummonHasNoRewards);
        }
        let committed = self
            .corpse_projections
            .iter()
            .any(|projection| projection.occurrence.actor == ExactActorRef(actor_ref));
        if !committed {
            return Err(CarrierError::CommittedLethalUnavailable);
        }
        if let Some(existing) = self
            .death_reward_occurrences
            .iter()
            .find(|entry| entry.actor == actor_ref)
        {
            return if existing.character == character {
                Ok((existing.occurrence, false))
            } else {
                Err(CarrierError::RewardPrincipalConflict)
            };
        }
        self.party_reward_occurrence_inner(continuity, actor_ref, character)
    }
    fn party_reward_occurrence_inner(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        actor_ref: ActorRef,
        character: [u8; 16],
    ) -> Result<([u8; 16], bool), CarrierError> {
        self.validate_ref(continuity, actor_ref)?;
        if self.is_native_summon(ExactActorRef(actor_ref)) {
            return Err(CarrierError::SummonHasNoRewards);
        }
        if !self
            .corpse_projections
            .iter()
            .any(|p| p.occurrence.actor == ExactActorRef(actor_ref))
        {
            return Err(CarrierError::CommittedLethalUnavailable);
        }
        if let Some(existing) = self
            .death_reward_occurrences
            .iter()
            .find(|e| e.actor == actor_ref && e.character == character)
        {
            return Ok((existing.occurrence, false));
        }
        if self
            .death_reward_occurrences
            .iter()
            .filter(|e| e.actor == actor_ref)
            .count()
            >= 50
        {
            return Err(CarrierError::CapacityExceeded);
        }
        let occurrence = mint_reward_occurrence_bytes(actor_ref, character);
        self.death_reward_occurrences.push(DeathRewardOccurrence {
            actor: actor_ref,
            character,
            occurrence,
        });
        Ok((occurrence, true))
    }
}

impl CurrentOwnerCombatDeath<'_> {
    // Insert inside existing impl CurrentOwnerCombatDeath<'_>.
    // Actual carrier-owned data; neither method mints mutation authority or accepts caller facts.
    pub fn creature_target_identity(&self, actor: ExactActorRef) -> Result<&[u8], CarrierError> {
        let index = self.carrier.validate_ref(self.continuity, actor.0)?;
        match &self.carrier.slots[index] {
            Slot::CreatureOccupied {
                target_identity, ..
            } => Ok(target_identity.as_ref()),
            _ => Err(CarrierError::NotCreature),
        }
    }
    /// D109: memoize one occurrence per (death, Character), bounded to the 50 party principals.
    pub fn party_reward_occurrence(
        &mut self,
        actor: ExactActorRef,
        character: CharacterId,
    ) -> Result<([u8; 16], bool), CarrierError> {
        self.carrier
            .party_reward_occurrence_inner(self.continuity, actor.0, *character.as_bytes())
    }
    /// Damage shares from this exact creature generation, captured with its lethal projection.
    pub fn reward_damage_shares(
        &self,
        actor: ExactActorRef,
    ) -> Result<Vec<RewardDamageShare>, CarrierError> {
        let index = self.carrier.validate_ref(self.continuity, actor.0)?;
        match &self.carrier.slots[index] {
            Slot::CreatureOccupied {
                damage_contributors,
                ..
            } => Ok(damage_contributors
                .entries
                .iter()
                .map(|entry| RewardDamageShare {
                    character: entry.character,
                    total: entry.total,
                    lease_generation: entry.high_water.map(|(lease, _, _, _)| lease),
                    session: entry.high_water.map(|(_, session, _, _)| session),
                })
                .collect()),
            _ => Err(CarrierError::NotCreature),
        }
    }
    pub fn damage_contributing_characters(
        &self,
        actor: ExactActorRef,
    ) -> Result<Vec<CharacterId>, CarrierError> {
        let index = self.carrier.validate_ref(self.continuity, actor.0)?;
        match &self.carrier.slots[index] {
            Slot::CreatureOccupied {
                damage_contributors,
                ..
            } => {
                let mut result: Vec<CharacterId> = damage_contributors
                    .entries
                    .iter()
                    .map(|entry| entry.character)
                    .collect();
                result.sort();
                Ok(result)
            }
            _ => Err(CarrierError::NotCreature),
        }
    }
}

/// Project the already committed lethal transition. Combat cannot manufacture
/// lethality, actor identity or position: it can only ask the physical owner
/// for an opaque receipt and immediately return that receipt for projection.
pub fn project_fixed_one_creature_death<'a>(
    owner: &'a mut CurrentOwnerCombatDeath<'_>,
    actor: ExactActorRef,
) -> Result<&'a RuntimeCorpseProjection, CarrierError> {
    let receipt = owner.committed_lethal_receipt(actor)?;
    owner.project_committed_lethal(receipt)
}
