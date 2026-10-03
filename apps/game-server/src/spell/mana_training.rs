//! D151: actual paid casts accumulate training in the player's existing state.
//! A level advance/checkpoint is staged before mutation and exposed only after
//! its genuine Character build receipt commits in the shared transaction.
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildFormula, BuildOccurrence, CommittedBuildChange,
    DurableBuildState, MAX_BUILD_LEVEL, MAX_BUILD_PROGRESS, skill_tries_required,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrainingError {
    InvalidProfile,
    UnknownVocation,
    InvalidBuild,
    StaleTime,
    StaleReceipt,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceFile {
    path: String,
    sha256: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    repository: String,
    revision: String,
    files: Vec<SourceFile>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct VocationProfile {
    key: String,
    mana_multiplier: String,
    skill_multipliers: [String; 7],
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    schema: String,
    source: Source,
    magic_base: u16,
    skill_bases: [u16; 7],
    multiplier_storage: String,
    vocations: Vec<VocationProfile>,
}
#[derive(Debug, Clone)]
pub(crate) struct CompiledTrainingFormula {
    profile: Profile,
    content_revision: String,
    digest: [u8; 32],
}
impl CompiledTrainingFormula {
    pub(crate) fn from_profile(
        bytes: &[u8],
        content_revision: &str,
    ) -> Result<Self, TrainingError> {
        if bytes.len() > 32768
            || content_revision.is_empty()
            || content_revision.len() > 128
            || !content_revision.bytes().all(|b| b.is_ascii_graphic())
        {
            return Err(TrainingError::InvalidProfile);
        }
        let profile: Profile =
            serde_json::from_slice(bytes).map_err(|_| TrainingError::InvalidProfile)?;
        // This independently qualified extraction is evidence, never the active
        // runtime artifact. The Content caller still owns bytes/pin/revision.
        let evidence: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/native-gameplay/build-training.json"
        ))
        .map_err(|_| TrainingError::InvalidProfile)?;
        let supplied: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| TrainingError::InvalidProfile)?;
        if supplied != evidence
            || profile.schema != "OTERYN_BUILD_TRAINING_PROFILE/v1"
            || profile.multiplier_storage != "ieee754_binary32"
            || profile.magic_base != 1600
            || profile.vocations.len() != 11
            || profile.source.files.len() != 3
            || profile.source.repository != "opentibiabr/canary"
            || profile.source.revision.len() != 40
            || profile
                .source
                .files
                .iter()
                .any(|f| f.path.is_empty() || f.sha256.len() != 64)
        {
            return Err(TrainingError::InvalidProfile);
        }
        for v in &profile.vocations {
            for value in std::iter::once(&v.mana_multiplier).chain(v.skill_multipliers.iter()) {
                let value = value
                    .parse::<f32>()
                    .map_err(|_| TrainingError::InvalidProfile)?;
                if !value.is_finite() || value <= 0.0 {
                    return Err(TrainingError::InvalidProfile);
                }
            }
        }
        let digest = Sha256::new()
            .chain_update(b"oteryn:build-training:v1\0")
            .chain_update(bytes)
            .chain_update(content_revision.as_bytes())
            .finalize()
            .into();
        Ok(Self {
            profile,
            content_revision: content_revision.to_owned(),
            digest,
        })
    }
    pub(crate) fn content_revision(&self) -> &str {
        &self.content_revision
    }
}
impl BuildFormula for CompiledTrainingFormula {
    fn required(&self, vocation: &str, family: usize, level: u16) -> Option<u64> {
        let v = self.profile.vocations.iter().find(|v| v.key == vocation)?;
        if family == 0 {
            if !(1..=MAX_BUILD_LEVEL).contains(&level) {
                return None;
            }
            // Source stores the XML literal in float, then promotes it to double.
            let multiplier = f64::from(v.mana_multiplier.parse::<f32>().ok()?);
            let value = f64::from(self.profile.magic_base) * multiplier.powf(f64::from(level - 1));
            (value.is_finite() && value < 9_223_372_036_854_775_808.0)
                .then_some(value.floor() as u64)
        } else {
            let index = family.checked_sub(1)?;
            skill_tries_required(
                *self.profile.skill_bases.get(index)?,
                f64::from(v.skill_multipliers.get(index)?.parse::<f32>().ok()?),
                level,
            )
        }
    }
    fn digest(&self) -> [u8; 32] {
        self.digest
    }
    fn content_revision(&self) -> &str {
        &self.content_revision
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiveManaTraining {
    durable: DurableBuildState,
    pending_mana: u64,
    checkpoint_started_micros: Option<u64>,
    last_cast_micros: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedManaTraining {
    before: LiveManaTraining,
    after: LiveManaTraining,
    pub(crate) request: BuildChangeRequest,
}
#[allow(
    clippy::large_enum_variant,
    reason = "transient owner result; boxing would add an allocation to the owner turn"
)]
pub(crate) enum TrainingStage {
    Live(LiveManaTraining),
    Durable(PreparedManaTraining),
}
impl LiveManaTraining {
    pub(crate) fn from_owned_build(build: &DurableBuildState) -> Self {
        Self {
            durable: build.clone(),
            pending_mana: 0,
            checkpoint_started_micros: None,
            last_cast_micros: None,
        }
    }
    pub(crate) fn durable(&self) -> &DurableBuildState {
        &self.durable
    }
    pub(crate) fn pending_mana(&self) -> u64 {
        self.pending_mana
    }
    pub(crate) fn prepare_paid_cast(
        &self,
        paid_mana: u32,
        now: u64,
        formula: &CompiledTrainingFormula,
        occurrence: BuildOccurrence,
    ) -> Result<TrainingStage, TrainingError> {
        self.prepare_delta(paid_mana, now, formula, occurrence, false)
    }
    fn prepare_delta(
        &self,
        paid_mana: u32,
        now: u64,
        formula: &CompiledTrainingFormula,
        occurrence: BuildOccurrence,
        force: bool,
    ) -> Result<TrainingStage, TrainingError> {
        if self.last_cast_micros.is_some_and(|previous| now < previous) {
            return Err(TrainingError::StaleTime);
        }
        let mut live = self.clone();
        live.pending_mana = live
            .pending_mana
            .saturating_add(u64::from(paid_mana))
            .min(MAX_BUILD_PROGRESS);
        live.last_cast_micros = Some(now);
        if live.pending_mana == 0 {
            return Ok(TrainingStage::Live(live));
        }
        let started = *live.checkpoint_started_micros.get_or_insert(now);
        let (mut level, stored) = self.durable.magic();
        let mut progress = stored
            .saturating_add(live.pending_mana)
            .min(MAX_BUILD_PROGRESS);
        if formula.required(self.durable.vocation(), 0, 1).is_none() {
            return Err(TrainingError::UnknownVocation);
        }
        while level < MAX_BUILD_LEVEL {
            let Some(required) = formula.required(self.durable.vocation(), 0, level + 1) else {
                break;
            };
            if progress < required {
                break;
            }
            progress -= required;
            level += 1;
        }
        if !force && level == self.durable.magic().0 && now.saturating_sub(started) < 60_000_000 {
            return Ok(TrainingStage::Live(live));
        }
        let after = DurableBuildState::new(
            self.durable.vocation(),
            (level, progress),
            self.durable.skills(),
        )
        .map_err(|_| TrainingError::InvalidBuild)?;
        let request = BuildChangeRequest {
            occurrence,
            cause: BuildCause::Training,
            before: self.durable.clone(),
            after: after.clone(),
            pruned_stance: None,
        };
        Ok(TrainingStage::Durable(PreparedManaTraining {
            before: self.clone(),
            after: LiveManaTraining {
                durable: after,
                pending_mana: 0,
                checkpoint_started_micros: None,
                last_cast_micros: Some(now),
            },
            request,
        }))
    }
}
impl LiveManaTraining {
    pub(crate) fn prepare_checkpoint(
        &self,
        now: u64,
        formula: &CompiledTrainingFormula,
        occurrence: BuildOccurrence,
        force: bool,
    ) -> Result<Option<PreparedManaTraining>, TrainingError> {
        match self.prepare_delta(0, now, formula, occurrence, force)? {
            TrainingStage::Live(_) => Ok(None),
            TrainingStage::Durable(value) => Ok(Some(value)),
        }
    }
}

impl PreparedManaTraining {
    pub(crate) fn commit(
        self,
        current: &LiveManaTraining,
        receipt: &CommittedBuildChange,
    ) -> Result<LiveManaTraining, TrainingError> {
        if current != &self.before
            || receipt.occurrence != self.request.occurrence
            || receipt.cause != BuildCause::Training
            || receipt.before != self.request.before
            || receipt.after != self.request.after
            || receipt.pruned_stance.is_some()
        {
            return Err(TrainingError::StaleReceipt);
        }
        Ok(self.after)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]
    use super::*;
    fn formula() -> CompiledTrainingFormula {
        CompiledTrainingFormula::from_profile(
            include_bytes!("../../../../tools/content-schema/native-gameplay/build-training.json"),
            "actual-content-r1",
        )
        .unwrap()
    }
    fn occurrence() -> BuildOccurrence {
        BuildOccurrence::from_bytes([0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]).unwrap()
    }
    fn seed() -> DurableBuildState {
        DurableBuildState::new("druid", (0, 0), [(10, 0); 7]).unwrap()
    }
    #[test]
    fn source_float_storage_and_all_vocations_are_preserved() {
        let f = formula();
        assert_eq!(f.required("druid", 0, 1), Some(1600));
        assert_eq!(f.required("druid", 0, 2), Some(1760));
        assert_eq!(f.required("knight", 0, 2), Some(4800));
        assert_eq!(f.required("none", 0, 2), Some(6400));
        assert_eq!(f.required("missing", 0, 1), None);
        assert_eq!(f.required("druid", 7, 11), Some(20));
        let mut profile: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../../tools/content-schema/native-gameplay/build-training.json"
        ))
        .unwrap();
        profile["magic_base"] = 1700.into();
        assert!(
            CompiledTrainingFormula::from_profile(
                &serde_json::to_vec(&profile).unwrap(),
                "actual-content-r1"
            )
            .is_err()
        );
    }
    #[test]
    fn live_cost_does_not_write_until_advance_or_sixty_second_checkpoint() {
        let f = formula();
        let initial = LiveManaTraining::from_owned_build(&seed());
        let TrainingStage::Live(first) = initial
            .prepare_paid_cast(20, 100, &f, occurrence())
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(first.pending_mana(), 20);
        assert!(matches!(
            first
                .prepare_paid_cast(20, 60_000_099, &f, occurrence())
                .unwrap(),
            TrainingStage::Live(_)
        ));
        let TrainingStage::Durable(checkpoint) = first
            .prepare_paid_cast(20, 60_000_100, &f, occurrence())
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(checkpoint.request.after.magic(), (0, 40));
        assert!(first.prepare_paid_cast(20, 99, &f, occurrence()).is_err());
        let TrainingStage::Durable(advance) = first
            .prepare_paid_cast(1580, 101, &f, occurrence())
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(advance.request.after.magic(), (1, 0));
        assert_eq!(first.pending_mana(), 20);
    }
    #[test]
    fn zero_cost_skips_a_receipt_and_multi_level_advance_is_exact() {
        let f = formula();
        let live = LiveManaTraining::from_owned_build(&seed());
        let TrainingStage::Live(zero) = live.prepare_paid_cast(0, 1, &f, occurrence()).unwrap()
        else {
            panic!()
        };
        assert_eq!(zero.pending_mana(), 0);
        let TrainingStage::Durable(advance) =
            zero.prepare_paid_cast(3360, 2, &f, occurrence()).unwrap()
        else {
            panic!()
        };
        assert_eq!(advance.request.after.magic(), (2, 0));
    }
}

/// A paid caster's training transition, staged without changing the actor.
/// The durable branch cannot install its new level without a real build receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedPlayerTraining {
    before: LiveManaTraining,
    live_after: Option<LiveManaTraining>,
    durable: Option<PreparedManaTraining>,
    installation: Option<PreparedTrainingInstall>,
    source_training_mana: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedTrainingInstall {
    after: LiveManaTraining,
    witness: TrainingPaymentWitness,
}
impl super::cast::PlayerSpellState {
    pub(crate) fn enable_owned_training(
        &mut self,
        build: &DurableBuildState,
        formula: &CompiledTrainingFormula,
    ) -> Result<(), TrainingError> {
        if super::Vocation::from_key(build.vocation()) != Some(self.character_facts().vocation)
            || u32::from(build.magic().0) != self.character_facts().magic_level
            || formula.required(build.vocation(), 0, 1).is_none()
        {
            return Err(TrainingError::InvalidBuild);
        }
        match &self.training {
            Some(current) if current.durable() != build => Err(TrainingError::InvalidBuild),
            Some(_) => Ok(()),
            None => {
                self.training = Some(LiveManaTraining::from_owned_build(build));
                Ok(())
            }
        }
    }
    pub(crate) fn prepare_paid_training(
        &self,
        paid: &Self,
        anchor: &super::combat_batch::SpellAnchor,
        formula: &CompiledTrainingFormula,
        occurrence: BuildOccurrence,
        now: u64,
    ) -> Result<PreparedPlayerTraining, TrainingError> {
        self.prepare_training_with_source_mana(
            paid,
            anchor,
            formula,
            occurrence,
            now,
            anchor.paid_mana,
        )
    }
    /// Summon and Convince source scripts train their qualified Creature mana
    /// cost even when the actual account-owned InfiniteMana flag skips deduction.
    /// A primitive supplied amount cannot authorize this exception.
    pub(crate) fn prepare_direct_acquisition_training(
        &self,
        paid: &Self,
        anchor: &super::combat_batch::SpellAnchor,
        formula: &CompiledTrainingFormula,
        occurrence: BuildOccurrence,
        now: u64,
        acquisition: &super::companion_lifecycle::DirectCompanionReservation,
    ) -> Result<PreparedPlayerTraining, TrainingError> {
        if acquisition.mana_to_deduct() != anchor.paid_mana {
            return Err(TrainingError::InvalidBuild);
        }
        self.prepare_training_with_source_mana(
            paid,
            anchor,
            formula,
            occurrence,
            now,
            acquisition.mana_spent(),
        )
    }
    fn prepare_training_with_source_mana(
        &self,
        paid: &Self,
        anchor: &super::combat_batch::SpellAnchor,
        formula: &CompiledTrainingFormula,
        occurrence: BuildOccurrence,
        now: u64,
        source_training_mana: u32,
    ) -> Result<PreparedPlayerTraining, TrainingError> {
        if !paid.paid_successor_of(self, anchor) || paid.training != self.training {
            return Err(TrainingError::InvalidBuild);
        }
        let current = self.training.as_ref().ok_or(TrainingError::InvalidBuild)?;
        let (live_after, durable) =
            match current.prepare_paid_cast(source_training_mana, now, formula, occurrence)? {
                TrainingStage::Live(live) => (Some(live), None),
                TrainingStage::Durable(prepared) => (None, Some(prepared)),
            };
        // Every successor/witness allocation is complete before the committing
        // SQL await. The eventual receipt only qualifies these exact data.
        let after = match (&live_after, &durable) {
            (Some(after), None) => after.clone(),
            (None, Some(prepared)) => prepared.after.clone(),
            _ => return Err(TrainingError::InvalidBuild),
        };
        let installation = PreparedTrainingInstall {
            witness: TrainingPaymentWitness {
                before: current.clone(),
                after: after.clone(),
                paid_mana: anchor.paid_mana,
                source_training_mana,
            },
            after,
        };
        Ok(PreparedPlayerTraining {
            before: current.clone(),
            live_after,
            durable,
            installation: Some(installation),
            source_training_mana,
        })
    }
}
impl PreparedPlayerTraining {
    pub(crate) fn request(&self) -> Option<&BuildChangeRequest> {
        self.durable.as_ref().map(|value| &value.request)
    }
    /// The caller installs this fully prepared successor with its physical batch.
    /// Call before any owner mutation; mismatch leaves the actual state untouched.
    #[allow(
        clippy::expect_used,
        reason = "post-validation commit invariant; a fallible exit here would leave a partial owner write"
    )]
    pub(crate) fn prepare_install(
        &mut self,
        current: &super::cast::PlayerSpellState,
        paid: &mut super::cast::PlayerSpellState,
        anchor: &super::combat_batch::SpellAnchor,
        receipt: Option<&CommittedBuildChange>,
    ) -> Result<(), TrainingError> {
        if current.training.as_ref() != Some(&self.before)
            || !paid.paid_successor_of(current, anchor)
            || paid.training != current.training
        {
            return Err(TrainingError::InvalidBuild);
        }
        let expected_after = match (&self.live_after, &self.durable, receipt) {
            (Some(live), None, None) => live,
            (None, Some(prepared), Some(receipt)) => {
                if receipt.occurrence != prepared.request.occurrence
                    || receipt.cause != BuildCause::Training
                    || receipt.before != prepared.request.before
                    || receipt.after != prepared.request.after
                    || receipt.pruned_stance.is_some()
                    || prepared.before != self.before
                {
                    return Err(TrainingError::StaleReceipt);
                }
                &prepared.after
            }
            _ => return Err(TrainingError::StaleReceipt),
        };
        let installation = self
            .installation
            .as_ref()
            .ok_or(TrainingError::StaleReceipt)?;
        if &installation.after != expected_after
            || installation.witness.before != self.before
            || installation.witness.after != installation.after
            || installation.witness.paid_mana != anchor.paid_mana
            || installation.witness.source_training_mana != self.source_training_mana
        {
            return Err(TrainingError::StaleReceipt);
        }
        // No clone, allocation, calculation or fallible branch remains.
        let installation = self
            .installation
            .take()
            .expect("validated allocated training successor");
        paid.facts.magic_level = u32::from(installation.after.durable().magic().0);
        paid.training_payment = Some(installation.witness);
        paid.training = Some(installation.after);
        Ok(())
    }
}

/// Private construction follows complete staging and actual receipt validation.
/// This proves only the training/payment data relation; current authority is
/// checked independently at the physical owner mutation boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrainingPaymentWitness {
    before: LiveManaTraining,
    after: LiveManaTraining,
    paid_mana: u32,
    source_training_mana: u32,
}
impl TrainingPaymentWitness {
    pub(crate) fn matches(
        &self,
        before: Option<&LiveManaTraining>,
        after: Option<&LiveManaTraining>,
        paid_mana: u32,
        before_facts: super::cast::CharacterCastFacts,
        mut after_facts: super::cast::CharacterCastFacts,
    ) -> bool {
        let trained_magic = u32::from(self.after.durable().magic().0);
        if after_facts.magic_level != trained_magic {
            return false;
        }
        after_facts.magic_level = before_facts.magic_level;
        before == Some(&self.before)
            && after == Some(&self.after)
            && paid_mana == self.paid_mana
            && before_facts == after_facts
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RetainedTrainingCheckpoint {
    pub(crate) fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    pub(crate) prepared: PreparedManaTraining,
    pub(crate) formula_digest: [u8; 32],
}
impl super::cast::PlayerSpellState {
    pub(crate) fn prepare_training_checkpoint(
        &self,
        now: u64,
        formula: &CompiledTrainingFormula,
        occurrence: BuildOccurrence,
        force: bool,
    ) -> Result<Option<PreparedManaTraining>, TrainingError> {
        self.training
            .as_ref()
            .map(|value| value.prepare_checkpoint(now, formula, occurrence, force))
            .transpose()
            .map(Option::flatten)
    }
    pub(crate) fn commit_training_checkpoint(
        &mut self,
        prepared: PreparedManaTraining,
        receipt: &CommittedBuildChange,
    ) -> Result<(), TrainingError> {
        let current = self.training.as_ref().ok_or(TrainingError::InvalidBuild)?;
        let next = prepared.commit(current, receipt)?;
        self.facts.magic_level = u32::from(next.durable().magic().0);
        self.training = Some(next);
        self.training_payment = None;
        self.training_checkpoint = None;
        Ok(())
    }
}
impl super::cast::PlayerSpellState {
    pub(crate) fn pending_training_checkpoint(&self) -> Option<&RetainedTrainingCheckpoint> {
        self.training_checkpoint.as_deref()
    }
    pub(crate) fn retain_training_checkpoint(
        &mut self,
        value: RetainedTrainingCheckpoint,
    ) -> Result<(), TrainingError> {
        if self.training_checkpoint.is_some()
            || self.training.as_ref() != Some(&value.prepared.before)
        {
            return Err(TrainingError::InvalidBuild);
        }
        self.training_checkpoint = Some(Box::new(value));
        Ok(())
    }
}

impl super::cast::PlayerSpellState {
    /// Move the receipt-qualified training fields into an already staged player
    /// successor, retaining all of its pre-applied combat effects. No allocation.
    pub(crate) fn rebind_staged_training(
        &mut self,
        before: &Self,
        anchor: &super::combat_batch::SpellAnchor,
        mut qualified_paid: Self,
    ) -> Result<(), TrainingError> {
        if !qualified_paid.paid_successor_of(before, anchor)
            || self.training != before.training
            || self.facts != before.facts
            || self.revision != anchor.next_revision
            || qualified_paid.training_payment.is_none()
        {
            return Err(TrainingError::InvalidBuild);
        }
        self.facts.magic_level = qualified_paid.facts.magic_level;
        self.training = qualified_paid.training.take();
        self.training_payment = qualified_paid.training_payment.take();
        Ok(())
    }
}

impl super::cast::PlayerSpellState {
    /// A definitely rolled-back attempt may refresh only the independently read
    /// root revision. The original occurrence and training successor are retained.
    pub(crate) fn refresh_training_checkpoint_revision(
        &mut self,
        expected: &RetainedTrainingCheckpoint,
        current: crate::durability::character_progression::CurrentCharacterGameplayFence,
    ) -> Result<(), TrainingError> {
        let retained = self
            .training_checkpoint
            .as_mut()
            .ok_or(TrainingError::InvalidBuild)?;
        let mut comparable = current;
        comparable.expected_character_revision = expected.fence.expected_character_revision;
        if retained.as_ref() != expected
            || comparable != expected.fence
            || current.expected_character_revision.get()
                <= expected.fence.expected_character_revision.get()
        {
            return Err(TrainingError::InvalidBuild);
        }
        retained.fence = current;
        Ok(())
    }
}
