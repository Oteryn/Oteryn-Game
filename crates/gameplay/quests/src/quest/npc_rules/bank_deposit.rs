//! Crystal 201189f8 data-global/npc/paulie.lua:160-169.
use super::{NpcRuleCatalog, NpcRuleError};
use crate::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    valid_quest_key,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BankDepositBonus {
    pub key: String,
    pub npc: String,
    pub track: String,
    pub before: i64,
    pub after: i64,
    pub gold: u64,
    pub reply: String,
    pub sources: Vec<String>,
}

impl BankDepositBonus {
    pub fn valid(&self) -> bool {
        self.key
            .starts_with("oteryn:quest-transition/bank-deposit/")
            && self.npc.starts_with("oteryn:npc.")
            && [&self.key, &self.npc, &self.track]
                .into_iter()
                .all(|key| valid_quest_key(key))
            && self.after != self.before
            && (1..=20_000_000).contains(&self.gold)
            && !self.reply.is_empty()
            && self.reply.len() <= 512
            && self.reply.contains("{amount}")
            && self.reply.contains("{bonus}")
    }

    // Only the bank owner computes this change after committing the conversion.
    pub fn transition(&self, quest: &str) -> QuestTransition {
        QuestTransition {
            key: self.key.clone(),
            quest: quest.into(),
            effects: vec![QuestEffect {
                track: self.track.clone(),
                from: QuestComparison::Eq(self.before),
                effect: QuestEffectKind::Computed,
            }],
            completes: false,
            experience: None,
            world_change: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundBankDepositBonus {
    pub quest: String,
    pub content_revision: String,
    pub definition_hashes: Vec<[u8; 32]>,
    pub rule: BankDepositBonus,
}

impl NpcRuleCatalog {
    /// Freeze only the pinned source binding behind this banker and its native quest.
    pub fn bank_deposit_bonus(
        npc: &str,
        native: &QuestStateCatalogue,
    ) -> Result<Option<BoundBankDepositBonus>, NpcRuleError> {
        let source = Self::for_catalogue(native)?;
        Ok(source.programs().iter().find_map(|program| {
            let quest = &program.document.quest;
            let rule = program
                .document
                .bank_deposit_bonus
                .as_ref()
                .filter(|rule| rule.npc == npc)?;
            let hash = native.definition_hash(quest)?;
            (native.transition(&rule.key)? == &rule.transition(quest)).then(|| {
                let mut definition_hashes = vec![hash];
                if let Some((previous, _)) = native.npc_legacy.get(quest) {
                    definition_hashes.push(*previous);
                }
                if let Some(previous) = native.predecessors.get(quest) {
                    definition_hashes.extend(previous.iter().map(|definition| definition.hash));
                }
                definition_hashes.sort_unstable();
                definition_hashes.dedup();
                BoundBankDepositBonus {
                    quest: quest.clone(),
                    content_revision: native.content_revision().into(),
                    definition_hashes,
                    rule: rule.clone(),
                }
            })
        }))
    }
}
