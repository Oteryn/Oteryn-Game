//! Registered source-expiry consequences; no client command substitutes for a timer.
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug)]
pub struct Recipe {
    pub key: &'static str,
    pub quest: &'static str,
    pub source_lease_recipe: &'static str,
    pub track: &'static str,
    pub maximum_before: i64,
    pub value_after: i64,
}
pub const RECIPES: &[Recipe] = &[
    Recipe {
        key: "oteryn:quest-timer/ancient-tombs-thalas-treasure",
        quest: "oteryn:quest.the_ancient_tombs_quest",
        source_lease_recipe: "oteryn:quest-callback/ancient-tombs-thalas-switch",
        track: "oteryn:quest-track/ancient-tombs/thalas-treasure",
        maximum_before: 0,
        value_after: 1,
    },
    Recipe {
        key: "oteryn:quest-timer/ancient-tombs-diprath-treasure",
        quest: "oteryn:quest.the_ancient_tombs_quest",
        source_lease_recipe: "oteryn:quest-callback/ancient-tombs-diprath-switch",
        track: "oteryn:quest-track/ancient-tombs/diprath-treasure",
        maximum_before: 1,
        value_after: 2,
    },
];
pub fn recipe(key: &str) -> Option<&'static Recipe> {
    RECIPES.iter().find(|r| r.key == key)
}
impl Recipe {
    pub fn binding(&self) -> [u8; 32] {
        Sha256::digest(
            format!(
                "oteryn:quest-timer/v1|{}|{}|{}|{}|-1|{}|{}",
                self.key,
                self.quest,
                self.source_lease_recipe,
                self.track,
                self.maximum_before,
                self.value_after
            )
            .as_bytes(),
        )
        .into()
    }
    pub fn transition(&self) -> QuestTransition {
        QuestTransition {
            world_change: None,
            key: self.key.into(),
            quest: self.quest.into(),
            effects: vec![QuestEffect {
                track: self.track.into(),
                from: QuestComparison::Le(self.maximum_before),
                effect: QuestEffectKind::SetWithBinding {
                    value: self.value_after,
                    binding: self.binding(),
                },
            }],
            completes: false,
            experience: None,
        }
    }
}
pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for recipe in RECIPES {
        if let Some(track) = tracks.iter().find(|t| t.key == recipe.track) {
            if track.quest != recipe.quest
                || track.initial != -1
                || track.min != -1
                || track.max < recipe.value_after
            {
                return Err("timer track owner/bounds mismatch".into());
            }
        } else {
            tracks.push(QuestTrack {
                key: recipe.track.into(),
                quest: recipe.quest.into(),
                initial: -1,
                min: -1,
                max: i64::MAX,
            });
        }
        transitions.push(recipe.transition());
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_expiry_binding_cannot_be_renamed_stripped_or_retargeted() -> Result<(), String> {
        for recipe in RECIPES {
            let operation = recipe.transition();
            crate::quest::admission_binding::validate(&operation, false)?;
            let mut renamed = operation.clone();
            renamed.key.push_str("-shadow");
            renamed.effects[0].effect = QuestEffectKind::Set(recipe.value_after);
            assert!(crate::quest::admission_binding::validate(&renamed, false).is_err());
            let mut changed = operation;
            changed.effects[0].from = QuestComparison::Any;
            assert!(crate::quest::admission_binding::validate(&changed, false).is_err());
        }
        Ok(())
    }
}
