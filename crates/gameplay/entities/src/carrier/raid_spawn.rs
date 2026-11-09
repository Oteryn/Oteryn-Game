//! Raid realization and receipts in the existing Channel creature owner.
//! A durable raid claim supplies identities; the current owner still proves
//! its generation, active creature revision and present map/occupancy facts.
use super::*;
use crate::CompanionSnapshot;
use oteryn_gameplay_foundation::foundation::RuntimeWorkStamp;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RaidSpawnIdentity {
    pub firing_id: [u8; 16],
    pub wave: u16,
    pub entry: u16,
    pub index: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRaidSpawnSpec {
    creature: Box<str>,
    revision: Box<str>,
    content: [u8; 32],
    boss: bool,
}

impl NativeRaidSpawnSpec {
    pub fn qualified(
        creature: &str,
        revision: &str,
        content: [u8; 32],
        boss: bool,
    ) -> Result<Self, CarrierError> {
        if content == [0; 32] {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        Ok(Self {
            creature: copy_native_summon_key(creature)?,
            revision: copy_native_summon_key(revision)?,
            content,
            boss,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidSpawnOutcome {
    Placed(ExactActorRef),
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaidSpawnReceipt {
    pub identity: RaidSpawnIdentity,
    pub outcome: RaidSpawnOutcome,
    pub replayed: bool,
    spec: NativeRaidSpawnSpec,
    position: Option<MovementLocalPosition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaidCreatureStatus {
    pub identity: RaidSpawnIdentity,
    pub outcome: RaidSpawnOutcome,
    pub alive: bool,
    pub boss: bool,
    pub replacement_pending: bool,
}

#[derive(Debug, Clone)]
struct CurrentRaidBody {
    actor: ExactActorRef,
    spec: NativeRaidSpawnSpec,
    replacement_pending: bool,
}

#[derive(Debug, Default)]
pub(super) struct RaidSpawnLedger {
    receipts: BTreeMap<RaidSpawnIdentity, RaidSpawnReceipt>,
    // Keys are a subset of receipts: one current body per admitted identity,
    // regardless of form count. Original placement receipts remain immutable.
    bodies: BTreeMap<RaidSpawnIdentity, (ExactActorRef, NativeRaidSpawnSpec)>,
    bodies: BTreeMap<RaidSpawnIdentity, CurrentRaidBody>,
    shared_attempts: BTreeMap<([u8; 16], u16), u8>,
    // A closing marker is reserved before the first actor admission. Closing
    // an existing run never needs more capacity than that run already owns.
    runs: BTreeMap<[u8; 16], bool>,
    // If an unknown LOST run arrives at the hard limit, reject all future new
    // firing identities until reset; never prevent cleanup of existing runs.
    sealed: bool,
    #[cfg(test)]
    test_limit: Option<usize>,
}

impl RaidSpawnLedger {
    fn limit(&self) -> usize {
        #[cfg(test)]
        if let Some(limit) = self.test_limit {
            return limit;
        }
        CREATUREAI0_CREATURE_ACTORS_PER_CHANNEL_MAX
    }

    fn size(&self) -> usize {
        self.receipts.len() + self.runs.len() + self.shared_attempts.len()
    }
}

impl ChannelRuntimeV1 {
    /// Continue an actual source death without completing the logical raid.
    pub fn replace_dead_raid_body(
        &mut self,
        stamp: RuntimeWorkStamp,
        identity: RaidSpawnIdentity,
        expected: &CompanionSnapshot,
        next: &NativeRaidSpawnSpec,
    ) -> Result<ExactActorRef, EncounterSpawnError> {
        if expected.health != 0 {
            return Err(EncounterSpawnError::Native(
                CarrierError::CreatureTargetMismatch,
            ));
        }
        self.replace_raid_body_with_liveness(stamp, identity, expected, next, true)
    }

    /// Source create/remove at the exact current subject position. The new body
    /// starts with its own full policy and actor generation; the raid stays open.
    pub fn replace_raid_body(
        &mut self,
        stamp: RuntimeWorkStamp,
        identity: RaidSpawnIdentity,
        expected: &CompanionSnapshot,
        next: &NativeRaidSpawnSpec,
    ) -> Result<ExactActorRef, EncounterSpawnError> {
        self.replace_raid_body_with_liveness(stamp, identity, expected, next, false)
    }

    fn replace_raid_body_with_liveness(
        &mut self,
        stamp: RuntimeWorkStamp,
        identity: RaidSpawnIdentity,
        expected: &CompanionSnapshot,
        next: &NativeRaidSpawnSpec,
        death: bool,
    ) -> Result<ExactActorRef, EncounterSpawnError> {
        use EncounterSpawnError as E;
        self.check_raid_stamp(stamp).map_err(E::Native)?;
        if self.raid_spawns.runs.get(&identity.firing_id) != Some(&false) {
            return Err(E::Native(CarrierError::PlanConflict));
        }
        let receipt = self
            .raid_spawns
            .receipts
            .get(&identity)
            .ok_or(E::Native(CarrierError::InvalidSpawnDefinition))?;
        let (actor, current) = if let Some(body) = self.raid_spawns.bodies.get(&identity) {
            body.clone()
            (body.actor, body.spec.clone())
        } else {
            let RaidSpawnOutcome::Placed(actor) = receipt.outcome else {
                return Err(E::Native(CarrierError::PlanConflict));
            };
            (actor, receipt.spec.clone())
        };
        if actor != expected.actor
            || expected.health <= 0
            || if death {
                expected.health != 0
            } else {
                expected.health <= 0
            }
            || expected.state.master.is_some()
            || expected.state.policy.is_familiar
            || expected.forge.is_some()
            || current.creature.as_ref() != expected.state.policy.definition_key
            || next.content != current.content
            || next.boss != current.boss
        {
            return Err(E::Native(CarrierError::CreatureTargetMismatch));
        }
        self.validate_companion_snapshot(expected)
            .map_err(E::Native)?;
        let index = self
            .preflight_native_masterless_removal(actor, &current.creature, current.content)
            .map_err(E::Native)?;
            .native_masterless_subject_index(actor, &current.creature, current.content, death)
            .map_err(E::Native)?;
        let new_body = !self.raid_spawns.bodies.contains_key(&identity);
        // An actual source death owns its continuation even while a spell
        // reservation or capacity pressure defers physical successor admission.
        if death {
            self.raid_spawns
                .bodies
                .entry(identity)
                .or_insert_with(|| CurrentRaidBody {
                    actor,
                    spec: current.clone(),
                    replacement_pending: false,
                })
                .replacement_pending = true;
        }
        self.preflight_native_masterless_removal_with_liveness(
            actor,
            &current.creature,
            current.content,
            death,
        )
        .map_err(E::Native)?;
        let policy = self.companion_policy(&next.creature).map_err(E::Native)?;
        if policy.definition_revision.as_str() != next.revision.as_ref() || policy.is_familiar {
            return Err(E::Native(CarrierError::CreatureTargetMismatch));
        }
        let replacement = next.clone();
        let new_body = !self.raid_spawns.bodies.contains_key(&identity);
        // Reserve the bounded identity entry before either physical body changes.
        self.raid_spawns
            .bodies
            .entry(identity)
            .or_insert((actor, current.clone()));
        self.raid_spawns
            .bodies
            .entry(identity)
            .or_insert_with(|| CurrentRaidBody {
                actor,
                spec: current.clone(),
                replacement_pending: false,
            });
        let position = match &mut self.carrier.slots[index] {
            Slot::CreatureOccupied { position, .. } => position.take(),
            _ => None,
        };
        // This exclusive, synchronous turn replaces only the current subject's
        // occupancy. Native admission still refuses every other occupant and
        // rolls its free slot back on failure; the old position is always restored.
        let result = self.admit_native_masterless_at(
            &next.creature,
            &next.revision,
            next.content,
            expected.position,
            |_, _| Some(true),
        );
        if let Slot::CreatureOccupied { position: old, .. } = &mut self.carrier.slots[index] {
            *old = position;
        }
        let successor = match result {
            Ok(successor) => successor,
            Err(error) => {
                if new_body {
                if new_body && !death {
                    self.raid_spawns.bodies.remove(&identity);
                }
                return Err(error);
            }
        };
        if let Err(error) =
            self.remove_native_masterless_subject(actor, &current.creature, current.content)
        {
            let _ = self.remove_native_masterless_subject(successor, &next.creature, next.content);
            if new_body {
            self.remove_native_raid_body_subject(actor, &current.creature, current.content, death)
        {
            let _ = self.remove_native_masterless_subject(successor, &next.creature, next.content);
            if new_body && !death {
                self.raid_spawns.bodies.remove(&identity);
            }
            return Err(E::Native(error));
        }
        self.raid_spawns
            .bodies
            .insert(identity, (successor, replacement));
        self.raid_spawns.bodies.insert(
            identity,
            CurrentRaidBody {
                actor: successor,
                spec: replacement,
                replacement_pending: false,
            },
        );
        Ok(successor)
    }

    fn check_raid_stamp(&self, stamp: RuntimeWorkStamp) -> Result<(), CarrierError> {
        if !self.owner_fence()?.accepts_stamp(stamp) {
            return Err(CarrierError::WrongScope);
        }
        Ok(())
    }

    /// Flower Month shares250 placement attempts per wave (Crystal flower_month.lua:28-46).
    /// Spend before a physical read: failures and partial retries cannot refund attempts.
    /// The durable caller proves this firing/step; a restarted run is LOST, never replayed.
    pub fn spend_raid_shared_attempt(
        &mut self,
        stamp: RuntimeWorkStamp,
        firing_id: [u8; 16],
        wave: u16,
    ) -> Result<Option<u8>, CarrierError> {
        self.check_raid_stamp(stamp)?;
        // RAID-1's MAX_WAVES=50; at most50 counters can belong to one admitted run.
        if wave >= 50 || WorldId::decode(&firing_id).is_err() {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        if self.raid_spawns.runs.get(&firing_id) == Some(&true) {
            return Err(CarrierError::PlanConflict);
        }
        let new_run = !self.raid_spawns.runs.contains_key(&firing_id);
        let new_wave = !self
            .raid_spawns
            .shared_attempts
            .contains_key(&(firing_id, wave));
        let required = usize::from(new_run) + usize::from(new_wave);
        if (new_run && self.raid_spawns.sealed)
            || self.raid_spawns.size().saturating_add(required) > self.raid_spawns.limit()
        {
            return Err(CarrierError::CapacityExceeded);
        }
        // Reserve the existing close marker before the first read, including failed reads.
        self.raid_spawns.runs.entry(firing_id).or_insert(false);
        let spent = self
            .raid_spawns
            .shared_attempts
            .entry((firing_id, wave))
            .or_insert(0);
        if *spent >= 250 {
            return Ok(None);
        }
        let attempt = *spent;
        *spent += 1;
        Ok(Some(attempt))
    }

    /// Called only after the durable writer commits the exact step. A repeated
    /// identity returns its original result even after death or failed placement.
    /// An old ownership generation is never restored into a new runtime.
    pub fn realize_raid_spawn(
        &mut self,
        stamp: RuntimeWorkStamp,
        identity: RaidSpawnIdentity,
        spec: &NativeRaidSpawnSpec,
        position: Option<MovementLocalPosition>,
        map_admits: impl FnOnce(&ChannelRuntimeV1, MovementLocalPosition) -> Option<bool>,
    ) -> Result<RaidSpawnReceipt, EncounterSpawnError> {
        use EncounterSpawnError as E;
        self.check_raid_stamp(stamp).map_err(E::Native)?;
        if WorldId::decode(&identity.firing_id).is_err() {
            return Err(E::Native(CarrierError::InvalidSpawnDefinition));
        }
        if let Some(previous) = self.raid_spawns.receipts.get(&identity) {
            if &previous.spec != spec {
                return Err(E::Native(CarrierError::InvalidSpawnDefinition));
            }
            let mut replay = previous.clone();
            replay.replayed = true;
            return Ok(replay);
        }
        if self.raid_spawns.runs.get(&identity.firing_id) == Some(&true) {
            return Err(E::Native(CarrierError::InvalidSpawnDefinition));
        }
        // Reuse the existing Channel creature resource envelope for retained
        // raid outcomes (including failed requests); reset drops this ledger.
        let new_run = !self.raid_spawns.runs.contains_key(&identity.firing_id);
        let required = 1 + usize::from(new_run);
        if (new_run && self.raid_spawns.sealed)
            || self.raid_spawns.size().saturating_add(required) > self.raid_spawns.limit()
        {
            return Err(E::Native(CarrierError::CapacityExceeded));
        }
        if spec.content != self.content_pin().server_artifact_digest() {
            return Err(E::Native(CarrierError::ContentPinWorldMismatch));
        }
        let policy = self.companion_policy(&spec.creature).map_err(E::Native)?;
        if policy.definition_key.as_str() != spec.creature.as_ref()
            || policy.definition_revision.as_str() != spec.revision.as_ref()
        {
            return Err(E::Native(CarrierError::CreatureTargetMismatch));
        }
        let outcome = if let Some(at) = position {
            match self.admit_native_masterless_at(
                &spec.creature,
                &spec.revision,
                spec.content,
                at,
                map_admits,
            ) {
                Ok(actor) => RaidSpawnOutcome::Placed(actor),
                Err(E::RefusedMap | E::Native(CarrierError::CapacityExceeded)) => {
                    RaidSpawnOutcome::Failed
                }
                Err(error) => return Err(error),
            }
        } else {
            RaidSpawnOutcome::Failed
        };
        let receipt = RaidSpawnReceipt {
            identity,
            outcome,
            replayed: false,
            spec: spec.clone(),
            position,
        };
        self.raid_spawns
            .runs
            .entry(identity.firing_id)
            .or_insert(false);
        self.raid_spawns.receipts.insert(identity, receipt.clone());
        Ok(receipt)
    }

    /// Current liveness joins retained raid identities to the actual actor
    /// carrier; recycled slots never make an old raid creature alive again.
    /// Read the existing ledger's stable identity for one exact live actor.
    /// This immutable binding never grants a placement or a world contribution.
    pub fn raid_identity_for_actor(
        &self,
        actor: ExactActorRef,
    ) -> Result<Option<RaidSpawnIdentity>, CarrierError> {
        self.owner_fence()?;
        if !self.contains_live_creature(actor) {
            return Ok(None);
        }
        self.raid_identity_for_actor_including_dead(actor)
    }

    pub fn raid_identity_for_actor_including_dead(
        &self,
        actor: ExactActorRef,
    ) -> Result<Option<RaidSpawnIdentity>, CarrierError> {
        self.owner_fence()?;
        if self.companion_snapshot_including_dead(actor).is_err() {
            return Ok(None);
        }
        let mut rows = self
            .raid_spawns
            .receipts
            .iter()
            .filter_map(|(identity, receipt)| {
                let current = self
                    .raid_spawns
                    .bodies
                    .get(identity)
                    .map_or(receipt.outcome, |body| RaidSpawnOutcome::Placed(body.0));
                    .map_or(receipt.outcome, |body| RaidSpawnOutcome::Placed(body.actor));
                (current == RaidSpawnOutcome::Placed(actor)).then_some(*identity)
            });
        let first = rows.next();
        if rows.next().is_some() {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        Ok(first)
    }

    pub fn raid_creatures(
        &self,
        firing_id: [u8; 16],
    ) -> Result<Vec<RaidCreatureStatus>, CarrierError> {
        self.owner_fence()?;
        let first = RaidSpawnIdentity {
            firing_id,
            wave: 0,
            entry: 0,
            index: 0,
        };
        let last = RaidSpawnIdentity {
            firing_id,
            wave: u16::MAX,
            entry: u16::MAX,
            index: u16::MAX,
        };
        Ok(self.raid_spawns.receipts.range(first..=last).map(|(_, receipt)| {
            let outcome = self.raid_spawns.bodies.get(&receipt.identity)
                .map_or(receipt.outcome, |body| RaidSpawnOutcome::Placed(body.0));
                .map_or(receipt.outcome, |body| RaidSpawnOutcome::Placed(body.actor));
            RaidCreatureStatus {
                identity: receipt.identity,
                outcome,
                alive: matches!(outcome, RaidSpawnOutcome::Placed(actor) if self.contains_live_creature(actor)),
                boss: receipt.spec.boss,
                replacement_pending: self.raid_spawns.bodies.get(&receipt.identity)
                    .is_some_and(|body| body.replacement_pending),
            }
        }).collect())
    }

    /// Closing a persistent run keeps its actors. Explicit despawn and LOST
    /// cleanup remove them through the same administrative removal as native
    /// encounters, with no death, corpse, loot or reward mint.
    pub fn finish_raid_run(
        &mut self,
        stamp: RuntimeWorkStamp,
        firing_id: [u8; 16],
        cleanup: bool,
    ) -> Result<usize, CarrierError> {
        self.check_raid_stamp(stamp)?;
        if WorldId::decode(&firing_id).is_err() {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        // Close before removal: a reservation can defer cleanup but cannot
        // permit another wave to appear after the end decision.
        if let Some(closed) = self.raid_spawns.runs.get_mut(&firing_id) {
            *closed = true;
        } else if self.raid_spawns.size() < self.raid_spawns.limit() {
            self.raid_spawns.runs.insert(firing_id, true);
        } else {
            // Every firing with any local actor already owns a marker, so this
            // is a LOST run with no local effects. Fail closed for new firings
            // without allocating or obstructing cleanup of a tracked firing.
            self.raid_spawns.sealed = true;
            return Ok(0);
        }
        // The retained close marker rejects replays; attempt counters can now be released.
        self.raid_spawns
            .shared_attempts
            .retain(|(firing, _), _| *firing != firing_id);
        if !cleanup {
            return Ok(0);
        }
        let identities = self.raid_creatures(firing_id)?;
        let mut removed = 0;
        for status in identities {
            let RaidSpawnOutcome::Placed(actor) = status.outcome else {
                continue;
            };
            let pending_death = !status.alive
                && status.replacement_pending
                && self
                    .companion_snapshot_including_dead(actor)
                    .is_ok_and(|body| body.health == 0);
            if !status.alive && !pending_death {
                continue;
            }
            let receipt = self
                .raid_spawns
                .receipts
                .remove(&status.identity)
                .ok_or(CarrierError::InvalidSpawnDefinition)?;
            let current = self
                .raid_spawns
                .bodies
                .get(&status.identity)
                .map_or(&receipt.spec, |body| &body.1)
                .clone();
            let result =
                self.remove_native_masterless_subject(actor, &current.creature, current.content);
                .map_or(&receipt.spec, |body| &body.spec)
                .clone();
            let result = self.remove_native_raid_body_subject(
                actor,
                &current.creature,
                current.content,
                pending_death,
            );
            self.raid_spawns.receipts.insert(status.identity, receipt);
            result?;
            removed += 1;
        }
        Ok(removed)
    }

    /// After the durable end effect is acknowledged, per-creature failures and
    /// dead/removed actor receipts need not be replayed. Keep the closing marker
    /// and every surviving persistent actor, so neither replay nor later LOST
    /// cleanup can create or lose an actor. Never call this after an uncertain ack.
    pub fn acknowledge_raid_run_end(
        &mut self,
        stamp: RuntimeWorkStamp,
        firing_id: [u8; 16],
    ) -> Result<usize, CarrierError> {
        self.check_raid_stamp(stamp)?;
        if WorldId::decode(&firing_id).is_err() {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        match self.raid_spawns.runs.get(&firing_id) {
            Some(true) => {}
            None if self.raid_spawns.sealed => return Ok(0),
            _ => return Err(CarrierError::InvalidSpawnDefinition),
        }
        let terminal = self
            .raid_creatures(firing_id)?
            .into_iter()
            .filter(|status| !status.alive)
            .map(|status| status.identity)
            .collect::<Vec<_>>();
        for identity in &terminal {
            self.raid_spawns.receipts.remove(identity);
            self.raid_spawns.bodies.remove(identity);
        }
        Ok(terminal.len())
    }
}

#[cfg(test)]
#[path = "raid_spawn_tests.rs"]
mod tests;
