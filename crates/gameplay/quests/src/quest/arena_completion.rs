//! Canonical completion consumes the native arena's durable circuits and trophy claims.
use super::*;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub const QUEST: &str = "oteryn:quest.barbarian_arena_quest";
pub const COMPLETE: &str = "oteryn:quest-transition/chosen-source/barbarian_arena_quest/completion";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Stage {
    pub track: String,
    pub goal: i64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    schema: u8,
    quest: String,
    transition: String,
    pub stages: Vec<Stage>,
    pub completed_rank: u8,
    pub trophy_claims: Vec<String>,
    pub requires_premium: bool,
    pub prerequisite: String,
    recipe_sha256: String,
    #[serde(skip_serializing)]
    sources: Vec<String>,
}

pub fn rule() -> Result<&'static Rule, String> {
    static DATA: OnceLock<Result<Rule, String>> = OnceLock::new();
    DATA.get_or_init(|| {
        let rule: Rule = serde_json::from_str(include_str!(
            "../../../../../content/quests/arena-completion.json"
        ))
        .map_err(|error| error.to_string())?;
        if rule.schema != 1
            || rule.quest != QUEST
            || rule.transition != COMPLETE
            || rule.completed_rank != 3
            || !rule.requires_premium
            || rule.prerequisite != "oteryn:quest.barbarian_test"
            || rule.stages.len() != 5
            || rule.trophy_claims.len() != 3
            || rule.recipe_sha256.len() != 64
            || !rule
                .recipe_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || rule.sources.is_empty()
        {
            return Err("arena completion binding identity/bounds".into());
        }
        for (index, (stage, goal)) in rule.stages.iter().zip([1, 10, 10, 10, 3]).enumerate() {
            if stage.track
                != format!(
                    "oteryn:quest-progress/chosen-source/barbarian_arena_quest/s{}",
                    index + 1
                )
                || stage.goal != goal
            {
                return Err("arena completion stage binding".into());
            }
        }
        for (index, claim) in rule.trophy_claims.iter().enumerate() {
            if claim != &format!("oteryn:reward.svargrond-arena.{}.4", index + 1) {
                return Err("arena trophy claim binding".into());
            }
        }
        Ok(rule)
    })
    .as_ref()
    .map_err(Clone::clone)
}

pub fn transition() -> Result<QuestTransition, String> {
    Ok(QuestTransition {
        key: COMPLETE.into(),
        quest: QUEST.into(),
        completes: true,
        world_change: None,
        experience: None,
        effects: rule()?
            .stages
            .iter()
            .map(|stage| QuestEffect {
                track: stage.track.clone(),
                from: QuestComparison::Eq(0),
                effect: QuestEffectKind::Set(stage.goal),
            })
            .collect(),
    })
}

pub(super) fn extend(mut base: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let operation = transition()?;
    let tracks: Vec<_> = rule()?
        .stages
        .iter()
        .map(|stage| QuestTrack {
            key: stage.track.clone(),
            quest: QUEST.into(),
            initial: 0,
            min: 0,
            max: stage.goal,
        })
        .collect();
    let checked = QuestStateCatalogue::new(
        base.content_revision.clone(),
        base.tracks
            .values()
            .cloned()
            .chain(tracks.iter().cloned())
            .collect(),
        base.transitions
            .values()
            .cloned()
            .chain([operation.clone()])
            .collect(),
    )
    .map_err(|error| format!("{error:?}"))?;
    let hash = checked
        .definition_hash(QUEST)
        .ok_or("arena completion owner")?;
    let hash = Sha256::new()
        .chain_update(hash)
        .chain_update(serde_json::to_vec(rule()?).map_err(|error| error.to_string())?)
        .finalize()
        .into();
    for track in tracks {
        base.tracks.insert(track.key.clone(), track);
    }
    base.transitions.insert(operation.key.clone(), operation);
    base.hashes.insert(QUEST.into(), hash);
    Ok(base)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arena_completion_writes_all_five_canonical_goals() -> Result<(), String> {
        let lowered = super::super::loader::load_embedded_quest_state("arena-test")
            .map_err(|e| format!("{e:?}"))?;
        let catalogue = lowered.catalogue();
        let operation = catalogue
            .transition(COMPLETE)
            .ok_or("native arena binding")?;
        assert_eq!(operation, &transition()?);
        let changes = catalogue
            .evaluate(operation, &BTreeMap::new(), 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            changes
                .iter()
                .map(|change| change.after)
                .collect::<Vec<_>>(),
            [1, 10, 10, 10, 3]
        );
        assert!(catalogue.completes_after(operation, &changes));
        Ok(())
    }
    #[test]
    fn arena_completion_rejects_renamed_or_stripped_completion_aliases() -> Result<(), String> {
        let operation = transition()?;
        super::super::admission_binding::validate(&operation, false)?;
        let mut alias = operation.clone();
        alias.key = "oteryn:quest-transition/arena-alias".into();
        alias.effects.clear();
        assert!(super::super::admission_binding::validate(&alias, false).is_err());
        Ok(())
    }
}
