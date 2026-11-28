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
    /// Optional final interaction and personal progress to settle alongside the claims.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    finish: Option<Finish>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Finish {
    transition: String,
    minimum_tracks: BTreeMap<String, i64>,
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
                let value = |key: &str| {
                    changes
                        .iter()
                        .find(|change| change.track == key)
                        .map(|change| change.after)
                        .or_else(|| stored.get(key).copied())
                        .unwrap_or(0)
                };
                rule.finish.as_ref().is_none_or(|finish| {
                    finish.transition == transition.key
                        && finish
                            .minimum_tracks
                            .iter()
                            .all(|(key, minimum)| value(key) >= *minimum)
                }) && rule
                    .groups
                    .iter()
                    .all(|group| group.iter().any(|key| value(key) >= 1))
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
        if rule.finish.as_ref().is_some_and(|finish| {
            !super::valid_quest_key(&finish.transition)
                || finish.minimum_tracks.is_empty()
                || finish.minimum_tracks.len() > 16
                || finish
                    .minimum_tracks
                    .iter()
                    .any(|(key, minimum)| !super::valid_quest_key(key) || *minimum <= 0)
        }) {
            return Err("completion finish bounds".into());
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
        if let Some(finish) = &declaration.finish {
            let has_binding = base.authored_bindings().is_some_and(|bindings| {
                bindings
                    .quests
                    .iter()
                    .any(|binding| binding.quest == *quest)
            });
            if base
                .transition(&finish.transition)
                .is_none_or(|transition| transition.quest != *quest)
                || finish.minimum_tracks.iter().any(|(key, minimum)| {
                    base.track(key).map_or_else(
                        || has_binding,
                        |track| track.quest != *quest || !(track.min..=track.max).contains(minimum),
                    )
                })
            {
                return Err(
                    "completion finish requires its exact quest tracks and transition".into(),
                );
            }
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
                finish: declaration.finish.clone(),
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
    retain_alawar_predecessor(&mut catalogue, &rules)?;
    retain_draconia_predecessor(&mut catalogue, &rules)?;
    catalogue.completion_sets = rules;
    Ok(catalogue)
}

// The two corrected door placements changed their transition binding digests.
// Reconstruct the saved completion-set hash using the old depth byte.
fn retain_draconia_predecessor(
    catalogue: &mut QuestStateCatalogue,
    rules: &BTreeMap<String, CompletionSet>,
) -> Result<(), String> {
    const QUEST: &str = "oteryn:quest.draconia_quest";
    let Some(binding) = catalogue.authored_bindings().and_then(|bindings| {
        bindings
            .quests
            .iter()
            .find(|binding| binding.quest == QUEST)
    }) else {
        return Ok(());
    };
    let mut transitions: BTreeMap<_, _> = catalogue
        .transitions
        .iter()
        .filter(|(_, transition)| transition.quest == QUEST)
        .map(|(key, transition)| (key.clone(), transition.clone()))
        .collect();
    for (x, y) in [(32675_u64, 31649_u64), (32676, 31671)] {
        let placement = (x << 32) | (y << 16) | (10 << 8) | 1;
        let Some((door, completes)) =
            binding
                .stages
                .iter()
                .find_map(|stage| match &stage.trigger {
                    authored_bindings::Trigger::DoorUse { usage }
                        if usage.native_placement_key == placement && usage.key_number == 3012 =>
                    {
                        Some((usage, stage.completes))
                    }
                    _ => None,
                })
        else {
            return Ok(());
        };
        let mut previous = door.clone();
        previous.native_placement_key = (x << 32) | (y << 16) | (246 << 8) | 1;
        transitions.insert(previous.key.clone(), previous.transition(QUEST, completes)?);
    }
    let rule = rules.get(QUEST).ok_or("Draconia completion rule")?;
    let hash = Sha256::new()
        .chain_update(super::definition_hash(
            QUEST,
            &catalogue.tracks,
            &transitions,
        ))
        .chain_update(serde_json::to_vec(rule).map_err(|error| error.to_string())?)
        .finalize()
        .into();
    catalogue
        .predecessors
        .entry(QUEST.into())
        .or_default()
        .push(NpcDefinitionPredecessor {
            hash,
            defaults: BTreeMap::new(),
        });
    Ok(())
}

// The original approach and the two independent chest transitions have the same
// track defaults. A saved partial quest may finish the new route with those claims.
fn retain_alawar_predecessor(
    catalogue: &mut QuestStateCatalogue,
    rules: &BTreeMap<String, CompletionSet>,
) -> Result<(), String> {
    const QUEST: &str = "oteryn:quest.alawar_s_vault_quest";
    let Some(rule) = rules.get(QUEST).filter(|rule| rule.finish.is_some()) else {
        return Ok(());
    };
    let mut transitions = BTreeMap::new();
    let mut tracks = BTreeMap::new();
    for key in [
        "oteryn:quest-callback/m01b/alawar-s-vault/s1/entry",
        "oteryn:quest-transition/kv/alawars_vault1/chest",
        "oteryn:quest-transition/kv/alawars_vault2/chest",
    ] {
        let transition = catalogue
            .transition(key)
            .filter(|t| t.quest == QUEST)
            .ok_or("Alawar predecessor transition")?;
        for effect in &transition.effects {
            let track = catalogue
                .track(&effect.track)
                .filter(|t| t.quest == QUEST)
                .ok_or("Alawar predecessor track")?;
            tracks.insert(track.key.clone(), track.clone());
        }
        transitions.insert(key.to_owned(), transition.clone());
    }
    let original_rule = CompletionSet {
        finish: None,
        ..rule.clone()
    };
    let hash = Sha256::new()
        .chain_update(super::definition_hash(QUEST, &tracks, &transitions))
        .chain_update(serde_json::to_vec(&original_rule).map_err(|error| error.to_string())?)
        .finalize()
        .into();
    catalogue
        .predecessors
        .entry(QUEST.into())
        .or_default()
        .push(super::NpcDefinitionPredecessor {
            hash,
            defaults: BTreeMap::new(),
        });
    Ok(())
}

pub(super) fn journey_predecessors(
    catalogue: &QuestStateCatalogue,
    bindings: &super::authored_bindings::Bindings,
) -> Result<BTreeMap<String, super::NpcDefinitionPredecessor>, String> {
    const KOSHEI: &str = "oteryn:quest.koshei_the_deathless_quest";
    const PARADOX: &str = "oteryn:quest.the_paradox_tower_quest";
    let mut predecessors = BTreeMap::new();
    for quest in [KOSHEI, PARADOX] {
        if !bindings.quests.iter().any(|binding| binding.quest == quest) {
            continue;
        }
        let mut tracks: BTreeMap<_, _> = catalogue
            .tracks
            .iter()
            .filter(|(_, track)| track.quest == quest)
            .map(|(key, track)| (key.clone(), track.clone()))
            .collect();
        let mut transitions: BTreeMap<_, _> = catalogue
            .transitions
            .iter()
            .filter(|(_, transition)| transition.quest == quest)
            .map(|(key, transition)| (key.clone(), transition.clone()))
            .collect();
        let hash = if quest == KOSHEI {
            // Before the tomb journey, the two independent chest tracks already
            // belonged to the published definition.
            let declaration = declarations()?.get(quest).ok_or("Koshei completion rule")?;
            let mut rule = declaration.clone();
            for marker in rule.groups.iter_mut().flatten() {
                let transition = member_transition(quest, marker, declaration)?;
                let key = transition.effects[0].track.clone();
                tracks.insert(
                    key.clone(),
                    QuestTrack {
                        key: key.clone(),
                        quest: quest.into(),
                        initial: 0,
                        min: 0,
                        max: 1,
                    },
                );
                transitions.insert(transition.key.clone(), transition);
                *marker = key;
            }
            Sha256::new()
                .chain_update(super::definition_hash(quest, &tracks, &transitions))
                .chain_update(serde_json::to_vec(&rule).map_err(|error| error.to_string())?)
                .finalize()
                .into()
        } else {
            // The published dialogue only wrote the story track. Reconstruct
            // the exact prior transition, including its admission binding.
            for mut callback in super::reward_callbacks::load()?
                .into_iter()
                .filter(|callback| {
                    callback.quest == quest
                        && callback.key.starts_with("oteryn:quest-reward/l6-paradox-")
                })
            {
                callback.storage.truncate(1);
                let transition = super::reward_callbacks::primary_transition(&callback)?;
                transitions.insert(transition.key.clone(), transition);
            }
            super::definition_hash(quest, &tracks, &transitions)
        };
        predecessors.insert(
            quest.into(),
            super::NpcDefinitionPredecessor {
                hash,
                defaults: BTreeMap::new(),
            },
        );
    }
    Ok(predecessors)
}

impl QuestStateCatalogue {
    pub fn completion_tracks(&self, quest: &str) -> impl Iterator<Item = &str> {
        self.completion_sets
            .get(quest)
            .into_iter()
            .flat_map(|rule| {
                rule.groups
                    .iter()
                    .flatten()
                    .chain(
                        rule.finish
                            .iter()
                            .flat_map(|finish| finish.minimum_tracks.keys()),
                    )
                    .map(String::as_str)
            })
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
                finish: None,
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
    fn every_declared_set_requires_all_groups_in_any_order() -> Result<(), String> {
        let lowered = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        let catalogue = lowered.catalogue();
        // Devil Helmet and Dragon Corpse add two source claim sets.
        assert_eq!(catalogue.completion_sets.len(), 28);
        for (quest, rule) in &catalogue.completion_sets {
            if quest == "oteryn:quest.fanfare_quest" {
                // Crystal chest.lua:1680-1687: the access key is not final loot.
                assert_eq!(
                    rule.groups,
                    vec![vec!["oteryn:quest-progress/kv/fanfare/claimed".to_owned()]]
                );
            } else if quest == "oteryn:quest.sanctuary_of_the_lizard_god" {
                // TibiaWiki: the Legion Helmet chest is the quest's only reward.
                assert_eq!(
                    rule.groups,
                    vec![vec![
                        "oteryn:quest-progress/kv/legiondawnport/claimed".to_owned()
                    ]]
                );
            } else if quest == "oteryn:quest.dragon_corpse_quest" {
                assert!(rule.mint_only);
                assert_eq!(
                    rule.groups,
                    vec![vec![
                        "oteryn:quest-progress/kv/dragoncorpse/claimed".to_owned()
                    ]]
                );
            } else {
                assert!(
                    rule.groups.len() >= 2,
                    "{quest}: independent treasure claims"
                );
            }
            // Exercise every group as the first and the last claim. Each group is
            // independently necessary.
            for shift in 0..rule.groups.len() {
                for reverse in [false, true] {
                    let mut stored = BTreeMap::new();
                    for index in 0..rule.groups.len() {
                        let position = if reverse {
                            (shift + rule.groups.len() - index) % rule.groups.len()
                        } else {
                            (shift + index) % rule.groups.len()
                        };
                        let track = &rule.groups[position][0];
                        let marker = track
                            .strip_prefix("oteryn:quest-progress/")
                            .and_then(|key| key.strip_suffix("/claimed"))
                            .ok_or("claim track")?;
                        let key = format!("oteryn:quest-transition/{marker}/chest");
                        let transition = catalogue.transition(&key).ok_or("claim transition")?;
                        assert_eq!(&transition.quest, quest);
                        let changes = catalogue
                            .evaluate(transition, &stored, 0)
                            .map_err(|e| format!("{e:?}"))?;
                        assert_eq!(
                            catalogue.completes_after_claim_sets(transition, &stored, &changes),
                            rule.finish.is_none() && index + 1 == rule.groups.len(),
                            "{quest}: {key}"
                        );
                        for change in changes {
                            stored.insert(change.track, change.after);
                        }
                    }
                    if let Some(finish) = &rule.finish {
                        let transition =
                            catalogue.transition(&finish.transition).ok_or("finish")?;
                        assert!(!catalogue.completes_after_claim_sets(transition, &stored, &[]));
                        stored.extend(finish.minimum_tracks.clone());
                        assert!(catalogue.completes_after_claim_sets(transition, &stored, &[]));
                    }
                }
            }
        }
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
mod journey_predecessor_tests {
    use crate::quest::{authored_bindings::Bindings, loader, npc_rules, reward_callbacks};
    use std::collections::BTreeMap;

    #[test]
    fn cael_and_eleonore_predecessors_resume_the_authored_routes() -> Result<(), String> {
        let empty = Bindings::parse_documents(&[])?;
        let previous = loader::load_embedded_quest_state_with_bindings("content-1", &empty)
            .map_err(|error| format!("{error:?}"))?;
        let active =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let active = active.catalogue();
        for (quest, transition, saved) in [
            (
                "oteryn:quest.the_new_frontier_quest",
                "oteryn:quest-reward/l2-cael-tome-2",
                "oteryn:quest-progress/quest/u8_54/the_new_frontier/tomeof_knowledge",
            ),
            (
                "oteryn:quest.the_shattered_isles_quest",
                "oteryn:quest-callback/authored-shattered-isles-ring",
                "oteryn:quest-progress/quest/u7_8/the_shattered_isles/the_governor_daughter",
            ),
        ] {
            let pin = previous.catalogue().definition_hash(quest).ok_or(quest)?;
            assert_ne!(active.definition_hash(quest), Some(pin));
            let mut progress = npc_rules::legacy_defaults(active, quest, &pin)
                .ok_or_else(|| format!("published pin for {quest}"))?
                .clone();
            progress.insert(saved.to_owned(), 1);
            let transition = active.transition(transition).ok_or(transition)?;
            let changes = active
                .evaluate(transition, &progress, 0)
                .map_err(|error| format!("{quest}: {error:?}"))?;
            assert!(
                changes
                    .iter()
                    .any(|change| change.track == saved && change.after == 2)
            );
            assert!(npc_rules::legacy_defaults(active, quest, &[0; 32]).is_none());
        }
        Ok(())
    }

    #[test]
    fn ape_city_restores_amphorae_and_refreshes_an_expired_window() -> Result<(), String> {
        let active =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let active = active.catalogue();
        let prefix = "oteryn:quest-progress/chosen/ape-city/";
        let window = format!("{prefix}amphora-window");
        for number in 1..=4 {
            let key = format!("oteryn:quest-callback/authored-ape-city-amphora-{number}");
            let callback = active
                .authored_bindings()
                .ok_or("authored quests")?
                .world_callbacks()
                .find(|callback| callback.key == key)
                .ok_or_else(|| key.clone())?;
            assert_eq!(callback.effects.reset_after_ms, Some(300_000));
            assert_eq!(callback.effects.transform, Some(4996));
            let transition = active.transition(&key).ok_or(key)?;
            let saved = BTreeMap::from([
                (format!("{prefix}stage"), 11),
                (format!("{prefix}amphora-{number}"), 1),
                (window.clone(), 1000),
            ]);
            let updated = active
                .evaluate(transition, &saved, 1301)
                .map_err(|error| format!("retry: {error:?}"))?;
            assert!(
                updated
                    .iter()
                    .any(|change| change.track == window && change.after == 1301)
            );
        }
        Ok(())
    }

    #[test]
    fn koshei_tomb_admits_a_character_with_the_pre_journey_quest_pin() -> Result<(), String> {
        let empty = Bindings::parse_documents(&[])?;
        let previous = loader::load_embedded_quest_state_with_bindings("content-1", &empty)
            .map_err(|error| format!("{error:?}"))?;
        let active =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let quest = "oteryn:quest.koshei_the_deathless_quest";
        let pin = previous
            .catalogue()
            .definition_hash(quest)
            .ok_or("old Koshei pin")?;
        let catalogue = active.catalogue();
        assert_ne!(catalogue.definition_hash(quest), Some(pin));
        assert_eq!(
            npc_rules::legacy_defaults(catalogue, quest, &pin),
            Some(&BTreeMap::new())
        );
        let transition = catalogue
            .transition("oteryn:quest-callback/koshei/tomb-entry")
            .ok_or("tomb entry")?;
        let stored = BTreeMap::from([
            (
                "oteryn:quest-progress/chosen-source/koshei_the_deathless_quest/s1".into(),
                4,
            ),
            ("oteryn:quest-progress/qnpcfx/koshei/amulet".into(), 2),
        ]);
        assert!(catalogue.evaluate(transition, &stored, 0).is_ok());
        Ok(())
    }

    #[test]
    fn riddler_source_program_is_admitted_with_the_authored_paradox_route() -> Result<(), String> {
        let active =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let source = npc_rules::NpcRuleCatalog::runtime().map_err(|error| error.to_string())?;
        assert!(
            source
                .admitted_for_npc("oteryn:npc.riddler", active.catalogue())
                .map_err(|error| error.to_string())?
                .is_some()
        );
        Ok(())
    }

    #[test]
    fn paradox_story_resumes_when_the_saved_character_has_no_hugo_mirror() -> Result<(), String> {
        let active =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let catalogue = active.catalogue();
        let callbacks = reward_callbacks::load()?;
        let story = "oteryn:quest-progress/l6/paradox/story";
        let hugo = "oteryn:quest-progress/quest/u7_24/the_paradox_tower/the_feared_hugo";
        for (npc, stage) in [("zoltan", 1), ("padreia", 2), ("lubo", 3)] {
            let key = format!("oteryn:quest-reward/l6-paradox-{npc}");
            let callback = callbacks
                .iter()
                .find(|callback| callback.key == key)
                .ok_or("story NPC")?;
            let transition = catalogue
                .transition(&callback.key)
                .ok_or("story transition")?;
            let stored = BTreeMap::from([(story.into(), stage)]);
            let changes = catalogue
                .evaluate(transition, &stored, 0)
                .map_err(|error| format!("{error:?}"))?;
            assert!(
                changes
                    .iter()
                    .any(|change| change.track == hugo && change.after == stage + 1)
            );
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
            finish: None,
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
