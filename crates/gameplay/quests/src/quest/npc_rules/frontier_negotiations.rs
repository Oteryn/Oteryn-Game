//! The source's five private negotiation selections are assigned at mission admission.
//! Crystal 201189f data-global/npc/ongulf.lua:163-195.
use super::{NpcRuleEffect, NpcTrackChange};
use std::collections::BTreeMap;

pub const NPC: &str = "oteryn:npc.ongulf";
pub const RULE: &str = "oteryn:quest-npc/new-frontier/ongulf/start-negotiations";
pub const QUEST: &str = "oteryn:quest.the_new_frontier_quest";
pub const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/questline";
const PREFIX: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/";
const SOURCE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/ongulf.lua:163-195";

pub const KEYWORDS: [(&str, u8); 5] = [
    (
        "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission05/angus_keyword",
        2,
    ),
    (
        "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission05/humgolf_keyword",
        2,
    ),
    (
        "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission05/leeland_keyword",
        2,
    ),
    (
        "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission05/telas_keyword",
        2,
    ),
    (
        "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission05/wyrdin_keyword",
        4,
    ),
];

pub(super) fn program() -> String {
    let mut tracks = vec![track(QUESTLINE, 27)];
    let mut effects = vec![NpcRuleEffect::Set {
        track: QUESTLINE.into(),
        value: 14,
    }];
    for (path, max) in [
        ("mission05/angus", 3),
        ("mission05/humgolf", 3),
        ("mission05/king_tibianus", 3),
        ("mission05/leeland", 3),
        ("mission05/telas", 3),
        ("mission05/wyrdin", 3),
        ("mission05_1", 2),
    ] {
        let key = format!("{PREFIX}{path}");
        tracks.push(track(&key, max));
        effects.push(NpcRuleEffect::Set {
            track: key,
            value: 1,
        });
    }
    for (key, choices) in KEYWORDS {
        tracks.push(track(key, i64::from(choices)));
    }
    effects.extend([
        NpcRuleEffect::AssignFrontierKeywords,
        NpcRuleEffect::SetTopic { value: 0 },
    ]);
    serde_json::json!({
        "schema":1,"quest":QUEST,"tracks":tracks,
        "rules":[{
            "key":RULE,"quest":QUEST,"npc":NPC,"trigger":"say","keywords":["mission"],
            "guards":[{"kind":"storage","track":QUESTLINE,"op":"eq","value":13}],
            "effects":effects,"sources":[SOURCE]
        }]
    })
    .to_string()
}

fn track(key: &str, max: i64) -> serde_json::Value {
    serde_json::json!({"key":key,"quest":QUEST,"initial":0,"min":0,"max":max,"sources":[SOURCE]})
}

/// One uniform database draw supplies six independent bits. Each track consumes
/// only its own bits, so the four binary outcomes and Wyrdin's four-way outcome
/// are independent. The writer persists these values in the same mission receipt.
pub fn assign_keywords(
    mut bits: u8,
    tracks: &BTreeMap<String, i64>,
) -> impl Iterator<Item = NpcTrackChange> + '_ {
    KEYWORDS.into_iter().map(move |(key, choices)| {
        let after = 1 + i64::from(bits & (choices - 1));
        bits >>= choices.trailing_zeros();
        NpcTrackChange {
            key: key.into(),
            before: tracks.get(key).copied().unwrap_or_default(),
            after,
        }
    })
}

/// Existing characters may already have a Frontier state pinned before tree
/// USE and negotiation admission were registered. Preserve that exact pin.
pub(super) fn pre_tree_definition(
    catalogue: &crate::quest::QuestStateCatalogue,
) -> Result<Option<crate::quest::NpcDefinitionPredecessor>, String> {
    use crate::quest::{NpcDefinitionPredecessor, QuestStateCatalogue, frontier_tree};
    let mut removed_tree = false;
    let transitions = catalogue
        .transitions
        .values()
        .filter(|transition| transition.quest == QUEST)
        .filter(|transition| {
            let tree = frontier_tree::for_callback(&transition.key).is_some();
            removed_tree |= tree;
            !tree
        })
        .cloned()
        .collect();
    if !removed_tree {
        return Ok(None);
    }
    let tracks = catalogue
        .tracks
        .values()
        .filter(|track| track.quest == QUEST)
        .cloned()
        .collect();
    let previous = QuestStateCatalogue::new(catalogue.content_revision(), tracks, transitions)
        .map_err(|error| format!("{error:?}"))?;
    let hash = previous
        .definition_hash(QUEST)
        .ok_or("missing preceding Frontier definition")?;
    Ok(Some(NpcDefinitionPredecessor {
        hash,
        defaults: BTreeMap::new(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn frontier_keyword_draws_cover_every_independent_combination() {
        let previous = BTreeMap::from([(KEYWORDS[0].0.to_owned(), 2)]);
        let combinations = (0..64)
            .map(|entropy| {
                assign_keywords(entropy, &previous)
                    .zip(KEYWORDS)
                    .map(|(change, (key, options))| {
                        assert_eq!(change.key, key);
                        assert_eq!(change.before, previous.get(key).copied().unwrap_or(0));
                        assert!((1..=i64::from(options)).contains(&change.after));
                        change.after
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(combinations.len(), 64);
    }
}
