//! Completion is evaluated after independent claims, under the quest writer's locks.
//!
//! Bear Room's three reward markers are captured in
//! tools/content-schema/quest-authoring/samples/chests/claims.json (Canary
//! 04b83b51 + Crystal 9f5a72c6). The separate key-4601 claim opens access;
//! it is not a fourth treasure chest. This source slice does not certify the
//! chosen recipe's full travel or switch sequence.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompletionSet {
    /// Every group is required; one track in each group suffices.
    pub groups: Vec<Vec<String>>,
}

impl QuestStateCatalogue {
    pub fn completes_after_claim_sets(
        &self,
        transition: &QuestTransition,
        stored: &BTreeMap<String, i64>,
        changes: &[QuestTrackChange],
    ) -> bool {
        self.completion_sets
            .get(&transition.quest)
            .map_or(self.completes_after(transition, changes), |rule| {
                rule.groups.iter().all(|group| {
                    group.iter().any(|key| {
                        changes
                            .iter()
                            .find(|change| &change.track == key)
                            .map(|change| change.after)
                            .or_else(|| stored.get(key).copied())
                            .unwrap_or(0)
                            >= 1
                    })
                })
            })
    }
}

pub fn extend(base: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let declarations: BTreeMap<String, CompletionSet> = serde_json::from_str(include_str!(
        "../../../../../content/quests/claim-sets.json"
    ))
    .map_err(|e| e.to_string())?;
    let mut tracks: Vec<_> = base.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = base.transitions.values().cloned().collect();
    let mut rules = BTreeMap::new();
    for (quest, declaration) in declarations {
        if declaration.groups.is_empty() || declaration.groups.len() > 32 {
            return Err("completion set bounds".into());
        }
        let mut seen = BTreeSet::new();
        let mut groups = Vec::new();
        for group in declaration.groups {
            if group.is_empty() || group.len() > 32 {
                return Err("completion group bounds".into());
            }
            let mut keys = Vec::new();
            for marker in group {
                if !seen.insert(marker.clone()) {
                    return Err("duplicate completion claim".into());
                }
                let key = format!("oteryn:quest-progress/{marker}/claimed");
                tracks.push(QuestTrack {
                    key: key.clone(),
                    quest: quest.clone(),
                    initial: 0,
                    min: 0,
                    max: 1,
                });
                transitions.push(QuestTransition {
                    key: format!("oteryn:quest-transition/{marker}/chest"),
                    quest: quest.clone(),
                    effects: vec![QuestEffect {
                        track: key.clone(),
                        from: QuestComparison::Eq(0),
                        effect: QuestEffectKind::Set(1),
                    }],
                    completes: false,
                    experience: None,
                    world_change: None,
                });
                keys.push(key);
            }
            groups.push(keys);
        }
        rules.insert(quest, CompletionSet { groups });
    }
    let mut catalogue =
        QuestStateCatalogue::new(base.content_revision.clone(), tracks, transitions)
            .map_err(|e| format!("{e:?}"))?;
    for (quest, hash) in &base.hashes {
        if !rules.contains_key(quest) {
            catalogue.hashes.insert(quest.clone(), *hash);
        }
    }
    for (quest, rule) in &rules {
        let hash = catalogue.hashes.get_mut(quest).ok_or("completion owner")?;
        *hash = Sha256::new()
            .chain_update(*hash)
            .chain_update(serde_json::to_vec(rule).map_err(|e| e.to_string())?)
            .finalize()
            .into();
    }
    catalogue.npc_legacy = base.npc_legacy;
    catalogue.predecessors = base.predecessors;
    catalogue.authored_bindings = base.authored_bindings;
    catalogue.native_recipe_evidence = base.native_recipe_evidence;
    catalogue.completion_sets = rules;
    Ok(catalogue)
}

impl QuestStateCatalogue {
    pub fn completion_tracks(&self, quest: &str) -> impl Iterator<Item = &str> {
        self.completion_sets
            .get(quest)
            .into_iter()
            .flat_map(|rule| rule.groups.iter().flatten().map(String::as_str))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alternative_claim_group_accepts_either_member() -> Result<(), String> {
        let lowered = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        let mut catalogue = lowered.catalogue().clone();
        let quest = "oteryn:quest.bear_room_quest";
        let keys: Vec<_> = catalogue
            .completion_tracks(quest)
            .map(str::to_owned)
            .collect();
        catalogue.completion_sets.insert(
            quest.into(),
            CompletionSet {
                groups: vec![
                    vec![keys[0].clone(), keys[1].clone()],
                    vec![keys[2].clone()],
                ],
            },
        );
        let transition = catalogue
            .transition("oteryn:quest-transition/kv/bear_chest3/chest")
            .ok_or("claim")?;
        for alternative in &keys[..2] {
            let stored = BTreeMap::from([(alternative.clone(), 1)]);
            let changes = catalogue
                .evaluate(transition, &stored, 0)
                .map_err(|e| format!("{e:?}"))?;
            assert!(catalogue.completes_after_claim_sets(transition, &stored, &changes));
        }
        assert!(!catalogue.completes_after_claim_sets(transition, &BTreeMap::new(), &[]));
        Ok(())
    }

    #[test]
    fn unordered_claims_complete_only_after_all_groups() -> Result<(), String> {
        let catalogue = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        let catalogue = catalogue.catalogue();
        let mut stored = BTreeMap::new();
        for (index, marker) in ["bear_chest3", "bear_chest1", "bear_chest2"]
            .iter()
            .enumerate()
        {
            let key = format!("oteryn:quest-transition/kv/{marker}/chest");
            let transition = catalogue
                .transition(&key)
                .ok_or("missing Bear Room claim binding")?;
            let changes = catalogue
                .evaluate(transition, &stored, 0)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(
                catalogue.completes_after_claim_sets(transition, &stored, &changes),
                index == 2
            );
            for change in changes {
                stored.insert(change.track, change.after);
            }
        }
        Ok(())
    }
}
