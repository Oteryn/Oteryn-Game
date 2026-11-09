//! Completion is evaluated after independent claims, under the quest writer's locks.
//!
//! Bear Room's three reward markers are captured in
//! tools/content-schema/quest-authoring/samples/chests/claims.json (Canary
//! 04b83b51 + Crystal 9f5a72c6). The separate key-4601 claim opens access;
//! it is not a fourth treasure chest. This source slice does not certify the
//! chosen recipe's full travel or switch sequence.
//!
//! Behemoth's four required treasure claims use kv/behemothquest1..4 from
//! the same pinned capture, at (33294..33298,31658,13). The first is a
//! container reward; its existing native contents and grant owner are retained.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompletionSet {
    /// Every group is required; one track in each group suffices.
    pub groups: Vec<Vec<String>>,
    /// These members can advance only from their own durable once-claim MINT.
    #[serde(default, skip_serializing_if = "is_false")]
    pub mint_only: bool,
}

impl QuestStateCatalogue {
    pub fn completes_after_claim_sets(
        &self,
        transition: &QuestTransition,
        stored: &BTreeMap<String, i64>,
        changes: &[QuestTrackChange],
    ) -> bool {
        self.completion_sets.get(&transition.quest).map_or(
            self.completes_after(transition, changes),
            |rule| {
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
            },
        )
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn parse(text: &str) -> Result<BTreeMap<String, CompletionSet>, String> {
    let rules: BTreeMap<String, CompletionSet> =
        serde_json::from_str(text).map_err(|e| e.to_string())?;
    if rules.len() > super::QUESTSTATE0_RL_05 {
        return Err("completion owner bounds".into());
    }
    let mut seen = BTreeSet::new();
    let mut claims = BTreeSet::new();
    for (quest, rule) in &rules {
        if !super::valid_quest_key(quest) || rule.groups.is_empty() || rule.groups.len() > 32 {
            return Err("completion set bounds".into());
        }
        for group in &rule.groups {
            if group.is_empty() || group.len() > 32 {
                return Err("completion group bounds".into());
            }
            for marker in group {
                if !super::valid_quest_key(&format!("oteryn:quest-progress/{marker}/claimed"))
                    || !seen.insert(marker.clone())
                    || !claims.insert(marker.replace('/', "."))
                {
                    return Err("invalid or duplicate completion member".into());
                }
            }
        }
    }
    Ok(rules)
}
fn declarations() -> Result<&'static BTreeMap<String, CompletionSet>, String> {
    static RULES: std::sync::OnceLock<Result<BTreeMap<String, CompletionSet>, String>> =
        std::sync::OnceLock::new();
    RULES
        .get_or_init(|| {
            parse(include_str!(
                "../../../../../content/quests/claim-sets.json"
            ))
        })
        .as_ref()
        .map_err(Clone::clone)
}
fn member_transition(
    quest: &str,
    marker: &str,
    rule: &CompletionSet,
) -> Result<QuestTransition, String> {
    let effect = if rule.mint_only {
        QuestEffectKind::SetWithBinding {
            value: 1,
            binding: Sha256::new()
                .chain_update(b"oteryn:mint-completion-member/v1")
                .chain_update(
                    serde_json::to_vec(&(quest, marker, rule)).map_err(|e| e.to_string())?,
                )
                .finalize()
                .into(),
        }
    } else {
        QuestEffectKind::Set(1)
    };
    Ok(QuestTransition {
        key: format!("oteryn:quest-transition/{marker}/chest"),
        quest: quest.into(),
        effects: vec![QuestEffect {
            track: format!("oteryn:quest-progress/{marker}/claimed"),
            from: QuestComparison::Eq(0),
            effect,
        }],
        completes: false,
        experience: None,
        world_change: None,
    })
}
struct MintMember {
    claim: String,
    transition: QuestTransition,
}
fn mint_members() -> Result<&'static [MintMember], String> {
    static MEMBERS: std::sync::OnceLock<Result<Vec<MintMember>, String>> =
        std::sync::OnceLock::new();
    MEMBERS
        .get_or_init(|| {
            let mut members = Vec::new();
            for (quest, rule) in declarations()? {
                if rule.mint_only {
                    for marker in rule.groups.iter().flatten() {
                        members.push(MintMember {
                            claim: format!("oteryn:reward-claim.{}", marker.replace('/', ".")),
                            transition: member_transition(quest, marker, rule)?,
                        });
                    }
                }
            }
            Ok(members)
        })
        .as_ref()
        .map(Vec::as_slice)
        .map_err(Clone::clone)
}
pub fn mint_transitions() -> Result<Vec<QuestTransition>, String> {
    Ok(mint_members()?
        .iter()
        .map(|m| m.transition.clone())
        .collect())
}
/// Only the shared canonical claim binder selects this immutable obligation.
pub fn mint_transition_for_claim(claim: &str) -> Result<Option<&'static str>, String> {
    Ok(mint_members()?
        .iter()
        .find(|m| m.claim == claim)
        .map(|m| m.transition.key.as_str()))
}
/// The durable writer requires the matching MINT receipt, including on cold recovery.
pub fn required_mint_claim(transition: &str) -> Result<Option<&'static str>, String> {
    Ok(mint_members()?
        .iter()
        .find(|m| m.transition.key == transition)
        .map(|m| m.claim.as_str()))
}

pub fn extend(base: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let declarations = declarations()?;
    let mut tracks: Vec<_> = base.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = base.transitions.values().cloned().collect();
    let mut rules = BTreeMap::new();
    for (quest, declaration) in declarations {
        if declaration.groups.is_empty() || declaration.groups.len() > 32 {
            return Err("completion set bounds".into());
        }
        let mut seen = BTreeSet::new();
        let mut groups = Vec::new();
        for group in &declaration.groups {
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
                transitions.push(member_transition(quest, marker, declaration)?);
                keys.push(key);
            }
            groups.push(keys);
        }
        rules.insert(
            quest.clone(),
            CompletionSet {
                groups,
                mint_only: declaration.mint_only,
            },
        );
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
                mint_only: false,
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

#[cfg(test)]
mod mint_authority_tests {
    use super::*;
    #[test]
    fn engclaim_m35_completion_document_refuses_missing_duplicate_and_unknown_members() {
        for text in [
            r#"{"oteryn:quest.test":{"groups":[],"mint_only":true}}"#,
            r#"{"oteryn:quest.test":{"groups":[[]],"mint_only":true}}"#,
            r#"{"oteryn:quest.test":{"groups":[["kv/a"],["kv/a"]],"mint_only":true}}"#,
            r#"{"oteryn:quest.test":{"groups":[["kv/a.b"],["kv/a/b"]],"mint_only":true}}"#,
            r#"{"oteryn:quest.test":{"groups":[["kv/a"]],"invented_grant":true}}"#,
            r#"{"oteryn:quest.test":{"groups":[["bad marker"]],"mint_only":true}}"#,
        ] {
            assert!(parse(text).is_err(), "{text}");
        }
    }
    #[test]
    fn engclaim_m35_required_members_are_bound_without_changing_legacy_encoding()
    -> Result<(), String> {
        let mut rule = CompletionSet {
            groups: vec![vec!["kv/a".into()], vec!["kv/b".into()]],
            mint_only: false,
        };
        assert_eq!(
            serde_json::to_string(&rule).map_err(|e| e.to_string())?,
            r#"{"groups":[["kv/a"],["kv/b"]]}"#
        );
        rule.mint_only = true;
        let original = member_transition("oteryn:quest.test", "kv/a", &rule)?;
        rule.groups[1].push("kv/c".into());
        assert_ne!(
            member_transition("oteryn:quest.test", "kv/a", &rule)?,
            original
        );
        Ok(())
    }
}
