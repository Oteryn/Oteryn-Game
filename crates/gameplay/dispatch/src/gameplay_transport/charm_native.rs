//! Native CHARM-2/3 port. Content indices belong to the supplied revision binding; durable
//! authority remains in DurabilityRoot. This module does not advertise or activate capability 1.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use oteryn_protocol_oteryn::bestiary::{
    BestiaryRaceProgress, MAX_BESTIARY_KILL_THRESHOLD, MAX_BESTIARY_VIEW_ENTRIES,
    encode_bestiary_view,
};
use oteryn_protocol_oteryn::charm::{
    COMMAND_TYPE_CHARM_ASSIGN_INTENT, COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT,
    CharmAssignDisposition, CharmAssignIntent, CharmKind, CharmState, CharmUnlockDisposition,
    CharmUnlockStageIntent, CharmView, MAX_CHARM_BALANCE, MAX_CHARM_STAGE_COST, MAX_CHARMS,
    encode_charm_assign_result, encode_charm_unlock_stage_result, encode_charm_view,
};
use oteryn_protocol_oteryn::charm_progression::CharmRemoveDisposition;

use super::{
    CharmCommandIdentity, CharmCommandReply, CharmPortUnavailable, CharmProgressionPort,
    dispatch_charm_command,
};
use crate::domain::CharacterRevision;
use crate::domain::bestiary::BestiaryRace;
use crate::domain::charm::{
    BestiaryRaceKey, CharmCatalogue, CharmCategory, CharmCurrency, CharmKey, CharmRuleError,
};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::character_revision_sequencer::{CharacterRevisionSequencer, RevisionSlot};
use crate::durability::charm_state::{
    CharacterCharmProgressionSnapshot, CharmCommand, CharmCommandEffect, CharmCommandOccurrence,
    CharmCommandOutcome, CharmCommandRequest, CharmFacts, CharmProgressionReadRequest,
    CharmRemoveFee, CharmStateError, CommittedCharmCommand,
};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::{DurabilityError, DurabilityRoot};
use oteryn_gameplay_progression::charm_effects::{
    CharmCatalogueRead, CharmCategory as EffectCategory, CharmDefinition as EffectDefinition,
};

/// The supplier provides validated CHARM-4 definitions for both wire indices and combat lookup.
/// Comparing its revision with the Character root checks expected binding only. Content-owner
/// loader/digest integration must establish provenance before claiming a qualified generation.
pub struct CharmConnectionContent {
    revision: String,
    catalogue: CharmCatalogue,
    charms: Vec<CharmKey>,
    races: Vec<(BestiaryRaceKey, BestiaryRace)>,
    effects: BTreeMap<String, EffectDefinition>,
    runtime_consumers: bool,
}

impl CharmConnectionContent {
    pub fn new(
        revision: String,
        catalogue: CharmCatalogue,
        races: Vec<BestiaryRace>,
        effects: &impl CharmCatalogueRead,
    ) -> Result<Self, CharmPortUnavailable> {
        if races.len() > MAX_BESTIARY_VIEW_ENTRIES
            || catalogue.definitions().take(MAX_CHARMS + 1).count() > MAX_CHARMS
        {
            return Err(CharmPortUnavailable);
        }
        let mut bound_races = Vec::with_capacity(races.len());
        for race in races {
            if race.kill_thresholds().len() != 3
                || race.final_kill_threshold() > MAX_BESTIARY_KILL_THRESHOLD
            {
                return Err(CharmPortUnavailable);
            }
            let key = BestiaryRaceKey::new(race.key()).map_err(|_| CharmPortUnavailable)?;
            bound_races.push((key, race));
        }
        bound_races.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        if bound_races.windows(2).any(|pair| pair[0].0 == pair[1].0) {
            return Err(CharmPortUnavailable);
        }
        let mut bound_effects = BTreeMap::new();
        let mut charms = Vec::new();
        for definition in catalogue.definitions() {
            let effect = effects
                .charm(definition.key.as_str())
                .ok_or(CharmPortUnavailable)?;
            let category = match definition.category {
                CharmCategory::Major => EffectCategory::Major,
                CharmCategory::Minor => EffectCategory::Minor,
            };
            if effect.key() != definition.key.as_str()
                || effect.category() != category
                || definition
                    .stage_costs
                    .iter()
                    .any(|cost| *cost > MAX_CHARM_STAGE_COST)
            {
                return Err(CharmPortUnavailable);
            }
            charms.push(definition.key.clone());
            bound_effects.insert(definition.key.as_str().to_owned(), effect.clone());
        }
        Ok(Self {
            revision,
            catalogue,
            charms,
            races: bound_races,
            effects: bound_effects,
            runtime_consumers: false,
        })
    }

    pub(in super::super) fn with_runtime_consumers(mut self, ready: bool) -> Self {
        self.runtime_consumers = ready;
        self
    }

    fn request(&self) -> CharmProgressionReadRequest {
        CharmProgressionReadRequest {
            catalogue_revision: self.revision.clone(),
            catalogue: self.catalogue.clone(),
            races: self.races.iter().map(|(key, _)| key.clone()).collect(),
        }
    }

    fn race_index(&self, key: &BestiaryRaceKey) -> Result<NonZeroU32, CharmPortUnavailable> {
        let index = self
            .races
            .binary_search_by(|(race, _)| race.cmp(key))
            .map_err(|_| CharmPortUnavailable)?;
        wire_index(index)
    }

    pub(crate) fn project(
        &self,
        snapshot: CharacterCharmProgressionSnapshot,
    ) -> Result<CharmConnectionViews, CharmPortUnavailable> {
        // An obsolete assignment has no generation index. Do not hide it or invent a new index.
        for key in snapshot
            .state
            .unlocks
            .keys()
            .chain(snapshot.state.assignments.keys())
        {
            self.catalogue.get(key).map_err(|_| CharmPortUnavailable)?;
        }
        let mut bestiary = Vec::new();
        for (index, (key, race)) in self.races.iter().enumerate() {
            if let Some(count) = snapshot
                .bestiary_counts
                .get(key)
                .copied()
                .filter(|count| *count > 0)
            {
                bestiary.push(BestiaryRaceProgress {
                    race: wire_index(index)?,
                    // Durable history survives a later threshold reduction. The current wire
                    // projects complete progress within its bound without rewriting history.
                    kill_count: count.min(race.final_kill_threshold()),
                    kill_thresholds: race
                        .kill_thresholds()
                        .try_into()
                        .map_err(|_| CharmPortUnavailable)?,
                });
            }
        }
        let mut charms = Vec::with_capacity(self.charms.len());
        for (index, key) in self.charms.iter().enumerate() {
            let definition = self.catalogue.get(key).map_err(|_| CharmPortUnavailable)?;
            let stage = snapshot
                .state
                .unlocks
                .get(key)
                .map_or(0, |stage| stage.get());
            charms.push(CharmState {
                charm: wire_index(index)?,
                kind: match definition.category {
                    CharmCategory::Major => CharmKind::Major,
                    CharmCategory::Minor => CharmKind::Minor,
                },
                unlocked_stage: stage,
                assigned_race: snapshot
                    .state
                    .assignments
                    .get(key)
                    .map(|race| self.race_index(race))
                    .transpose()?,
                next_stage_cost: definition
                    .stage_costs
                    .get(usize::from(stage))
                    .copied()
                    .unwrap_or(0),
                // Standalone readers stay unavailable; the qualified native owner connects consumers.
                effect_active: self.runtime_consumers,
            });
        }
        let charms = CharmView {
            charms,
            charm_points_available: wire_balance(
                snapshot.balance.available(CharmCurrency::CharmPoints),
            )?,
            minor_charm_echoes_available: wire_balance(
                snapshot.balance.available(CharmCurrency::MinorCharmEchoes),
            )?,
            assignment_slot_limit: snapshot
                .slot_entitlement
                .assignment_slots()
                .map(|slots| {
                    u32::try_from(slots)
                        .ok()
                        .and_then(NonZeroU32::new)
                        .ok_or(CharmPortUnavailable)
                })
                .transpose()?,
        };
        encode_bestiary_view(&bestiary).map_err(|_| CharmPortUnavailable)?;
        encode_charm_view(&charms).map_err(|_| CharmPortUnavailable)?;
        Ok(CharmConnectionViews {
            combat_state: snapshot.state,
            revision: snapshot.character_revision,
            bestiary,
            charms,
        })
    }
}

impl CharmCatalogueRead for CharmConnectionContent {
    fn charm(&self, key: &str) -> Option<&EffectDefinition> {
        self.effects.get(key)
    }
}

fn wire_index(zero_based: usize) -> Result<NonZeroU32, CharmPortUnavailable> {
    zero_based
        .checked_add(1)
        .and_then(|index| u32::try_from(index).ok())
        .and_then(NonZeroU32::new)
        .ok_or(CharmPortUnavailable)
}

fn wire_balance(value: i128) -> Result<u32, CharmPortUnavailable> {
    u32::try_from(value)
        .ok()
        .filter(|value| *value <= MAX_CHARM_BALANCE)
        .ok_or(CharmPortUnavailable)
}

fn indexed<T>(entries: &[T], index: NonZeroU32) -> Option<&T> {
    entries.get(usize::try_from(index.get() - 1).ok()?)
}

/// Both snapshots have the same durable revision, ready for connection domain envelopes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharmConnectionViews {
    pub(crate) combat_state: crate::durability::charm_state::CharacterCharmState,
    pub revision: CharacterRevision,
    pub bestiary: Vec<BestiaryRaceProgress>,
    pub charms: CharmView,
}

/// The runtime independently supplies live session/lease/scope evidence and its Character
/// revision sequencer. A fresh command commits at the sequencer's cursor; a receipt's revision is
/// used solely to replay its original semantic binding.
pub struct NativeCharmProgressionPort<'a, 'f, 's, F> {
    root: &'a DurabilityRoot,
    authority: &'a ReconciledCharacterAuthority<'f, 's>,
    node: &'a NodeIncarnationProof,
    sequencer: &'a CharacterRevisionSequencer,
    fence: CurrentCharacterGameplayFence,
    content: &'a CharmConnectionContent,
    facts: F,
}

impl<'a, 'f, 's, F: CharmFacts + Clone> NativeCharmProgressionPort<'a, 'f, 's, F> {
    pub async fn bind(
        root: &'a DurabilityRoot,
        authority: &'a ReconciledCharacterAuthority<'f, 's>,
        node: &'a NodeIncarnationProof,
        sequencer: &'a CharacterRevisionSequencer,
        fence: CurrentCharacterGameplayFence,
        content: &'a CharmConnectionContent,
        facts: F,
    ) -> Result<Self, CharmPortUnavailable> {
        Self::bind_checked(root, authority, node, sequencer, fence, content, facts)
            .await
            .map(|(port, _)| port)
            .map_err(|_| CharmPortUnavailable)
    }

    pub(crate) async fn bind_checked(
        root: &'a DurabilityRoot,
        authority: &'a ReconciledCharacterAuthority<'f, 's>,
        node: &'a NodeIncarnationProof,
        sequencer: &'a CharacterRevisionSequencer,
        fence: CurrentCharacterGameplayFence,
        content: &'a CharmConnectionContent,
        facts: F,
    ) -> Result<(Self, CharacterCharmProgressionSnapshot), CharmStateError> {
        let port = Self {
            root,
            authority,
            node,
            sequencer,
            fence,
            content,
            facts,
        };
        // The supplied revision is checked, together with current live authority, before binding.
        let snapshot = port.snapshot(fence).await?;
        Ok((port, snapshot))
    }

    async fn current_fence(&self) -> Result<CurrentCharacterGameplayFence, CharmStateError> {
        let state = self
            .root
            .read_character_progression(self.authority, self.fence.character_id)
            .await
            .map_err(|error| match error {
                crate::durability::character_progression::CharacterProgressionError::Unavailable(error) => CharmStateError::Unavailable(error),
                _ => CharmStateError::AuthorityRejected,
            })?
            .ok_or(CharmStateError::MissingProgressionState)?;
        if state.context.content != self.content.revision {
            return Err(CharmStateError::CharmContextMismatch);
        }
        Ok(CurrentCharacterGameplayFence {
            expected_character_revision: state.character_revision,
            ..self.fence
        })
    }

    async fn snapshot(
        &self,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<CharacterCharmProgressionSnapshot, CharmStateError> {
        self.root
            .read_character_charm_progression(
                self.authority,
                self.node,
                fence,
                self.content.request(),
                self.facts.clone(),
            )
            .await
    }

    pub async fn views(&self) -> Result<CharmConnectionViews, CharmPortUnavailable> {
        self.views_checked().await.map_err(|_| CharmPortUnavailable)
    }

    pub(crate) async fn views_checked(&self) -> Result<CharmConnectionViews, CharmStateError> {
        let snapshot = self.snapshot(self.current_fence().await?).await?;
        self.content
            .project(snapshot)
            .map_err(|_| CharmStateError::InvalidInput)
    }

    /// Dispatch only the bound runtime session's identity. This prepares the registered adapter;
    /// the connection owner still gates routing and capability advertisement.
    pub async fn dispatch(
        &mut self,
        identity: CharmCommandIdentity,
        command_type: u32,
        payload: &[u8],
    ) -> Option<CharmCommandReply> {
        if identity.game_session_id != self.fence.game_session_id {
            let result_payload = match command_type {
                COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT => {
                    encode_charm_unlock_stage_result(CharmUnlockDisposition::Rejected)
                }
                COMMAND_TYPE_CHARM_ASSIGN_INTENT => {
                    encode_charm_assign_result(CharmAssignDisposition::Rejected)
                }
                _ => return None,
            };
            return Some(CharmCommandReply {
                rejected: true,
                result_payload,
            });
        }
        dispatch_charm_command(self, identity, command_type, payload).await
    }

    /// Remove `charm` from its race for the `level * 100` gold fee (Canary `sendBuyCharmRune`
    /// action 2). Returns the disposition and the fee paid, 0 unless removed.
    pub async fn remove(
        &mut self,
        occurrence: CharmCommandOccurrence,
        charm: NonZeroU32,
        fee: CharmRemoveFee,
    ) -> (CharmRemoveDisposition, u64) {
        self.remove_checked(occurrence, charm, fee)
            .await
            .unwrap_or((CharmRemoveDisposition::Rejected, 0))
    }

    pub(crate) async fn remove_checked(
        &self,
        occurrence: CharmCommandOccurrence,
        charm: NonZeroU32,
        fee: CharmRemoveFee,
    ) -> Result<(CharmRemoveDisposition, u64), DurabilityError> {
        let Some(charm) = indexed(&self.content.charms, charm).cloned() else {
            return Ok((CharmRemoveDisposition::UnknownCharm, 0));
        };
        let mut slot = self.sequencer.acquire(self.fence.character_id).await;
        let original = match without_busy(self.retained(occurrence).await)? {
            Ok(Some(receipt)) => Some(receipt.original_character_revision),
            Ok(None) => None,

            Err(_) => return Ok((CharmRemoveDisposition::Rejected, 0)),
        };
        Ok(remove_disposition(without_busy(
            self.commit(
                &mut slot,
                occurrence,
                CharmCommand::Remove {
                    charm,
                    fee: Box::new(fee),
                },
                original,
                None,
            )
            .await,
        )?))
    }

    pub async fn reset_all(
        &mut self,
        occurrence: CharmCommandOccurrence,
        fee: CharmRemoveFee,
    ) -> (CharmRemoveDisposition, u64) {
        self.reset_all_checked(occurrence, fee)
            .await
            .unwrap_or((CharmRemoveDisposition::Rejected, 0))
    }

    pub(crate) async fn reset_all_checked(
        &self,
        occurrence: CharmCommandOccurrence,
        fee: CharmRemoveFee,
    ) -> Result<(CharmRemoveDisposition, u64), DurabilityError> {
        let Ok(charm) = CharmKey::new("oteryn:charm.reset_all") else {
            return Ok((CharmRemoveDisposition::Rejected, 0));
        };
        let mut slot = self.sequencer.acquire(self.fence.character_id).await;
        let original = match without_busy(self.retained(occurrence).await)? {
            Ok(receipt) => receipt.map(|r| r.original_character_revision),

            Err(_) => return Ok((CharmRemoveDisposition::Rejected, 0)),
        };
        Ok(remove_disposition(without_busy(
            self.commit(
                &mut slot,
                occurrence,
                CharmCommand::ResetAll {
                    charm,
                    fee: Box::new(fee),
                },
                original,
                None,
            )
            .await,
        )?))
    }

    async fn retained(
        &self,
        occurrence: CharmCommandOccurrence,
    ) -> Result<Option<CommittedCharmCommand>, CharmStateError> {
        self.root
            .reconcile_charm_command(self.authority, occurrence)
            .await
    }

    /// Commit under the held `slot`: at its cursor, or as the exact replay of a retained
    /// receipt at its `original` revision. A mismatch is not retried at another revision.
    async fn commit(
        &self,
        slot: &mut RevisionSlot,
        occurrence: CharmCommandOccurrence,
        command: CharmCommand,
        original: Option<CharacterRevision>,
        expected_stage: Option<u8>,
    ) -> Result<CharmCommandOutcome, CharmStateError> {
        let request = CharmCommandRequest {
            occurrence,
            command,
            catalogue_revision: self.content.revision.clone(),
            catalogue: self.content.catalogue.clone(),
        };
        let result = slot
            .commit_charm(
                self.root,
                self.authority,
                self.node,
                self.fence,
                request.clone(),
                self.facts.clone(),
                original,
            )
            .await;
        // A same-occurrence commit can race the first reconciliation/read. Replay it once with
        // its original semantic revision. Never copy live authority fields from the receipt.
        if matches!(
            result,
            Err(CharmStateError::ConflictingOccurrence | CharmStateError::CharacterRevisionMismatch)
        ) && let Some(receipt) = self
            .root
            .reconcile_charm_command(self.authority, occurrence)
            .await?
        {
            if expected_stage.is_some_and(|expected| !receipt_stage_matches(&receipt, expected)) {
                return Err(CharmStateError::ConflictingOccurrence);
            }
            return slot
                .commit_charm(
                    self.root,
                    self.authority,
                    self.node,
                    self.fence,
                    request,
                    self.facts.clone(),
                    Some(receipt.original_character_revision),
                )
                .await;
        }
        result
    }

    pub(crate) async fn unlock_next_stage_checked(
        &self,
        occurrence: CharmCommandOccurrence,
        intent: CharmUnlockStageIntent,
    ) -> Result<CharmUnlockDisposition, DurabilityError> {
        let Some(key) = indexed(&self.content.charms, intent.charm).cloned() else {
            return Ok(CharmUnlockDisposition::UnknownCharm);
        };
        // The stage is checked and committed under one slot, so no write lands in between.
        let mut slot = self.sequencer.acquire(self.fence.character_id).await;
        let retained = match without_busy(self.retained(occurrence).await)? {
            Ok(receipt) => receipt,

            Err(_) => return Ok(CharmUnlockDisposition::Rejected),
        };
        let original = if let Some(receipt) = retained {
            if !receipt_stage_matches(&receipt, intent.expected_stage) {
                return Ok(CharmUnlockDisposition::Rejected);
            }
            Some(receipt.original_character_revision)
        } else {
            let snapshot = match self.current_fence().await {
                Ok(fence) => self.snapshot(fence).await,
                Err(error) => Err(error),
            };
            match without_busy(snapshot)? {
                Ok(snapshot)
                    if snapshot
                        .state
                        .unlocks
                        .get(&key)
                        .map_or(0, |stage| stage.get())
                        == intent.expected_stage =>
                {
                    None
                }
                refusal => {
                    // A concurrent identical unlock may have just advanced the stage/revision.
                    match without_busy(self.retained(occurrence).await)? {
                        Ok(Some(receipt))
                            if receipt_stage_matches(&receipt, intent.expected_stage) =>
                        {
                            Some(receipt.original_character_revision)
                        }
                        Ok(None) if refusal.is_ok() => {
                            return Ok(CharmUnlockDisposition::StageMismatch);
                        }
                        _ => return Ok(CharmUnlockDisposition::Rejected),
                    }
                }
            }
        };
        let category = match self.content.catalogue.get(&key) {
            Ok(definition) => definition.category,
            Err(_) => return Ok(CharmUnlockDisposition::UnknownCharm),
        };
        Ok(unlock_disposition(
            without_busy(
                self.commit(
                    &mut slot,
                    occurrence,
                    CharmCommand::UnlockNextStage { charm: key },
                    original,
                    Some(intent.expected_stage),
                )
                .await,
            )?,
            category,
        ))
    }

    pub(crate) async fn assign_checked(
        &self,
        occurrence: CharmCommandOccurrence,
        intent: CharmAssignIntent,
    ) -> Result<CharmAssignDisposition, DurabilityError> {
        let Some(charm) = indexed(&self.content.charms, intent.charm).cloned() else {
            return Ok(CharmAssignDisposition::UnknownCharm);
        };
        let Some((race, _)) = indexed(&self.content.races, intent.race) else {
            return Ok(CharmAssignDisposition::UnknownRace);
        };
        let mut slot = self.sequencer.acquire(self.fence.character_id).await;
        let original = match without_busy(self.retained(occurrence).await)? {
            Ok(Some(receipt)) => Some(receipt.original_character_revision),
            Ok(None) => None,

            Err(_) => return Ok(CharmAssignDisposition::Rejected),
        };
        Ok(assign_disposition(without_busy(
            self.commit(
                &mut slot,
                occurrence,
                CharmCommand::Assign {
                    charm,
                    race: race.clone(),
                },
                original,
                None,
            )
            .await,
        )?))
    }
}

fn without_busy<T>(
    result: Result<T, CharmStateError>,
) -> Result<Result<T, CharmStateError>, DurabilityError> {
    match result {
        Err(CharmStateError::Unavailable(error @ DurabilityError::RootUnavailable)) => Err(error),
        other => Ok(other),
    }
}

fn receipt_stage_matches(receipt: &CommittedCharmCommand, expected: u8) -> bool {
    matches!(receipt.effect, CharmCommandEffect::Unlocked { stage_before, .. } if stage_before == expected)
}

impl<F: CharmFacts + Clone> CharmProgressionPort for NativeCharmProgressionPort<'_, '_, '_, F> {
    async fn bestiary(&self) -> Result<Vec<BestiaryRaceProgress>, CharmPortUnavailable> {
        Ok(self.views().await?.bestiary)
    }

    async fn charms(&self) -> Result<CharmView, CharmPortUnavailable> {
        Ok(self.views().await?.charms)
    }

    async fn unlock_next_stage(
        &mut self,
        occurrence: CharmCommandOccurrence,
        intent: CharmUnlockStageIntent,
    ) -> CharmUnlockDisposition {
        self.unlock_next_stage_checked(occurrence, intent)
            .await
            .unwrap_or(CharmUnlockDisposition::Rejected)
    }

    async fn assign(
        &mut self,
        occurrence: CharmCommandOccurrence,
        intent: CharmAssignIntent,
    ) -> CharmAssignDisposition {
        self.assign_checked(occurrence, intent)
            .await
            .unwrap_or(CharmAssignDisposition::Rejected)
    }
}

fn unlock_disposition(
    result: Result<CharmCommandOutcome, CharmStateError>,
    category: CharmCategory,
) -> CharmUnlockDisposition {
    match result {
        Ok(
            CharmCommandOutcome::Committed(receipt)
            | CharmCommandOutcome::AlreadyCommitted(receipt),
        ) if matches!(receipt.effect, CharmCommandEffect::Unlocked { .. }) => {
            CharmUnlockDisposition::Unlocked
        }
        Err(CharmStateError::Rule(CharmRuleError::UnknownCharm)) => {
            CharmUnlockDisposition::UnknownCharm
        }
        Err(CharmStateError::Rule(CharmRuleError::FinalStageReached)) => {
            CharmUnlockDisposition::StageMismatch
        }
        Err(CharmStateError::Rule(CharmRuleError::InsufficientBalance)) => match category {
            CharmCategory::Major => CharmUnlockDisposition::NotEnoughCharmPoints,
            CharmCategory::Minor => CharmUnlockDisposition::NotEnoughMinorCharmEchoes,
        },
        _ => CharmUnlockDisposition::Rejected,
    }
}

fn assign_disposition(
    result: Result<CharmCommandOutcome, CharmStateError>,
) -> CharmAssignDisposition {
    match result {
        Ok(
            CharmCommandOutcome::Committed(receipt)
            | CharmCommandOutcome::AlreadyCommitted(receipt),
        ) if matches!(receipt.effect, CharmCommandEffect::Assigned { .. }) => {
            CharmAssignDisposition::Assigned
        }
        Err(CharmStateError::Rule(rule)) => match rule {
            CharmRuleError::UnknownCharm => CharmAssignDisposition::UnknownCharm,
            CharmRuleError::CharmLocked => CharmAssignDisposition::CharmLocked,
            CharmRuleError::CharmAlreadyAssigned => CharmAssignDisposition::AlreadyAssigned,
            CharmRuleError::BestiaryStageTooLow => CharmAssignDisposition::RaceStageTooLow,
            CharmRuleError::RaceCapacityReached => CharmAssignDisposition::RaceCharmLimit,
            CharmRuleError::AssignmentSlotsFull => CharmAssignDisposition::AssignmentSlotsFull,
            _ => CharmAssignDisposition::Rejected,
        },
        _ => CharmAssignDisposition::Rejected,
    }
}

fn remove_disposition(
    result: Result<CharmCommandOutcome, CharmStateError>,
) -> (CharmRemoveDisposition, u64) {
    match result {
        Ok(
            CharmCommandOutcome::Committed(receipt)
            | CharmCommandOutcome::AlreadyCommitted(receipt),
        ) => match receipt.effect {
            CharmCommandEffect::Removed { fee_gold_units, .. }
            | CharmCommandEffect::Reset { fee_gold_units } => {
                (CharmRemoveDisposition::Removed, fee_gold_units)
            }
            _ => (CharmRemoveDisposition::Rejected, 0),
        },
        Err(CharmStateError::NotAssigned) => (CharmRemoveDisposition::NotAssigned, 0),
        Err(CharmStateError::InsufficientGold) => (CharmRemoveDisposition::NotEnoughGold, 0),
        Err(CharmStateError::FeeChangeDoesNotFit) => (CharmRemoveDisposition::ChangeDoesNotFit, 0),
        Err(CharmStateError::Rule(CharmRuleError::UnknownCharm)) => {
            (CharmRemoveDisposition::UnknownCharm, 0)
        }
        _ => (CharmRemoveDisposition::Rejected, 0),
    }
}

#[cfg(test)]
#[path = "charm_native_tests.rs"]
mod tests;
