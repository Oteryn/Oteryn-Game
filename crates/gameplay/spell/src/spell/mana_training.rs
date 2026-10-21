//! D151: actual paid casts accumulate training in the player's existing state.
//! A level advance/checkpoint is staged before mutation and exposed only after
//! its genuine Character build receipt commits in the shared transaction.
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildFormula, BuildOccurrence, CommittedBuildChange,
    DurableBuildState, MAX_BUILD_PROGRESS,
};

pub mod hit_training;
use hit_training::{HitTrainingCounters, SkillFamily};

pub use crate::content::training_formula::{CompiledTrainingFormula, TrainingError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveManaTraining {
    durable: DurableBuildState,
    pending_mana: u64,
    /// W2b: skill tries from hits, banked only by a checkpoint receipt (never by a cast, whose
    /// paid successor keeps the owned skill levels).
    pending_skills: [u64; 7],
    hits: HitTrainingCounters,
    checkpoint_started_micros: Option<u64>,
    last_cast_micros: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedManaTraining {
    before: LiveManaTraining,
    after: LiveManaTraining,
    pub request: BuildChangeRequest,
    /// Hits resolved while this checkpoint is retained for replay. They cannot touch the live
    /// training (the replay commits only onto its exact `before`), so they wait here and merge
    /// into `after` with the receipt.
    post_checkpoint: Option<PostCheckpointTries>,
}
/// Skill tries and hit streaks after a retained checkpoint was prepared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PostCheckpointTries {
    pending_skills: [u64; 7],
    hits: HitTrainingCounters,
    started_micros: u64,
    last_micros: u64,
}
#[allow(
    clippy::large_enum_variant,
    reason = "transient owner result; boxing would add an allocation to the owner turn"
)]
pub enum TrainingStage {
    Live(LiveManaTraining),
    Durable(PreparedManaTraining),
}
impl LiveManaTraining {
    pub fn from_owned_build(build: &DurableBuildState) -> Self {
        Self {
            durable: build.clone(),
            pending_mana: 0,
            pending_skills: [0; 7],
            hits: HitTrainingCounters::default(),
            checkpoint_started_micros: None,
            last_cast_micros: None,
        }
    }
    pub fn pending_skills(&self) -> [u64; 7] {
        self.pending_skills
    }
    /// Whether the pending tries of `family` reach its next level.
    fn skill_level_due(
        &self,
        formula: &CompiledTrainingFormula,
        family: SkillFamily,
    ) -> Result<bool, TrainingError> {
        skill_level_due(
            formula,
            self.durable.vocation(),
            family,
            self.durable.skills()[family.index()],
            self.pending_skills[family.index()],
        )
    }
    /// Whether pending mana or any pending skill tries reach a level now.
    fn level_due(&self, formula: &CompiledTrainingFormula) -> bool {
        level_due(
            formula,
            self.durable.vocation(),
            self.durable.magic(),
            self.pending_mana,
            self.durable.skills(),
            self.pending_skills,
        )
    }
    /// The hit streaks the caller resolves a hit with, before it records the tries.
    pub fn hits_mut(&mut self) -> &mut HitTrainingCounters {
        &mut self.hits
    }
    /// Bank `tries` for `family` live. The next checkpoint (tick, logout) commits them; one that
    /// reaches a level writes at once, as A13 §2 requires.
    pub fn record_skill_tries(
        &mut self,
        family: SkillFamily,
        tries: u64,
        now: u64,
    ) -> Result<(), TrainingError> {
        if self.last_cast_micros.is_some_and(|previous| now < previous) {
            return Err(TrainingError::StaleTime);
        }
        if tries == 0 {
            return Ok(());
        }
        let pending = &mut self.pending_skills[family.index()];
        *pending = pending.saturating_add(tries).min(MAX_BUILD_PROGRESS);
        self.checkpoint_started_micros.get_or_insert(now);
        self.last_cast_micros = Some(now);
        Ok(())
    }
    pub fn durable(&self) -> &DurableBuildState {
        &self.durable
    }
    pub fn pending_mana(&self) -> u64 {
        self.pending_mana
    }
    pub fn prepare_paid_cast(
        &self,
        paid_mana: u32,
        now: u64,
        formula: &CompiledTrainingFormula,
        occurrence: BuildOccurrence,
    ) -> Result<TrainingStage, TrainingError> {
        self.prepare_delta(paid_mana, now, formula, occurrence, false, false)
    }
    /// `with_skills`: a checkpoint also banks the pending skill tries; a cast banks mana only.
    fn prepare_delta(
        &self,
        paid_mana: u32,
        now: u64,
        formula: &CompiledTrainingFormula,
        occurrence: BuildOccurrence,
        force: bool,
        with_skills: bool,
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
        let skills_pending = live.pending_skills.iter().any(|tries| *tries > 0);
        if live.pending_mana == 0 && !(with_skills && skills_pending) {
            return Ok(TrainingStage::Live(live));
        }
        let started = *live.checkpoint_started_micros.get_or_insert(now);
        let vocation = self.durable.vocation();
        if formula.required(vocation, 0, 1).is_none() {
            return Err(TrainingError::UnknownVocation);
        }
        let magic = advance(
            formula,
            vocation,
            0,
            self.durable.magic(),
            live.pending_mana,
        );
        let mut advanced = magic.0 != self.durable.magic().0;
        let mut skills = self.durable.skills();
        if with_skills {
            for (index, skill) in skills.iter_mut().enumerate() {
                let next = advance(
                    formula,
                    vocation,
                    index + 1,
                    *skill,
                    live.pending_skills[index],
                );
                advanced |= next.0 != skill.0;
                *skill = next;
            }
        }
        if !force && !advanced && now.saturating_sub(started) < 60_000_000 {
            return Ok(TrainingStage::Live(live));
        }
        let after = DurableBuildState::new(vocation, magic, skills)
            .map_err(|_| TrainingError::InvalidBuild)?;
        // A cast leaves the skill tries pending in their original checkpoint window, so casting
        // never postpones them: the next tick past that window banks them.
        let (pending_skills, checkpoint_started_micros) = if with_skills || !skills_pending {
            ([0; 7], None)
        } else {
            (live.pending_skills, Some(started))
        };
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
                pending_skills,
                hits: live.hits,
                checkpoint_started_micros,
                last_cast_micros: Some(now),
            },
            request,
            post_checkpoint: None,
        }))
    }
}
impl LiveManaTraining {
    pub fn prepare_checkpoint(
        &self,
        now: u64,
        formula: &CompiledTrainingFormula,
        occurrence: BuildOccurrence,
        force: bool,
    ) -> Result<Option<PreparedManaTraining>, TrainingError> {
        match self.prepare_delta(0, now, formula, occurrence, force, true)? {
            TrainingStage::Live(_) => Ok(None),
            TrainingStage::Durable(value) => Ok(Some(value)),
        }
    }
}

/// Bank `pending` onto `(level, progress)` of `family` (0 magic, 1..=7 the skills): every level
/// the formula reaches advances; progress saturates at the stored bound.
pub use oteryn_gameplay_progression::mana_training::{advance, level_due, skill_level_due};

impl PreparedManaTraining {
    pub fn commit(
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
    /// `after` with the hits resolved since this checkpoint was prepared.
    fn after_with_post_checkpoint(&self) -> LiveManaTraining {
        let mut next = self.after.clone();
        if let Some(post) = self.post_checkpoint {
            for (pending, tries) in next.pending_skills.iter_mut().zip(post.pending_skills) {
                *pending = pending.saturating_add(tries).min(MAX_BUILD_PROGRESS);
            }
            next.hits = post.hits;
            let started = next
                .checkpoint_started_micros
                .map_or(post.started_micros, |value| value.min(post.started_micros));
            next.checkpoint_started_micros = Some(started);
            next.last_cast_micros = next.last_cast_micros.max(Some(post.last_micros));
        }
        next
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]
    use super::*;
    fn formula() -> CompiledTrainingFormula {
        CompiledTrainingFormula::from_profile(
            include_bytes!(
                "../../../../../tools/content-schema/native-gameplay/build-training.json"
            ),
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
    #[test]
    fn skill_tries_bank_on_a_checkpoint_and_a_reached_level_writes_at_once() {
        let f = formula();
        let mut live = LiveManaTraining::from_owned_build(&seed());
        live.record_skill_tries(SkillFamily::Sword, 10, 5).unwrap();
        assert!(
            live.prepare_checkpoint(6, &f, occurrence(), false)
                .unwrap()
                .is_none()
        );
        // The 60 s window from the first try, then a forced (logout) checkpoint.
        let window = live
            .prepare_checkpoint(60_000_005, &f, occurrence(), false)
            .unwrap()
            .unwrap();
        assert_eq!(window.request.after.skills()[2], (10, 10));
        assert_eq!(window.request.after.magic(), (0, 0));
        assert_eq!(window.after.pending_skills(), [0; 7]);
        let forced = live
            .prepare_checkpoint(6, &f, occurrence(), true)
            .unwrap()
            .unwrap();
        assert_eq!(forced.request.after, window.request.after);
        // Druid sword level 11 needs 50 tries (SKILLS-0 §2): reaching it writes before 60 s,
        // with any leftover kept and every family banked in the one receipt.
        live.record_skill_tries(SkillFamily::Sword, 45, 7).unwrap();
        live.record_skill_tries(SkillFamily::Fishing, 3, 7).unwrap();
        let advance = live
            .prepare_checkpoint(8, &f, occurrence(), false)
            .unwrap()
            .unwrap();
        assert_eq!(advance.request.after.skills()[2], (11, 5));
        assert_eq!(advance.request.after.skills()[6], (10, 3));
        assert_eq!(advance.request.cause, BuildCause::Training);
        assert!(live.record_skill_tries(SkillFamily::Fist, 1, 6).is_err());
    }
    #[test]
    fn a_cast_banks_mana_only_and_keeps_skill_tries_pending() {
        let f = formula();
        let mut live = LiveManaTraining::from_owned_build(&seed());
        live.record_skill_tries(SkillFamily::Sword, 60, 1).unwrap();
        let TrainingStage::Durable(cast) =
            live.prepare_paid_cast(1600, 2, &f, occurrence()).unwrap()
        else {
            panic!()
        };
        assert_eq!(cast.request.after.magic(), (1, 0));
        assert_eq!(cast.request.after.skills(), seed().skills());
        assert_eq!(cast.after.pending_skills()[2], 60);
        // The cast keeps the skill window opened by the first try at t=1.
        assert!(
            cast.after
                .prepare_checkpoint(60_000_001, &f, occurrence(), false)
                .unwrap()
                .is_some()
        );
        let mut slow = LiveManaTraining::from_owned_build(&seed());
        slow.record_skill_tries(SkillFamily::Sword, 5, 1).unwrap();
        let TrainingStage::Durable(slow_cast) =
            slow.prepare_paid_cast(1600, 2, &f, occurrence()).unwrap()
        else {
            panic!()
        };
        assert!(
            slow_cast
                .after
                .prepare_checkpoint(60_000_000, &f, occurrence(), false)
                .unwrap()
                .is_none()
        );
        let window = slow_cast
            .after
            .prepare_checkpoint(60_000_001, &f, occurrence(), false)
            .unwrap()
            .unwrap();
        assert_eq!(window.request.after.skills()[2], (10, 5));
        let TrainingStage::Live(only_mana) =
            live.prepare_paid_cast(20, 3, &f, occurrence()).unwrap()
        else {
            panic!()
        };
        assert_eq!(only_mana.pending_skills()[2], 60);
        let bank = cast
            .after
            .prepare_checkpoint(4, &f, occurrence(), false)
            .unwrap()
            .unwrap();
        assert_eq!(bank.request.after.skills()[2], (11, 10));
        assert_eq!(bank.request.after.magic(), (1, 0));
    }
    #[test]
    fn a_skill_level_is_live_only_after_its_checkpoint_receipt() {
        use crate::domain::{CharacterId, CharacterRevision};
        let f = formula();
        let facts = crate::spell::cast::CharacterCastFacts {
            vocation: crate::spell::Vocation::Druid,
            level: 8,
            magic_level: 0,
            max_health: 185,
            max_mana: 90,
            max_soul: 100,
        };
        let mut state = crate::spell::cast::PlayerSpellState::new(facts, 0, 0).unwrap();
        // Without owned training (the production off switch) nothing trains.
        assert_eq!(
            state
                .train_skill(SkillFamily::Sword, 1, &f, |c| c
                    .melee_hit(hit_training::HitBlock::Unblocked))
                .unwrap(),
            TrainedSkill::default()
        );
        state.load_owned_build(&seed()).unwrap();
        state.enable_owned_training(&seed(), &f).unwrap();
        assert!(!state.has_training_to_flush());
        // 49 of the 50 druid sword tries wait for the cadence; the 50th is due at once.
        assert_eq!(
            state
                .train_skill(SkillFamily::Sword, 2, &f, |_| 49)
                .unwrap(),
            TrainedSkill {
                tries: 49,
                checkpoint_due: false
            }
        );
        assert!(
            state
                .prepare_training_checkpoint(2, &f, occurrence(), false)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            state.train_skill(SkillFamily::Sword, 2, &f, |_| 1).unwrap(),
            TrainedSkill {
                tries: 1,
                checkpoint_due: true
            }
        );
        assert!(state.has_training_to_flush());
        assert_eq!(state.owned_skills.unwrap()[2], 10);
        let prepared = state
            .prepare_training_checkpoint(3, &f, occurrence(), false)
            .unwrap()
            .unwrap();
        let receipt = CommittedBuildChange {
            occurrence: occurrence(),
            character_id: CharacterId::from_bytes([
                7, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 7,
            ])
            .unwrap(),
            original_character_revision: CharacterRevision::new(1).unwrap(),
            committed_character_revision: CharacterRevision::new(2).unwrap(),
            cause: BuildCause::Training,
            before: prepared.request.before.clone(),
            after: prepared.request.after.clone(),
            pruned_stance: None,
        };
        state
            .commit_training_checkpoint(prepared, &receipt)
            .unwrap();
        assert_eq!(state.owned_skills.unwrap()[2], 11);
        assert!(!state.has_training_to_flush());
    }
    fn test_fence() -> crate::durability::character_progression::CurrentCharacterGameplayFence {
        use crate::domain::{CharacterId, CharacterRevision};
        use crate::foundation::{
            ChannelId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1,
            ScopeOwnershipGeneration, WorldId,
        };
        let id = |seed: u8| {
            [
                seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
            ]
        };
        crate::durability::character_progression::CurrentCharacterGameplayFence {
            character_id: CharacterId::from_bytes(id(7)).unwrap(),
            game_session_id: GameSessionId::decode(&id(3)).unwrap(),
            connection_generation: ConnectionGeneration::new(1).unwrap(),
            character_lease_generation: 1,
            runtime_scope: RuntimeScopeRefV1::channel(
                WorldId::decode(&id(4)).unwrap(),
                ChannelId::decode(&id(5)).unwrap(),
            ),
            scope_ownership_generation: ScopeOwnershipGeneration::new(1).unwrap(),
            expected_character_revision: CharacterRevision::new(1).unwrap(),
        }
    }
    #[test]
    fn hits_during_a_retained_checkpoint_merge_and_their_level_commits() {
        use crate::domain::{CharacterId, CharacterRevision};
        let f = formula();
        let facts = crate::spell::cast::CharacterCastFacts {
            vocation: crate::spell::Vocation::Druid,
            level: 8,
            magic_level: 0,
            max_health: 185,
            max_mana: 90,
            max_soul: 100,
        };
        let mut state = crate::spell::cast::PlayerSpellState::new(facts, 0, 0).unwrap();
        state.load_owned_build(&seed()).unwrap();
        state.enable_owned_training(&seed(), &f).unwrap();
        let receipt_for = |prepared: &PreparedManaTraining, revision: u64| CommittedBuildChange {
            occurrence: prepared.request.occurrence,
            character_id: CharacterId::from_bytes([
                7, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 7,
            ])
            .unwrap(),
            original_character_revision: CharacterRevision::new(revision).unwrap(),
            committed_character_revision: CharacterRevision::new(revision + 1).unwrap(),
            cause: BuildCause::Training,
            before: prepared.request.before.clone(),
            after: prepared.request.after.clone(),
            pruned_stance: None,
        };
        // A forced checkpoint is retained: its save returned `Unknown`.
        state
            .train_skill(SkillFamily::Sword, 1, &f, |_| 10)
            .unwrap();
        let prepared = state
            .prepare_training_checkpoint(2, &f, occurrence(), true)
            .unwrap()
            .unwrap();
        state
            .retain_training_checkpoint(RetainedTrainingCheckpoint {
                fence: test_fence(),
                prepared: prepared.clone(),
                formula_digest: f.digest(),
            })
            .unwrap();
        // Hits after it are kept, and the one that reaches a level is still due at once.
        assert_eq!(
            state
                .train_skill(SkillFamily::Sword, 3, &f, |_| 39)
                .unwrap(),
            TrainedSkill {
                tries: 39,
                checkpoint_due: false
            }
        );
        assert_eq!(
            state.train_skill(SkillFamily::Sword, 4, &f, |_| 1).unwrap(),
            TrainedSkill {
                tries: 1,
                checkpoint_due: true
            }
        );
        // The retry commits the retained receipt and merges the later tries.
        let retained = state.pending_training_checkpoint().unwrap().clone();
        let receipt = receipt_for(&retained.prepared, 1);
        state
            .commit_training_checkpoint(retained.prepared, &receipt)
            .unwrap();
        assert_eq!(state.owned_skills.unwrap()[2], 10);
        assert!(state.has_training_to_flush());
        // The save drains it before reporting `Saved`: the reached level writes at once, without waiting for the cadence.
        let level = state
            .prepare_training_checkpoint(5, &f, occurrence(), false)
            .unwrap()
            .unwrap();
        assert!(state.training_successor_due(&f, false));
        assert_eq!(level.request.after.skills()[2], (11, 0));
        let receipt = receipt_for(&level, 2);
        state.commit_training_checkpoint(level, &receipt).unwrap();
        assert_eq!(state.owned_skills.unwrap()[2], 11);
        assert!(!state.has_training_to_flush());
        assert!(!state.training_successor_due(&f, false));
    }
    #[test]
    fn a_forced_save_drains_post_checkpoint_tries_below_a_level() {
        use crate::domain::{CharacterId, CharacterRevision};
        let f = formula();
        let facts = crate::spell::cast::CharacterCastFacts {
            vocation: crate::spell::Vocation::Druid,
            level: 8,
            magic_level: 0,
            max_health: 185,
            max_mana: 90,
            max_soul: 100,
        };
        let mut state = crate::spell::cast::PlayerSpellState::new(facts, 0, 0).unwrap();
        state.load_owned_build(&seed()).unwrap();
        state.enable_owned_training(&seed(), &f).unwrap();
        let receipt_for = |prepared: &PreparedManaTraining| CommittedBuildChange {
            occurrence: prepared.request.occurrence,
            character_id: CharacterId::from_bytes([
                7, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 7,
            ])
            .unwrap(),
            original_character_revision: CharacterRevision::new(1).unwrap(),
            committed_character_revision: CharacterRevision::new(2).unwrap(),
            cause: BuildCause::Training,
            before: prepared.request.before.clone(),
            after: prepared.request.after.clone(),
            pruned_stance: None,
        };
        state
            .train_skill(SkillFamily::Sword, 1, &f, |_| 10)
            .unwrap();
        let prepared = state
            .prepare_training_checkpoint(2, &f, occurrence(), true)
            .unwrap()
            .unwrap();
        state
            .retain_training_checkpoint(RetainedTrainingCheckpoint {
                fence: test_fence(),
                prepared,
                formula_digest: f.digest(),
            })
            .unwrap();
        // Tries after the retained checkpoint that reach no level.
        assert!(
            !state
                .train_skill(SkillFamily::Sword, 3, &f, |_| 5)
                .unwrap()
                .checkpoint_due
        );
        let retained = state.pending_training_checkpoint().unwrap().clone();
        let receipt = receipt_for(&retained.prepared);
        state
            .commit_training_checkpoint(retained.prepared, &receipt)
            .unwrap();
        // A cadence save may leave them; a forced (logout) save writes them before `Saved`.
        assert!(!state.training_successor_due(&f, false));
        assert!(state.training_successor_due(&f, true));
        let rest = state
            .prepare_training_checkpoint(4, &f, occurrence(), true)
            .unwrap()
            .unwrap();
        assert_eq!(rest.request.after.skills()[2], (10, 15));
        let receipt = receipt_for(&rest);
        state.commit_training_checkpoint(rest, &receipt).unwrap();
        assert!(!state.training_successor_due(&f, true));
    }
    #[test]
    fn hit_streaks_feed_the_pending_tries() {
        let mut live = LiveManaTraining::from_owned_build(&seed());
        let tries = live.hits_mut().melee_hit(hit_training::HitBlock::Unblocked);
        live.record_skill_tries(SkillFamily::Club, tries, 1)
            .unwrap();
        let tries = live.hits_mut().block_hit(true);
        live.record_skill_tries(SkillFamily::Shielding, tries, 2)
            .unwrap();
        live.record_skill_tries(SkillFamily::Fist, 0, 3).unwrap();
        assert_eq!(live.pending_skills(), [0, 1, 0, 0, 0, 1, 0]);
    }
}

/// A paid caster's training transition, staged without changing the actor.
/// The durable branch cannot install its new level without a real build receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlayerTraining {
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
    pub fn enable_owned_training(
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
    pub fn prepare_paid_training(
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
    pub fn prepare_direct_acquisition_training(
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
    pub fn request(&self) -> Option<&BuildChangeRequest> {
        self.durable.as_ref().map(|value| &value.request)
    }
    /// The caller installs this fully prepared successor with its physical batch.
    /// Call before any owner mutation; mismatch leaves the actual state untouched.
    pub fn prepare_install(
        &mut self,
        current: &super::cast::PlayerSpellState,
        paid: &mut super::cast::PlayerSpellState,
        anchor: &super::combat_batch::SpellAnchor,
        receipt: Option<&CommittedBuildChange>,
    ) -> Result<(), TrainingError> {
        self.check_install(current, paid, anchor, receipt)?;
        self.install_into(paid);
        Ok(())
    }

    /// The borrowing check of [`Self::prepare_install`] (ARCH-SPELL-LOCK-2 §1.6): it takes
    /// nothing from the prepared training and changes no payment.
    pub fn check_install(
        &self,
        current: &super::cast::PlayerSpellState,
        paid: &super::cast::PlayerSpellState,
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
        Ok(())
    }

    /// The payment [`Self::install_into`] would produce, for the phase-1 checks that follow it.
    /// It clones and takes nothing.
    pub fn qualified_preview(
        &self,
        paid: &super::cast::PlayerSpellState,
    ) -> Result<super::cast::PlayerSpellState, TrainingError> {
        let installation = self
            .installation
            .as_ref()
            .ok_or(TrainingError::StaleReceipt)?;
        let mut preview = paid.clone();
        preview.facts.magic_level = u32::from(installation.after.durable().magic().0);
        preview.training_payment = Some(installation.witness.clone());
        preview.training = Some(installation.after.clone());
        Ok(preview)
    }

    /// The infallible move, only after [`Self::check_install`] passed under the same guards.
    #[allow(
        clippy::expect_used,
        reason = "post-validation commit invariant; a fallible exit here would leave a partial owner write"
    )]
    pub fn install_into(&mut self, paid: &mut super::cast::PlayerSpellState) {
        // No clone, allocation, calculation or fallible branch remains.
        let installation = self
            .installation
            .take()
            .expect("validated allocated training successor");
        paid.facts.magic_level = u32::from(installation.after.durable().magic().0);
        paid.training_payment = Some(installation.witness);
        paid.training = Some(installation.after);
    }
}

/// Private construction follows complete staging and actual receipt validation.
/// This proves only the training/payment data relation; current authority is
/// checked independently at the physical owner mutation boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainingPaymentWitness {
    before: LiveManaTraining,
    after: LiveManaTraining,
    paid_mana: u32,
    source_training_mana: u32,
}
impl TrainingPaymentWitness {
    pub fn matches(
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

/// The outcome of [`super::cast::PlayerSpellState::train_skill`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrainedSkill {
    pub tries: u64,
    /// The pending tries reach a level: the caller saves the checkpoint now.
    pub checkpoint_due: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedTrainingCheckpoint {
    pub fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    pub prepared: PreparedManaTraining,
    pub formula_digest: [u8; 32],
}
impl super::cast::PlayerSpellState {
    pub fn prepare_training_checkpoint(
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
    pub fn commit_training_checkpoint(
        &mut self,
        prepared: PreparedManaTraining,
        receipt: &CommittedBuildChange,
    ) -> Result<(), TrainingError> {
        let current = self.training.as_ref().ok_or(TrainingError::InvalidBuild)?;
        // The post-checkpoint hits bank on the retained copy; the caller may pass a clone.
        let mut prepared = prepared;
        if let Some(retained) = self.training_checkpoint.as_ref() {
            prepared.post_checkpoint = retained.prepared.post_checkpoint;
        }
        let next = prepared.after_with_post_checkpoint();
        prepared.commit(current, receipt)?;
        self.facts.magic_level = u32::from(next.durable().magic().0);
        // A banked skill level is live only now, after its receipt (A13 §2).
        if self.owned_skills.is_some() {
            self.owned_skills = Some(next.durable().skills().map(|(level, _)| level));
        }
        self.training = Some(next);
        self.training_payment = None;
        self.training_checkpoint = None;
        Ok(())
    }
    /// DEATH-ML-LOSS-1 (A13 §4.6, CP D1433): a committed death lowered the durable build to
    /// `build`, so the live actor continues from it. Owned training restarts from `build` with
    /// nothing pending, the magic level and owned skill levels drop to its levels, and every
    /// pre-death checkpoint and payment witness is retired: the next training save submits
    /// `build` as its `before`. The caller drained pending training before the death.
    pub fn rebase_after_death(&mut self, build: &DurableBuildState) {
        if self.training.is_some() {
            self.training = Some(LiveManaTraining::from_owned_build(build));
        }
        self.facts.magic_level = u32::from(build.magic().0);
        if self.owned_skills.is_some() {
            self.owned_skills = Some(build.skills().map(|(level, _)| level));
        }
        self.training_checkpoint = None;
        self.training_payment = None;
    }
}
impl super::cast::PlayerSpellState {
    /// Whether a forced checkpoint now has anything to write: a retained checkpoint to replay or
    /// unbanked training mana or skill tries. A read-only in-memory check (OPS-SAVE-2 shutdown
    /// pre-filter).
    pub fn has_training_to_flush(&self) -> bool {
        self.training_checkpoint.is_some()
            || self.training.as_ref().is_some_and(|value| {
                value.pending_mana > 0 || value.pending_skills.iter().any(|tries| *tries > 0)
            })
    }
    /// W2b: one resolved hit or use trains `family` by the tries `rule` derives from the hit
    /// streaks. Without owned training (no active formula, the production off switch) nothing
    /// trains. While a checkpoint is retained for replay (a save returned `Unknown`) the live
    /// training must stay its exact `before`, so the hits bank on the retained checkpoint and
    /// merge into the training with its receipt.
    ///
    /// SKILLS-0 §3.4: a level advance commits at once. When the banked tries reach a level the
    /// result says `checkpoint_due`, and the caller saves the training checkpoint in the same
    /// owner turn (`save_spell_training`), not at the cadence tick. The level is live only after
    /// that receipt (A13 §2).
    pub fn train_skill(
        &mut self,
        family: SkillFamily,
        now: u64,
        formula: &CompiledTrainingFormula,
        rule: impl FnOnce(&mut HitTrainingCounters) -> u64,
    ) -> Result<TrainedSkill, TrainingError> {
        if let Some(retained) = self.training_checkpoint.as_mut() {
            let prepared = &mut retained.prepared;
            let base = prepared.after_with_post_checkpoint();
            let mut next = base.clone();
            let tries = rule(next.hits_mut());
            next.record_skill_tries(family, tries, now)?;
            let checkpoint_due = next.skill_level_due(formula, family)?;
            let mut post = prepared.post_checkpoint.unwrap_or(PostCheckpointTries {
                pending_skills: [0; 7],
                hits: base.hits,
                started_micros: now,
                last_micros: now,
            });
            let pending = &mut post.pending_skills[family.index()];
            *pending = pending.saturating_add(tries).min(MAX_BUILD_PROGRESS);
            post.hits = next.hits;
            post.last_micros = post.last_micros.max(now);
            prepared.post_checkpoint = Some(post);
            return Ok(TrainedSkill {
                tries,
                checkpoint_due,
            });
        }
        let Some(current) = self.training.as_ref() else {
            return Ok(TrainedSkill::default());
        };
        let mut next = current.clone();
        let tries = rule(next.hits_mut());
        next.record_skill_tries(family, tries, now)?;
        let checkpoint_due = next.skill_level_due(formula, family)?;
        self.training = Some(next);
        Ok(TrainedSkill {
            tries,
            checkpoint_due,
        })
    }
    /// After a replayed receipt merged the hits resolved meanwhile: whether the save must write
    /// them before it reports `Saved`. A level they reach is due at once (SKILLS-0 §3.4); a
    /// forced save (logout, shutdown) also writes tries that reach no level.
    pub fn training_successor_due(&self, formula: &CompiledTrainingFormula, force: bool) -> bool {
        self.training_checkpoint.is_none()
            && if force {
                self.has_training_to_flush()
            } else {
                self.training
                    .as_ref()
                    .is_some_and(|value| value.level_due(formula))
            }
    }
    pub fn pending_training_checkpoint(&self) -> Option<&RetainedTrainingCheckpoint> {
        self.training_checkpoint.as_deref()
    }
    pub fn retain_training_checkpoint(
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
    pub fn rebind_staged_training(
        &mut self,
        before: &Self,
        anchor: &super::combat_batch::SpellAnchor,
        qualified_paid: Self,
    ) -> Result<(), TrainingError> {
        self.check_rebind_staged_training(before, anchor, &qualified_paid)?;
        self.install_rebind_staged_training(qualified_paid);
        Ok(())
    }

    /// The borrowing check of [`Self::rebind_staged_training`] (ARCH-SPELL-LOCK-2 §1.6).
    pub fn check_rebind_staged_training(
        &self,
        before: &Self,
        anchor: &super::combat_batch::SpellAnchor,
        qualified_paid: &Self,
    ) -> Result<(), TrainingError> {
        if !qualified_paid.paid_successor_of(before, anchor)
            || self.training != before.training
            || self.facts != before.facts
            || self.revision != anchor.next_revision
            || qualified_paid.training_payment.is_none()
        {
            return Err(TrainingError::InvalidBuild);
        }
        Ok(())
    }

    /// The infallible move, only after [`Self::check_rebind_staged_training`] passed.
    pub fn install_rebind_staged_training(&mut self, mut qualified_paid: Self) {
        self.facts.magic_level = qualified_paid.facts.magic_level;
        self.training = qualified_paid.training.take();
        self.training_payment = qualified_paid.training_payment.take();
    }
}

impl super::cast::PlayerSpellState {
    /// A definitely rolled-back attempt may refresh only the independently read
    /// root revision. The original occurrence and training successor are retained.
    pub fn refresh_training_checkpoint_revision(
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

    /// #2193 Codex P1 4307264455: the store refused this exact retained checkpoint's fence
    /// before any write (`AuthorityRejected`, no receipt for its occurrence), so the session's
    /// connection, lease or scope generation moved after it was prepared. The checkpoint is
    /// re-fenced to the independently resolved current authority of the same Character and
    /// session; its occurrence, request and formula stay, so a later replay can never commit a
    /// second build. A fence that did not move, another Character or session, a lower revision
    /// or another retained checkpoint is refused and changes nothing.
    pub fn refence_uncommitted_training_checkpoint(
        &mut self,
        expected: &RetainedTrainingCheckpoint,
        current: crate::durability::character_progression::CurrentCharacterGameplayFence,
    ) -> Result<(), TrainingError> {
        let retained = self
            .training_checkpoint
            .as_mut()
            .ok_or(TrainingError::InvalidBuild)?;
        if retained.as_ref() != expected
            || current == expected.fence
            || current.character_id != expected.fence.character_id
            || current.game_session_id != expected.fence.game_session_id
            || current.expected_character_revision.get()
                < expected.fence.expected_character_revision.get()
        {
            return Err(TrainingError::InvalidBuild);
        }
        retained.fence = current;
        Ok(())
    }
}
