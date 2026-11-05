//! Typed source-program context for the existing fenced QuestState transaction.
//! Source selection is repeated over locked tracks and database time; a planned
//! reply is never evidence that any write committed.
use super::quest_state::QuestTransitionRequest;
use super::{DurabilityError, character_progression::CharacterProgressionError};
use oteryn_gameplay_quests::quest::npc_rules::{
    NpcInventorySnapshot, NpcRuleCatalog, NpcRuleInput, NpcRulePlan, NpcRuleTrigger,
    program_transition_key,
};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrackChange};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcProgramContext {
    pub quest: String,
    pub program_digest: [u8; 32],
    pub npc: String,
    pub message: String,
    pub topic: i32,
    pub trigger: NpcRuleTrigger,
    pub selected_rule_keys: Vec<String>,
}

pub(super) fn catalogue_source() -> Result<&'static NpcRuleCatalog, CharacterProgressionError> {
    static CATALOGUE: OnceLock<Result<NpcRuleCatalog, String>> = OnceLock::new();
    CATALOGUE
        .get_or_init(|| NpcRuleCatalog::load_embedded().map_err(|e| e.to_string()))
        .as_ref()
        .map_err(|_| CharacterProgressionError::InvalidInput)
}

impl NpcProgramContext {
    pub(super) fn bound_request(
        &self,
        binding: &mut [u8; 33],
    ) -> Result<(), CharacterProgressionError> {
        let bytes =
            serde_json::to_vec(self).map_err(|_| CharacterProgressionError::InvalidInput)?;
        let digest = Sha256::new()
            .chain_update(*binding)
            .chain_update(b"npc-program/v1")
            .chain_update(bytes)
            .finalize();
        binding[1..].copy_from_slice(&digest);
        Ok(())
    }

    pub(super) fn tracks(&self) -> Result<Vec<String>, CharacterProgressionError> {
        let program = catalogue_source()?
            .program(&self.quest)
            .ok_or(CharacterProgressionError::InvalidInput)?;
        let mut keys: std::collections::BTreeSet<String> = program
            .document
            .tracks
            .iter()
            .map(|t| t.key.clone())
            .collect();
        for rule in &program.document.rules {
            for guard in &rule.guards {
                if let oteryn_gameplay_quests::quest::npc_rules::NpcRuleGuard::Storage {
                    track,
                    ..
                }
                | oteryn_gameplay_quests::quest::npc_rules::NpcRuleGuard::StorageTime {
                    track,
                    ..
                } = guard
                {
                    keys.insert(track.clone());
                }
            }
        }
        Ok(keys.into_iter().collect())
    }

    pub(super) fn validate(
        &self,
        catalogue: &QuestStateCatalogue,
        request: &QuestTransitionRequest,
    ) -> Result<(), CharacterProgressionError> {
        let program = catalogue_source()?
            .program(&self.quest)
            .ok_or(CharacterProgressionError::InvalidInput)?;
        if program.digest != self.program_digest
            || !oteryn_gameplay_quests::quest::in_runtime_scope(&self.quest)
            || self.selected_rule_keys.is_empty()
            || self.message.len() > 4096
            || self.topic < 0
            || !matches!(self.trigger, NpcRuleTrigger::Say | NpcRuleTrigger::Greet)
            || !matches!(request.cause, super::quest_state::QuestCause::Command(_))
            || request.transition_key != program_transition_key(self.program_digest)
            || catalogue
                .transition(&request.transition_key)
                .is_none_or(|transition| transition.quest != self.quest)
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        Ok(())
    }

    pub(super) fn plan(
        &self,
        request: &QuestTransitionRequest,
        tracks: &BTreeMap<String, i64>,
        now: i64,
    ) -> Result<Option<NpcRulePlan>, DurabilityError> {
        let inventory = NpcInventorySnapshot::default();
        let outfits = BTreeMap::new();
        let catalogue = catalogue_source().map_err(|_| DurabilityError::InvalidStoredState)?;
        let input = NpcRuleInput {
            npc: &self.npc,
            trigger: self.trigger,
            message: &self.message,
            topic: self.topic,
            tracks,
            inventory: &inventory,
            now_epoch_seconds: now,
            calendar_month: 0,
            level: 0,
            vocation: "",
            sex: 0,
            outfits: &outfits,
        };
        let planned = match catalogue.plan_storage(&input) {
            Ok(Some(plan)) => plan,
            // A whole domain refusal writes neither earlier continuing effects
            // nor success text. Domain absence is a stage mismatch for callers.
            Ok(None) | Err(_) => return Ok(None),
        };
        if planned.quest != self.quest
            || planned.program_digest != self.program_digest
            || planned.selected_rule_keys != self.selected_rule_keys
            || request.transition_key != program_transition_key(self.program_digest)
        {
            return Ok(None);
        }
        Ok(Some(planned))
    }
}

pub(super) fn changes(plan: &NpcRulePlan) -> Vec<QuestTrackChange> {
    plan.track_changes
        .iter()
        .map(|change| QuestTrackChange {
            track: change.key.clone(),
            before: change.before,
            after: change.after,
        })
        .collect()
}
