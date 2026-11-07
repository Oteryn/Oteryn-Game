//! Crystal 201189f Threatened Dreams conversation decisions, consumed by the NPC caller.
use serde::Deserialize;
use std::collections::BTreeMap;

pub const ALKESTIOS: &str = "oteryn:npc.alkestios";
pub const ANIMALS: &str =
    "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission01/troubled_animals";
pub const QUESTLINE: &str = "oteryn:quest-progress/quest/u11_40/threatened_dreams/quest_line";
pub const NPCS: &[&str] = &[
    ALKESTIOS,
    "oteryn:npc.ikassis",
    "oteryn:npc.ghostly_wolf",
    "oteryn:npc.a_swan",
    "oteryn:npc.chief_grarkharok",
    "oteryn:npc.aurita",
    "oteryn:npc.taegen",
    "oteryn:npc.valindara",
    "oteryn:npc.tired_tree",
    "oteryn:npc.grumpy_stone",
    "oteryn:npc.maelyrra",
    "oteryn:npc.dancing_fairy",
    "oteryn:npc.tooth_fairy",
];
const BASE: &str = "oteryn:quest-progress/quest/u11_40/threatened_dreams/";
const COUPLE: &str = "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission03_1";
const COUPLE_PROGRESS: &str =
    "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission03/unlikely_couple";
pub const NIGHTMARE: &str = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/nightmare_intruders";
const FAIRIES: &str =
    "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/fairies_counter";
const MOON_MIRROR: &str =
    "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/dark_moon_mirror";
pub const KILL_REPORT: &str = "oteryn:quest-transition/crystalserver/quest/u11_40/threatened_dreams/mission02/nightmare_intruders/npc_2";
const ENFEEBLED: &str =
    "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/enfeebled_count";
const FRAZZLEMAWS: &str = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/frazzlemaws_count";
const KROAZUR: &str =
    "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/kroazur_kill";

// Only new source declarations use -1. Historical qualified initial values are
// preserved; no stored zero is migrated. Crystal player.cpp:1341-1349.
const SOURCE_UNSET_TRACKS: &[&str] = &[
    COUPLE_PROGRESS,
    MOON_MIRROR,
    "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission01/poacher_chest",
    "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/stone1",
    "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/stone2",
    "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/stone3",
    "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/stone4",
    "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/stone5",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Topic {
    pub npc: &'static str,
    pub step: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub topic: Option<Topic>,
    pub branches: Vec<u16>,
    pub transition: Option<&'static str>,
}

#[must_use]
pub fn choose(
    npc: &str,
    text: &str,
    topic: Option<Topic>,
    tracks: &BTreeMap<String, i64>,
) -> Option<Choice> {
    let npc = *NPCS.iter().find(|key| **key == npc)?;
    // Source npc_handler.lua:446-451 consumes registered KeywordHandler/Focus
    // messages before this default callback. Captured lore remains source-only:
    // interception must preserve the old topic and cannot manufacture a writer.
    if keyword_handler_intercepts(npc, text)? || super::quests_2_focus::intercepts(text) {
        return None;
    }
    let topic = topic.filter(|current| current.npc == npc);
    let value = |key: &str| {
        tracks.get(key).copied().unwrap_or_else(|| {
            if SOURCE_UNSET_TRACKS.contains(&key) {
                -1
            } else {
                0
            }
        })
    };
    let animals = value(ANIMALS);
    let couple = value(COUPLE);
    let at = |step| topic == Some(Topic { npc, step });
    let answer = |line, step, transition| Choice {
        topic: (step != 0).then_some(Topic { npc, step }),
        branches: vec![line],
        transition,
    };
    match npc {
        ALKESTIOS => {
            // npc/alkestios.lua:61-111, including source elseif precedence.
            if contains(text, "mission") {
                return match animals {
                    1 if value(&format!("{BASE}mission01/poacher_chest")) == 1 => {
                        Some(answer(63, topic.map_or(0, |t| t.step), None))
                    }
                    2 => Some(answer(65, topic.map_or(0, |t| t.step), None)),
                    3 => Some(answer(67, 2, None)),
                    15 => Some(answer(
                        73,
                        topic.map_or(0, |t| t.step),
                        Some("oteryn:quest-transition/threatened-dreams/alkestios-access"),
                    )),
                    _ => None,
                };
            }
            if contains(text, "help") && animals < 1 {
                return Some(answer(80, 1, None));
            }
            if contains(text, "yes") {
                if at(1) {
                    return Some(if value(QUESTLINE) == 1 {
                        answer(88, 0, None)
                    } else {
                        answer(
                            91,
                            0,
                            Some("oteryn:quest-transition/threatened-dreams/alkestios-start"),
                        )
                    });
                }
                if at(2) {
                    return Some(answer(
                        101,
                        0,
                        Some("oteryn:quest-transition/threatened-dreams/alkestios-real-animals"),
                    ));
                }
            } else if contains(text, "no") {
                return Some(answer(109, 0, None));
            }
        }
        "oteryn:npc.ikassis" => {
            // npc/ikassis.lua:61-89. Topic1 yes also grants a talisman; the
            // inventory owner handles it, never this state-only path.
            if contains(text, "mission") {
                return Some(match animals {
                    4 => answer(63, 1, None),
                    10 => answer(
                        70,
                        0,
                        Some("oteryn:quest-transition/threatened-dreams/ikassis-swan"),
                    ),
                    _ => answer(74, 0, None),
                });
            }
            if !contains(text, "yes") && contains(text, "no") {
                return Some(answer(88, 0, None));
            }
        }
        "oteryn:npc.ghostly_wolf" => {
            // npc/ghostly_wolf.lua:66-95.
            if contains(text, "mission") {
                return Some(match animals {
                    5 => answer(68, 1, None),
                    9 => answer(
                        74,
                        0,
                        Some("oteryn:quest-transition/threatened-dreams/wolf-peace"),
                    ),
                    10.. => answer(78, 0, None),
                    _ => answer(81, 0, None),
                });
            }
            if contains(text, "yes") && at(1) {
                return Some(answer(
                    86,
                    0,
                    Some("oteryn:quest-transition/threatened-dreams/wolf-search"),
                ));
            }
            if !contains(text, "yes") && contains(text, "no") {
                return Some(answer(94, 0, None));
            }
        }
        "oteryn:npc.a_swan" => {
            // npc/a_swan.lua:56-88. Stage14 requires five feathers atomically.
            if contains(text, "mission") {
                return match animals {
                    11 => Some(answer(58, 1, None)),
                    14 => None,
                    _ => Some(answer(78, 0, None)),
                };
            }
            if contains(text, "yes") && at(1) {
                return Some(answer(
                    83,
                    0,
                    Some("oteryn:quest-transition/threatened-dreams/swan-search"),
                ));
            }
        }
        "oteryn:npc.chief_grarkharok" => {
            // npc/chief_grarkharok.lua:66-72; the Troll Sabotage branches have
            // their own owner and are never claimed by this selector.
            if ["cloak", "feather", "swan", "maiden"]
                .iter()
                .any(|word| contains(text, word))
            {
                return Some(if animals == 12 {
                    answer(68, topic.map_or(0, |t| t.step), None)
                } else {
                    answer(70, 0, None)
                });
            }
        }
        "oteryn:npc.aurita" => {
            // npc/aurita.lua:56-108.
            if contains(text, "mission") {
                return Some(match couple {
                    ..1 => answer(58, 1, None),
                    1 => answer(65, 0, None),
                    2..=3 => answer(70, 0, None),
                    _ => answer(75, 0, None),
                });
            }
            if contains(text, "starlight vial") || contains(text, "empty starlight vial") {
                return (couple == 4 && value("oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission03/empty_starlight_vial") >= 1)
                    .then(||answer(91, topic.map_or(0,|t|t.step), None));
            }
            if contains(text, "yes") && at(1) {
                return Some(answer(
                    98,
                    0,
                    Some("oteryn:quest-transition/threatened-dreams/aurita-start"),
                ));
            }
            if !contains(text, "yes") && contains(text, "no") {
                return Some(answer(107, 0, None));
            }
        }
        "oteryn:npc.taegen" => {
            // npc/taegen.lua:108-198. Stage2 and topic2/3 require Item facts.
            if contains(text, "mission") {
                return match couple {
                    ..1 => Some(answer(110, 1, None)),
                    1 => Some(answer(117, 0, None)),
                    2 => None,
                    _ => Some(answer(131, 0, None)),
                };
            }
            if contains(text, "sun catcher") {
                return match couple {
                    3 => Some(answer(138, 2, None)),
                    4 => Some(answer(143, 3, None)),
                    _ => None,
                };
            }
            if contains(text, "yes") && at(1) {
                return Some(answer(
                    151,
                    0,
                    Some("oteryn:quest-transition/threatened-dreams/taegen-start"),
                ));
            }
            if !contains(text, "yes") && contains(text, "no") {
                return Some(answer(196, 0, None));
            }
        }
        "oteryn:npc.valindara" => {
            // npc/valindara.lua:74-94; source intentionally re-offers at stage4.
            if contains(text, "cloak") || contains(text, "swan") {
                return match couple {
                    ..4 => Some(answer(76, topic.map_or(0, |t| t.step), None)),
                    4 => Some(answer(
                        79,
                        1,
                        Some("oteryn:quest-transition/threatened-dreams/cloak-offer"),
                    )),
                    _ => None,
                };
            }
        }
        "oteryn:npc.tired_tree" => {
            // npc/tired_tree.lua:64-87. Story conclusion also mints map24944.
            let tired = value(&format!("{BASE}mission04/tired_tree"));
            if contains(text, "mission") {
                if tired < 1 && value(&format!("{BASE}mission04/tooth_fairy")) == 2 {
                    return Some(answer(66, 1, None));
                }
                if tired == 1 {
                    return Some(answer(70, 2, None));
                }
                return None;
            }
            if contains(text, "yes") {
                if at(1) {
                    return Some(answer(
                        74,
                        0,
                        Some("oteryn:quest-transition/threatened-dreams/tree-start"),
                    ));
                }
                if at(2) {
                    return Some(answer(78, 3, None));
                }
            }
        }
        "oteryn:npc.grumpy_stone" => {
            // npc/grumpy_stone.lua:64-97. The five-stone completion also mints24945.
            let stage = value(&format!("{BASE}mission04/grumpy_stone"));
            if contains(text, "mission") {
                if stage < 1 && value(&format!("{BASE}mission04/tired_tree")) == 2 {
                    return Some(answer(67, 1, None));
                }
                if stage == 1 && (1..=5).any(|i| value(&format!("{BASE}mission04/stone{i}")) != 1) {
                    return Some(answer(91, topic.map_or(0, |t| t.step), None));
                }
                return None;
            }
            if contains(text, "yes") && at(1) {
                return Some(answer(
                    95,
                    0,
                    Some("oteryn:quest-transition/threatened-dreams/stone-start"),
                ));
            }
        }
        "oteryn:npc.maelyrra" => {
            // Crystal npc/maelyrra.lua:61-170. The report's sum and boss gates
            // are rechecked against the writer's complete locked snapshot.
            let nightmare = value(NIGHTMARE);
            if contains(text, "mission") {
                return Some(match nightmare {
                    ..1 => answer(63, 1, None),
                    1..=2 => answer(66, 2, None),
                    3..=4 => answer(69, 4, None),
                    5 if couple == 4 => answer(72, 6, None),
                    6 => answer(75, 7, None),
                    7 => answer(78, 8, None),
                    _ => answer(81, 0, None),
                });
            }
            if contains(text, "yes") {
                if at(1) {
                    return Some(answer(
                        86,
                        0,
                        Some("oteryn:quest-transition/threatened-dreams/maelyrra-start"),
                    ));
                }
                if at(2) {
                    if nightmare == 2 {
                        return Some(answer(102, 3, None));
                    }
                    let enfeebled = value(
                        "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/enfeebled_count",
                    );
                    let frazzlemaws = value(
                        "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/frazzlemaws_count",
                    );
                    let kroazur = value(
                        "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/kroazur_kill",
                    );
                    if nightmare == 1
                        && kroazur >= 1
                        && i128::from(enfeebled) + i128::from(frazzlemaws) >= 200
                    {
                        return Some(answer(98, 3, Some(KILL_REPORT)));
                    }
                    return Some(answer(105, 0, None));
                }
                if at(3) {
                    return Some(answer(
                        109,
                        0,
                        Some("oteryn:quest-transition/threatened-dreams/maelyrra-fairies"),
                    ));
                }
                if at(4) {
                    if value(FAIRIES) == 5 && value(MOON_MIRROR) == 1 {
                        return Some(answer(
                            119,
                            5,
                            Some(
                                "oteryn:quest-transition/threatened-dreams/maelyrra-fairies-report",
                            ),
                        ));
                    }
                    return Some(if nightmare == 4 {
                        answer(123, 5, None)
                    } else {
                        answer(126, 0, None)
                    });
                }
                if at(5) {
                    return Some(answer(
                        130,
                        0,
                        Some("oteryn:quest-transition/threatened-dreams/maelyrra-vessels"),
                    ));
                }
                if at(6) {
                    return Some(answer(
                        137,
                        0,
                        Some("oteryn:quest-transition/threatened-dreams/maelyrra-gather-lights"),
                    ));
                }
                // Topics7/8 require current held items or an atomic bag mint.
            } else if contains(text, "no") {
                return Some(answer(169, 0, None));
            }
        }
        "oteryn:npc.dancing_fairy" => {
            // npc/dancing_fairy.lua:56-82. Source topic1 has no writer in this
            // NPC; do not fabricate a route to its otherwise unreachable yes.
            if contains(text, "spell") {
                return Some(if couple == 1 && value(COUPLE_PROGRESS) == 1 {
                    answer(
                        58,
                        0,
                        Some("oteryn:quest-transition/threatened-dreams/fairy-spell"),
                    )
                } else {
                    answer(66, 0, None)
                });
            }
            if !contains(text, "yes") && contains(text, "no") {
                return Some(answer(81, 0, None));
            }
        }
        "oteryn:npc.tooth_fairy" => {
            // npc/tooth_fairy.lua:69-130. The earlier mission block has a level
            // guard and inventory effects. Keep the whole combined input held.
            if contains(text, "mission") {
                return None;
            }
            if ["cause", "tooth", "teeth", "fang"]
                .iter()
                .any(|word| contains(text, word))
                && value(&format!("{BASE}mission04/tooth_fairy")) == 2
            {
                let collection = value(&format!("{BASE}mission04/teeth_collection"));
                if collection < 1 {
                    // Consecutive if statements observe the just-written one,
                    // so both literal replies belong to one atomic operation.
                    return Some(Choice {
                        topic: Some(Topic { npc, step: 2 }),
                        branches: vec![109, 116],
                        transition: Some(
                            "oteryn:quest-transition/threatened-dreams/tooth-collection-offer",
                        ),
                    });
                }
                if collection == 1 {
                    return Some(answer(116, 2, None));
                }
            }
            // yes/topic2 consumes six species and mints a reward; never split.
        }
        _ => return None,
    }
    None
}

/// Crystal data/npclib/npc.lua:13-20: an alphanumeric prefix anywhere vetoes
/// the match. The donor does not require a right-hand word boundary.
fn contains(text: &str, keyword: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let mut found = false;
    for (index, _) in lower.match_indices(keyword) {
        if index > 0 && lower.as_bytes()[index - 1].is_ascii_alphanumeric() {
            return false;
        }
        found = true;
    }
    found
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u8,
    source_revision: String,
    npcs: Vec<Npc>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Npc {
    npc: String,
    source: String,
    intercepted_keywords: Vec<InterceptedKeyword>,
    branches: Vec<Branch>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InterceptedKeyword {
    line: u16,
    keywords: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Branch {
    line: u16,
    reply: Vec<String>,
}

fn dialogue() -> Option<&'static Document> {
    static CONTENT: std::sync::LazyLock<Option<Document>> = std::sync::LazyLock::new(|| {
        let data: Document = serde_json::from_str(include_str!(
            "../../../../../content/dialogues/quests-2/threatened-dreams.json"
        ))
        .ok()?;
        if data.schema != 1
            || data.source_revision != "201189f871d23597b82caf88fd611454a2ddefb8"
            || data.npcs.len() > 20
        {
            return None;
        }
        let mut keys = std::collections::BTreeSet::new();
        for npc in &data.npcs {
            if !keys.insert(&npc.npc) || !npc.source.starts_with("zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/") || npc.branches.len() > 256 { return None; }
            let mut intercepted_lines = std::collections::BTreeSet::new();
            if npc.intercepted_keywords.len() > 128
                || npc.intercepted_keywords.iter().any(|entry| {
                    entry.line == 0
                        || !intercepted_lines.insert(entry.line)
                        || entry.keywords.is_empty()
                        || entry.keywords.len() > 3
                        || entry.keywords.iter().any(|word| {
                            word.is_empty()
                                || word.len() > 64
                                || !word.bytes().all(|byte| {
                                    byte.is_ascii_lowercase() || matches!(byte, b' ' | b'\'')
                                })
                        })
                })
            {
                return None;
            }
            let mut lines = std::collections::BTreeSet::new();
            for branch in &npc.branches {
                if !lines.insert(branch.line)
                    || branch.reply.is_empty()
                    || branch.reply.len() > 32
                    || branch
                        .reply
                        .iter()
                        .any(|text| text.is_empty() || text.len() > 2048 || text.contains('\0'))
                {
                    return None;
                }
            }
        }
        Some(data)
    });
    CONTENT.as_ref()
}

/// Crystal keyword_handler.lua:55-68,178-189: every literal is searched from
/// the beginning, and their first matches must be nondecreasing. There is no
/// word boundary. The generator rejects patterns, conditions and side effects.
fn keyword_handler_intercepts(npc: &str, text: &str) -> Option<bool> {
    let entry = dialogue()?.npcs.iter().find(|entry| entry.npc == npc)?;
    let message = text.to_ascii_lowercase();
    Some(entry.intercepted_keywords.iter().any(|entry| {
        let mut previous = 0;
        entry.keywords.iter().all(|keyword| {
            let Some(start) = message.find(keyword.as_str()) else {
                return false;
            };
            if start < previous {
                return false;
            }
            previous = start;
            true
        })
    }))
}

#[must_use]
pub fn reply(npc: &str, line: u16) -> Option<&'static [String]> {
    dialogue()?
        .npcs
        .iter()
        .find(|n| n.npc == npc)?
        .branches
        .iter()
        .find(|b| b.line == line)
        .map(|b| b.reply.as_slice())
}

pub(super) fn extend(
    catalogue: super::QuestStateCatalogue,
) -> Result<super::QuestStateCatalogue, String> {
    let overlay = super::loader::parse_quest_state(
        include_str!("../../../../../content/quests/threatened-dreams-npc.json"),
        catalogue.content_revision(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let mut tracks = catalogue.tracks.clone();
    let mut transitions = catalogue.transitions.clone();
    for (key, track) in &overlay.catalogue().tracks {
        if tracks.get(key).is_some_and(|existing| existing != track) {
            return Err(format!("Threatened Dreams track conflict {key}"));
        }
        tracks.insert(key.clone(), track.clone());
    }
    for (key, transition) in &overlay.catalogue().transitions {
        if transitions
            .insert(key.clone(), transition.clone())
            .is_some()
        {
            return Err(format!("Threatened Dreams transition conflict {key}"));
        }
    }
    // Crystal 201189f, npc/maelyrra.lua:94-100; identical source blob
    // 16acd67b566785af18a6a5cfce6d03a7a41a9f66717596774a3c309eca0eea88
    // retained in donor-source/refinements/dialogue.json.gz, source_records/492.
    // Replace the inexact scalar lowering with the complete guarded operation.
    // Guard inputs are effects too: the existing writer locks and receipts all
    // four together, while preserving the three observed kill values.
    use super::{QuestComparison as C, QuestEffect, QuestEffectKind as E};
    let report = transitions
        .get_mut(KILL_REPORT)
        .ok_or("missing source kill report")?;
    report.effects = vec![
        QuestEffect {
            track: NIGHTMARE.into(),
            from: C::Eq(1),
            effect: E::Set(2),
        },
        QuestEffect {
            track: ENFEEBLED.into(),
            from: C::Ge(0),
            effect: E::RequireSumAtLeast {
                other_effect: 2,
                minimum: 200,
            },
        },
        QuestEffect {
            track: FRAZZLEMAWS.into(),
            from: C::Ge(0),
            effect: E::Add(0),
        },
        QuestEffect {
            track: KROAZUR.into(),
            from: C::Ge(1),
            effect: E::Add(0),
        },
    ];
    super::QuestStateCatalogue::new(
        catalogue.content_revision,
        tracks.into_values().collect(),
        transitions.into_values().collect(),
    )
    .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threatened_keyword_handler_lore_cannot_open_or_confirm_a_mission() -> Result<(), String> {
        // Crystal201189f npc_handler.lua:446-451 runs KeywordHandler before
        // CALLBACK_MESSAGE_DEFAULT; alkestios.lua:115 consumes any "deer" match.
        let empty = BTreeMap::new();
        for message in ["help deer", "help xdeer", "help DeEr"] {
            if let Some(choice) = choose(ALKESTIOS, message, None, &empty) {
                assert!(
                    choice.transition.is_none(),
                    "static lore cannot write state"
                );
                assert!(
                    choice.topic.is_none(),
                    "static lore cannot open the help topic"
                );
            }
        }
        let offer = choose(ALKESTIOS, "help", None, &empty).ok_or("source help offer")?;
        for message in ["yes deer", "yes xdeer", "yes DeEr"] {
            if let Some(choice) = choose(ALKESTIOS, message, offer.topic, &empty) {
                assert!(
                    choice.transition.is_none(),
                    "static lore cannot confirm the mission"
                );
                assert_eq!(
                    choice.topic, offer.topic,
                    "StdModule.say preserves the topic"
                );
            }
        }
        assert!(
            choose(ALKESTIOS, "yes", offer.topic, &empty)
                .ok_or("plain confirmation")?
                .transition
                .is_some()
        );
        Ok(())
    }

    #[test]
    fn threatened_keyword_handler_other_source_npcs_preserve_their_topics() {
        // grumpy_stone.lua:102/109, tired_tree.lua:98, tooth_fairy.lua:138/140.
        // Multi-keyword registrations require every literal in source order.
        for (npc, message, prerequisite) in [
            (
                "oteryn:npc.grumpy_stone",
                "mission name",
                "mission04/tired_tree",
            ),
            (
                "oteryn:npc.grumpy_stone",
                "mission thais carlin venore",
                "mission04/tired_tree",
            ),
            (
                "oteryn:npc.tired_tree",
                "mission job",
                "mission04/tooth_fairy",
            ),
            (
                "oteryn:npc.tooth_fairy",
                "cause tooth fairy",
                "mission04/tooth_fairy",
            ),
            (
                "oteryn:npc.tooth_fairy",
                "cause fae siblings",
                "mission04/tooth_fairy",
            ),
        ] {
            let tracks = BTreeMap::from([(format!("{BASE}{prerequisite}"), 2)]);
            if let Some(choice) = choose(npc, message, None, &tracks) {
                assert!(
                    choice.transition.is_none(),
                    "intercepted source input: {npc}: {message}"
                );
                assert!(
                    choice.topic.is_none(),
                    "intercepted source input: {npc}: {message}"
                );
            }
        }
    }

    #[test]
    fn threatened_keyword_handler_focus_words_cannot_confirm_the_help_topic() -> Result<(), String>
    {
        // Crystal201189f modules.lua:5-8,286-310,361-387: the source FocusModule
        // returns true before default callback handling, including a regreeting.
        let empty = BTreeMap::new();
        let offer = choose(ALKESTIOS, "help", None, &empty).ok_or("source help offer")?;
        for message in ["yes hi", "yes hello", "yes trade", "yes offers"] {
            if let Some(choice) = choose(ALKESTIOS, message, offer.topic, &empty) {
                assert!(
                    choice.transition.is_none(),
                    "FocusModule must intercept {message}"
                );
                assert_eq!(
                    choice.topic, offer.topic,
                    "no default callback topic change"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn threatened_keyword_handler_lists_are_ordered_and_not_alternative_words() -> Result<(), String>
    {
        let stone = "oteryn:npc.grumpy_stone";
        let tracks = BTreeMap::from([(format!("{BASE}mission04/tired_tree"), 2)]);
        assert!(choose(stone, "mission thais carlin venore", None, &tracks).is_none());
        for message in ["mission thais", "mission venore carlin thais"] {
            assert_eq!(
                choose(stone, message, None, &tracks)
                    .ok_or("unintercepted mission")?
                    .topic,
                Some(Topic {
                    npc: stone,
                    step: 1
                }),
            );
        }
        assert_eq!(
            keyword_handler_intercepts("oteryn:npc.chief_grarkharok", "cloak money"),
            Some(true),
            "a source alias inherits the earlier plain-speech callback",
        );
        Ok(())
    }

    #[test]
    fn threatened_animals_start_requires_exact_npc_topic_and_source_precedence()
    -> Result<(), String> {
        let empty = BTreeMap::new();
        assert!(choose(ALKESTIOS, "yes", None, &empty).is_none());
        let offer = choose(ALKESTIOS, "help", None, &empty).ok_or("source help offer")?;
        assert_eq!(
            offer.topic,
            Some(Topic {
                npc: ALKESTIOS,
                step: 1
            })
        );
        assert_eq!(offer.branches, [80]);
        let accepted = choose(ALKESTIOS, "yes", offer.topic, &empty).ok_or("source acceptance")?;
        assert_eq!(
            accepted.transition,
            Some("oteryn:quest-transition/threatened-dreams/alkestios-start")
        );
        assert!(choose("oteryn:npc.ikassis", "yes", offer.topic, &empty).is_none());
        // Source `mission` is the earlier elseif, even when help occurs too.
        assert!(choose(ALKESTIOS, "mission help", None, &empty).is_none());
        let begun = BTreeMap::from([(QUESTLINE.into(), 1)]);
        let repeated = choose(ALKESTIOS, "yes", offer.topic, &begun).ok_or("already begun")?;
        assert!(repeated.transition.is_none());
        assert_eq!(repeated.branches, [88]);
        Ok(())
    }

    #[test]
    fn threatened_completed_wolf_is_idempotent_and_missing_inventory_is_not_success()
    -> Result<(), String> {
        let wolf = "oteryn:npc.ghostly_wolf";
        let at_nine = BTreeMap::from([(ANIMALS.into(), 9)]);
        assert_eq!(
            choose(wolf, "mission", None, &at_nine)
                .ok_or("wolf completion")?
                .transition,
            Some("oteryn:quest-transition/threatened-dreams/wolf-peace")
        );
        let at_ten = BTreeMap::from([(ANIMALS.into(), 10)]);
        assert!(
            choose(wolf, "mission", None, &at_ten)
                .ok_or("wolf acknowledgement")?
                .transition
                .is_none()
        );
        let swan = BTreeMap::from([(ANIMALS.into(), 14)]);
        assert!(
            choose("oteryn:npc.a_swan", "mission", None, &swan).is_none(),
            "the five-feather hand-in must reach the inventory owner, not a state-only grant"
        );
        Ok(())
    }

    #[test]
    fn threatened_expansion_maelyrra_source_topic_and_atomic_start() -> Result<(), String> {
        let npc = "oteryn:npc.maelyrra";
        let empty = BTreeMap::new();
        assert!(choose(npc, "yes", None, &empty).is_none());
        let offered = choose(npc, "mission", None, &empty).ok_or("source mission offer")?;
        assert_eq!(offered.branches, [63]);
        let accepted = choose(npc, "yes", offered.topic, &empty).ok_or("source accepted start")?;
        assert_eq!(
            accepted.transition,
            Some("oteryn:quest-transition/threatened-dreams/maelyrra-start")
        );
        assert_eq!(accepted.branches, [86]);
        let data = super::super::loader::parse_quest_state(
            include_str!("../../../../../content/quests/threatened-dreams-npc.json"),
            "threatened-test",
        )
        .map_err(|e| format!("{e:?}"))?;
        let catalogue = data.catalogue();
        let rule = catalogue
            .transition(accepted.transition.ok_or("transition")?)
            .ok_or("source start")?;
        let changes = catalogue.evaluate(rule, &BTreeMap::from([
            ("oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/enfeebled_count".into(), 123),
            ("oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/frazzlemaws_count".into(), 77),
        ]), 0).map_err(|e|format!("{e:?}"))?;
        assert_eq!(
            changes.len(),
            6,
            "all five source writes and the legacy journal mirror are atomic"
        );
        for change in &changes {
            if change.track.ends_with("_count") {
                assert_eq!(change.after, 0);
            }
        }
        assert!(
            changes
                .iter()
                .any(|c| c.track.ends_with("/kroazur_access") && c.after == 1)
        );
        assert!(changes.iter().any(|c| c.track == QUESTLINE && c.after == 2));
        Ok(())
    }

    #[test]
    fn threatened_expansion_maelyrra_each_state_only_followup_keeps_source_guards()
    -> Result<(), String> {
        let npc = "oteryn:npc.maelyrra";
        let nightmare = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/nightmare_intruders";
        let data = super::super::loader::parse_quest_state(
            include_str!("../../../../../content/quests/threatened-dreams-npc.json"),
            "threatened-test",
        )
        .map_err(|e| format!("{e:?}"))?;
        for (topic, line, key, value) in [
            (3, 109, "maelyrra-fairies", 3),
            (5, 130, "maelyrra-vessels", 5),
            (6, 137, "maelyrra-gather-lights", 6),
        ] {
            let answer = choose(
                npc,
                "yes",
                Some(Topic { npc, step: topic }),
                &BTreeMap::new(),
            )
            .ok_or("source topic continuation")?;
            assert_eq!(answer.branches, [line]);
            let rule_key = format!("oteryn:quest-transition/threatened-dreams/{key}");
            assert_eq!(answer.transition, Some(rule_key.as_str()));
            let rule = data
                .catalogue()
                .transition(&rule_key)
                .ok_or("source continuation transition")?;
            let changes = data
                .catalogue()
                .evaluate(rule, &BTreeMap::new(), 0)
                .map_err(|e| format!("{e:?}"))?;
            assert!(
                changes
                    .iter()
                    .any(|c| c.track == nightmare && c.after == value)
            );
        }
        let fairies = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/fairies_counter";
        let mirror = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/dark_moon_mirror";
        let tracks = BTreeMap::from([(fairies.into(), 5), (mirror.into(), 1)]);
        let answer = choose(npc, "yes", Some(Topic { npc, step: 4 }), &tracks)
            .ok_or("source fairies report")?;
        assert_eq!(answer.branches, [119]);
        let rule = data
            .catalogue()
            .transition(answer.transition.ok_or("fairies transition")?)
            .ok_or("rule")?;
        assert!(data.catalogue().evaluate(rule, &tracks, 0).is_ok());
        for missing in [fairies, mirror] {
            let mut stale = tracks.clone();
            stale.insert(missing.into(), 0);
            assert!(
                data.catalogue().evaluate(rule, &stale, 0).is_err(),
                "guard {missing} must be checked in the writer"
            );
        }
        Ok(())
    }

    #[test]
    fn threatened_expansion_fairy_spell_and_teeth_offer_keep_two_source_reads() -> Result<(), String>
    {
        let npc = "oteryn:npc.dancing_fairy";
        let progress = "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission03/unlikely_couple";
        let tracks = BTreeMap::from([(COUPLE.into(), 1), (progress.into(), 1)]);
        let selected = choose(npc, "spell yes", None, &tracks).ok_or("source spell priority")?;
        assert_eq!(selected.branches, [58]);
        let data = super::super::loader::parse_quest_state(
            include_str!("../../../../../content/quests/threatened-dreams-npc.json"),
            "threatened-test",
        )
        .map_err(|e| format!("{e:?}"))?;
        let rule = data
            .catalogue()
            .transition(selected.transition.ok_or("spell transition")?)
            .ok_or("spell rule")?;
        assert!(data.catalogue().evaluate(rule, &tracks, 0).is_ok());
        for changed in [COUPLE, progress] {
            let mut stale = tracks.clone();
            stale.insert(changed.into(), 0);
            assert!(data.catalogue().evaluate(rule, &stale, 0).is_err());
        }
        let tooth = "oteryn:npc.tooth_fairy";
        let tooth_flag = format!("{BASE}mission04/tooth_fairy");
        let teeth = format!("{BASE}mission04/teeth_collection");
        let available = BTreeMap::from([(tooth_flag.clone(), 2)]);
        let offer =
            choose(tooth, "cause", None, &available).ok_or("source tooth collection offer")?;
        assert_eq!(
            offer.branches,
            [109, 116],
            "consecutive source if statements both reply after the flag becomes one"
        );
        assert_eq!(
            offer.topic,
            Some(Topic {
                npc: tooth,
                step: 2
            })
        );
        let rule = data
            .catalogue()
            .transition(offer.transition.ok_or("teeth transition")?)
            .ok_or("teeth rule")?;
        assert!(data.catalogue().evaluate(rule, &available, 0).is_ok());
        let stale = BTreeMap::from([(tooth_flag, 1), (teeth, 0)]);
        assert!(data.catalogue().evaluate(rule, &stale, 0).is_err());
        assert!(
            choose(tooth, "mission cause", None, &available).is_none(),
            "the earlier level and mint branch stays deferred as a whole"
        );
        Ok(())
    }

    #[test]
    fn threatened_expansion_reports_kills_only_through_atomic_guards_and_holds_inventory()
    -> Result<(), String> {
        let npc = "oteryn:npc.maelyrra";
        let tracks=BTreeMap::from([
            ("oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/nightmare_intruders".into(),1),
            ("oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/enfeebled_count".into(),150),
            ("oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/frazzlemaws_count".into(),50),
            ("oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/kroazur_kill".into(),1),
        ]);
        let report =
            choose(npc, "yes", Some(Topic { npc, step: 2 }), &tracks).ok_or("kill report")?;
        assert_eq!(report.transition, Some(KILL_REPORT));
        assert_eq!(report.branches, vec![98]);
        assert_eq!(report.topic, Some(Topic { npc, step: 3 }));
        let lowered = super::super::loader::load_embedded_quest_state("sum-report")
            .map_err(|e| format!("{e:?}"))?;
        let rule = lowered
            .catalogue()
            .transition(KILL_REPORT)
            .ok_or("report rule")?;
        assert!(lowered.catalogue().evaluate(rule, &tracks, 0).is_ok());
        for (track, value) in [(ENFEEBLED, 149), (KROAZUR, 0), (NIGHTMARE, 2)] {
            let mut stale = tracks.clone();
            stale.insert(track.into(), value);
            assert_eq!(
                lowered.catalogue().evaluate(rule, &stale, 0),
                Err(super::super::QuestRefusal::StageMismatch)
            );
        }
        for step in [7, 8] {
            assert!(
                choose(npc, "yes", Some(Topic { npc, step }), &tracks).is_none(),
                "no atomic source guard or inventory owner for this completion"
            );
        }
        assert!(
            choose(
                "oteryn:npc.tooth_fairy",
                "yes",
                Some(Topic {
                    npc: "oteryn:npc.tooth_fairy",
                    step: 2
                }),
                &tracks
            )
            .is_none()
        );
        assert!(!contains("yes xyes", "yes"));
        assert!(contains("yesplease", "yes"));
        Ok(())
    }
}
