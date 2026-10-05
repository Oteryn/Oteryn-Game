//! Herald death -> existing serialized Character QuestState writer.
//! PROJECT selection: only the physical owner's top-damage Character. Party fan-out,
//! permanent portal publication and speech delivery are separate owning consumers.
use crate::durability::{
    DurabilityRoot,
    character_authority::ReconciledCharacterAuthority,
    character_progression::{
        CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceRewardOccurrence,
    },
    character_revision_sequencer::CharacterRevisionSequencer,
    quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
        quest::{QuestComparison, QuestEffectKind, QuestStateCatalogue},
    },
    runtime_scope_assignment::NodeIncarnationProof,
};
use crate::foundation::{RuntimeWorkStamp, ScopeRuntimeFence};
use std::sync::Arc;

pub(crate) const MISSION: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_24/targuna/burning_heart/mission";
pub(crate) const KILLED: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_24/targuna/burning_heart/herald_killed";
pub(crate) const QUEST: &str = "oteryn:quest.targuna_quest";
pub(crate) const TRANSITIONS: [&str; 3] = [
    "oteryn:quest-transition/crystalserver/targuna/herald-death/mission-1",
    "oteryn:quest-transition/crystalserver/targuna/herald-death/mission-2",
    "oteryn:quest-transition/crystalserver/targuna/herald-death/progressed",
];

#[derive(Debug)]
pub(crate) enum HeraldQuestError {
    InvalidCatalogue,
    FenceChanged,
    SourceChanged,
    Writer(CharacterProgressionError),
}

// Pure source predicate; the SQL writer re-evaluates BOTH tracks under its root lock.
// Different branch snapshots can refuse, but cannot lower a concurrently advanced mission.
pub(crate) fn select_transition(
    catalogue: &QuestStateCatalogue,
    stored: &std::collections::BTreeMap<String, i64>,
) -> Result<Option<&'static str>, HeraldQuestError> {
    for (key, max) in [(MISSION, 5), (KILLED, 1)] {
        let track = catalogue
            .track(key)
            .ok_or(HeraldQuestError::InvalidCatalogue)?;
        if track.quest != QUEST || track.initial != 0 || track.min != 0 || track.max != max {
            return Err(HeraldQuestError::InvalidCatalogue);
        }
    }
    for (i, key) in TRANSITIONS.iter().enumerate() {
        let transition = catalogue
            .transition(key)
            .ok_or(HeraldQuestError::InvalidCatalogue)?;
        if transition.quest != QUEST
            || transition.completes
            || transition.experience.is_some()
            || transition.effects.len() != 2
        {
            return Err(HeraldQuestError::InvalidCatalogue);
        }
        let mission = &transition.effects[0];
        let killed = &transition.effects[1];
        let from = if i < 2 {
            QuestComparison::Eq(i as i64 + 1)
        } else {
            QuestComparison::Ge(3)
        };
        let effect = if i < 2 {
            QuestEffectKind::Set(3)
        } else {
            QuestEffectKind::Add(0)
        };
        if mission.track != MISSION
            || mission.from != from
            || mission.effect != effect
            || killed.track != KILLED
            || killed.from != QuestComparison::Ne(1)
            || killed.effect != QuestEffectKind::Set(1)
        {
            return Err(HeraldQuestError::InvalidCatalogue);
        }
    }
    if stored.get(KILLED).copied().unwrap_or(0) == 1 {
        return Ok(None);
    }
    Ok(match stored.get(MISSION).copied().unwrap_or(0) {
        1 => Some(TRANSITIONS[0]),
        2 => Some(TRANSITIONS[1]),
        3..=5 => Some(TRANSITIONS[2]),
        _ => None,
    })
}

/// Frozen death occurrence, source-selected recipient and branch. No public constructor.
/// Prepare under the current physical owner; drop the runtime lock BEFORE acquiring the
/// Character revision slot or awaiting SQL. Retry THIS request, never reselect its branch.
#[derive(Debug)]
pub(crate) struct HeraldQuestRequest {
    fence: CurrentCharacterGameplayFence,
    request: QuestTransitionRequest,
    catalogue: Arc<QuestStateCatalogue>,
    source_stamp: RuntimeWorkStamp,
}
impl HeraldQuestRequest {
    // Only the qualified Crystal physical-death owner calls this internal constructor.
    pub(super) fn from_owner(
        fence: CurrentCharacterGameplayFence,
        occurrence: ExperienceRewardOccurrence,
        transition: &'static str,
        catalogue: Arc<QuestStateCatalogue>,
        source_stamp: RuntimeWorkStamp,
    ) -> Self {
        Self {
            fence,
            request: QuestTransitionRequest {
                transition_key: transition.to_owned(),
                cause: QuestCause::CreatureDeath(occurrence),
            },
            catalogue,
            source_stamp,
        }
    }
    pub(crate) const fn selection_policy(&self) -> &'static str {
        "PROJECT_TOP_DAMAGE_CHARACTER_ONLY_NOT_GLOBAL_PARTY_PARITY"
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "Owner ABI keeps independently qualified source, current fence, exact actor and commit facts explicit"
    )]
    pub(crate) async fn commit(
        &self,
        sequencer: &CharacterRevisionSequencer,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        current: CurrentCharacterGameplayFence,
        source_fence: &ScopeRuntimeFence,
        runtime: &tokio::sync::Mutex<crate::foundation::ChannelRuntimeV1>,
        qualified: &crate::crystal_death_composition::QualifiedCrystalDeaths,
    ) -> Result<QuestTransitionOutcome, HeraldQuestError> {
        if self.fence.character_id != current.character_id
            || self.fence.game_session_id != current.game_session_id
            || self.fence.connection_generation != current.connection_generation
            || self.fence.character_lease_generation != current.character_lease_generation
            || self.fence.runtime_scope != current.runtime_scope
            || self.fence.scope_ownership_generation != current.scope_ownership_generation
        {
            return Err(HeraldQuestError::FenceChanged);
        }
        let mut slot = sequencer.acquire(current.character_id).await;
        if !source_fence
            .is_current_for_scope(current.runtime_scope, current.scope_ownership_generation)
            || !source_fence.accepts_stamp(self.source_stamp)
        {
            return Err(HeraldQuestError::SourceChanged);
        }
        {
            // Revalidate the actual full current native pin AFTER both awaits. Scope work
            // stamps alone do not fence a same-scope content activation/revision change.
            let runtime = runtime.lock().await;
            qualified
                .validate_current_work(&runtime, source_fence, self.source_stamp)
                .map_err(|_| HeraldQuestError::SourceChanged)?;
        } // No runtime mutex is held across the existing durable SQL transaction.
        slot.commit_quest_transition(
            root,
            authority,
            node,
            current,
            self.request.clone(),
            Arc::clone(&self.catalogue),
        )
        .await
        .map_err(HeraldQuestError::Writer)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::durability::quest_state::quest::loader::load_embedded_quest_state;
    #[test]
    fn actual_grouped_herald_transition_guards_both_tracks_and_preserves_later_mission() {
        let lowered =
            load_embedded_quest_state("content-1").expect("actual embedded quest catalogue");
        let c = lowered.catalogue();
        for mission in 0..=5 {
            let stored = [(MISSION.to_owned(), mission), (KILLED.to_owned(), 0)]
                .into_iter()
                .collect();
            let key = select_transition(c, &stored).expect("exact grouped source definition");
            if mission == 0 {
                assert!(key.is_none());
                continue;
            }
            let key = key.expect("eligible");
            let transition = c.transition(key).expect("member");
            let changes = c
                .evaluate(transition, &stored, 1)
                .expect("both source guards hold");
            assert_eq!(changes.len(), 2);
            assert_eq!(changes[0].after, mission.max(3));
            assert_eq!(changes[1].after, 1);
            let done = [
                (MISSION.to_owned(), changes[0].after),
                (KILLED.to_owned(), 1),
            ]
            .into_iter()
            .collect();
            assert!(c.evaluate(transition, &done, 1).is_err());
            assert!(select_transition(c, &done).expect("definition").is_none());
        }
        let progressed = [(MISSION.to_owned(), 4), (KILLED.to_owned(), 0)]
            .into_iter()
            .collect();
        assert!(
            c.evaluate(
                c.transition(TRANSITIONS[0]).expect("branch"),
                &progressed,
                1
            )
            .is_err()
        );
        let empty = QuestStateCatalogue::empty("content-1").expect("empty catalogue");
        assert!(matches!(
            select_transition(&empty, &progressed),
            Err(HeraldQuestError::InvalidCatalogue)
        ));
    }
}
